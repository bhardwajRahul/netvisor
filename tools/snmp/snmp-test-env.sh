#!/bin/bash
set -euo pipefail

# SNMP Test Environment — manages 10 snmpd instances on a Proxmox LXC
# Subnet: 192.168.4.0/22 (hosts at 192.168.7.230–239)
# Usage: tools/snmp/snmp-test-env.sh verify|status|ssh-setup

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SNMPGET="${SNMPGET:-/opt/homebrew/opt/net-snmp/bin/snmpget}"

HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235 192.168.7.236 192.168.7.237 192.168.7.238 192.168.7.239)
VERSIONS=(v2c v2c v2c v2c v2c v2c v1 v3 v2c v2c)
COMMUNITIES=(netdefault netdefault secret42 secret42 public netdefault legacyv1 - netdefault netdefault)
SYSNAMES=("switch-core-01" "switch-access-01" "router-gw-01" "firewall-01" "printer-lobby" "ap-wireless-01" "legacy-switch-01" "secure-switch-01" "switch-exos-01" "switch-voss-01")

# SNMPv3 USM credentials for secure-switch-01 (must match lxc/setup.sh).
V3_USER="${V3_USER:-scanopyv3}"
V3_AUTH_PASS="${V3_AUTH_PASS:-authpass12345}"
V3_PRIV_PASS="${V3_PRIV_PASS:-privpass12345}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

# ap-wireless-01 advertises 172.30.10.1/24 on a `br-` prefixed interface — the
# #663 fixture, where an access point's NAT guest network was misclassified as a
# Docker bridge. It's the only agent serving its own ipAddrTable, which means it
# is also the only one that breaks silently: if snmpd fails to displace its
# built-in IP module, the `pass` directive loses the duplicate registration and
# the agent quietly falls back to reporting only the scanned subnet. Check it
# explicitly so a scan is never run against a fixture that isn't there.
verify_guest_subnet_fixture() {
    local host="${HOSTS[5]}" community="${COMMUNITIES[5]}"
    local if_index="4" guest_ip="172.30.10.1" if_name="br-guest"

    local got_index got_name
    got_index=$("$SNMPGET" -v2c -c "$community" -t 2 -r 1 -Ovq \
        "$host" ".1.3.6.1.2.1.4.20.1.2.${guest_ip}" 2>/dev/null || echo "FAILED")
    got_name=$("$SNMPGET" -v2c -c "$community" -t 2 -r 1 -Ovq \
        "$host" ".1.3.6.1.2.1.31.1.1.1.1.${if_index}" 2>/dev/null | tr -d '"' || echo "FAILED")

    if [ "$(echo "$got_index" | tr -d ' ')" = "$if_index" ] &&
        [ "$(echo "$got_name" | tr -d ' ')" = "$if_name" ]; then
        printf "  ${GREEN}✓${NC} %-18s  %-20s  %s/24 on %s (#663 fixture)\n" \
            "$host" "guest-subnet" "$guest_ip" "$if_name"
        return 0
    fi

    printf "  ${RED}✗${NC} %-18s  %-20s  ipAdEntIfIndex=%s ifName=%s\n" \
        "$host" "guest-subnet" "$got_index" "$got_name"
    printf "      expected ipAdEntIfIndex=%s and ifName=%s\n" "$if_index" "$if_name"
    printf "      check for a duplicate registration:\n"
    printf "      ssh root@%s 'journalctl -u snmpd-ap-wireless-01 | grep -i duplicate'\n" "${HOSTS[0]}"
    return 1
}

cmd_verify() {
    echo "Verifying SNMP test hosts..."
    echo ""
    local all_ok=true
    for i in "${!HOSTS[@]}"; do
        local host="${HOSTS[$i]}"
        local version="${VERSIONS[$i]}"
        local community="${COMMUNITIES[$i]}"
        local expected="${SYSNAMES[$i]}"

        local result detail
        case "$version" in
            v1)
                result=$("$SNMPGET" -v1 -c "$community" -t 2 -r 1 "$host" sysName.0 2>/dev/null | sed 's/.*= STRING: //' || echo "FAILED")
                detail="v1 community=$community"
                ;;
            v3)
                result=$("$SNMPGET" -v3 -l authPriv -u "$V3_USER" -a SHA-256 -A "$V3_AUTH_PASS" -x AES -X "$V3_PRIV_PASS" -t 2 -r 1 "$host" sysName.0 2>/dev/null | sed 's/.*= STRING: //' || echo "FAILED")
                detail="v3 user=$V3_USER"
                ;;
            *)
                result=$("$SNMPGET" -v2c -c "$community" -t 2 -r 1 "$host" sysName.0 2>/dev/null | sed 's/.*= STRING: //' || echo "FAILED")
                detail="v2c community=$community"
                ;;
        esac

        if echo "$result" | grep -q "$expected"; then
            printf "  ${GREEN}✓${NC} %-18s  %-20s  %s\n" "$host" "$expected" "$detail"
        else
            printf "  ${RED}✗${NC} %-18s  expected=%-20s  got=%s\n" "$host" "$expected" "$result"
            all_ok=false
        fi
    done

    echo ""
    verify_guest_subnet_fixture || all_ok=false

    echo ""
    if $all_ok; then
        printf "${GREEN}All %d SNMP test hosts are reachable.${NC}\n" "${#HOSTS[@]}"
        echo ""
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        echo "  LXC hosts on 192.168.4.0/22"
        echo ""
        printf "  %-18s %-22s %-6s %s\n" "IP" "Host" "Ver" "Credential"
        printf "  %-18s %-22s %-6s %s\n" "────────────────" "────────────────────" "─────" "────────────"
        for i in "${!HOSTS[@]}"; do
            local cred="${COMMUNITIES[$i]}"
            [ "${VERSIONS[$i]}" = "v3" ] && cred="user=$V3_USER"
            printf "  %-18s %-22s %-6s %s\n" "${HOSTS[$i]}" "${SYSNAMES[$i]}" "${VERSIONS[$i]}" "$cred"
        done
        echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    else
        printf "${YELLOW}Some hosts are unreachable. Is the LXC running?${NC}\n"
        echo "  Check with: ssh root@${HOSTS[0]} 'systemctl list-units snmpd-*'"
    fi
}

cmd_status() {
    echo "SNMP Test Environment Status"
    echo "=============================="
    echo ""
    echo "Checking reachability (ICMP)..."
    for i in "${!HOSTS[@]}"; do
        local host="${HOSTS[$i]}"
        local name="${SYSNAMES[$i]}"
        if ping -c 1 -W 1 "$host" &>/dev/null; then
            printf "  ${GREEN}✓${NC} %-18s  %s\n" "$host" "$name"
        else
            printf "  ${RED}✗${NC} %-18s  %s  (unreachable)\n" "$host" "$name"
        fi
    done
}

case "${1:-}" in
    verify)
        cmd_verify
        ;;
    status)
        cmd_status
        ;;
    *)
        echo "Usage: $0 {verify|status}"
        echo ""
        echo "  verify — Query each SNMP host and check sysName"
        echo "  status — Ping each host to check reachability"
        echo ""
        echo "LXC setup: copy tools/snmp/ to the container and run lxc/setup.sh"
        exit 1
        ;;
esac
