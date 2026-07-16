#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# SNMP Test Environment — Proxmox VM setup (self-contained)
#
# Paste this entire script into a Debian/Ubuntu VM terminal.
# Creates 10 snmpd instances on secondary IPs, each simulating a
# different network device with its own community string.
#
# Edit HOSTS/CIDR/IFACE below to match your network.
# ══════════════════════════════════════════════════════════════════════

HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235 192.168.7.236 192.168.7.237 192.168.7.238 192.168.7.239)
CIDR="22"
IFACE="eth0"

# Per-host SNMP version. Most are v2c (community string); .236/.237 exercise the
# v1-only and v3-only code paths (#557). .238 (EXOS) and .239 (VOSS) exercise the
# LLDP local-port remap (Issue 2, July 2026): EXOS reports lldpRemTable local-port
# numbers in a namespace distinct from ifIndex and needs lldpLocPortTable to
# resolve; VOSS reports local-port == ifIndex. Per-host communities are written
# directly into each snmpd config below.
VERSIONS=(v2c v2c v2c v2c v2c v2c v1 v3 v2c v2c)
SYSNAMES=(switch-core-01 switch-access-01 router-gw-01 firewall-01 printer-lobby ap-wireless-01 legacy-switch-01 secure-switch-01 switch-exos-01 switch-voss-01)

# SNMPv3 USM credentials for secure-switch-01 (192.168.7.237).
# AuthPriv with SHA-256 / AES-128 — the broadly-supported pure-Rust default.
V3_USER="scanopyv3"
V3_AUTH_PASS="authpass12345"
V3_PRIV_PASS="privpass12345"

CONF_DIR="/etc/snmp-test"
DATA_DIR="$CONF_DIR/data"

echo "=== SNMP Test Environment Setup ==="

# ── 1. Install net-snmp ───────────────────────────────────────────────
if ! command -v snmpd &>/dev/null; then
    echo "Installing net-snmp..."
    apt-get update -qq && apt-get install -y -qq snmpd snmp gawk >/dev/null
fi
systemctl stop snmpd 2>/dev/null || true
systemctl disable snmpd 2>/dev/null || true
sleep 1

# ── 2. Add macvlan interfaces (each with unique MAC) ────────────────
echo "Configuring macvlan interfaces on $IFACE..."
for i in "${!HOSTS[@]}"; do
    ip="${HOSTS[$i]}"
    mvname="mv-snmp${i}"
    if ip link show "$mvname" &>/dev/null; then
        echo "  $mvname ($ip) already exists"
    else
        ip link add "$mvname" link "$IFACE" type macvlan mode bridge
        ip addr add "$ip/$CIDR" dev "$mvname"
        ip link set "$mvname" up
        mac=$(ip link show "$mvname" | awk '/ether/{print $2}')
        echo "  Created $mvname ($ip) mac=$mac"
    fi
done

# ── 3. Write pass handler ────────────────────────────────────────────
mkdir -p "$CONF_DIR" "$DATA_DIR"

cat > "$CONF_DIR/snmp-pass-handler.sh" << 'PASSEOF'
#!/bin/bash
DATA_FILE="$1"
REQUEST="$2"
OID="$3"

if [ ! -f "$DATA_FILE" ]; then
    echo "NONE"
    exit 0
fi

case "$REQUEST" in
    -g)
        LINE=$(awk -v oid="$OID" '$1 == oid { print; exit }' "$DATA_FILE")
        if [ -z "$LINE" ]; then
            echo "NONE"
            exit 0
        fi
        echo "$LINE" | awk '{ print $1; print $2; $1=""; $2=""; sub(/^  */, ""); print }'
        ;;
    -n)
        LINE=$(awk -v oid="$OID" '
            {
                if (oid_gt($1, oid)) {
                    print
                    exit
                }
            }
            function oid_gt(a, b,    na, nb, sa, sb, i) {
                na = split(a, sa, ".")
                nb = split(b, sb, ".")
                for (i = 1; i <= (na > nb ? na : nb); i++) {
                    ai = (i <= na) ? sa[i]+0 : -1
                    bi = (i <= nb) ? sb[i]+0 : -1
                    if (ai > bi) return 1
                    if (ai < bi) return 0
                }
                return 0
            }
        ' "$DATA_FILE")
        if [ -z "$LINE" ]; then
            echo "NONE"
            exit 0
        fi
        echo "$LINE" | awk '{ print $1; print $2; $1=""; $2=""; sub(/^  */, ""); print }'
        ;;
    *)
        echo "NONE"
        exit 0
        ;;
esac
PASSEOF
chmod +x "$CONF_DIR/snmp-pass-handler.sh"

# ── 4. Write MIB data files ──────────────────────────────────────────
echo "Writing MIB data..."

# switch-core-01 IF-MIB
cat > "$DATA_DIR/switch-core-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.1.4 integer 4
.1.3.6.1.2.1.2.2.1.2.1 string GigabitEthernet0/1
.1.3.6.1.2.1.2.2.1.2.2 string GigabitEthernet0/2
.1.3.6.1.2.1.2.2.1.2.3 string GigabitEthernet0/3
.1.3.6.1.2.1.2.2.1.2.4 string Vlan10
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.3.4 integer 53
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.4 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:10:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:10:02
.1.3.6.1.2.1.2.2.1.6.3 string 0:1a:2b:0:10:03
.1.3.6.1.2.1.2.2.1.6.4 string 0:1a:2b:0:10:00
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.7.4 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.2.2.1.8.4 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Gi0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string Gi0/2
.1.3.6.1.2.1.31.1.1.1.1.3 string Gi0/3
.1.3.6.1.2.1.31.1.1.1.1.4 string Vl10
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.4 gauge 0
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-access-01
.1.3.6.1.2.1.31.1.1.1.18.2 string Uplink to router-gw-01
.1.3.6.1.2.1.31.1.1.1.18.3 string Server port
.1.3.6.1.2.1.31.1.1.1.18.4 string Management VLAN
EOF

# switch-core-01 LLDP
cat > "$DATA_DIR/switch-core-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:10:0
.1.0.8802.1.1.2.1.3.3.0 string switch-core-01
.1.0.8802.1.1.2.1.3.4.0 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:11:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/1
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-access-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
.1.0.8802.1.1.2.1.4.1.1.4.0.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.2.1 string 0:1a:2b:0:12:0
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string ge-0/0/0
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string ge-0/0/0
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string router-gw-01
.1.0.8802.1.1.2.1.4.1.1.10.0.2.1 string Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
EOF

# switch-core-01 extra tables — make a scan exercise the getbulk walks (and the
# shared per-host session) for the subtrees stock snmpd does NOT answer itself:
# BRIDGE-MIB/Q-BRIDGE (17), ENTITY-MIB (47) and CDP (enterprise). ipAddrTable and
# ipNetToMedia (ARP) are already answered by snmpd's built-in IP module, so those
# walks are exercised on every device without extra data here.
# (net-snmp `pass` can't emit binary MAC octet-strings, so dot1dTpFdb/dot1qTpFdb
# rows and ARP MACs are not simulated; the daemon still walks those subtrees via
# getbulk and terminates cleanly — the walk mechanism is what we're covering.)
cat > "$DATA_DIR/switch-core-01-bridge.txt" << 'EOF'
.1.3.6.1.2.1.17.1.4.1.2.1 integer 1
.1.3.6.1.2.1.17.1.4.1.2.2 integer 2
.1.3.6.1.2.1.17.1.4.1.2.3 integer 3
.1.3.6.1.2.1.17.7.1.4.3.1.1.10 string DATA
.1.3.6.1.2.1.17.7.1.4.3.1.1.20 string VOICE
.1.3.6.1.2.1.17.7.1.4.5.1.1.1 integer 10
.1.3.6.1.2.1.17.7.1.4.5.1.1.2 integer 10
.1.3.6.1.2.1.17.7.1.4.5.1.1.3 integer 20
EOF

cat > "$DATA_DIR/switch-core-01-entity.txt" << 'EOF'
.1.3.6.1.2.1.47.1.1.1.1.2.1 string Cisco Catalyst 2960-24TC-L
.1.3.6.1.2.1.47.1.1.1.1.5.1 integer 3
.1.3.6.1.2.1.47.1.1.1.1.7.1 string Chassis
.1.3.6.1.2.1.47.1.1.1.1.11.1 string FOC1234X5YZ
.1.3.6.1.2.1.47.1.1.1.1.12.1 string Cisco
.1.3.6.1.2.1.47.1.1.1.1.13.1 string WS-C2960-24TC-L
EOF

cat > "$DATA_DIR/switch-core-01-cdp.txt" << 'EOF'
.1.3.6.1.4.1.9.9.23.1.2.1.1.6.2.1 string router-gw-01
.1.3.6.1.4.1.9.9.23.1.2.1.1.7.2.1 string ge-0/0/0
.1.3.6.1.4.1.9.9.23.1.2.1.1.8.2.1 string Juniper MX204
EOF

# switch-access-01 IF-MIB
cat > "$DATA_DIR/switch-access-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string GigabitEthernet0/1
.1.3.6.1.2.1.2.2.1.2.2 string GigabitEthernet0/2
.1.3.6.1.2.1.2.2.1.2.3 string GigabitEthernet0/3
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:11:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:11:02
.1.3.6.1.2.1.2.2.1.6.3 string 0:1a:2b:0:11:03
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Gi0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string Gi0/2
.1.3.6.1.2.1.31.1.1.1.1.3 string Gi0/3
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 1000
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-core-01
.1.3.6.1.2.1.31.1.1.1.18.2 string Access port - Floor 2
.1.3.6.1.2.1.31.1.1.1.18.3 string Downlink to ap-wireless-01
EOF

# switch-access-01 LLDP
cat > "$DATA_DIR/switch-access-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:11:0
.1.0.8802.1.1.2.1.3.3.0 string switch-access-01
.1.0.8802.1.1.2.1.3.4.0 string Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:10:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/1
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
.1.0.8802.1.1.2.1.4.1.1.4.0.3.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.3.1 string 0:1a:2b:0:15:0
.1.0.8802.1.1.2.1.4.1.1.6.0.3.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.3.1 string eth0
.1.0.8802.1.1.2.1.4.1.1.8.0.3.1 string eth0
.1.0.8802.1.1.2.1.4.1.1.9.0.3.1 string ap-wireless-01
.1.0.8802.1.1.2.1.4.1.1.10.0.3.1 string Ubiquiti UniFi AP AC Pro, firmware 6.5.28
EOF

# router-gw-01 IF-MIB
cat > "$DATA_DIR/router-gw-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string ge-0/0/0
.1.3.6.1.2.1.2.2.1.2.2 string ge-0/0/1
.1.3.6.1.2.1.2.2.1.2.3 string lo0.0
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 24
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:12:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:12:02
.1.3.6.1.2.1.2.2.1.6.3 string
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string ge-0/0/0
.1.3.6.1.2.1.31.1.1.1.1.2 string ge-0/0/1
.1.3.6.1.2.1.31.1.1.1.1.3 string lo0.0
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 0
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-core-01
.1.3.6.1.2.1.31.1.1.1.18.2 string Link to firewall-01
.1.3.6.1.2.1.31.1.1.1.18.3 string Loopback
EOF

# router-gw-01 LLDP
cat > "$DATA_DIR/router-gw-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:12:0
.1.0.8802.1.1.2.1.3.3.0 string router-gw-01
.1.0.8802.1.1.2.1.3.4.0 string Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:10:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/2
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/2
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
.1.0.8802.1.1.2.1.4.1.1.4.0.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.2.1 string 0:1a:2b:0:13:0
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string port1
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string port1
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string firewall-01
.1.0.8802.1.1.2.1.4.1.1.10.0.2.1 string Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)
EOF

# firewall-01 IF-MIB
cat > "$DATA_DIR/firewall-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string port1
.1.3.6.1.2.1.2.2.1.2.2 string port2
.1.3.6.1.2.1.2.2.1.2.3 string port3
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:13:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:13:02
.1.3.6.1.2.1.2.2.1.6.3 string 0:1a:2b:0:13:03
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string port1
.1.3.6.1.2.1.31.1.1.1.1.2 string port2
.1.3.6.1.2.1.31.1.1.1.1.3 string port3
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 1000
.1.3.6.1.2.1.31.1.1.1.18.1 string WAN - to router-gw-01
.1.3.6.1.2.1.31.1.1.1.18.2 string LAN - internal
.1.3.6.1.2.1.31.1.1.1.18.3 string DMZ
EOF

# firewall-01 LLDP
cat > "$DATA_DIR/firewall-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:13:0
.1.0.8802.1.1.2.1.3.3.0 string firewall-01
.1.0.8802.1.1.2.1.3.4.0 string Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:12:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string ge-0/0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string ge-0/0/1
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string router-gw-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
EOF

# printer-lobby IF-MIB (no LLDP)
cat > "$DATA_DIR/printer-lobby-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.2.1 string Ethernet
.1.3.6.1.2.1.2.2.1.2.2 string USB
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 480000000
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:14:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:14:02
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Ethernet
.1.3.6.1.2.1.31.1.1.1.1.2 string USB
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 100
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 480
.1.3.6.1.2.1.31.1.1.1.18.1 string Network port
.1.3.6.1.2.1.31.1.1.1.18.2 string USB port
EOF

# ap-wireless-01 IF-MIB
cat > "$DATA_DIR/ap-wireless-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string eth0
.1.3.6.1.2.1.2.2.1.2.2 string ath0
.1.3.6.1.2.1.2.2.1.2.3 string ath1
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 71
.1.3.6.1.2.1.2.2.1.3.3 integer 71
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 0
.1.3.6.1.2.1.2.2.1.5.3 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:15:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:15:02
.1.3.6.1.2.1.2.2.1.6.3 string 0:1a:2b:0:15:03
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string eth0
.1.3.6.1.2.1.31.1.1.1.1.2 string ath0
.1.3.6.1.2.1.31.1.1.1.1.3 string ath1
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 867
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 400
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-access-01
.1.3.6.1.2.1.31.1.1.1.18.2 string 5GHz radio
.1.3.6.1.2.1.31.1.1.1.18.3 string 2.4GHz radio
EOF

# ap-wireless-01 LLDP
cat > "$DATA_DIR/ap-wireless-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:15:0
.1.0.8802.1.1.2.1.3.3.0 string ap-wireless-01
.1.0.8802.1.1.2.1.3.4.0 string Ubiquiti UniFi AP AC Pro, firmware 6.5.28
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:11:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/3
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/3
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-access-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
EOF

# legacy-switch-01 IF-MIB (SNMPv1-only device)
cat > "$DATA_DIR/legacy-switch-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.2.1 string FastEthernet0/1
.1.3.6.1.2.1.2.2.1.2.2 string FastEthernet0/2
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 100000000
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:16:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:16:02
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Fa0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string Fa0/2
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 100
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 100
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-access-01
.1.3.6.1.2.1.31.1.1.1.18.2 string Access port
EOF

# legacy-switch-01 LLDP
cat > "$DATA_DIR/legacy-switch-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:16:0
.1.0.8802.1.1.2.1.3.3.0 string legacy-switch-01
.1.0.8802.1.1.2.1.3.4.0 string Cisco IOS Software, C2950 Software, Version 12.1(22)EA14
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:11:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/2
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/2
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-access-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
EOF

# legacy-switch-01 BRIDGE — gives the v1-only device a non-ifTable table to walk,
# so a scan exercises the getbulk -> getnext fallback (v1 rejects getbulk) across
# more than just ifTable/LLDP.
cat > "$DATA_DIR/legacy-switch-01-bridge.txt" << 'EOF'
.1.3.6.1.2.1.17.1.4.1.2.1 integer 1
.1.3.6.1.2.1.17.1.4.1.2.2 integer 2
.1.3.6.1.2.1.17.7.1.4.3.1.1.1 string default
EOF

# secure-switch-01 IF-MIB (SNMPv3-only device — hardened, mirrors Huawei S5000)
cat > "$DATA_DIR/secure-switch-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string GigabitEthernet0/0/1
.1.3.6.1.2.1.2.2.1.2.2 string GigabitEthernet0/0/2
.1.3.6.1.2.1.2.2.1.2.3 string GigabitEthernet0/0/3
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 string 0:1a:2b:0:17:01
.1.3.6.1.2.1.2.2.1.6.2 string 0:1a:2b:0:17:02
.1.3.6.1.2.1.2.2.1.6.3 string 0:1a:2b:0:17:03
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string GE0/0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string GE0/0/2
.1.3.6.1.2.1.31.1.1.1.1.3 string GE0/0/3
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 1000
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-core-01
.1.3.6.1.2.1.31.1.1.1.18.2 string Server port
.1.3.6.1.2.1.31.1.1.1.18.3 string Server port
EOF

# secure-switch-01 LLDP
cat > "$DATA_DIR/secure-switch-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:1a:2b:0:17:0
.1.0.8802.1.1.2.1.3.3.0 string secure-switch-01
.1.0.8802.1.1.2.1.3.4.0 string Huawei S5000 Series, VRP V200R019C10
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:10:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/1
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
EOF

# switch-exos-01 IF-MIB (ExtremeXOS — ifIndex 1001+, ifName "1:N")
cat > "$DATA_DIR/switch-exos-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1001 integer 1001
.1.3.6.1.2.1.2.2.1.1.1002 integer 1002
.1.3.6.1.2.1.2.2.1.1.1003 integer 1003
.1.3.6.1.2.1.2.2.1.2.1001 string 1:1
.1.3.6.1.2.1.2.2.1.2.1002 string 1:2
.1.3.6.1.2.1.2.2.1.2.1003 string 1:3
.1.3.6.1.2.1.2.2.1.3.1001 integer 6
.1.3.6.1.2.1.2.2.1.3.1002 integer 6
.1.3.6.1.2.1.2.2.1.3.1003 integer 6
.1.3.6.1.2.1.2.2.1.5.1001 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.1002 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.1003 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1001 string 0:4:96:1:e0:01
.1.3.6.1.2.1.2.2.1.6.1002 string 0:4:96:1:e0:02
.1.3.6.1.2.1.2.2.1.6.1003 string 0:4:96:1:e0:03
.1.3.6.1.2.1.2.2.1.7.1001 integer 1
.1.3.6.1.2.1.2.2.1.7.1002 integer 1
.1.3.6.1.2.1.2.2.1.7.1003 integer 1
.1.3.6.1.2.1.2.2.1.8.1001 integer 1
.1.3.6.1.2.1.2.2.1.8.1002 integer 1
.1.3.6.1.2.1.2.2.1.8.1003 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1001 string 1:1
.1.3.6.1.2.1.31.1.1.1.1.1002 string 1:2
.1.3.6.1.2.1.31.1.1.1.1.1003 string 1:3
EOF

# switch-exos-01 LLDP — lldpRemTable local-port numbers (1, 3) are lldpLocPortNum
# values in a namespace distinct from ifIndex (1001+). lldpLocPortTable maps
# lldpLocPortNum -> lldpLocPortId ("1".."3", subtype interfaceName(5)), which
# suffix-matches ifName "1:N". Before the Issue 2 fix these neighbours are dropped.
cat > "$DATA_DIR/switch-exos-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:4:96:1:e0:0
.1.0.8802.1.1.2.1.3.3.0 string switch-exos-01
.1.0.8802.1.1.2.1.3.4.0 string ExtremeXOS version 31.7 X435-24P
.1.0.8802.1.1.2.1.3.7.1.2.1 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.2 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.3 integer 5
.1.0.8802.1.1.2.1.3.7.1.3.1 string 1
.1.0.8802.1.1.2.1.3.7.1.3.2 string 2
.1.0.8802.1.1.2.1.3.7.1.3.3 string 3
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.3.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 0:1a:2b:0:10:0
.1.0.8802.1.1.2.1.4.1.1.5.0.3.1 string 0:1a:2b:0:12:0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.0.3.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string 1
.1.0.8802.1.1.2.1.4.1.1.7.0.3.1 string 3
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string 1:1
.1.0.8802.1.1.2.1.4.1.1.8.0.3.1 string 1:3
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.9.0.3.1 string router-gw-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960
.1.0.8802.1.1.2.1.4.1.1.10.0.3.1 string Juniper Networks JunOS MX204
EOF

# switch-voss-01 IF-MIB (Extreme VOSS — ifIndex 192+, ifName "1/N"; local-port == ifIndex)
cat > "$DATA_DIR/switch-voss-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.192 integer 192
.1.3.6.1.2.1.2.2.1.1.193 integer 193
.1.3.6.1.2.1.2.2.1.1.194 integer 194
.1.3.6.1.2.1.2.2.1.2.192 string 1/1
.1.3.6.1.2.1.2.2.1.2.193 string 1/2
.1.3.6.1.2.1.2.2.1.2.194 string 1/3
.1.3.6.1.2.1.2.2.1.3.192 integer 6
.1.3.6.1.2.1.2.2.1.3.193 integer 6
.1.3.6.1.2.1.2.2.1.3.194 integer 6
.1.3.6.1.2.1.2.2.1.5.192 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.193 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.194 gauge 10000000000
.1.3.6.1.2.1.2.2.1.6.192 string 0:4:38:2:e0:01
.1.3.6.1.2.1.2.2.1.6.193 string 0:4:38:2:e0:02
.1.3.6.1.2.1.2.2.1.6.194 string 0:4:38:2:e0:03
.1.3.6.1.2.1.2.2.1.7.192 integer 1
.1.3.6.1.2.1.2.2.1.7.193 integer 1
.1.3.6.1.2.1.2.2.1.7.194 integer 1
.1.3.6.1.2.1.2.2.1.8.192 integer 1
.1.3.6.1.2.1.2.2.1.8.193 integer 1
.1.3.6.1.2.1.2.2.1.8.194 integer 1
.1.3.6.1.2.1.31.1.1.1.1.192 string 1/1
.1.3.6.1.2.1.31.1.1.1.1.193 string 1/2
.1.3.6.1.2.1.31.1.1.1.1.194 string 1/3
EOF

# switch-voss-01 LLDP — here lldpRemTable local-port == ifIndex (192, 194) and
# lldpLocPortId ("1/N") matches ifName exactly, so resolution is the identity/exact
# path. Confirms the Issue 2 fix keeps VOSS correct.
cat > "$DATA_DIR/switch-voss-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 0:4:38:2:e0:0
.1.0.8802.1.1.2.1.3.3.0 string switch-voss-01
.1.0.8802.1.1.2.1.3.4.0 string Extreme Networks VSP-7400, VOSS 8.10
.1.0.8802.1.1.2.1.3.7.1.2.192 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.193 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.194 integer 5
.1.0.8802.1.1.2.1.3.7.1.3.192 string 1/1
.1.0.8802.1.1.2.1.3.7.1.3.193 string 1/2
.1.0.8802.1.1.2.1.3.7.1.3.194 string 1/3
.1.0.8802.1.1.2.1.4.1.1.4.0.192.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.194.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.192.1 string 0:1a:2b:0:10:0
.1.0.8802.1.1.2.1.4.1.1.5.0.194.1 string 0:1a:2b:0:11:0
.1.0.8802.1.1.2.1.4.1.1.6.0.192.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.0.194.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.192.1 string 1/1
.1.0.8802.1.1.2.1.4.1.1.7.0.194.1 string 1/3
.1.0.8802.1.1.2.1.4.1.1.8.0.192.1 string 1/1
.1.0.8802.1.1.2.1.4.1.1.8.0.194.1 string 1/3
.1.0.8802.1.1.2.1.4.1.1.9.0.192.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.9.0.194.1 string switch-access-01
.1.0.8802.1.1.2.1.4.1.1.10.0.192.1 string Cisco IOS Software, C2960
.1.0.8802.1.1.2.1.4.1.1.10.0.194.1 string Cisco IOS Software, C3750
EOF

# ── 5. Write snmpd configs ───────────────────────────────────────────
echo "Writing snmpd configs..."

D="$CONF_DIR/data"
H="$CONF_DIR/snmp-pass-handler.sh"

cat > "$CONF_DIR/snmpd-switch-core-01.conf" << EOF
agentAddress udp:${HOSTS[0]}:161
rocommunity netdefault
sysdescr Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
syscontact netops@example.com
sysname switch-core-01
syslocation Server Room A, Rack 1
sysobjectid .1.3.6.1.4.1.9.1.1208
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-core-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-core-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-core-01-lldp.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-core-01-bridge.txt
pass .1.3.6.1.2.1.47 /bin/bash $H $D/switch-core-01-entity.txt
pass .1.3.6.1.4.1.9.9.23 /bin/bash $H $D/switch-core-01-cdp.txt
EOF

cat > "$CONF_DIR/snmpd-switch-access-01.conf" << EOF
agentAddress udp:${HOSTS[1]}:161
rocommunity netdefault
sysdescr Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
syscontact netops@example.com
sysname switch-access-01
syslocation Floor 2, IDF B
sysobjectid .1.3.6.1.4.1.9.1.516
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-access-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-access-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-access-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-router-gw-01.conf" << EOF
agentAddress udp:${HOSTS[2]}:161
rocommunity secret42
sysdescr Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
syscontact netops@example.com
sysname router-gw-01
syslocation Server Room A, Rack 3
sysobjectid .1.3.6.1.4.1.2636.1.1.1.2.29
sysservices 76
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/router-gw-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/router-gw-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/router-gw-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-firewall-01.conf" << EOF
agentAddress udp:${HOSTS[3]}:161
rocommunity secret42
sysdescr Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)
syscontact netops@example.com
sysname firewall-01
syslocation Server Room A, Rack 2
sysobjectid .1.3.6.1.4.1.12356.101.1.1
sysservices 76
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/firewall-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/firewall-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/firewall-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-printer-lobby.conf" << EOF
agentAddress udp:${HOSTS[4]}:161
rocommunity public
sysdescr HP LaserJet Pro MFP M428fdw, FW 2406334_042882
syscontact facilities@example.com
sysname printer-lobby
syslocation Lobby, Reception Desk
sysobjectid .1.3.6.1.4.1.11.2.3.9.1
sysservices 72
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/printer-lobby-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/printer-lobby-iftable.txt
EOF

cat > "$CONF_DIR/snmpd-ap-wireless-01.conf" << EOF
agentAddress udp:${HOSTS[5]}:161
rocommunity netdefault
sysdescr Ubiquiti UniFi AP AC Pro, firmware 6.5.28
syscontact netops@example.com
sysname ap-wireless-01
syslocation Floor 3, Ceiling
sysobjectid .1.3.6.1.4.1.41112.1.6.1
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/ap-wireless-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/ap-wireless-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/ap-wireless-01-lldp.txt
EOF

# legacy-switch-01 — SNMPv1-ONLY. VACM grants access only via the v1 security
# model, so v2c/v3 queries are denied (a plain `rocommunity` would answer both
# v1 and v2c, which wouldn't prove version negotiation).
cat > "$CONF_DIR/snmpd-legacy-switch-01.conf" << EOF
agentAddress udp:${HOSTS[6]}:161
com2sec v1sec default legacyv1
group   v1group v1 v1sec
view    all included .1
access  v1group "" v1 noauth exact all none none
sysdescr Cisco IOS Software, C2950 Software, Version 12.1(22)EA14
syscontact netops@example.com
sysname legacy-switch-01
syslocation Closet 1, Legacy Rack
sysobjectid .1.3.6.1.4.1.9.1.359
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/legacy-switch-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/legacy-switch-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/legacy-switch-01-lldp.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/legacy-switch-01-bridge.txt
EOF

# secure-switch-01 — SNMPv3-ONLY (AuthPriv). No rocommunity, so v1/v2c are
# denied. createUser is consumed on first start; localized keys persist to the
# per-instance persistentDir. SHA-256 / AES-128 (broadly supported).
mkdir -p "$CONF_DIR/state/secure-switch-01"
cat > "$CONF_DIR/snmpd-secure-switch-01.conf" << EOF
agentAddress udp:${HOSTS[7]}:161
persistentDir $CONF_DIR/state/secure-switch-01
createUser $V3_USER SHA-256 "$V3_AUTH_PASS" AES "$V3_PRIV_PASS"
rouser $V3_USER priv
sysdescr Huawei S5000 Series, VRP V200R019C10
syscontact netops@example.com
sysname secure-switch-01
syslocation Server Room A, Rack 4
sysobjectid .1.3.6.1.4.1.2011.2.23.999
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/secure-switch-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/secure-switch-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/secure-switch-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-switch-exos-01.conf" << EOF
agentAddress udp:${HOSTS[8]}:161
rocommunity netdefault
sysdescr ExtremeXOS version 31.7 X435-24P
syscontact netops@example.com
sysname switch-exos-01
syslocation Floor 3, IDF C
sysobjectid .1.3.6.1.4.1.1916.2.219
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-exos-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-exos-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-exos-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-switch-voss-01.conf" << EOF
agentAddress udp:${HOSTS[9]}:161
rocommunity netdefault
sysdescr Extreme Networks VSP-7400, VOSS 8.10
syscontact netops@example.com
sysname switch-voss-01
syslocation Server Room A, Rack 5
sysobjectid .1.3.6.1.4.1.2272.30
sysservices 6
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-voss-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-voss-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-voss-01-lldp.txt
EOF

# ── 6. Create systemd services ───────────────────────────────────────
echo "Creating systemd services..."
for i in "${!SYSNAMES[@]}"; do
    name="${SYSNAMES[$i]}"
    cat > "/etc/systemd/system/snmpd-${name}.service" << EOF
[Unit]
Description=SNMP Test Agent — ${name} (${HOSTS[$i]})
After=network.target

[Service]
Type=simple
ExecStart=/usr/sbin/snmpd -f -Lo -I -ifTable,-ifXTable -C -c ${CONF_DIR}/snmpd-${name}.conf
Restart=on-failure
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF
done

# ── 7. Persist macvlan interfaces ────────────────────────────────────
if [ -d /etc/netplan ]; then
    echo "Persisting macvlan interfaces via netplan..."
    cat > /etc/netplan/60-snmp-test.yaml << EOF
network:
  version: 2
  ethernets:
$(for i in "${!HOSTS[@]}"; do
        mvname="mv-snmp${i}"
        mac=$(ip link show "$mvname" 2>/dev/null | awk '/ether/{print $2}')
        cat << INNER
    ${mvname}:
      match:
        macaddress: "${mac}"
      addresses:
        - ${HOSTS[$i]}/${CIDR}
INNER
done)
EOF
    netplan apply 2>/dev/null || true
elif [ -f /etc/network/interfaces ]; then
    echo "Persisting macvlan interfaces in /etc/network/interfaces..."
    for i in "${!HOSTS[@]}"; do
        mvname="mv-snmp${i}"
        if ! grep -q "$mvname" /etc/network/interfaces; then
            cat >> /etc/network/interfaces << EOF

auto ${mvname}
iface ${mvname} inet static
    address ${HOSTS[$i]}/${CIDR}
EOF
        fi
    done
fi

# ── 8. Start everything ──────────────────────────────────────────────
echo "Starting SNMP agents..."
systemctl daemon-reload
for name in "${SYSNAMES[@]}"; do
    systemctl enable "snmpd-${name}" --quiet
    systemctl restart "snmpd-${name}"
    printf "  %-28s started\n" "snmpd-${name}"
done

# ── 9. Verify ─────────────────────────────────────────────────────────
#
# NOTE: we check systemd service health here, NOT snmpget. The agents bind to
# macvlan interfaces, and the Linux kernel does not let a host reach its own
# macvlan child interfaces — so an snmpget from THIS VM to 192.168.7.x always
# fails even when the agents are perfectly healthy. Query them from an external
# host instead (see the end of this output).
echo ""
echo "Verifying service health..."
sleep 1
all_ok=true
for i in "${!HOSTS[@]}"; do
    name="${SYSNAMES[$i]}"
    ip="${HOSTS[$i]}"
    version="${VERSIONS[$i]}"
    if systemctl is-active --quiet "snmpd-${name}"; then
        printf "  \033[0;32m✓\033[0m %-18s %-20s %s (active)\n" "$ip" "$name" "$version"
    else
        printf "  \033[0;31m✗\033[0m %-18s %-20s %s (not active — journalctl -u snmpd-%s)\n" "$ip" "$name" "$version" "$name"
        all_ok=false
    fi
done

echo ""
if $all_ok; then
    printf "\033[0;32mAll %d SNMP agents are active.\033[0m\n" "${#HOSTS[@]}"
    echo ""
    echo "macvlan blocks queries from this VM. Verify reachability from an"
    echo "external host (e.g. your Mac) with: make snmp-verify"
    echo "Or manually, e.g.:"
    echo "  snmpget -v1  -c legacyv1 192.168.7.236 sysName.0"
    echo "  snmpget -v3 -l authPriv -u ${V3_USER} -a SHA-256 -A ${V3_AUTH_PASS} -x AES -X ${V3_PRIV_PASS} 192.168.7.237 sysName.0"
else
    echo "Some agents are not active. Check: journalctl -u snmpd-<name>"
fi
