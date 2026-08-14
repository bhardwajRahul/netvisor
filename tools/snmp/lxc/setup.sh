#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# SNMP Test Environment — Proxmox VM setup (self-contained)
#
# Paste this entire script into a Debian/Ubuntu VM terminal.
# Creates 14 snmpd instances on secondary IPs, each simulating a
# different network device with its own community string.
#
# Edit HOSTS/CIDR/IFACE below to match your network.
#
# EXPECT TRUNCATION WARNINGS. See "Known chaos" at section 3 — a scan of this
# environment normally reports several incomplete SNMP walks, and that is the
# simulator, not the product under test.
# ══════════════════════════════════════════════════════════════════════

HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235 192.168.7.236 192.168.7.237 192.168.7.238 192.168.7.239 192.168.7.240 192.168.7.241 192.168.7.242 192.168.7.243 192.168.7.244 192.168.7.245)
CIDR="22"
IFACE="eth0"

# Per-host SNMP version. Most are v2c (community string); .236/.237 exercise the
# v1-only and v3-only code paths (#557). .238 (EXOS) and .239 (VOSS) exercise the
# LLDP local-port remap (Issue 2, July 2026): EXOS reports lldpRemTable local-port
# numbers in a namespace distinct from ifIndex and needs lldpLocPortTable to
# resolve; VOSS reports local-port == ifIndex. Per-host communities are written
# directly into each snmpd config below.
#
# .240/.241/.242 cover the three L2-resolution defects from GH #664/#649/#614
# (July 2026) — see the block comments on their MIB data below.
#
# .244 covers the port-id shapes from GH #668 (August 2026): a D-Link that labels its
# neighbour port ids `interfaceName` while sending a bare port number, and one that can
# only be matched through lldpRemPortDesc.
#
# .245 covers the last device from the same report: a TP-Link that indexes lldpRemTable
# without lldpRemTimeMark, so every neighbour row arrives one sub-id short of what the MIB
# describes.
VERSIONS=(v2c v2c v2c v2c v2c v2c v1 v3 v2c v2c v2c v2c v2c v2c v2c v2c)
SYSNAMES=(switch-core-01 switch-access-01 router-gw-01 firewall-01 printer-lobby ap-wireless-01 legacy-switch-01 secure-switch-01 switch-exos-01 switch-voss-01 switch-netgear-01 switch-aruba-01 switch-omada-01 switch-flaky-01 switch-dlink-01 switch-tplink-01)

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
#
# KNOWN CHAOS — read this before chasing a truncation warning.
#
# snmpd forks this script, which then forks awk, once per SNMP request. With 14
# agents on one VM and ~17 column walks per host, a single scan is hundreds of
# concurrent forks, and under that load the agents answer some requests with the
# WRONG OID — one belonging to a request the daemon made earlier.
#
# Measured 2026-07-27, walking all 12 v2c devices from a single client:
#
#   serial      0 of 12 walks truncated
#   concurrent  4-5 of 12 truncated, a DIFFERENT set of devices each run
#
# Every truncation was `StaleResponse`: an in-subtree walk answered with an OID
# lower than the one requested, e.g. asking for lldpRemChassisId (.5) and getting
# lldpRemChassisIdSubtype (.4) back, or asking within ifXTable and being handed an
# LLDP OID that sorts below the entire subtree. A correct agent walking forward
# cannot produce that. It is not our client desyncing: the responses pass request-id
# and community validation, each session owns its own connected socket and its own
# request-id range, and the same walks are clean when run serially.
#
# So: a scan here normally emits several "was incomplete" warnings. They mean the
# simulator is thrashing. Judge a change by whether DATA was lost — interfaces
# pruned, neighbours wiped, links frozen — not by whether warnings appeared.
#
# This misbehaviour is worth keeping. A free adversarial agent surfaced three real
# defects in July 2026 (a foreign interface appearing on a switch, a chassis id
# overwritten with NULL leaving a link permanently unresolvable, and a truncated
# column reported as authoritative). If the noise ever needs quieting, `pass_persist`
# replaces the fork-per-request with one long-lived handler — but leave a device or
# two on `pass` deliberately, or the environment loses the property that found those.
#
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
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:00:10:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:00:10:02
.1.3.6.1.2.1.2.2.1.6.3 string 00:1a:2b:00:10:03
.1.3.6.1.2.1.2.2.1.6.4 string 00:1a:2b:00:10:00
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
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.3.3.0 string switch-core-01
.1.0.8802.1.1.2.1.3.4.0 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:11:00
.1.0.8802.1.1.2.1.4.1.1.5.0.2.1 string 00:1a:2b:00:12:00
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/1
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string ge-0/0/0
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string ge-0/0/0
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-access-01
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string router-gw-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
.1.0.8802.1.1.2.1.4.1.1.10.0.2.1 string Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
EOF

# switch-core-01 extra tables — make a scan exercise the getbulk walks (and the
# shared per-host session) for the subtrees stock snmpd does NOT answer itself:
# BRIDGE-MIB/Q-BRIDGE (17), ENTITY-MIB (47) and CDP (enterprise). ipAddrTable and
# ipNetToMedia (ARP) are already answered by snmpd's built-in IP module, so those
# walks are exercised on every device without extra data here. (ap-wireless-01 is
# the one exception — it serves its own ipAddrTable so it can advertise a guest
# subnet the VM's kernel doesn't have; see the #663 fixture below.)
# (dot1dTpFdb/dot1qTpFdb rows are not simulated here; the daemon still walks those
# subtrees via getbulk and terminates cleanly — the walk mechanism is what we're
# covering. This is a gap in the data, not a limitation of the transport: `pass`
# emits binary via type `octet` (space-separated hex), which is how the physAddress
# and ARP columns elsewhere in this file send real six-byte MACs. An earlier version
# of this note claimed the transport could not carry them, and that is what led two
# separate fixtures to encode MACs as `string` and silently test nothing.)
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
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:00:11:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:00:11:02
.1.3.6.1.2.1.2.2.1.6.3 string 00:1a:2b:00:11:03
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
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:11:00
.1.0.8802.1.1.2.1.3.3.0 string switch-access-01
.1.0.8802.1.1.2.1.3.4.0 string Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.3.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.5.0.3.1 string 00:1a:2b:00:15:00
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.0.3.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/1
.1.0.8802.1.1.2.1.4.1.1.7.0.3.1 string eth0
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.3.1 string eth0
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.9.0.3.1 string ap-wireless-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
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
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:00:12:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:00:12:02
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
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:12:00
.1.0.8802.1.1.2.1.3.3.0 string router-gw-01
.1.0.8802.1.1.2.1.3.4.0 string Juniper Networks, Inc. JunOS 21.4R3-S5, MX204
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.5.0.2.1 string 00:1a:2b:00:13:00
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/2
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string port1
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/2
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string port1
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string firewall-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
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
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:00:13:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:00:13:02
.1.3.6.1.2.1.2.2.1.6.3 string 00:1a:2b:00:13:03
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
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:13:00
.1.0.8802.1.1.2.1.3.3.0 string firewall-01
.1.0.8802.1.1.2.1.3.4.0 string Fortinet FortiGate 60F v7.2.6 build1517 (GA.F)
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:12:00
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
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:00:14:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:00:14:02
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
#
# ifIndex 4 is `br-guest`: an access point's built-in NAT guest network, bridged
# onto its own subnet. This is the #663 fixture — the reporter's Araknis
# AN-810-AP-I-AC advertised exactly this shape, and a `br-` prefixed ifName used
# to be classified as a Docker bridge. See the ipAddrTable data below, which
# hangs 172.30.10.1/24 off this ifIndex.
cat > "$DATA_DIR/ap-wireless-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.1.4 integer 4
.1.3.6.1.2.1.2.2.1.2.1 string eth0
.1.3.6.1.2.1.2.2.1.2.2 string ath0
.1.3.6.1.2.1.2.2.1.2.3 string ath1
.1.3.6.1.2.1.2.2.1.2.4 string br-guest
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 71
.1.3.6.1.2.1.2.2.1.3.3 integer 71
.1.3.6.1.2.1.2.2.1.3.4 integer 209
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 0
.1.3.6.1.2.1.2.2.1.5.3 gauge 0
.1.3.6.1.2.1.2.2.1.5.4 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:00:15:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:00:15:02
.1.3.6.1.2.1.2.2.1.6.3 string 00:1a:2b:00:15:03
.1.3.6.1.2.1.2.2.1.6.4 string 00:1a:2b:00:15:04
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.7.4 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.2.2.1.8.4 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string eth0
.1.3.6.1.2.1.31.1.1.1.1.2 string ath0
.1.3.6.1.2.1.31.1.1.1.1.3 string ath1
.1.3.6.1.2.1.31.1.1.1.1.4 string br-guest
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 867
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 400
.1.3.6.1.2.1.31.1.1.1.15.4 gauge 0
.1.3.6.1.2.1.31.1.1.1.18.1 string Uplink to switch-access-01
.1.3.6.1.2.1.31.1.1.1.18.2 string 5GHz radio
.1.3.6.1.2.1.31.1.1.1.18.3 string 2.4GHz radio
.1.3.6.1.2.1.31.1.1.1.18.4 string NAT guest network
EOF

# ap-wireless-01 ipAddrTable (#663 fixture)
#
# Every other agent lets snmpd's built-in IP module answer ipAddrTable from the
# VM's real kernel state, which only ever yields addresses inside the scanned
# 192.168.4.0/22 — so no agent advertises a second subnet, and the misclassified
# guest network from #663 can't be reproduced. This host overrides that module
# (see the `pass -p 1` lines in its config) and serves the table itself, so it
# can report 172.30.10.1/24 on the `br-guest` interface the way a real AP does.
#
# Rows must stay in numeric OID order (column-major, then ascending IP): the pass
# handler answers GETNEXT by returning the first line greater than the request,
# scanning the file top-down.
cat > "$DATA_DIR/ap-wireless-01-ipaddr.txt" << EOF
.1.3.6.1.2.1.4.20.1.1.172.30.10.1 ipaddress 172.30.10.1
.1.3.6.1.2.1.4.20.1.1.${HOSTS[5]} ipaddress ${HOSTS[5]}
.1.3.6.1.2.1.4.20.1.2.172.30.10.1 integer 4
.1.3.6.1.2.1.4.20.1.2.${HOSTS[5]} integer 1
.1.3.6.1.2.1.4.20.1.3.172.30.10.1 ipaddress 255.255.255.0
.1.3.6.1.2.1.4.20.1.3.${HOSTS[5]} ipaddress 255.255.252.0
EOF

# ap-wireless-01 LLDP
cat > "$DATA_DIR/ap-wireless-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:15:00
.1.0.8802.1.1.2.1.3.3.0 string ap-wireless-01
.1.0.8802.1.1.2.1.3.4.0 string Ubiquiti UniFi AP AC Pro, firmware 6.5.28
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:11:00
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
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:00:16:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:00:16:02
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
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:16:00
.1.0.8802.1.1.2.1.3.3.0 string legacy-switch-01
.1.0.8802.1.1.2.1.3.4.0 string Cisco IOS Software, C2950 Software, Version 12.1(22)EA14
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:11:00
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
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:00:17:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:00:17:02
.1.3.6.1.2.1.2.2.1.6.3 string 00:1a:2b:00:17:03
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
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:17:00
.1.0.8802.1.1.2.1.3.3.0 string secure-switch-01
.1.0.8802.1.1.2.1.3.4.0 string Huawei S5000 Series, VRP V200R019C10
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
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
.1.3.6.1.2.1.2.2.1.6.1001 string 00:04:96:01:e0:01
.1.3.6.1.2.1.2.2.1.6.1002 string 00:04:96:01:e0:02
.1.3.6.1.2.1.2.2.1.6.1003 string 00:04:96:01:e0:03
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

# switch-exos-01 LLDP. Its own chassis id is deliberately left with UNPADDED octets
# (0:4:96:1:e0:0) — every other device here uses the padded form. ExtremeXOS is one of the two
# vendors known to send a MAC-subtype identifier as an ASCII string rather than six octets, and
# firmware that formats a MAC itself is as likely to use %x as %02x. Rejecting that form doesn't
# degrade a neighbour, it discards it entirely and the device silently contributes nothing to L2,
# so this host is the standing guard that the daemon still accepts it.
#
# lldpRemTable local-port numbers (1, 3) are lldpLocPortNum
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
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.5.0.3.1 string 00:1a:2b:00:12:00
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
.1.3.6.1.2.1.2.2.1.6.192 string 00:04:38:02:e0:01
.1.3.6.1.2.1.2.2.1.6.193 string 00:04:38:02:e0:02
.1.3.6.1.2.1.2.2.1.6.194 string 00:04:38:02:e0:03
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
.1.0.8802.1.1.2.1.3.2.0 string 00:04:38:02:e0:00
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
.1.0.8802.1.1.2.1.4.1.1.5.0.192.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.5.0.194.1 string 00:1a:2b:00:11:00
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

# ══════════════════════════════════════════════════════════════════════
# switch-netgear-01 / switch-aruba-01 / switch-omada-01 — L2 resolution
#
# These three devices exist to reproduce the L2-topology failures reported in
# GH #664, #649 and #614 on real hardware. They form a connected pair plus one
# standalone switch:
#
#   switch-netgear-01 g1  <->  switch-aruba-01 port 41
#   switch-netgear-01 g2  <->  switch-aruba-01 port A5 (ifIndex 197)
#
# What each one proves is documented above its MIB data.
# ══════════════════════════════════════════════════════════════════════

# switch-netgear-01 IF-MIB (Netgear GS724Tv3). The device's chassis/management
# MAC (…:63, in its LLDP local identity below) appears on NO port and NO IP —
# ports are …:65/:66/:67. A neighbour advertising the chassis MAC is therefore
# unresolvable by MAC alone and only matches via the host's own chassis_id.
cat > "$DATA_DIR/switch-netgear-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string g1
.1.3.6.1.2.1.2.2.1.2.2 string g2
.1.3.6.1.2.1.2.2.1.2.3 string g3
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:3c:4d:65
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:3c:4d:66
.1.3.6.1.2.1.2.2.1.6.3 string 00:1a:2b:3c:4d:67
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 2
.1.3.6.1.2.1.31.1.1.1.1.1 string g1
.1.3.6.1.2.1.31.1.1.1.1.2 string g2
.1.3.6.1.2.1.31.1.1.1.1.3 string g3
EOF

# switch-netgear-01 LLDP. Its own chassis id (…:63) differs from every port MAC
# — this is what gets recorded as the host's chassis_id and is the ONLY
# server-side record of that MAC (GH #664).
#
# Its two neighbour entries advertise switch-aruba-01's ports with port-ID
# subtype 7 (locallyAssigned): "41" matches that switch's ifDescr, and "197"
# matches only its ifIndex. Before the fix both resolved to the host and stopped
# there, and a host-only neighbour draws no L2 edge at all (GH #649).
cat > "$DATA_DIR/switch-netgear-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:3c:4d:63
.1.0.8802.1.1.2.1.3.3.0 string switch-netgear-01
.1.0.8802.1.1.2.1.3.4.0 string NETGEAR GS724Tv3 ProSAFE 24-port Gigabit Smart Switch
.1.0.8802.1.1.2.1.3.7.1.2.1 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.2 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.3 integer 5
.1.0.8802.1.1.2.1.3.7.1.3.1 string g1
.1.0.8802.1.1.2.1.3.7.1.3.2 string g2
.1.0.8802.1.1.2.1.3.7.1.3.3 string g3
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:0c:29:aa:bb:c0
.1.0.8802.1.1.2.1.4.1.1.5.0.2.1 string 00:0c:29:aa:bb:c0
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 7
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 7
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string 41
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string 197
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string 41
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string A5
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-aruba-01
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string switch-aruba-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string ProCurve J9145A 2910al-24G, revision W.15.16.0007
.1.0.8802.1.1.2.1.4.1.1.10.0.2.1 string ProCurve J9145A 2910al-24G, revision W.15.16.0007
EOF

# switch-aruba-01 IF-MIB (HP/Aruba ProCurve). Two port-naming shapes in one
# device: ports whose ifDescr IS the bare port number ("41", "42"), and a port
# whose label ("A5") has nothing to do with its ifIndex (197). A locally-assigned
# port id is one or the other, so both legs need covering.
cat > "$DATA_DIR/switch-aruba-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.41 integer 41
.1.3.6.1.2.1.2.2.1.1.42 integer 42
.1.3.6.1.2.1.2.2.1.1.197 integer 197
.1.3.6.1.2.1.2.2.1.2.41 string 41
.1.3.6.1.2.1.2.2.1.2.42 string 42
.1.3.6.1.2.1.2.2.1.2.197 string A5
.1.3.6.1.2.1.2.2.1.3.41 integer 6
.1.3.6.1.2.1.2.2.1.3.42 integer 6
.1.3.6.1.2.1.2.2.1.3.197 integer 6
.1.3.6.1.2.1.2.2.1.5.41 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.42 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.197 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.41 string 00:0c:29:aa:bb:29
.1.3.6.1.2.1.2.2.1.6.42 string 00:0c:29:aa:bb:2a
.1.3.6.1.2.1.2.2.1.6.197 string 00:0c:29:aa:bb:c5
.1.3.6.1.2.1.2.2.1.7.41 integer 1
.1.3.6.1.2.1.2.2.1.7.42 integer 1
.1.3.6.1.2.1.2.2.1.7.197 integer 1
.1.3.6.1.2.1.2.2.1.8.41 integer 1
.1.3.6.1.2.1.2.2.1.8.42 integer 2
.1.3.6.1.2.1.2.2.1.8.197 integer 1
.1.3.6.1.2.1.31.1.1.1.1.41 string 41
.1.3.6.1.2.1.31.1.1.1.1.42 string 42
.1.3.6.1.2.1.31.1.1.1.1.197 string A5
EOF

# switch-aruba-01 LLDP. The return direction of the same two links, and the
# GH #664 case seen from this side: both neighbour entries carry
# switch-netgear-01's CHASSIS mac (…:63), which exists on none of that device's
# ports — it resolves only through the host's own chassis_id. The remote port is
# then identified by its real port MAC (…:65 / …:66).
cat > "$DATA_DIR/switch-aruba-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 00:0c:29:aa:bb:c0
.1.0.8802.1.1.2.1.3.3.0 string switch-aruba-01
.1.0.8802.1.1.2.1.3.4.0 string ProCurve J9145A 2910al-24G, revision W.15.16.0007
.1.0.8802.1.1.2.1.3.7.1.2.41 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.42 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.197 integer 5
.1.0.8802.1.1.2.1.3.7.1.3.41 string 41
.1.0.8802.1.1.2.1.3.7.1.3.42 string 42
.1.0.8802.1.1.2.1.3.7.1.3.197 string A5
.1.0.8802.1.1.2.1.4.1.1.4.0.41.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.197.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.41.1 string 00:1a:2b:3c:4d:63
.1.0.8802.1.1.2.1.4.1.1.5.0.197.1 string 00:1a:2b:3c:4d:63
.1.0.8802.1.1.2.1.4.1.1.6.0.41.1 integer 3
.1.0.8802.1.1.2.1.4.1.1.6.0.197.1 integer 3
.1.0.8802.1.1.2.1.4.1.1.7.0.41.1 string 00:1a:2b:3c:4d:65
.1.0.8802.1.1.2.1.4.1.1.7.0.197.1 string 00:1a:2b:3c:4d:66
.1.0.8802.1.1.2.1.4.1.1.8.0.41.1 string g1
.1.0.8802.1.1.2.1.4.1.1.8.0.197.1 string g2
.1.0.8802.1.1.2.1.4.1.1.9.0.41.1 string switch-netgear-01
.1.0.8802.1.1.2.1.4.1.1.9.0.197.1 string switch-netgear-01
.1.0.8802.1.1.2.1.4.1.1.10.0.41.1 string NETGEAR GS724Tv3 ProSAFE 24-port Gigabit Smart Switch
.1.0.8802.1.1.2.1.4.1.1.10.0.197.1 string NETGEAR GS724Tv3 ProSAFE 24-port Gigabit Smart Switch
EOF

# switch-omada-01 IF-MIB (TP-Link Omada TL-SG3216, GH #614). ifIndex 1 is the
# only interface with a name and the only one bearing an IP; the 16 physical
# ports live at 49153-49168, report NO ifXTable ifName, and all share the
# chassis ifPhysAddress. All 17 must survive discovery — before the v0.17.1 fix
# the 16 nameless ports collapsed onto the management interface via the MAC tier.
# Emitted column-by-column so the file is in ascending numeric-OID order, which
# is what the GETNEXT pass handler above requires (it returns the first line
# greater than the requested OID and stops).
INDEXES="1 $(seq 49153 49168)"
{
    for idx in $INDEXES; do echo ".1.3.6.1.2.1.2.2.1.1.${idx} integer ${idx}"; done
    for idx in $INDEXES; do
        if [ "$idx" = 1 ]; then
            echo ".1.3.6.1.2.1.2.2.1.2.1 string Vlan-interface1"
        else
            echo ".1.3.6.1.2.1.2.2.1.2.${idx} string gigabitEthernet 1/0/$((idx - 49152))"
        fi
    done
    for idx in $INDEXES; do echo ".1.3.6.1.2.1.2.2.1.3.${idx} integer 6"; done
    for idx in $INDEXES; do echo ".1.3.6.1.2.1.2.2.1.5.${idx} gauge 1000000000"; done
    for idx in $INDEXES; do echo ".1.3.6.1.2.1.2.2.1.6.${idx} string 30:de:4b:30:f0:ac"; done
    for idx in $INDEXES; do echo ".1.3.6.1.2.1.2.2.1.7.${idx} integer 1"; done
    for idx in $INDEXES; do echo ".1.3.6.1.2.1.2.2.1.8.${idx} integer 1"; done
    # ifXTable carries a name for the management interface only — the 16 physical
    # ports report none, which is the whole point of this profile.
    echo ".1.3.6.1.2.1.31.1.1.1.1.1 string Vlan-interface1"
} > "$DATA_DIR/switch-omada-01-iftable.txt"

# ══════════════════════════════════════════════════════════════════════
# switch-flaky-01 — a neighbour record that is missing its chassis ID
#
# A truncated lldpRemChassisId column and a device that simply serves no chassis
# ID are indistinguishable to the daemon: both yield a neighbour with a port ID
# and a system name but no chassis ID. The second is static, so it reproduces on
# every scan where the first is a transient nobody can schedule.
#
# That shape is destructive if taken at face value. The chassis ID is a mandatory
# TLV (IEEE 802.1AB), so the record is malformed — but written through it would
# overwrite a good chassis ID with NULL, and a row without one is excluded from L2
# resolution entirely, freezing the link at whatever it last resolved to with no
# way back. This device exists to prove that does not happen.
#
# Two LLDP variants are written; the agent serves whichever is copied over
# `-lldp-active.txt`. The `pass` handler re-reads the file per request, so
# swapping takes effect immediately with NO snmpd restart:
#
#   cp /etc/snmp-test/data/switch-flaky-01-lldp-nochassis.txt \
#      /etc/snmp-test/data/switch-flaky-01-lldp-active.txt     # break it
#   cp /etc/snmp-test/data/switch-flaky-01-lldp-complete.txt \
#      /etc/snmp-test/data/switch-flaky-01-lldp-active.txt     # restore it
#
# Variants: -complete, -nochassis, -nosubtype, -badsubtype, -ghost. Each drives a different
# per-cause counter, and the four failing ones now drive different warning text as well — the
# advice a customer acts on differs between them (GH #668).
# ══════════════════════════════════════════════════════════════════════

cat > "$DATA_DIR/switch-flaky-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.2.1 string uplink0
.1.3.6.1.2.1.2.2.1.2.2 string uplink1
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 string 00:1a:2b:00:1f:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1a:2b:00:1f:02
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string uplink0
.1.3.6.1.2.1.31.1.1.1.1.2 string uplink1
EOF

# Complete: a well-formed neighbour on port 1, pointing at switch-core-01's Gi0/3
# (the one port on that switch with no other neighbour). Resolves port-to-port.
cat > "$DATA_DIR/switch-flaky-01-lldp-complete.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:1f:00
.1.0.8802.1.1.2.1.3.3.0 string switch-flaky-01
.1.0.8802.1.1.2.1.3.4.0 string Scanopy SNMP simulator, flaky-LLDP profile
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/3
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/3
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960
EOF

# Chassis-less: the SAME neighbour minus lldpRemChassisIdSubtype (.4) and
# lldpRemChassisId (.5). Everything else is unchanged, which is exactly what a
# cut-short chassis column leaves behind. The device's own local chassis id
# (.3.2.0) stays, so only the *neighbour* record is malformed.
cat > "$DATA_DIR/switch-flaky-01-lldp-nochassis.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:1f:00
.1.0.8802.1.1.2.1.3.3.0 string switch-flaky-01
.1.0.8802.1.1.2.1.3.4.0 string Scanopy SNMP simulator, flaky-LLDP profile
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/3
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/3
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960
EOF

# Two more shapes that reach the same discard, added for GH #668. The old logging collapsed all
# three causes into one `dropped=N`, so a customer had to run snmpwalk by hand to tell us which
# one their switch produced. These make each counter reproducible against a real agent.

# Subtype absent, value present: `.5` answers and `.4` does not. Recoverable in principle — the
# subtype could be inferred from the value's shape — so it is the one worth being able to see.
cat > "$DATA_DIR/switch-flaky-01-lldp-nosubtype.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:1f:00
.1.0.8802.1.1.2.1.3.3.0 string switch-flaky-01
.1.0.8802.1.1.2.1.3.4.0 string Scanopy SNMP simulator, flaky-LLDP profile
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/3
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/3
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960
EOF

# Subtype present but the wrong ASN.1 type: `.4` is an INTEGER per the MIB and this agent sends a
# string. Reads as a complete walk — no truncation signal anywhere — so before the per-cause
# counters the only evidence it had happened at all was the record going missing.
cat > "$DATA_DIR/switch-flaky-01-lldp-badsubtype.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:1f:00
.1.0.8802.1.1.2.1.3.3.0 string switch-flaky-01
.1.0.8802.1.1.2.1.3.4.0 string Scanopy SNMP simulator, flaky-LLDP profile
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 string macAddress
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/3
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/3
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960
EOF

# Ghost rows: a sparse chassis column against a fuller port column. Local port 1 is complete;
# local port 2 appears in the port-id, port-desc and sys-name columns and in neither chassis
# column. That is the third cause behind the same `dropped=N`, and the only one of the four with
# no simulator coverage at all — it was reproducible only in unit tests, so the classification
# that separates it from a cut-short read had never been checked against a real agent.
#
# The distinction matters to whoever reads the warning: nothing was lost here, because there was
# never a chassis id on those rows to lose, so a rescan is wasted effort. A truncated chassis
# column looks identical in the record count and is worth retrying.
cat > "$DATA_DIR/switch-flaky-01-lldp-ghost.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 00:1a:2b:00:1f:00
.1.0.8802.1.1.2.1.3.3.0 string switch-flaky-01
.1.0.8802.1.1.2.1.3.4.0 string Scanopy SNMP simulator, flaky-LLDP profile
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string Gi0/3
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string Gi0/4
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/3
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string GigabitEthernet0/4
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960
.1.0.8802.1.1.2.1.4.1.1.10.0.2.1 string Cisco IOS Software, C2960
EOF

# Start healthy. Re-running setup.sh resets it, which is the intended way to undo
# a test that left the device broken.
cp "$DATA_DIR/switch-flaky-01-lldp-complete.txt" "$DATA_DIR/switch-flaky-01-lldp-active.txt"

# ══════════════════════════════════════════════════════════════════════
# switch-dlink-01 — port ids that name-only matching cannot resolve (GH #668)
#
# Modelled on a D-Link DGS-1210-48. Two things about that firmware break L2 resolution, and
# both are in its *neighbour* records rather than its own tables:
#
#   1. It sets lldpRemPortIdSubtype to 5 (interfaceName) and then sends a bare port number.
#      Subtype 5 used to get a name lookup and nothing else, so "2" matched no interface on a
#      switch whose ports are named Gi0/2 — the neighbour resolved as far as the host and
#      stopped, and a host-only neighbour draws no edge.
#   2. Where the port id matches nothing at all, lldpRemPortDesc still carries the remote
#      port's own ifDescr verbatim. That field was stored and never matched on.
#
# Its own ifTable uses the D-Link shape as well (ifDescr "…Port N", ifName "Slot0/N", ifIndex
# N), so it is also a ready-made target for a future fixture that needs a device where the
# advertised number is the ifIndex and the name is something else.
#
#   3. Every port reports the same ifPhysAddress — the chassis base MAC, 00:ad:24:af:4e:00, which
#      is also this device's lldpLocChassisId. This is what the real DGS-1210-48 does, it is legal
#      SNMP (RFC 2863 does not require per-port addresses), and it is the third report on the same
#      issue: the reporter saw one MAC repeated down the whole interface list and read it as
#      Scanopy mis-attributing them. It is not — but it did make an interface MAC a false identity
#      key. A MAC that names 3 ports names none of them, and the server used to take whichever row
#      the database returned first, drawing a port-precise link to an arbitrary port. Consumers now
#      resolve on a single match only; see `find_if_entry_by_mac` and `plan_interface_ip_links`.
#
#      The physAddress column uses type `octet`, not `string`. `string` sends the value as text,
#      so `00:ad:24:af:4e:00` arrives as 17 ASCII bytes where a PhysAddress is six raw octets;
#      `value_to_mac` rightly refuses it and the interface stores no MAC at all, which makes this
#      fixture look like it is testing something it never reaches. `octet` takes space-separated
#      hex and emits the six bytes a real agent sends. Verify with
#      `snmpwalk -v2c -c netdefault -Ox 192.168.7.244 1.3.6.1.2.1.2.2.1.6` — six octets, three
#      times over, not seventeen.
#
#      An earlier revision gave each port its own address (…:4e:01..03), which is the case that
#      never needed guarding. Nothing else here depended on those being distinct.
#
# NOT covered here: the NUL-terminated port ids from the same report. net-snmp's `pass`
# protocol is line-based — the handler prints OID, type and value as three lines — so an
# embedded 0x00 cannot survive the transport, and no data file can express it. That half is
# covered by unit tests instead (`value_to_string`, `LldpPortId::from_snmp`, `PgText`/`PgJson`).
# ══════════════════════════════════════════════════════════════════════

cat > "$DATA_DIR/switch-dlink-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string D-Link DGS-1210-48 Rev.GX/7.20.003 Port 1
.1.3.6.1.2.1.2.2.1.2.2 string D-Link DGS-1210-48 Rev.GX/7.20.003 Port 2
.1.3.6.1.2.1.2.2.1.2.3 string D-Link DGS-1210-48 Rev.GX/7.20.003 Port 3
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 octet 00 ad 24 af 4e 00
.1.3.6.1.2.1.2.2.1.6.2 octet 00 ad 24 af 4e 00
.1.3.6.1.2.1.2.2.1.6.3 octet 00 ad 24 af 4e 00
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Slot0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string Slot0/2
.1.3.6.1.2.1.31.1.1.1.1.3 string Slot0/3
EOF

# Both neighbours point at switch-core-01 (chassis 00:1a:2b:00:10:00). They deliberately share
# Gi0/1 and Gi0/2 with links other sim devices also claim: this profile exists to exercise port-id
# resolution, not to model a physically consistent lab, and resolution runs per interface row.
#
#   local port 1 → subtype 5, port id "2". No interface on switch-core-01 is named "2"
#                  (ifDescr GigabitEthernet0/2, ifName Gi0/2), so this resolves only via the
#                  ifIndex fallback subtype 5 did not previously get.
#   local port 2 → subtype 5, port id "ethernet1/0/44". Matches no name and is not a number, so
#                  the port id is a dead end; lldpRemPortDesc carries GigabitEthernet0/1, which
#                  is exactly switch-core-01's ifDescr for ifIndex 1.
#
# Rows are listed column-major (all of column 4, then all of column 5, …), matching every other
# LLDP fixture here. That is not cosmetic: `pass` answers GETNEXT by scanning this file in the
# order written, so grouping a row's columns together makes the traversal walk off the end of the
# table after the first row. Written row-major, the second neighbour below is served only as
# lldpRemSysDesc — an index with no chassis id, which the daemon correctly counts as a ghost row
# and discards, and the port-desc tier this device exists to exercise never runs at all.
cat > "$DATA_DIR/switch-dlink-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 00:ad:24:af:4e:00
.1.0.8802.1.1.2.1.3.3.0 string switch-dlink-01
.1.0.8802.1.1.2.1.3.4.0 string D-Link DGS-1210-48 Rev.GX/7.20.003
.1.0.8802.1.1.2.1.3.7.1.2.1 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.2 integer 5
.1.0.8802.1.1.2.1.3.7.1.3.1 string Slot0/1
.1.0.8802.1.1.2.1.3.7.1.3.2 string Slot0/2
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.5.0.2.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string 2
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string ethernet1/0/44
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/2
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string GigabitEthernet0/1
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960
.1.0.8802.1.1.2.1.4.1.1.10.0.2.1 string Cisco IOS Software, C2960
EOF

# ══════════════════════════════════════════════════════════════════════
# switch-tplink-01 — an lldpRemTable indexed without lldpRemTimeMark (GH #668)
#
# Modelled on a TP-Link TL-SX3016F, from the reporter's own snmpwalk. The MIB indexes
# lldpRemEntry as lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex; this firmware omits the
# time mark and indexes on the remaining two, so every row is one sub-id shorter than every
# other device here:
#
#   .1.0.8802.1.1.2.1.4.1.1.4.1.1 = INTEGER: 4              (local port 1, remIndex 1)
#   .1.0.8802.1.1.2.1.4.1.1.5.1.1 = STRING: "00:AD:24:89:CC:F0"
#
# That shape used to remove the device from the map without leaving any evidence: a parser
# requiring three sub-ids built no record, so nothing reached the discard counters, the walk
# still called itself complete, and an empty result from a sixteen-port switch was then treated
# as the device authoritatively reporting no neighbours — clearing the links the server held.
# The reporter's completed scan named every other problem device in a warning and this one in
# none, which is the signature worth being able to reproduce.
#
# Two further quirks from the same device are deliberately kept, because they decide whether a
# row that now survives can actually resolve:
#
#   - Chassis ids are subtype 4 (macAddress) carrying an uppercase ASCII MAC rather than six
#     raw octets. Handled by `parse_mac_id`, and this is the profile that proves it end to end.
#   - Ports are ifDescr "ten-gigabitEthernet 1/0/N" with no ifName, alongside a Vlan-interface1.
#     The neighbour port ids are the bare "1/0/N" suffix, which resolves through the boundary-
#     anchored suffix match rather than an exact name.
#
# Neighbours point at switch-core-01 (00:1a:2b:00:10:00), switch-dlink-01 (00:ad:24:af:4e:00)
# and switch-netgear-01 (00:1a:2b:00:20:00) so the rows resolve to real hosts in this lab.
#
# Column-major ordering, as everywhere else here: `pass` answers GETNEXT by scanning the file in
# the order written.
# ══════════════════════════════════════════════════════════════════════

cat > "$DATA_DIR/switch-tplink-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.1.4 integer 4
.1.3.6.1.2.1.2.2.1.1.5 integer 5
.1.3.6.1.2.1.2.2.1.1.17 integer 17
.1.3.6.1.2.1.2.2.1.2.1 string ten-gigabitEthernet 1/0/1
.1.3.6.1.2.1.2.2.1.2.2 string ten-gigabitEthernet 1/0/2
.1.3.6.1.2.1.2.2.1.2.3 string ten-gigabitEthernet 1/0/3
.1.3.6.1.2.1.2.2.1.2.4 string ten-gigabitEthernet 1/0/4
.1.3.6.1.2.1.2.2.1.2.5 string ten-gigabitEthernet 1/0/5
.1.3.6.1.2.1.2.2.1.2.17 string Vlan-interface1
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.3.4 integer 6
.1.3.6.1.2.1.2.2.1.3.5 integer 6
.1.3.6.1.2.1.2.2.1.3.17 integer 53
.1.3.6.1.2.1.2.2.1.5.1 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.4 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.5 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 string 18:66:da:5d:aa:01
.1.3.6.1.2.1.2.2.1.6.2 string 18:66:da:5d:aa:02
.1.3.6.1.2.1.2.2.1.6.3 string 18:66:da:5d:aa:03
.1.3.6.1.2.1.2.2.1.6.4 string 18:66:da:5d:aa:04
.1.3.6.1.2.1.2.2.1.6.5 string 18:66:da:5d:aa:05
.1.3.6.1.2.1.2.2.1.6.17 string 18:66:da:5d:aa:8e
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.7.4 integer 1
.1.3.6.1.2.1.2.2.1.7.5 integer 1
.1.3.6.1.2.1.2.2.1.7.17 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 2
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.2.2.1.8.4 integer 2
.1.3.6.1.2.1.2.2.1.8.5 integer 1
.1.3.6.1.2.1.2.2.1.8.17 integer 1
EOF

# The device's own local identity uses the conformant index (lldpLocPortNum only), which is
# what makes the remote table's missing time mark the single variable this profile changes.
#
# Note the local port numbers here equal ifIndex, so the local-port remap resolves to the identity
# mapping and cannot mask the index parse under test.
#
# Every far end below is a real device in this lab, matched on a value that device actually
# reports — checked against the scanned data, not invented. An earlier revision of this file used
# a made-up chassis MAC for switch-netgear-01 and a port (Gi0/4) that switch-core-01 does not
# have; both still "resolved", one by falling through to the sysName tier and one by stopping at
# a device-level edge, so the profile passed without exercising what it claims to. Each row now
# resolves through exactly one intended path:
#
#   port 1 → switch-core-01   chassis 00:1a:2b:00:10:00 is that switch's own lldpLocChassisId;
#                             port id "Gi0/3" is its ifName. Host and port both by name.
#   port 2 → nothing          a desk phone: no device in this lab bears this MAC or sysName, so
#                             every host tier fails. Deliberate — it is the only source of a
#                             non-zero `host_not_found`, which is what the server-side summary
#                             naming unmatched far ends needs in order to fire at all. Endpoints
#                             like this are what that counter legitimately consists of.
#   port 3 → switch-dlink-01  port id "Slot0/3" is its ifName, ifDescr is the long D-Link form.
#                             Its chassis id now sits on that switch's ports as well as on its
#                             hosts.chassis_id, so the host resolves at the MAC tier rather than
#                             the chassis-id fallback it used to reach. The #664 fallback is still
#                             covered on its own by port 5 below, whose chassis MAC is on no port.
#   port 4 → switch-dlink-01  the same device again, reached the way GH #668 exposed: port id
#                             subtype 3 (macAddress) carrying 00:AD:24:AF:4E:00, which that switch
#                             reports as ifPhysAddress on *every* port. This is the only subtype-3
#                             port id in the lab, so it is what proves subtype 3 parses and reaches
#                             a lookup at all, and that failing it leaves a device-level (amber)
#                             NeighborLink rather than no edge. lldpRemPortDesc is deliberately
#                             "Uplink to core", matching no ifName or ifDescr on that switch: the
#                             port-desc tier still runs after a failed port id, so anything
#                             matchable here would resolve the port and hide the case.
#
#                             The chassis id resolves the host; the port id must then resolve
#                             nothing, because a MAC belonging to three ports names none of them.
#                             Expect port_ambiguous=1 on the resolution summary, a named entry on
#                             the companion warning, and one device-level edge — not a port-precise
#                             link to whichever of Slot0/1..3 came back first.
#
#                             This only holds because switch-dlink-01 sends its physAddress as
#                             `octet`; with `string` the far end stores no MACs, the lookup returns
#                             port_not_found instead, and the same amber edge appears for an
#                             entirely different reason.
#   port 5 → switch-netgear-01 chassis 00:1a:2b:3c:4d:63 is on no port and no IP (the #664
#                             shape), so only the host's own recorded chassis_id can match it;
#                             port id "3" matches no name and falls through to ifIndex 3 (g3).
#                             Its lldpRemPortDesc deliberately matches nothing, so the ifIndex
#                             fallback is what is actually under test.
cat > "$DATA_DIR/switch-tplink-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 string 18:66:da:5d:aa:8e
.1.0.8802.1.1.2.1.3.3.0 string switch-tplink-01
.1.0.8802.1.1.2.1.3.4.0 string TL-SX3016F 1.0 - TP-Link Switch
.1.0.8802.1.1.2.1.3.7.1.2.1 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.2 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.3 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.4 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.5 integer 5
.1.0.8802.1.1.2.1.3.7.1.3.1 string ten-gigabitEthernet 1/0/1
.1.0.8802.1.1.2.1.3.7.1.3.2 string ten-gigabitEthernet 1/0/2
.1.0.8802.1.1.2.1.3.7.1.3.3 string ten-gigabitEthernet 1/0/3
.1.0.8802.1.1.2.1.3.7.1.3.4 string ten-gigabitEthernet 1/0/4
.1.0.8802.1.1.2.1.3.7.1.3.5 string ten-gigabitEthernet 1/0/5
.1.0.8802.1.1.2.1.4.1.1.4.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.3.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.4.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.5.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.1.1 string 00:1A:2B:00:10:00
.1.0.8802.1.1.2.1.4.1.1.5.2.1 string 9C:AD:97:1F:22:40
.1.0.8802.1.1.2.1.4.1.1.5.3.1 string 00:AD:24:AF:4E:00
.1.0.8802.1.1.2.1.4.1.1.5.4.1 string 00:AD:24:AF:4E:00
.1.0.8802.1.1.2.1.4.1.1.5.5.1 string 00:1A:2B:3C:4D:63
.1.0.8802.1.1.2.1.4.1.1.6.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.3.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.4.1 integer 3
.1.0.8802.1.1.2.1.4.1.1.6.5.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.7.1.1 string Gi0/3
.1.0.8802.1.1.2.1.4.1.1.7.2.1 string 1
.1.0.8802.1.1.2.1.4.1.1.7.3.1 string Slot0/3
.1.0.8802.1.1.2.1.4.1.1.7.4.1 string 00:AD:24:AF:4E:00
.1.0.8802.1.1.2.1.4.1.1.7.5.1 string 3
.1.0.8802.1.1.2.1.4.1.1.8.1.1 string GigabitEthernet0/3
.1.0.8802.1.1.2.1.4.1.1.8.2.1 string Port 1
.1.0.8802.1.1.2.1.4.1.1.8.3.1 string D-Link DGS-1210-48 Rev.GX/7.20.003 Port 3
.1.0.8802.1.1.2.1.4.1.1.8.4.1 string Uplink to core
.1.0.8802.1.1.2.1.4.1.1.8.5.1 string Slot: 0 Port: 3 Gigabit - Level
.1.0.8802.1.1.2.1.4.1.1.9.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.9.2.1 string desk-phone-4021
.1.0.8802.1.1.2.1.4.1.1.9.3.1 string switch-dlink-01
.1.0.8802.1.1.2.1.4.1.1.9.4.1 string switch-dlink-01
.1.0.8802.1.1.2.1.4.1.1.9.5.1 string switch-netgear-01
.1.0.8802.1.1.2.1.4.1.1.10.1.1 string Cisco IOS Software, C2960
.1.0.8802.1.1.2.1.4.1.1.10.2.1 string Polycom VVX 411
.1.0.8802.1.1.2.1.4.1.1.10.3.1 string D-Link DGS-1210-48 Rev.GX/7.20.003
.1.0.8802.1.1.2.1.4.1.1.10.4.1 string D-Link DGS-1210-48 Rev.GX/7.20.003
.1.0.8802.1.1.2.1.4.1.1.10.5.1 string GS724Tv3 ProSafe 24-port Gigabit Smart Switch
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

# The ipAddrTable override below is registered per COLUMN with an explicit
# priority, which looks redundant but is the only thing that works:
#
#  - `-I -ipaddr` / `-ipAddr` does NOT disable the built-in module. It keeps
#    registering (verify with `snmpd -Dregister_mib …` — it reports the module
#    as "mibII/ipaddr"), so unlike ifTable/ifXTable this subtree cannot be freed
#    up by disabling its module.
#  - mibII/ipaddr registers each column separately (.4.20.1.1 … .4.20.1.5), so a
#    single `pass` at the .4.20 subtree root loses on specificity no matter what
#    priority it carries — net-snmp prefers the more specific registration.
#
# Matching that column granularity and taking priority 1 (default is 255; lower
# wins) is what actually displaces it. Columns 4-5 are deliberately left to the
# built-in module — the daemon only reads addr/ifIndex/netmask.
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
pass -p 1 .1.3.6.1.2.1.4.20.1.1 /bin/bash $H $D/ap-wireless-01-ipaddr.txt
pass -p 1 .1.3.6.1.2.1.4.20.1.2 /bin/bash $H $D/ap-wireless-01-ipaddr.txt
pass -p 1 .1.3.6.1.2.1.4.20.1.3 /bin/bash $H $D/ap-wireless-01-ipaddr.txt
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

cat > "$CONF_DIR/snmpd-switch-netgear-01.conf" << EOF
agentAddress udp:${HOSTS[10]}:161
rocommunity netdefault
sysdescr NETGEAR GS724Tv3 ProSAFE 24-port Gigabit Smart Switch
syscontact netops@example.com
sysname switch-netgear-01
syslocation Floor 1, IDF A
sysobjectid .1.3.6.1.4.1.4526.100.4.15
sysservices 2
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-netgear-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-netgear-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-netgear-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-switch-aruba-01.conf" << EOF
agentAddress udp:${HOSTS[11]}:161
rocommunity netdefault
sysdescr ProCurve J9145A 2910al-24G, revision W.15.16.0007
syscontact netops@example.com
sysname switch-aruba-01
syslocation Floor 1, IDF B
sysobjectid .1.3.6.1.4.1.11.2.3.7.11.79
sysservices 2
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-aruba-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-aruba-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-aruba-01-lldp.txt
EOF

# No LLDP subtree: this device reports no neighbours at all (as the #614
# reporter's did), so it exercises the interface-persistence path in isolation.
cat > "$CONF_DIR/snmpd-switch-omada-01.conf" << EOF
agentAddress udp:${HOSTS[12]}:161
rocommunity public
sysdescr TP-Link Omada TL-SG3216 JetStream 16-Port Gigabit L2 Managed Switch
syscontact netops@example.com
sysname switch
syslocation Floor 2, Comms Cupboard
sysobjectid .1.3.6.1.4.1.11863.6.96
sysservices 2
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-omada-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-omada-01-iftable.txt
EOF

cat > "$CONF_DIR/snmpd-switch-flaky-01.conf" << EOF
agentAddress udp:${HOSTS[13]}:161
rocommunity netdefault
sysdescr Scanopy SNMP simulator, flaky-LLDP profile
syscontact netops@example.com
sysname switch-flaky-01
syslocation Lab
sysobjectid .1.3.6.1.4.1.99999.1
sysservices 2
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-flaky-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-flaky-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-flaky-01-lldp-active.txt
EOF

cat > "$CONF_DIR/snmpd-switch-dlink-01.conf" << EOF
agentAddress udp:${HOSTS[14]}:161
rocommunity netdefault
sysdescr D-Link DGS-1210-48 Rev.GX/7.20.003
syscontact netops@example.com
sysname switch-dlink-01
syslocation Lab
sysobjectid .1.3.6.1.4.1.171.10.76.28
sysservices 2
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-dlink-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-dlink-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-dlink-01-lldp.txt
EOF

# No ifXTable `pass` here on purpose: this switch serves no ifName, so its ports are known only
# by the ifDescr "ten-gigabitEthernet 1/0/N" — which is what the neighbour port ids have to be
# matched against.
cat > "$CONF_DIR/snmpd-switch-tplink-01.conf" << EOF
agentAddress udp:${HOSTS[15]}:161
rocommunity netdefault
sysdescr TL-SX3016F 1.0 - TP-Link 16-Port 10G SFP+ Managed Switch
syscontact netops@example.com
sysname switch-tplink-01
syslocation Lab
sysobjectid .1.3.6.1.4.1.11863.5.1.1
sysservices 2
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-tplink-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-tplink-01-lldp.txt
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
