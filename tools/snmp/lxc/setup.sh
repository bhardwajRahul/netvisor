#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# SNMP Test Environment — Proxmox VM setup (self-contained)
#
# Paste this entire script into a Debian/Ubuntu VM terminal.
# Creates 22 snmpd instances on secondary IPs, each simulating a
# different network device with its own community string.
#
# Edit HOSTS/CIDR/IFACE below to match your network.
#
# EXPECT TRUNCATION WARNINGS. See "Known chaos" at section 3 — a scan of this
# environment normally reports several incomplete SNMP walks, and that is the
# simulator, not the product under test.
# ══════════════════════════════════════════════════════════════════════

HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235 192.168.7.236 192.168.7.237 192.168.7.238 192.168.7.239 192.168.7.240 192.168.7.241 192.168.7.242 192.168.7.243 192.168.7.244 192.168.7.245 192.168.7.246 192.168.7.247 192.168.7.248 192.168.7.249 192.168.7.250 192.168.7.251)
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
#
# .246/.247 cover GH #674 and the Westermo report (August 2026). .246 serves its ARP table out
# of ascending OID order — a real firmware bug that `snmpbulkwalk -Cc` tolerates and a strict
# client refuses, which emptied every multi-column collection on the reporter's switch. It is
# the only device using the positional `pass` handler. .247 identifies every local LLDP port by
# `macAddress(3)` and names the interface only in `lldpLocPortDesc`, with local port numbers
# running backwards against the interfaces — so neither the port id nor arithmetic can place a
# neighbour, and only the description or a per-port MAC can.
VERSIONS=(v2c v2c v2c v2c v2c v2c v1 v3 v2c v2c v2c v2c v2c v2c v2c v2c v2c v2c v2c v2c v2c v3)
SYSNAMES=(switch-core-01 switch-access-01 router-gw-01 firewall-01 printer-lobby ap-wireless-01 legacy-switch-01 secure-switch-01 switch-exos-01 switch-voss-01 switch-netgear-01 switch-aruba-01 switch-omada-01 switch-flaky-01 switch-dlink-01 switch-tplink-01 switch-unsorted-01 switch-macport-01 switch-mute-01 switch-stuck-01 switch-dell-01 switch-cisco-01)

# SNMPv3 USM credentials for secure-switch-01 (192.168.7.237).
# AuthPriv with SHA-256 / AES-128 — the broadly-supported pure-Rust default.
V3_USER="scanopyv3"
V3_AUTH_PASS="authpass12345"
V3_PRIV_PASS="privpass12345"

# A second USM identity, for switch-cisco-01 (192.168.7.251) only.
#
# Deliberately not the one above. Every seeded credential is Broadcast-scoped to every network and
# only one SNMP credential per host ever executes — the last mapping that authenticates wins — so
# if the context-bearing credential and the plain one both answered here, which of them read the
# device would be down to mapping order and the fixture would report nine FDB entries or one at
# random. A user only this device accepts makes the winner deterministic.
V3_CTX_USER="scanopyctx"
V3_CTX_AUTH_PASS="ctxauthpass12345"
V3_CTX_PRIV_PASS="ctxprivpass12345"

# The back-end agent holding switch-cisco-01's VLAN 20 bridge tables. Loopback-only: it exists to
# be proxied, never to be scanned.
CTX_BACKEND_ADDR="127.0.0.1:16151"
CTX_BACKEND_COMMUNITY="ctxinternal"

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
# snmpd forks this script, which then forks awk, once per SNMP request. With 22
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

# A second handler that walks its data file in FILE order rather than OID order.
#
# The handler above answers GETNEXT with the first line numerically greater than the request, so
# a shuffled data file would simply end the walk early — it can only ever produce an ascending
# sequence. Firmware that stores a table unsorted and iterates it positionally does not: it hands
# back whatever row physically follows the one asked for, which is how a switch answers
# `...10.0.0.54` with `...10.0.0.7` and makes `snmpwalk` stop at "OID not increasing" while
# `snmpbulkwalk -Cc` reads the table in full (GH #674).
#
# Reproducing that needs the positional behaviour, so the two handlers coexist: this one is used
# only by the device that is meant to be broken.
cat > "$CONF_DIR/snmp-pass-handler-unsorted.sh" << 'PASSEOF'
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
        ;;
    -n)
        # The line after the requested one, in file order. A request naming no line of its own
        # — a bare column or table prefix — is answered with the first line under it, again in
        # file order, which is where the shuffle first shows.
        #
        # Single pass, exits as soon as it has an answer. That matters more than it looks: this
        # handler is forked per varbind, so a full scan runs it thousands of times against every
        # column. An earlier version read the whole file into awk arrays before deciding, and
        # under the load of an 18-host scan it was slow enough that snmpd gave up on it — which
        # the agent reports as endOfMibView, and a walk cannot tell that from a table that
        # genuinely ended. The symptom was a column returning one row and calling itself
        # complete, which looks exactly like a daemon bug and is not one.
        LINE=$(awk -v oid="$OID" '
            function oid_gt(a, b,   na, nb, sa, sb, i, ai, bi) {
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
            matched { print; answered = 1; exit }
            $1 == oid { matched = 1; next }
            !have_prefix && index($1, oid ".") == 1 { prefix = $0; have_prefix = 1 }
            !have_gt && oid_gt($1, oid) { gt = $0; have_gt = 1 }
            END {
                if (answered || matched) exit
                if (have_prefix) print prefix
                else if (have_gt) print gt
            }
        ' "$DATA_FILE")
        ;;
    *)
        echo "NONE"
        exit 0
        ;;
esac

if [ -z "$LINE" ]; then
    echo "NONE"
    exit 0
fi
echo "$LINE" | awk '{ print $1; print $2; $1=""; $2=""; sub(/^  */, ""); print }'
PASSEOF
chmod +x "$CONF_DIR/snmp-pass-handler-unsorted.sh"

# A third handler: one that never advances.
#
# Answers every GETNEXT with the same row, whatever was asked. That is the agent the walk's
# retry-then-stop guard was written for — left to itself it would have the daemon re-request the
# same page until the entry cap or the integration timeout. Here it is deliberate and permanent,
# so the guard has something to hold against, and so one device reliably produces the
# "did not finish reporting" warning that reports a walk falling short.
cat > "$CONF_DIR/snmp-pass-handler-stuck.sh" << 'PASSEOF'
#!/bin/bash
DATA_FILE="$1"
REQUEST="$2"
OID="$3"

if [ ! -f "$DATA_FILE" ]; then
    echo "NONE"
    exit 0
fi

case "$REQUEST" in
    -g) LINE=$(awk -v oid="$OID" '$1 == oid { print; exit }' "$DATA_FILE") ;;
    -n) LINE=$(head -1 "$DATA_FILE") ;;
    *)  echo "NONE"; exit 0 ;;
esac

if [ -z "$LINE" ]; then
    echo "NONE"
    exit 0
fi
echo "$LINE" | awk '{ print $1; print $2; $1=""; $2=""; sub(/^  */, ""); print }'
PASSEOF
chmod +x "$CONF_DIR/snmp-pass-handler-stuck.sh"

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
# The forwarding database, and the first one in this lab. GH #686 reports a Catalyst answering
# `dot1dTpFdbAddress` with nine rows to a raw walk and exactly one to a scan, and no fixture here
# could reproduce it because no device served the table at all.
#
# The MAC is the *index* — six decimal sub-ids, one per octet — and the address column repeats it
# as six raw bytes via `octet`. That repetition is the point: it is the only end-to-end coverage
# of a binary MAC on a table the daemon joins across three columns, and it is what a `string`
# encoding here would silently stop testing.
#
# Statuses are mixed deliberately. The daemon keeps learned(3) and mgmt(5) and drops self(4), so
# a walk that read every row still yields seven entries rather than eight — a filter that stopped
# working would show up as a count that is too high, not as an empty table.
#
# Far ends are real lab devices (`00:1a:2b:00:<device>:<port>`), so the entries resolve to hosts
# rather than to nothing: switch-access-01 on port 1, router-gw-01 on port 2, and the rest of the
# access-side devices on port 3.
cat > "$DATA_DIR/switch-core-01-bridge.txt" << 'EOF'
.1.3.6.1.2.1.17.1.4.1.2.1 integer 1
.1.3.6.1.2.1.17.1.4.1.2.2 integer 2
.1.3.6.1.2.1.17.1.4.1.2.3 integer 3
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.16.0 octet 00 1a 2b 00 10 00
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.16.1 octet 00 1a 2b 00 10 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.17.0 octet 00 1a 2b 00 11 00
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.17.1 octet 00 1a 2b 00 11 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.18.1 octet 00 1a 2b 00 12 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.19.1 octet 00 1a 2b 00 13 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.20.1 octet 00 1a 2b 00 14 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.21.1 octet 00 1a 2b 00 15 01
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.16.0 integer 0
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.16.1 integer 1
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.17.0 integer 1
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.17.1 integer 1
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.18.1 integer 2
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.19.1 integer 3
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.20.1 integer 3
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.21.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.16.0 integer 4
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.16.1 integer 5
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.17.0 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.17.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.18.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.19.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.20.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.21.1 integer 3
.1.3.6.1.2.1.17.7.1.2.2.1.2.10.0.26.43.0.17.1 integer 1
.1.3.6.1.2.1.17.7.1.2.2.1.2.10.0.26.43.0.18.1 integer 2
.1.3.6.1.2.1.17.7.1.2.2.1.2.20.0.26.43.0.20.1 integer 3
.1.3.6.1.2.1.17.7.1.2.2.1.3.10.0.26.43.0.17.1 integer 3
.1.3.6.1.2.1.17.7.1.2.2.1.3.10.0.26.43.0.18.1 integer 3
.1.3.6.1.2.1.17.7.1.2.2.1.3.20.0.26.43.0.20.1 integer 3
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
#
# Its FDB is small and deliberately so: the point here is that the forwarding table is read
# one varbind at a time over GETNEXT, which is the path a v1 device forces and the one where a
# multi-column join has the most opportunities to fall out of step. A device whose FDB arrives
# only under getbulk would look healthy on every other agent in this lab.
cat > "$DATA_DIR/legacy-switch-01-bridge.txt" << 'EOF'
.1.3.6.1.2.1.17.1.4.1.2.1 integer 1
.1.3.6.1.2.1.17.1.4.1.2.2 integer 2
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.16.1 octet 00 1a 2b 00 10 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.22.1 octet 00 1a 2b 00 16 01
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.16.1 integer 1
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.22.1 integer 2
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.16.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.22.1 integer 4
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

# switch-omada-01 LLDP — the reciprocal half of a link neither end can name a port on.
#
# This device and switch-dlink-01 both report one chassis address on every interface, so when
# each advertises that address as its port id neither can identify a port on the other: the MAC
# names the device and nothing narrower. Before the reciprocal tier the link degraded to a dashed
# device-level edge on both sides, which is what the August 2026 customer saw across an entire
# Ubiquiti/Westermo estate.
#
# What makes it resolvable is that each device names the other on exactly one port. Those two
# ports are locally known — each is attached by ifIndex on its own device — so the pair can be
# bound without either far-end port ever being identified. Keep it at exactly one port each way:
# a second link between these two is a LAG, genuinely ambiguous, and must stay device-level.
# switch-tplink-01 local ports 3 and 4 are that case and are deliberately left as they are.
#
# lldpLocPortNum here is 1..16 against ifIndex 49153..49168, so the local-port remap is load
# bearing: lldpLocPortId is subtype 5 carrying the ifDescr, the only column these nameless ports
# have. Without the remap every neighbour lands on an index no interface holds and is discarded
# whole — the drop this environment previously had no device to reproduce.
{
    echo ".1.0.8802.1.1.2.1.3.1.0 integer 4"
    echo ".1.0.8802.1.1.2.1.3.2.0 octet 30 de 4b 30 f0 ac"
    echo ".1.0.8802.1.1.2.1.3.3.0 string switch"
    echo ".1.0.8802.1.1.2.1.3.4.0 string TP-Link Omada TL-SG3216"
    for port in $(seq 1 16); do echo ".1.0.8802.1.1.2.1.3.7.1.2.${port} integer 5"; done
    for port in $(seq 1 16); do
        echo ".1.0.8802.1.1.2.1.3.7.1.3.${port} string gigabitEthernet 1/0/${port}"
    done
    # One neighbour, on local port 5 (ifIndex 49157): switch-dlink-01, addressed by the chassis
    # MAC it repeats across all of its ports.
    echo ".1.0.8802.1.1.2.1.4.1.1.4.0.5.1 integer 4"
    echo ".1.0.8802.1.1.2.1.4.1.1.5.0.5.1 octet 00 ad 24 af 4e 00"
    echo ".1.0.8802.1.1.2.1.4.1.1.6.0.5.1 integer 3"
    echo ".1.0.8802.1.1.2.1.4.1.1.7.0.5.1 octet 00 ad 24 af 4e 00"
    echo ".1.0.8802.1.1.2.1.4.1.1.8.0.5.1 string Uplink"
    echo ".1.0.8802.1.1.2.1.4.1.1.9.0.5.1 string switch-dlink-01"
    echo ".1.0.8802.1.1.2.1.4.1.1.10.0.5.1 string D-Link DGS-1210-48 Rev.GX/7.20.003"
} > "$DATA_DIR/switch-omada-01-lldp.txt"

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
.1.3.6.1.2.1.2.2.1.1.4 integer 4
.1.3.6.1.2.1.2.2.1.2.1 string D-Link DGS-1210-48 Rev.GX/7.20.003 Port 1
.1.3.6.1.2.1.2.2.1.2.2 string D-Link DGS-1210-48 Rev.GX/7.20.003 Port 2
.1.3.6.1.2.1.2.2.1.2.3 string D-Link DGS-1210-48 Rev.GX/7.20.003 Port 3
.1.3.6.1.2.1.2.2.1.2.4 string D-Link DGS-1210-48 Rev.GX/7.20.003 Port 4
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.3.4 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.4 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 octet 00 ad 24 af 4e 00
.1.3.6.1.2.1.2.2.1.6.2 octet 00 ad 24 af 4e 00
.1.3.6.1.2.1.2.2.1.6.3 octet 00 ad 24 af 4e 00
.1.3.6.1.2.1.2.2.1.6.4 octet 00 ad 24 af 4e 00
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.7.4 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.2.2.1.8.4 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Slot0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string Slot0/2
.1.3.6.1.2.1.31.1.1.1.1.3 string Slot0/3
.1.3.6.1.2.1.31.1.1.1.1.4 string Slot0/4
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
#   local port 3 → switch-macport-01, and the positive half of the shared-MAC pair. Port id
#                  subtype 3 (macAddress) carrying 00:07:7c:20:01:e3, which is that switch's
#                  ifPhysAddress for eth3 and for nothing else. One match, so it must resolve to
#                  that exact port — the counter-case to switch-tplink-01 local port 4, where the
#                  same subtype against a device repeating one MAC must resolve to no port at all.
#                  Both are needed: a guard that rejected this one too would look like it worked
#                  while quietly costing every vendor that addresses its ports individually.
#
#                  Deliberate choices worth keeping:
#                    - The chassis id is 00:07:7c:20:01:e0, that device's own lldpLocChassisId,
#                      which sits on none of its ports. Host resolution therefore goes through
#                      hosts.chassis_id and cannot borrow the answer from the port lookup.
#                    - lldpRemPortDesc is "Ring port to peer", matching no ifName or ifDescr over
#                      there (they are eth1..eth10). If the MAC tier ever breaks, this fails
#                      loudly instead of being rescued by the description tier — the failure mode
#                      that has already made three fixtures in this file look healthy while
#                      testing nothing.
#                    - Both identifiers are sent as `octet`, six raw bytes, which is what a real
#                      agent sends and what no other LLDP fixture here does; the ASCII form is
#                      covered by switch-tplink-01. This is the only end-to-end coverage of
#                      `parse_mac_id`'s raw-octet branch.
#
#                  switch-macport-01 lives on fix/snmp-walk-and-lldp-local-port. Until that
#                  merges, this row resolves to no host and counts as host_not_found — harmless,
#                  and it starts working the moment the device exists.
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
.1.0.8802.1.1.2.1.3.7.1.2.3 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.4 integer 5
.1.0.8802.1.1.2.1.3.7.1.3.1 string Slot0/1
.1.0.8802.1.1.2.1.3.7.1.3.2 string Slot0/2
.1.0.8802.1.1.2.1.3.7.1.3.3 string Slot0/3
.1.0.8802.1.1.2.1.3.7.1.3.4 string Slot0/4
.1.0.8802.1.1.2.1.4.1.1.4.0.1.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.2.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.3.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.0.4.1 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.0.1.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.5.0.2.1 string 00:1a:2b:00:10:00
.1.0.8802.1.1.2.1.4.1.1.5.0.3.1 octet 00 07 7c 20 01 e0
.1.0.8802.1.1.2.1.4.1.1.5.0.4.1 octet 30 de 4b 30 f0 ac
.1.0.8802.1.1.2.1.4.1.1.6.0.1.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.0.2.1 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.0.3.1 integer 3
.1.0.8802.1.1.2.1.4.1.1.6.0.4.1 integer 3
.1.0.8802.1.1.2.1.4.1.1.7.0.1.1 string 2
.1.0.8802.1.1.2.1.4.1.1.7.0.2.1 string ethernet1/0/44
.1.0.8802.1.1.2.1.4.1.1.7.0.3.1 octet 00 07 7c 20 01 e3
.1.0.8802.1.1.2.1.4.1.1.7.0.4.1 octet 30 de 4b 30 f0 ac
.1.0.8802.1.1.2.1.4.1.1.8.0.1.1 string GigabitEthernet0/2
.1.0.8802.1.1.2.1.4.1.1.8.0.2.1 string GigabitEthernet0/1
.1.0.8802.1.1.2.1.4.1.1.8.0.3.1 string Ring port to peer
.1.0.8802.1.1.2.1.4.1.1.8.0.4.1 string Uplink
.1.0.8802.1.1.2.1.4.1.1.9.0.1.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.9.0.2.1 string switch-core-01
.1.0.8802.1.1.2.1.4.1.1.9.0.3.1 string switch-macport-01
.1.0.8802.1.1.2.1.4.1.1.9.0.4.1 string switch
.1.0.8802.1.1.2.1.4.1.1.10.0.1.1 string Cisco IOS Software, C2960
.1.0.8802.1.1.2.1.4.1.1.10.0.2.1 string Cisco IOS Software, C2960
.1.0.8802.1.1.2.1.4.1.1.10.0.3.1 string Westermo WeOS
.1.0.8802.1.1.2.1.4.1.1.10.0.4.1 string TP-Link Omada TL-SG3216
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

# switch-unsorted-01 IF-MIB (GH #674). Ordinary and sorted: the interfaces must come back
# whole, so that an empty ARP table on this device is visibly a property of that table and not
# of the whole host.
cat > "$DATA_DIR/switch-unsorted-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.2.1 string GigabitEthernet0/1
.1.3.6.1.2.1.2.2.1.2.2 string GigabitEthernet0/2
.1.3.6.1.2.1.2.2.1.2.3 string Vlan1
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 53
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 string 00:1f:c6:aa:00:01
.1.3.6.1.2.1.2.2.1.6.2 string 00:1f:c6:aa:00:02
.1.3.6.1.2.1.2.2.1.6.3 string 00:1f:c6:aa:00:03
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string Gi0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string Gi0/2
.1.3.6.1.2.1.31.1.1.1.1.3 string Vlan1
EOF

# switch-unsorted-01 ARP (GH #674) — the defect itself.
#
# The physAddress column uses type `octet`, not `string`. This is the difference between a
# fixture that exercises the code and one that silently proves nothing: `string` sends the
# value as *text*, so `00:25:90:f0:00:02` arrives as 17 ASCII bytes, and a MAC is six raw
# octets. The daemon rightly refuses to read the text as an address, the ARP entry loses the
# MAC it is joined on, and all 45 rows are discarded — the walk succeeds and the table reports
# empty. `octet` takes space-separated hex and emits the six bytes a real agent sends.
#
# Every column is written with its rows deliberately shuffled, and this file is served by the
# POSITIONAL handler, so a GETNEXT walk follows the file rather than the numbers: asking after
# .54 answers .7. That is what makes `snmpwalk` stop at "OID not increasing" here and what a
# strict client reads as a table that ends after two rows.
#
# The shape matters as much as the disorder. Enough rows that the walk needs more than one
# GETBULK page (the daemon asks 20 at a time), and the shuffle arranged so a later page ends
# lower than an earlier one — which is the moment a strictly-ascending walk gives up. The four
# columns each need to survive, because an ARP entry is a join across all of them and one short
# column discards every row the others read.
cat > "$DATA_DIR/switch-unsorted-01-arp.txt" << 'EOF'
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.2 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.4 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.6 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.8 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.10 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.12 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.14 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.16 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.18 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.20 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.22 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.24 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.26 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.28 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.30 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.32 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.34 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.36 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.38 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.40 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.42 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.44 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.1 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.3 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.5 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.7 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.9 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.11 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.13 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.15 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.17 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.19 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.21 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.23 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.25 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.27 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.29 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.31 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.33 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.35 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.37 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.39 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.41 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.43 integer 2
.1.3.6.1.2.1.4.22.1.1.2.10.20.30.45 integer 2
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.2 octet 00 25 90 f0 00 02
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.4 octet 00 25 90 f0 00 04
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.6 octet 00 25 90 f0 00 06
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.8 octet 00 25 90 f0 00 08
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.10 octet 00 25 90 f0 00 10
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.12 octet 00 25 90 f0 00 12
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.14 octet 00 25 90 f0 00 14
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.16 octet 00 25 90 f0 00 16
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.18 octet 00 25 90 f0 00 18
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.20 octet 00 25 90 f0 00 20
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.22 octet 00 25 90 f0 00 22
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.24 octet 00 25 90 f0 00 24
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.26 octet 00 25 90 f0 00 26
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.28 octet 00 25 90 f0 00 28
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.30 octet 00 25 90 f0 00 30
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.32 octet 00 25 90 f0 00 32
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.34 octet 00 25 90 f0 00 34
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.36 octet 00 25 90 f0 00 36
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.38 octet 00 25 90 f0 00 38
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.40 octet 00 25 90 f0 00 40
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.42 octet 00 25 90 f0 00 42
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.44 octet 00 25 90 f0 00 44
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.1 octet 00 25 90 f0 00 01
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.3 octet 00 25 90 f0 00 03
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.5 octet 00 25 90 f0 00 05
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.7 octet 00 25 90 f0 00 07
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.9 octet 00 25 90 f0 00 09
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.11 octet 00 25 90 f0 00 11
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.13 octet 00 25 90 f0 00 13
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.15 octet 00 25 90 f0 00 15
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.17 octet 00 25 90 f0 00 17
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.19 octet 00 25 90 f0 00 19
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.21 octet 00 25 90 f0 00 21
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.23 octet 00 25 90 f0 00 23
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.25 octet 00 25 90 f0 00 25
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.27 octet 00 25 90 f0 00 27
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.29 octet 00 25 90 f0 00 29
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.31 octet 00 25 90 f0 00 31
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.33 octet 00 25 90 f0 00 33
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.35 octet 00 25 90 f0 00 35
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.37 octet 00 25 90 f0 00 37
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.39 octet 00 25 90 f0 00 39
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.41 octet 00 25 90 f0 00 41
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.43 octet 00 25 90 f0 00 43
.1.3.6.1.2.1.4.22.1.2.2.10.20.30.45 octet 00 25 90 f0 00 45
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.2 ipaddress 10.20.30.2
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.4 ipaddress 10.20.30.4
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.6 ipaddress 10.20.30.6
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.8 ipaddress 10.20.30.8
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.10 ipaddress 10.20.30.10
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.12 ipaddress 10.20.30.12
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.14 ipaddress 10.20.30.14
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.16 ipaddress 10.20.30.16
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.18 ipaddress 10.20.30.18
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.20 ipaddress 10.20.30.20
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.22 ipaddress 10.20.30.22
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.24 ipaddress 10.20.30.24
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.26 ipaddress 10.20.30.26
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.28 ipaddress 10.20.30.28
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.30 ipaddress 10.20.30.30
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.32 ipaddress 10.20.30.32
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.34 ipaddress 10.20.30.34
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.36 ipaddress 10.20.30.36
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.38 ipaddress 10.20.30.38
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.40 ipaddress 10.20.30.40
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.42 ipaddress 10.20.30.42
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.44 ipaddress 10.20.30.44
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.1 ipaddress 10.20.30.1
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.3 ipaddress 10.20.30.3
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.5 ipaddress 10.20.30.5
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.7 ipaddress 10.20.30.7
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.9 ipaddress 10.20.30.9
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.11 ipaddress 10.20.30.11
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.13 ipaddress 10.20.30.13
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.15 ipaddress 10.20.30.15
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.17 ipaddress 10.20.30.17
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.19 ipaddress 10.20.30.19
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.21 ipaddress 10.20.30.21
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.23 ipaddress 10.20.30.23
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.25 ipaddress 10.20.30.25
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.27 ipaddress 10.20.30.27
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.29 ipaddress 10.20.30.29
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.31 ipaddress 10.20.30.31
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.33 ipaddress 10.20.30.33
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.35 ipaddress 10.20.30.35
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.37 ipaddress 10.20.30.37
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.39 ipaddress 10.20.30.39
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.41 ipaddress 10.20.30.41
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.43 ipaddress 10.20.30.43
.1.3.6.1.2.1.4.22.1.3.2.10.20.30.45 ipaddress 10.20.30.45
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.2 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.4 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.6 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.8 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.10 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.12 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.14 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.16 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.18 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.20 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.22 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.24 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.26 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.28 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.30 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.32 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.34 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.36 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.38 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.40 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.42 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.44 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.1 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.3 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.5 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.7 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.9 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.11 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.13 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.15 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.17 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.19 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.21 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.23 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.25 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.27 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.29 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.31 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.33 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.35 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.37 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.39 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.41 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.43 integer 3
.1.3.6.1.2.1.4.22.1.4.2.10.20.30.45 integer 3
EOF

# switch-macport-01 IF-MIB — the customer's Westermo WeOS switch, from its own walk.
#
# Every column here is a shape that costs a link when it is guessed at:
#
#   ifIndex is not the port number and does not ascend with it. Ports run ifIndex 10..19 while
#   the interfaces they name run eth10 down to eth1, so ifIndex 11 is eth9 and 19 is eth1.
#
#   ifDescr carries the media type in front of the name — "100-T eth9", "1000-LX eth1" — so a
#   neighbour advertising the bare port name matches ifDescr on no device of this family.
#   ifName and ifAlias both hold the bare name, which is why both are served: the alias column is
#   the one that makes "eth9" resolvable, and no other fixture emits it.
#
#   ifPhysAddress is unique per *physical* port (…e1 through …ea) but the six VLAN interfaces all
#   repeat the chassis address …e0, which belongs to no physical port. A MAC lookup that counts
#   virtual rows finds six matches and declines, costing a port no physical interface contested.
#
#   lo (ifType 24) and the VLANs (ifType 53) are here because they are on the real device and
#   because their presence is the test — a fixture of ten clean ethernet rows cannot fail this way.
cat > "$DATA_DIR/switch-macport-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.10 integer 10
.1.3.6.1.2.1.2.2.1.1.11 integer 11
.1.3.6.1.2.1.2.2.1.1.12 integer 12
.1.3.6.1.2.1.2.2.1.1.13 integer 13
.1.3.6.1.2.1.2.2.1.1.14 integer 14
.1.3.6.1.2.1.2.2.1.1.15 integer 15
.1.3.6.1.2.1.2.2.1.1.16 integer 16
.1.3.6.1.2.1.2.2.1.1.17 integer 17
.1.3.6.1.2.1.2.2.1.1.18 integer 18
.1.3.6.1.2.1.2.2.1.1.19 integer 19
.1.3.6.1.2.1.2.2.1.1.22 integer 22
.1.3.6.1.2.1.2.2.1.1.23 integer 23
.1.3.6.1.2.1.2.2.1.1.26 integer 26
.1.3.6.1.2.1.2.2.1.1.28 integer 28
.1.3.6.1.2.1.2.2.1.1.29 integer 29
.1.3.6.1.2.1.2.2.1.1.30 integer 30
.1.3.6.1.2.1.2.2.1.2.1 string lo
.1.3.6.1.2.1.2.2.1.2.10 string 100-T eth10
.1.3.6.1.2.1.2.2.1.2.11 string 100-T eth9
.1.3.6.1.2.1.2.2.1.2.12 string 100-T eth8
.1.3.6.1.2.1.2.2.1.2.13 string 100-T eth7
.1.3.6.1.2.1.2.2.1.2.14 string 100-T eth6
.1.3.6.1.2.1.2.2.1.2.15 string 100-T eth5
.1.3.6.1.2.1.2.2.1.2.16 string 100-T eth4
.1.3.6.1.2.1.2.2.1.2.17 string 100-T eth3
.1.3.6.1.2.1.2.2.1.2.18 string 1000-T eth2
.1.3.6.1.2.1.2.2.1.2.19 string 1000-LX eth1
.1.3.6.1.2.1.2.2.1.2.22 string vlan1
.1.3.6.1.2.1.2.2.1.2.23 string vlan6
.1.3.6.1.2.1.2.2.1.2.26 string vlan832
.1.3.6.1.2.1.2.2.1.2.28 string vlan1302
.1.3.6.1.2.1.2.2.1.2.29 string vlan1305
.1.3.6.1.2.1.2.2.1.2.30 string vlan1251
.1.3.6.1.2.1.2.2.1.3.1 integer 24
.1.3.6.1.2.1.2.2.1.3.10 integer 6
.1.3.6.1.2.1.2.2.1.3.11 integer 6
.1.3.6.1.2.1.2.2.1.3.12 integer 6
.1.3.6.1.2.1.2.2.1.3.13 integer 6
.1.3.6.1.2.1.2.2.1.3.14 integer 6
.1.3.6.1.2.1.2.2.1.3.15 integer 6
.1.3.6.1.2.1.2.2.1.3.16 integer 6
.1.3.6.1.2.1.2.2.1.3.17 integer 6
.1.3.6.1.2.1.2.2.1.3.18 integer 6
.1.3.6.1.2.1.2.2.1.3.19 integer 6
.1.3.6.1.2.1.2.2.1.3.22 integer 53
.1.3.6.1.2.1.2.2.1.3.23 integer 53
.1.3.6.1.2.1.2.2.1.3.26 integer 53
.1.3.6.1.2.1.2.2.1.3.28 integer 53
.1.3.6.1.2.1.2.2.1.3.29 integer 53
.1.3.6.1.2.1.2.2.1.3.30 integer 53
.1.3.6.1.2.1.2.2.1.5.1 gauge 0
.1.3.6.1.2.1.2.2.1.5.10 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.11 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.12 gauge 0
.1.3.6.1.2.1.2.2.1.5.13 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.14 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.15 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.16 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.17 gauge 100000000
.1.3.6.1.2.1.2.2.1.5.18 gauge 0
.1.3.6.1.2.1.2.2.1.5.19 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.22 gauge 0
.1.3.6.1.2.1.2.2.1.5.23 gauge 0
.1.3.6.1.2.1.2.2.1.5.26 gauge 0
.1.3.6.1.2.1.2.2.1.5.28 gauge 0
.1.3.6.1.2.1.2.2.1.5.29 gauge 0
.1.3.6.1.2.1.2.2.1.5.30 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 octet 00 00 00 00 00 00
.1.3.6.1.2.1.2.2.1.6.10 octet 00 07 7c 20 01 ea
.1.3.6.1.2.1.2.2.1.6.11 octet 00 07 7c 20 01 e9
.1.3.6.1.2.1.2.2.1.6.12 octet 00 07 7c 20 01 e8
.1.3.6.1.2.1.2.2.1.6.13 octet 00 07 7c 20 01 e7
.1.3.6.1.2.1.2.2.1.6.14 octet 00 07 7c 20 01 e6
.1.3.6.1.2.1.2.2.1.6.15 octet 00 07 7c 20 01 e5
.1.3.6.1.2.1.2.2.1.6.16 octet 00 07 7c 20 01 e4
.1.3.6.1.2.1.2.2.1.6.17 octet 00 07 7c 20 01 e3
.1.3.6.1.2.1.2.2.1.6.18 octet 00 07 7c 20 01 e2
.1.3.6.1.2.1.2.2.1.6.19 octet 00 07 7c 20 01 e1
.1.3.6.1.2.1.2.2.1.6.22 octet 00 07 7c 20 01 e0
.1.3.6.1.2.1.2.2.1.6.23 octet 00 07 7c 20 01 e0
.1.3.6.1.2.1.2.2.1.6.26 octet 00 07 7c 20 01 e0
.1.3.6.1.2.1.2.2.1.6.28 octet 00 07 7c 20 01 e0
.1.3.6.1.2.1.2.2.1.6.29 octet 00 07 7c 20 01 e0
.1.3.6.1.2.1.2.2.1.6.30 octet 00 07 7c 20 01 e0
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.10 integer 1
.1.3.6.1.2.1.2.2.1.7.11 integer 1
.1.3.6.1.2.1.2.2.1.7.12 integer 1
.1.3.6.1.2.1.2.2.1.7.13 integer 1
.1.3.6.1.2.1.2.2.1.7.14 integer 1
.1.3.6.1.2.1.2.2.1.7.15 integer 1
.1.3.6.1.2.1.2.2.1.7.16 integer 1
.1.3.6.1.2.1.2.2.1.7.17 integer 1
.1.3.6.1.2.1.2.2.1.7.18 integer 1
.1.3.6.1.2.1.2.2.1.7.19 integer 1
.1.3.6.1.2.1.2.2.1.7.22 integer 1
.1.3.6.1.2.1.2.2.1.7.23 integer 1
.1.3.6.1.2.1.2.2.1.7.26 integer 1
.1.3.6.1.2.1.2.2.1.7.28 integer 1
.1.3.6.1.2.1.2.2.1.7.29 integer 1
.1.3.6.1.2.1.2.2.1.7.30 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.10 integer 1
.1.3.6.1.2.1.2.2.1.8.11 integer 1
.1.3.6.1.2.1.2.2.1.8.12 integer 2
.1.3.6.1.2.1.2.2.1.8.13 integer 1
.1.3.6.1.2.1.2.2.1.8.14 integer 1
.1.3.6.1.2.1.2.2.1.8.15 integer 1
.1.3.6.1.2.1.2.2.1.8.16 integer 1
.1.3.6.1.2.1.2.2.1.8.17 integer 1
.1.3.6.1.2.1.2.2.1.8.18 integer 2
.1.3.6.1.2.1.2.2.1.8.19 integer 1
.1.3.6.1.2.1.2.2.1.8.22 integer 2
.1.3.6.1.2.1.2.2.1.8.23 integer 1
.1.3.6.1.2.1.2.2.1.8.26 integer 1
.1.3.6.1.2.1.2.2.1.8.28 integer 1
.1.3.6.1.2.1.2.2.1.8.29 integer 1
.1.3.6.1.2.1.2.2.1.8.30 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string lo
.1.3.6.1.2.1.31.1.1.1.1.10 string eth10
.1.3.6.1.2.1.31.1.1.1.1.11 string eth9
.1.3.6.1.2.1.31.1.1.1.1.12 string eth8
.1.3.6.1.2.1.31.1.1.1.1.13 string eth7
.1.3.6.1.2.1.31.1.1.1.1.14 string eth6
.1.3.6.1.2.1.31.1.1.1.1.15 string eth5
.1.3.6.1.2.1.31.1.1.1.1.16 string eth4
.1.3.6.1.2.1.31.1.1.1.1.17 string eth3
.1.3.6.1.2.1.31.1.1.1.1.18 string eth2
.1.3.6.1.2.1.31.1.1.1.1.19 string eth1
.1.3.6.1.2.1.31.1.1.1.1.22 string vlan1
.1.3.6.1.2.1.31.1.1.1.1.23 string vlan6
.1.3.6.1.2.1.31.1.1.1.1.26 string vlan832
.1.3.6.1.2.1.31.1.1.1.1.28 string vlan1302
.1.3.6.1.2.1.31.1.1.1.1.29 string vlan1305
.1.3.6.1.2.1.31.1.1.1.1.30 string vlan1251
.1.3.6.1.2.1.31.1.1.1.18.1 string lo
.1.3.6.1.2.1.31.1.1.1.18.10 string eth10
.1.3.6.1.2.1.31.1.1.1.18.11 string eth9
.1.3.6.1.2.1.31.1.1.1.18.12 string eth8
.1.3.6.1.2.1.31.1.1.1.18.13 string eth7
.1.3.6.1.2.1.31.1.1.1.18.14 string eth6
.1.3.6.1.2.1.31.1.1.1.18.15 string eth5
.1.3.6.1.2.1.31.1.1.1.18.16 string eth4
.1.3.6.1.2.1.31.1.1.1.18.17 string eth3
.1.3.6.1.2.1.31.1.1.1.18.18 string eth2
.1.3.6.1.2.1.31.1.1.1.18.19 string eth1
.1.3.6.1.2.1.31.1.1.1.18.22 string vlan1
.1.3.6.1.2.1.31.1.1.1.18.23 string vlan6
.1.3.6.1.2.1.31.1.1.1.18.26 string vlan832
.1.3.6.1.2.1.31.1.1.1.18.28 string vlan1302
.1.3.6.1.2.1.31.1.1.1.18.29 string vlan1305
.1.3.6.1.2.1.31.1.1.1.18.30 string vlan1251
EOF

# switch-macport-01 LLDP — local ports keyed by ifIndex, remote ends of three different shapes.
#
# lldpLocPortNum is 10..19, which are exactly this device's ifIndex values: the local-port table is
# the identity mapping, and each port advertises subtype 3 with its own unique ifPhysAddress so the
# unique-MAC tier confirms it. The earlier version of this fixture modelled a device whose
# lldpLocPortNum was a separate namespace from ifIndex; the customer's walk shows it is not, which
# is why the local-port remap could never have been what broke this device. The reverse-numbering
# case that remap exists for is still covered, by unit test rather than by pretending this device
# is it.
#
# The three neighbours are the ones the real device reports, and each reaches its far end by a
# different route:
#
#   port 11 — chassis subtype 7 (local) "C230408" with **no sysName and no portDesc**. Nothing
#   about this identifies a MAC, an address or a name: the only way to find that device is
#   hosts.chassis_id, recorded from its own lldpLocChassisId. If those two paths ever canonicalise
#   differently the neighbour is unfindable, and nothing else in this environment covers it.
#
#   port 19 — an Extreme 5520 FabricEngine: chassis subtype 4 MAC, port id subtype 5 "1/19",
#   sysName present.
#
#   port 16 — a Lexmark printer advertising the same MAC as chassis and as port id, the ordinary
#   single-port-endpoint shape.
#
# `octet` throughout for the address columns, for the reason the ARP table above gives: `string`
# sends an address as text, and a MAC is six raw octets. The chassis id is deliberately sent as
# octets here while switch-dlink-01 names this same device with the text form, so one scan
# exercises both encodings reaching one identity.
cat > "$DATA_DIR/switch-macport-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 octet 00 07 7c 20 01 e0
.1.0.8802.1.1.2.1.3.3.0 string switch-macport-01
.1.0.8802.1.1.2.1.3.4.0 string WeOS 5.21.0 industrial ethernet switch
.1.0.8802.1.1.2.1.3.7.1.2.10 integer 3
.1.0.8802.1.1.2.1.3.7.1.2.11 integer 3
.1.0.8802.1.1.2.1.3.7.1.2.12 integer 3
.1.0.8802.1.1.2.1.3.7.1.2.13 integer 3
.1.0.8802.1.1.2.1.3.7.1.2.14 integer 3
.1.0.8802.1.1.2.1.3.7.1.2.15 integer 3
.1.0.8802.1.1.2.1.3.7.1.2.16 integer 3
.1.0.8802.1.1.2.1.3.7.1.2.17 integer 3
.1.0.8802.1.1.2.1.3.7.1.2.18 integer 3
.1.0.8802.1.1.2.1.3.7.1.2.19 integer 3
.1.0.8802.1.1.2.1.3.7.1.3.10 octet 00 07 7c 20 01 ea
.1.0.8802.1.1.2.1.3.7.1.3.11 octet 00 07 7c 20 01 e9
.1.0.8802.1.1.2.1.3.7.1.3.12 octet 00 07 7c 20 01 e8
.1.0.8802.1.1.2.1.3.7.1.3.13 octet 00 07 7c 20 01 e7
.1.0.8802.1.1.2.1.3.7.1.3.14 octet 00 07 7c 20 01 e6
.1.0.8802.1.1.2.1.3.7.1.3.15 octet 00 07 7c 20 01 e5
.1.0.8802.1.1.2.1.3.7.1.3.16 octet 00 07 7c 20 01 e4
.1.0.8802.1.1.2.1.3.7.1.3.17 octet 00 07 7c 20 01 e3
.1.0.8802.1.1.2.1.3.7.1.3.18 octet 00 07 7c 20 01 e2
.1.0.8802.1.1.2.1.3.7.1.3.19 octet 00 07 7c 20 01 e1
.1.0.8802.1.1.2.1.3.7.1.4.10 string 100-T eth10
.1.0.8802.1.1.2.1.3.7.1.4.11 string 100-T eth9
.1.0.8802.1.1.2.1.3.7.1.4.12 string 100-T eth8
.1.0.8802.1.1.2.1.3.7.1.4.13 string 100-T eth7
.1.0.8802.1.1.2.1.3.7.1.4.14 string 100-T eth6
.1.0.8802.1.1.2.1.3.7.1.4.15 string 100-T eth5
.1.0.8802.1.1.2.1.3.7.1.4.16 string 100-T eth4
.1.0.8802.1.1.2.1.3.7.1.4.17 string 100-T eth3
.1.0.8802.1.1.2.1.3.7.1.4.18 string 1000-T eth2
.1.0.8802.1.1.2.1.3.7.1.4.19 string 1000-LX eth1
.1.0.8802.1.1.2.1.4.1.1.4.100.11.1 integer 7
.1.0.8802.1.1.2.1.4.1.1.4.500.19.2 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.1400.16.3 integer 4
.1.0.8802.1.1.2.1.4.1.1.5.100.11.1 string C230408
.1.0.8802.1.1.2.1.4.1.1.5.500.19.2 octet f0 64 26 b3 84 00
.1.0.8802.1.1.2.1.4.1.1.5.1400.16.3 octet 78 8c 77 e5 92 7d
.1.0.8802.1.1.2.1.4.1.1.6.100.11.1 integer 3
.1.0.8802.1.1.2.1.4.1.1.6.500.19.2 integer 5
.1.0.8802.1.1.2.1.4.1.1.6.1400.16.3 integer 3
.1.0.8802.1.1.2.1.4.1.1.7.100.11.1 octet e8 80 88 be 30 e7
.1.0.8802.1.1.2.1.4.1.1.7.500.19.2 string 1/19
.1.0.8802.1.1.2.1.4.1.1.7.1400.16.3 octet 78 8c 77 e5 92 7d
.1.0.8802.1.1.2.1.4.1.1.8.500.19.2 string Extreme Networks 5520-24X-FabricEngine - GbicLx Port 1/19
.1.0.8802.1.1.2.1.4.1.1.8.1400.16.3 string eth0
.1.0.8802.1.1.2.1.4.1.1.9.500.19.2 string VSAFC11
.1.0.8802.1.1.2.1.4.1.1.9.1400.16.3 string M300.printers.motala.se
.1.0.8802.1.1.2.1.4.1.1.10.500.19.2 string 5520-24X-FabricEngine (9.3.1.0)
.1.0.8802.1.1.2.1.4.1.1.10.1400.16.3 string Lexmark Poky (Yocto Project Reference Distro) 4.0.14 (kirkstone) Linux 5.15.58-yocto-standard aarch64
EOF

# switch-mute-01 — answers the credential and serves nothing.
#
# An empty file, pointed at by every table the built-in modules would otherwise answer from the
# VM's own kernel state. Without these the "device" would report the VM's real addresses and ARP
# cache and would not be mute at all. ifTable/ifXTable are suppressed by the -I flag the units
# already carry; ipAddrTable and ipNetToMediaTable cannot be, so they are overridden per column
# at priority 1, the same technique ap-wireless-01 uses to serve its own ipAddrTable.
: > "$DATA_DIR/switch-mute-01-empty.txt"

# switch-stuck-01 — one ARP row, served for ever.
#
# The file holds a single line and the stuck handler returns it for every GETNEXT, so the walk
# collects it once and then sees the same OID again. The walk must recognise a page that
# contributes nothing new, retry its budget and stop — and report the ARP table as a walk that
# fell short rather than as a table that ended.
cat > "$DATA_DIR/switch-stuck-01-arp.txt" << 'EOF'
.1.3.6.1.2.1.4.22.1.1.1.10.40.50.1 integer 1
EOF

# switch-stuck-01 interfaces — ordinary, so this device is a *shortfall* case and not a mute one.
cat > "$DATA_DIR/switch-stuck-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.2.1 string ether1
.1.3.6.1.2.1.2.2.1.2.2 string ether2
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.6.1 octet 00 0c 42 7a 00 01
.1.3.6.1.2.1.2.2.1.6.2 octet 00 0c 42 7a 00 02
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string ether1
.1.3.6.1.2.1.31.1.1.1.1.2 string ether2
EOF

# switch-dell-01 IF-MIB — a Dell PowerSwitch S4112T-ON running OS10, port 14 broken out.
#
# 23 interfaces, as the reporter's switch has, and the reason this device is here is what they are
# called. OS10 names a breakout lane `ethernet1/1/14:1`, so one interface name carries both of the
# characters the local-port suffix tier anchors on — and `mgmt1/1/1` ends in the same `/1` those
# lanes end in. Nothing else in this environment has a name sharing a boundary with three other
# names on the same device (GH #685).
#
# ifIndex is OS10's own numbering, nowhere near the lldpLocPortNum values below: the two
# namespaces have to be joined through lldpLocPortTable, never by arithmetic or coincidence.
cat > "$DATA_DIR/switch-dell-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.17301505 integer 17301505
.1.3.6.1.2.1.2.2.1.1.17301506 integer 17301506
.1.3.6.1.2.1.2.2.1.1.17301507 integer 17301507
.1.3.6.1.2.1.2.2.1.1.17301508 integer 17301508
.1.3.6.1.2.1.2.2.1.1.17301509 integer 17301509
.1.3.6.1.2.1.2.2.1.1.17301510 integer 17301510
.1.3.6.1.2.1.2.2.1.1.17301511 integer 17301511
.1.3.6.1.2.1.2.2.1.1.17301512 integer 17301512
.1.3.6.1.2.1.2.2.1.1.17301513 integer 17301513
.1.3.6.1.2.1.2.2.1.1.17301514 integer 17301514
.1.3.6.1.2.1.2.2.1.1.17301515 integer 17301515
.1.3.6.1.2.1.2.2.1.1.17301516 integer 17301516
.1.3.6.1.2.1.2.2.1.1.17301517 integer 17301517
.1.3.6.1.2.1.2.2.1.1.17301518 integer 17301518
.1.3.6.1.2.1.2.2.1.1.17301519 integer 17301519
.1.3.6.1.2.1.2.2.1.1.17301520 integer 17301520
.1.3.6.1.2.1.2.2.1.1.22020097 integer 22020097
.1.3.6.1.2.1.2.2.1.1.22020106 integer 22020106
.1.3.6.1.2.1.2.2.1.1.35127296 integer 35127296
.1.3.6.1.2.1.2.2.1.1.1107787777 integer 1107787777
.1.3.6.1.2.1.2.2.1.1.1107787876 integer 1107787876
.1.3.6.1.2.1.2.2.1.1.1107787976 integer 1107787976
.1.3.6.1.2.1.2.2.1.2.1 string lo
.1.3.6.1.2.1.2.2.1.2.17301505 string ethernet1/1/1
.1.3.6.1.2.1.2.2.1.2.17301506 string ethernet1/1/2
.1.3.6.1.2.1.2.2.1.2.17301507 string ethernet1/1/3
.1.3.6.1.2.1.2.2.1.2.17301508 string ethernet1/1/4
.1.3.6.1.2.1.2.2.1.2.17301509 string ethernet1/1/5
.1.3.6.1.2.1.2.2.1.2.17301510 string ethernet1/1/6
.1.3.6.1.2.1.2.2.1.2.17301511 string ethernet1/1/7
.1.3.6.1.2.1.2.2.1.2.17301512 string ethernet1/1/8
.1.3.6.1.2.1.2.2.1.2.17301513 string ethernet1/1/9
.1.3.6.1.2.1.2.2.1.2.17301514 string ethernet1/1/10
.1.3.6.1.2.1.2.2.1.2.17301515 string ethernet1/1/11
.1.3.6.1.2.1.2.2.1.2.17301516 string ethernet1/1/12
.1.3.6.1.2.1.2.2.1.2.17301517 string ethernet1/1/13
.1.3.6.1.2.1.2.2.1.2.17301518 string ethernet1/1/14:1
.1.3.6.1.2.1.2.2.1.2.17301519 string ethernet1/1/14:2
.1.3.6.1.2.1.2.2.1.2.17301520 string ethernet1/1/14:3
.1.3.6.1.2.1.2.2.1.2.22020097 string port-channel1
.1.3.6.1.2.1.2.2.1.2.22020106 string port-channel10
.1.3.6.1.2.1.2.2.1.2.35127296 string mgmt1/1/1
.1.3.6.1.2.1.2.2.1.2.1107787777 string vlan1
.1.3.6.1.2.1.2.2.1.2.1107787876 string vlan100
.1.3.6.1.2.1.2.2.1.2.1107787976 string vlan200
.1.3.6.1.2.1.2.2.1.3.1 integer 24
.1.3.6.1.2.1.2.2.1.3.17301505 integer 6
.1.3.6.1.2.1.2.2.1.3.17301506 integer 6
.1.3.6.1.2.1.2.2.1.3.17301507 integer 6
.1.3.6.1.2.1.2.2.1.3.17301508 integer 6
.1.3.6.1.2.1.2.2.1.3.17301509 integer 6
.1.3.6.1.2.1.2.2.1.3.17301510 integer 6
.1.3.6.1.2.1.2.2.1.3.17301511 integer 6
.1.3.6.1.2.1.2.2.1.3.17301512 integer 6
.1.3.6.1.2.1.2.2.1.3.17301513 integer 6
.1.3.6.1.2.1.2.2.1.3.17301514 integer 6
.1.3.6.1.2.1.2.2.1.3.17301515 integer 6
.1.3.6.1.2.1.2.2.1.3.17301516 integer 6
.1.3.6.1.2.1.2.2.1.3.17301517 integer 6
.1.3.6.1.2.1.2.2.1.3.17301518 integer 6
.1.3.6.1.2.1.2.2.1.3.17301519 integer 6
.1.3.6.1.2.1.2.2.1.3.17301520 integer 6
.1.3.6.1.2.1.2.2.1.3.22020097 integer 161
.1.3.6.1.2.1.2.2.1.3.22020106 integer 161
.1.3.6.1.2.1.2.2.1.3.35127296 integer 6
.1.3.6.1.2.1.2.2.1.3.1107787777 integer 53
.1.3.6.1.2.1.2.2.1.3.1107787876 integer 53
.1.3.6.1.2.1.2.2.1.3.1107787976 integer 53
.1.3.6.1.2.1.2.2.1.4.1 integer 65535
.1.3.6.1.2.1.2.2.1.4.17301505 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301506 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301507 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301508 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301509 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301510 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301511 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301512 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301513 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301514 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301515 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301516 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301517 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301518 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301519 integer 1532
.1.3.6.1.2.1.2.2.1.4.17301520 integer 1532
.1.3.6.1.2.1.2.2.1.4.22020097 integer 1532
.1.3.6.1.2.1.2.2.1.4.22020106 integer 1532
.1.3.6.1.2.1.2.2.1.4.35127296 integer 1532
.1.3.6.1.2.1.2.2.1.4.1107787777 integer 1532
.1.3.6.1.2.1.2.2.1.4.1107787876 integer 1532
.1.3.6.1.2.1.2.2.1.4.1107787976 integer 1532
.1.3.6.1.2.1.2.2.1.5.1 gauge 0
.1.3.6.1.2.1.2.2.1.5.17301505 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301506 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301507 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301508 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301509 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301510 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301511 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301512 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301513 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301514 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301515 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301516 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301517 gauge 10000000000
.1.3.6.1.2.1.2.2.1.5.17301518 gauge 25000000000
.1.3.6.1.2.1.2.2.1.5.17301519 gauge 25000000000
.1.3.6.1.2.1.2.2.1.5.17301520 gauge 25000000000
.1.3.6.1.2.1.2.2.1.5.22020097 gauge 20000000000
.1.3.6.1.2.1.2.2.1.5.22020106 gauge 0
.1.3.6.1.2.1.2.2.1.5.35127296 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.1107787777 gauge 0
.1.3.6.1.2.1.2.2.1.5.1107787876 gauge 0
.1.3.6.1.2.1.2.2.1.5.1107787976 gauge 0
.1.3.6.1.2.1.2.2.1.6.17301505 octet 14 18 77 aa bb 11
.1.3.6.1.2.1.2.2.1.6.17301506 octet 14 18 77 aa bb 12
.1.3.6.1.2.1.2.2.1.6.17301507 octet 14 18 77 aa bb 13
.1.3.6.1.2.1.2.2.1.6.17301508 octet 14 18 77 aa bb 14
.1.3.6.1.2.1.2.2.1.6.17301509 octet 14 18 77 aa bb 15
.1.3.6.1.2.1.2.2.1.6.17301510 octet 14 18 77 aa bb 16
.1.3.6.1.2.1.2.2.1.6.17301511 octet 14 18 77 aa bb 17
.1.3.6.1.2.1.2.2.1.6.17301512 octet 14 18 77 aa bb 18
.1.3.6.1.2.1.2.2.1.6.17301513 octet 14 18 77 aa bb 19
.1.3.6.1.2.1.2.2.1.6.17301514 octet 14 18 77 aa bb 1a
.1.3.6.1.2.1.2.2.1.6.17301515 octet 14 18 77 aa bb 1b
.1.3.6.1.2.1.2.2.1.6.17301516 octet 14 18 77 aa bb 1c
.1.3.6.1.2.1.2.2.1.6.17301517 octet 14 18 77 aa bb 1d
.1.3.6.1.2.1.2.2.1.6.17301518 octet 14 18 77 aa bb 21
.1.3.6.1.2.1.2.2.1.6.17301519 octet 14 18 77 aa bb 22
.1.3.6.1.2.1.2.2.1.6.17301520 octet 14 18 77 aa bb 23
.1.3.6.1.2.1.2.2.1.6.35127296 octet 14 18 77 aa bb 01
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.17301505 integer 1
.1.3.6.1.2.1.2.2.1.7.17301506 integer 1
.1.3.6.1.2.1.2.2.1.7.17301507 integer 1
.1.3.6.1.2.1.2.2.1.7.17301508 integer 1
.1.3.6.1.2.1.2.2.1.7.17301509 integer 1
.1.3.6.1.2.1.2.2.1.7.17301510 integer 1
.1.3.6.1.2.1.2.2.1.7.17301511 integer 1
.1.3.6.1.2.1.2.2.1.7.17301512 integer 1
.1.3.6.1.2.1.2.2.1.7.17301513 integer 1
.1.3.6.1.2.1.2.2.1.7.17301514 integer 1
.1.3.6.1.2.1.2.2.1.7.17301515 integer 1
.1.3.6.1.2.1.2.2.1.7.17301516 integer 1
.1.3.6.1.2.1.2.2.1.7.17301517 integer 1
.1.3.6.1.2.1.2.2.1.7.17301518 integer 1
.1.3.6.1.2.1.2.2.1.7.17301519 integer 1
.1.3.6.1.2.1.2.2.1.7.17301520 integer 1
.1.3.6.1.2.1.2.2.1.7.22020097 integer 1
.1.3.6.1.2.1.2.2.1.7.22020106 integer 2
.1.3.6.1.2.1.2.2.1.7.35127296 integer 1
.1.3.6.1.2.1.2.2.1.7.1107787777 integer 1
.1.3.6.1.2.1.2.2.1.7.1107787876 integer 1
.1.3.6.1.2.1.2.2.1.7.1107787976 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.17301505 integer 1
.1.3.6.1.2.1.2.2.1.8.17301506 integer 1
.1.3.6.1.2.1.2.2.1.8.17301507 integer 1
.1.3.6.1.2.1.2.2.1.8.17301508 integer 1
.1.3.6.1.2.1.2.2.1.8.17301509 integer 2
.1.3.6.1.2.1.2.2.1.8.17301510 integer 2
.1.3.6.1.2.1.2.2.1.8.17301511 integer 2
.1.3.6.1.2.1.2.2.1.8.17301512 integer 2
.1.3.6.1.2.1.2.2.1.8.17301513 integer 2
.1.3.6.1.2.1.2.2.1.8.17301514 integer 2
.1.3.6.1.2.1.2.2.1.8.17301515 integer 2
.1.3.6.1.2.1.2.2.1.8.17301516 integer 2
.1.3.6.1.2.1.2.2.1.8.17301517 integer 2
.1.3.6.1.2.1.2.2.1.8.17301518 integer 1
.1.3.6.1.2.1.2.2.1.8.17301519 integer 1
.1.3.6.1.2.1.2.2.1.8.17301520 integer 1
.1.3.6.1.2.1.2.2.1.8.22020097 integer 2
.1.3.6.1.2.1.2.2.1.8.22020106 integer 2
.1.3.6.1.2.1.2.2.1.8.35127296 integer 1
.1.3.6.1.2.1.2.2.1.8.1107787777 integer 1
.1.3.6.1.2.1.2.2.1.8.1107787876 integer 1
.1.3.6.1.2.1.2.2.1.8.1107787976 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string lo
.1.3.6.1.2.1.31.1.1.1.1.17301505 string ethernet1/1/1
.1.3.6.1.2.1.31.1.1.1.1.17301506 string ethernet1/1/2
.1.3.6.1.2.1.31.1.1.1.1.17301507 string ethernet1/1/3
.1.3.6.1.2.1.31.1.1.1.1.17301508 string ethernet1/1/4
.1.3.6.1.2.1.31.1.1.1.1.17301509 string ethernet1/1/5
.1.3.6.1.2.1.31.1.1.1.1.17301510 string ethernet1/1/6
.1.3.6.1.2.1.31.1.1.1.1.17301511 string ethernet1/1/7
.1.3.6.1.2.1.31.1.1.1.1.17301512 string ethernet1/1/8
.1.3.6.1.2.1.31.1.1.1.1.17301513 string ethernet1/1/9
.1.3.6.1.2.1.31.1.1.1.1.17301514 string ethernet1/1/10
.1.3.6.1.2.1.31.1.1.1.1.17301515 string ethernet1/1/11
.1.3.6.1.2.1.31.1.1.1.1.17301516 string ethernet1/1/12
.1.3.6.1.2.1.31.1.1.1.1.17301517 string ethernet1/1/13
.1.3.6.1.2.1.31.1.1.1.1.17301518 string ethernet1/1/14:1
.1.3.6.1.2.1.31.1.1.1.1.17301519 string ethernet1/1/14:2
.1.3.6.1.2.1.31.1.1.1.1.17301520 string ethernet1/1/14:3
.1.3.6.1.2.1.31.1.1.1.1.22020097 string port-channel1
.1.3.6.1.2.1.31.1.1.1.1.22020106 string port-channel10
.1.3.6.1.2.1.31.1.1.1.1.35127296 string mgmt1/1/1
.1.3.6.1.2.1.31.1.1.1.1.1107787777 string vlan1
.1.3.6.1.2.1.31.1.1.1.1.1107787876 string vlan100
.1.3.6.1.2.1.31.1.1.1.1.1107787976 string vlan200
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 0
.1.3.6.1.2.1.31.1.1.1.15.17301505 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301506 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301507 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301508 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301509 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301510 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301511 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301512 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301513 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301514 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301515 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301516 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301517 gauge 10000
.1.3.6.1.2.1.31.1.1.1.15.17301518 gauge 25000
.1.3.6.1.2.1.31.1.1.1.15.17301519 gauge 25000
.1.3.6.1.2.1.31.1.1.1.15.17301520 gauge 25000
.1.3.6.1.2.1.31.1.1.1.15.22020097 gauge 20000
.1.3.6.1.2.1.31.1.1.1.15.22020106 gauge 0
.1.3.6.1.2.1.31.1.1.1.15.35127296 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.1107787777 gauge 0
.1.3.6.1.2.1.31.1.1.1.15.1107787876 gauge 0
.1.3.6.1.2.1.31.1.1.1.15.1107787976 gauge 0
.1.3.6.1.2.1.31.1.1.1.18.17301518 string breakout lane 1
.1.3.6.1.2.1.31.1.1.1.18.17301519 string breakout lane 2
.1.3.6.1.2.1.31.1.1.1.18.17301520 string breakout lane 3
.1.3.6.1.2.1.31.1.1.1.18.22020097 string uplink lag
.1.3.6.1.2.1.31.1.1.1.18.35127296 string out of band
EOF

# switch-dell-01 LLDP — the neighbour table from GH #685.
#
# Two things about the remote rows matter, both from the reporter's own walk:
#
#   lldpRemEntry is indexed lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex, and the time marks
#   here are large and hundreds of thousands of ticks apart — 31577700, 93300700, 123380800,
#   127153800. Every other device here uses 0 or a small mark, so nothing else walks a first index
#   sub-id of this size, and the local ports (570, 4, 568, 569) consequently arrive in an order
#   that has nothing to do with the ports themselves.
#
#   lldpLocPortNum is a separate namespace from ifIndex: 4 for the management port and 555-570 for
#   the front panel, against interfaces numbered in the millions. The three neighbours on port 14
#   sit on its breakout lanes, which is the mapping the reporter published:
#
#     4   -> mgmt1/1/1        (a host advertising only its MAC — no sysName, no port description)
#     568 -> ethernet1/1/14:1 -> EVILCORP
#     569 -> ethernet1/1/14:2 -> VIRTUALPC
#     570 -> ethernet1/1/14:3 -> TAMMIERENEW
#
# Three of the four advertise chassis subtype 7 (locally assigned) carrying a hostname rather than
# a MAC, which is what the reporter's walk shows and what an end host running LLDP typically does.
# The fourth is subtype 4 and sends six raw octets, so one device exercises both encodings.
#
# What this device does *not* stage is the walk falling short, which is the other half of #685: a
# `pass` agent answers single-threaded, so a handler that stalls long enough to time the daemon out
# blocks every later request behind it and the late answers arrive one request out of step for the
# rest of the walk — the fixture would then fail whether or not the fix is in. The transport
# retries are covered by unit test instead, the way `WalkCutShort` already is.
cat > "$DATA_DIR/switch-dell-01-lldp.txt" << 'EOF'
.1.0.8802.1.1.2.1.3.1.0 integer 4
.1.0.8802.1.1.2.1.3.2.0 octet 14 18 77 aa bb 00
.1.0.8802.1.1.2.1.3.3.0 string switch-dell-01
.1.0.8802.1.1.2.1.3.4.0 string Dell EMC Networking OS10 Enterprise. Dell EMC Networking S4112T-ON. OS Version 10.4.3.4
.1.0.8802.1.1.2.1.3.7.1.2.4 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.555 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.556 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.557 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.558 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.559 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.560 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.561 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.562 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.563 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.564 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.565 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.566 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.567 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.568 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.569 integer 5
.1.0.8802.1.1.2.1.3.7.1.2.570 integer 5
.1.0.8802.1.1.2.1.3.7.1.3.4 string mgmt1/1/1
.1.0.8802.1.1.2.1.3.7.1.3.555 string ethernet1/1/1
.1.0.8802.1.1.2.1.3.7.1.3.556 string ethernet1/1/2
.1.0.8802.1.1.2.1.3.7.1.3.557 string ethernet1/1/3
.1.0.8802.1.1.2.1.3.7.1.3.558 string ethernet1/1/4
.1.0.8802.1.1.2.1.3.7.1.3.559 string ethernet1/1/5
.1.0.8802.1.1.2.1.3.7.1.3.560 string ethernet1/1/6
.1.0.8802.1.1.2.1.3.7.1.3.561 string ethernet1/1/7
.1.0.8802.1.1.2.1.3.7.1.3.562 string ethernet1/1/8
.1.0.8802.1.1.2.1.3.7.1.3.563 string ethernet1/1/9
.1.0.8802.1.1.2.1.3.7.1.3.564 string ethernet1/1/10
.1.0.8802.1.1.2.1.3.7.1.3.565 string ethernet1/1/11
.1.0.8802.1.1.2.1.3.7.1.3.566 string ethernet1/1/12
.1.0.8802.1.1.2.1.3.7.1.3.567 string ethernet1/1/13
.1.0.8802.1.1.2.1.3.7.1.3.568 string ethernet1/1/14:1
.1.0.8802.1.1.2.1.3.7.1.3.569 string ethernet1/1/14:2
.1.0.8802.1.1.2.1.3.7.1.3.570 string ethernet1/1/14:3
.1.0.8802.1.1.2.1.3.7.1.4.4 string mgmt1/1/1
.1.0.8802.1.1.2.1.3.7.1.4.555 string ethernet1/1/1
.1.0.8802.1.1.2.1.3.7.1.4.556 string ethernet1/1/2
.1.0.8802.1.1.2.1.3.7.1.4.557 string ethernet1/1/3
.1.0.8802.1.1.2.1.3.7.1.4.558 string ethernet1/1/4
.1.0.8802.1.1.2.1.3.7.1.4.559 string ethernet1/1/5
.1.0.8802.1.1.2.1.3.7.1.4.560 string ethernet1/1/6
.1.0.8802.1.1.2.1.3.7.1.4.561 string ethernet1/1/7
.1.0.8802.1.1.2.1.3.7.1.4.562 string ethernet1/1/8
.1.0.8802.1.1.2.1.3.7.1.4.563 string ethernet1/1/9
.1.0.8802.1.1.2.1.3.7.1.4.564 string ethernet1/1/10
.1.0.8802.1.1.2.1.3.7.1.4.565 string ethernet1/1/11
.1.0.8802.1.1.2.1.3.7.1.4.566 string ethernet1/1/12
.1.0.8802.1.1.2.1.3.7.1.4.567 string ethernet1/1/13
.1.0.8802.1.1.2.1.3.7.1.4.568 string ethernet1/1/14:1
.1.0.8802.1.1.2.1.3.7.1.4.569 string ethernet1/1/14:2
.1.0.8802.1.1.2.1.3.7.1.4.570 string ethernet1/1/14:3
.1.0.8802.1.1.2.1.4.1.1.4.31577700.570.55 integer 7
.1.0.8802.1.1.2.1.4.1.1.4.93300700.4.78 integer 4
.1.0.8802.1.1.2.1.4.1.1.4.123380800.568.85 integer 7
.1.0.8802.1.1.2.1.4.1.1.4.127153800.569.87 integer 7
.1.0.8802.1.1.2.1.4.1.1.5.31577700.570.55 string TAMMIERENEW
.1.0.8802.1.1.2.1.4.1.1.5.93300700.4.78 octet f6 6b d4 b4 b9 df
.1.0.8802.1.1.2.1.4.1.1.5.123380800.568.85 string EVILCORP
.1.0.8802.1.1.2.1.4.1.1.5.127153800.569.87 string VIRTUALPC
.1.0.8802.1.1.2.1.4.1.1.6.31577700.570.55 integer 3
.1.0.8802.1.1.2.1.4.1.1.6.93300700.4.78 integer 3
.1.0.8802.1.1.2.1.4.1.1.6.123380800.568.85 integer 3
.1.0.8802.1.1.2.1.4.1.1.6.127153800.569.87 integer 3
.1.0.8802.1.1.2.1.4.1.1.7.31577700.570.55 octet 9c 6b 00 41 8d 21
.1.0.8802.1.1.2.1.4.1.1.7.93300700.4.78 octet f6 6b d4 b4 b9 df
.1.0.8802.1.1.2.1.4.1.1.7.123380800.568.85 octet 3c ec ef 40 12 aa
.1.0.8802.1.1.2.1.4.1.1.7.127153800.569.87 octet 00 15 5d 01 64 0c
.1.0.8802.1.1.2.1.4.1.1.8.31577700.570.55 string Realtek PCIe GbE Family Controller
.1.0.8802.1.1.2.1.4.1.1.8.123380800.568.85 string Intel(R) Ethernet Controller X550
.1.0.8802.1.1.2.1.4.1.1.8.127153800.569.87 string Hyper-V Virtual Ethernet Adapter
.1.0.8802.1.1.2.1.4.1.1.9.31577700.570.55 string TAMMIERENEW
.1.0.8802.1.1.2.1.4.1.1.9.123380800.568.85 string EVILCORP
.1.0.8802.1.1.2.1.4.1.1.9.127153800.569.87 string VIRTUALPC
.1.0.8802.1.1.2.1.4.1.1.10.31577700.570.55 string Windows 11 Pro 10.0.26100 x64
.1.0.8802.1.1.2.1.4.1.1.10.123380800.568.85 string Ubuntu 24.04.1 LTS Linux 6.8.0-51-generic x86_64
.1.0.8802.1.1.2.1.4.1.1.10.127153800.569.87 string Windows Server 2022 Datacenter 10.0.20348 x64
EOF

# switch-dell-01's bridge tables — the GH #686 shape on the GH #685 device.
#
# The reporter's Catalyst answers `dot1dTpFdbAddress` with nine rows to a raw walk and exactly
# one to a scan. Nine rows here, so a walk that stops after the first is unmistakable in the
# `count=` on the collection line rather than being a plausible number for a quiet switch.
#
# Bridge ports are numbered 1-12 against OS10's `ethernet1/1/N` ifIndexes (17301505+), which is
# the mapping the FDB is keyed by — a device whose `dot1dBasePortIfIndex` and FDB disagree
# resolves every entry to nothing, and that is worth being able to stage here.
#
# Far ends are the same lab devices switch-core-01 forwards to, so entries resolve to hosts
# rather than to nothing, plus one entry per breakout lane on port 14 — the case that made this
# device worth a fixture in the first place.
cat > "$DATA_DIR/switch-dell-01-bridge.txt" << 'EOF'
.1.3.6.1.2.1.17.1.4.1.2.1 integer 17301505
.1.3.6.1.2.1.17.1.4.1.2.2 integer 17301506
.1.3.6.1.2.1.17.1.4.1.2.3 integer 17301507
.1.3.6.1.2.1.17.1.4.1.2.4 integer 17301508
.1.3.6.1.2.1.17.1.4.1.2.10 integer 17301514
.1.3.6.1.2.1.17.1.4.1.2.11 integer 17301515
.1.3.6.1.2.1.17.1.4.1.2.12 integer 17301516
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.16.1 octet 00 1a 2b 00 10 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.17.1 octet 00 1a 2b 00 11 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.18.1 octet 00 1a 2b 00 12 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.19.1 octet 00 1a 2b 00 13 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.20.1 octet 00 1a 2b 00 14 01
.1.3.6.1.2.1.17.4.3.1.1.0.26.43.0.21.1 octet 00 1a 2b 00 15 01
.1.3.6.1.2.1.17.4.3.1.1.20.24.119.170.187.17 octet 14 18 77 aa bb 11
.1.3.6.1.2.1.17.4.3.1.1.20.24.119.170.187.18 octet 14 18 77 aa bb 12
.1.3.6.1.2.1.17.4.3.1.1.20.24.119.170.187.19 octet 14 18 77 aa bb 13
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.16.1 integer 1
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.17.1 integer 1
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.18.1 integer 2
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.19.1 integer 3
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.20.1 integer 4
.1.3.6.1.2.1.17.4.3.1.2.0.26.43.0.21.1 integer 4
.1.3.6.1.2.1.17.4.3.1.2.20.24.119.170.187.17 integer 10
.1.3.6.1.2.1.17.4.3.1.2.20.24.119.170.187.18 integer 11
.1.3.6.1.2.1.17.4.3.1.2.20.24.119.170.187.19 integer 12
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.16.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.17.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.18.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.19.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.20.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.26.43.0.21.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.20.24.119.170.187.17 integer 3
.1.3.6.1.2.1.17.4.3.1.3.20.24.119.170.187.18 integer 3
.1.3.6.1.2.1.17.4.3.1.3.20.24.119.170.187.19 integer 3
EOF

# switch-cisco-01 IF-MIB — a Catalyst 3850 running IOS-XE, from GH #686.
cat > "$DATA_DIR/switch-cisco-01-iftable.txt" << 'EOF'
.1.3.6.1.2.1.2.2.1.1.1 integer 1
.1.3.6.1.2.1.2.2.1.1.2 integer 2
.1.3.6.1.2.1.2.2.1.1.3 integer 3
.1.3.6.1.2.1.2.2.1.1.4 integer 4
.1.3.6.1.2.1.2.2.1.1.5 integer 5
.1.3.6.1.2.1.2.2.1.1.6 integer 6
.1.3.6.1.2.1.2.2.1.1.7 integer 7
.1.3.6.1.2.1.2.2.1.1.8 integer 8
.1.3.6.1.2.1.2.2.1.1.101 integer 101
.1.3.6.1.2.1.2.2.1.1.120 integer 120
.1.3.6.1.2.1.2.2.1.2.1 string GigabitEthernet1/0/1
.1.3.6.1.2.1.2.2.1.2.2 string GigabitEthernet1/0/2
.1.3.6.1.2.1.2.2.1.2.3 string GigabitEthernet1/0/3
.1.3.6.1.2.1.2.2.1.2.4 string GigabitEthernet1/0/4
.1.3.6.1.2.1.2.2.1.2.5 string GigabitEthernet1/0/5
.1.3.6.1.2.1.2.2.1.2.6 string GigabitEthernet1/0/6
.1.3.6.1.2.1.2.2.1.2.7 string GigabitEthernet1/0/7
.1.3.6.1.2.1.2.2.1.2.8 string GigabitEthernet1/0/8
.1.3.6.1.2.1.2.2.1.2.101 string Vlan1
.1.3.6.1.2.1.2.2.1.2.120 string Vlan20
.1.3.6.1.2.1.2.2.1.3.1 integer 6
.1.3.6.1.2.1.2.2.1.3.2 integer 6
.1.3.6.1.2.1.2.2.1.3.3 integer 6
.1.3.6.1.2.1.2.2.1.3.4 integer 6
.1.3.6.1.2.1.2.2.1.3.5 integer 6
.1.3.6.1.2.1.2.2.1.3.6 integer 6
.1.3.6.1.2.1.2.2.1.3.7 integer 6
.1.3.6.1.2.1.2.2.1.3.8 integer 6
.1.3.6.1.2.1.2.2.1.3.101 integer 53
.1.3.6.1.2.1.2.2.1.3.120 integer 53
.1.3.6.1.2.1.2.2.1.4.1 integer 1500
.1.3.6.1.2.1.2.2.1.4.2 integer 1500
.1.3.6.1.2.1.2.2.1.4.3 integer 1500
.1.3.6.1.2.1.2.2.1.4.4 integer 1500
.1.3.6.1.2.1.2.2.1.4.5 integer 1500
.1.3.6.1.2.1.2.2.1.4.6 integer 1500
.1.3.6.1.2.1.2.2.1.4.7 integer 1500
.1.3.6.1.2.1.2.2.1.4.8 integer 1500
.1.3.6.1.2.1.2.2.1.4.101 integer 1500
.1.3.6.1.2.1.2.2.1.4.120 integer 1500
.1.3.6.1.2.1.2.2.1.5.1 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.2 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.3 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.4 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.5 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.6 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.7 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.8 gauge 1000000000
.1.3.6.1.2.1.2.2.1.5.101 gauge 0
.1.3.6.1.2.1.2.2.1.5.120 gauge 0
.1.3.6.1.2.1.2.2.1.6.1 octet 00 1e 4a 7c 3b 01
.1.3.6.1.2.1.2.2.1.6.2 octet 00 1e 4a 7c 3b 02
.1.3.6.1.2.1.2.2.1.6.3 octet 00 1e 4a 7c 3b 03
.1.3.6.1.2.1.2.2.1.6.4 octet 00 1e 4a 7c 3b 04
.1.3.6.1.2.1.2.2.1.6.5 octet 00 1e 4a 7c 3b 05
.1.3.6.1.2.1.2.2.1.6.6 octet 00 1e 4a 7c 3b 06
.1.3.6.1.2.1.2.2.1.6.7 octet 00 1e 4a 7c 3b 07
.1.3.6.1.2.1.2.2.1.6.8 octet 00 1e 4a 7c 3b 08
.1.3.6.1.2.1.2.2.1.7.1 integer 1
.1.3.6.1.2.1.2.2.1.7.2 integer 1
.1.3.6.1.2.1.2.2.1.7.3 integer 1
.1.3.6.1.2.1.2.2.1.7.4 integer 1
.1.3.6.1.2.1.2.2.1.7.5 integer 1
.1.3.6.1.2.1.2.2.1.7.6 integer 1
.1.3.6.1.2.1.2.2.1.7.7 integer 1
.1.3.6.1.2.1.2.2.1.7.8 integer 1
.1.3.6.1.2.1.2.2.1.7.101 integer 1
.1.3.6.1.2.1.2.2.1.7.120 integer 1
.1.3.6.1.2.1.2.2.1.8.1 integer 1
.1.3.6.1.2.1.2.2.1.8.2 integer 1
.1.3.6.1.2.1.2.2.1.8.3 integer 1
.1.3.6.1.2.1.2.2.1.8.4 integer 1
.1.3.6.1.2.1.2.2.1.8.5 integer 1
.1.3.6.1.2.1.2.2.1.8.6 integer 1
.1.3.6.1.2.1.2.2.1.8.7 integer 1
.1.3.6.1.2.1.2.2.1.8.8 integer 1
.1.3.6.1.2.1.2.2.1.8.101 integer 1
.1.3.6.1.2.1.2.2.1.8.120 integer 1
.1.3.6.1.2.1.31.1.1.1.1.1 string GigabitEthernet1/0/1
.1.3.6.1.2.1.31.1.1.1.1.2 string GigabitEthernet1/0/2
.1.3.6.1.2.1.31.1.1.1.1.3 string GigabitEthernet1/0/3
.1.3.6.1.2.1.31.1.1.1.1.4 string GigabitEthernet1/0/4
.1.3.6.1.2.1.31.1.1.1.1.5 string GigabitEthernet1/0/5
.1.3.6.1.2.1.31.1.1.1.1.6 string GigabitEthernet1/0/6
.1.3.6.1.2.1.31.1.1.1.1.7 string GigabitEthernet1/0/7
.1.3.6.1.2.1.31.1.1.1.1.8 string GigabitEthernet1/0/8
.1.3.6.1.2.1.31.1.1.1.1.101 string Vlan1
.1.3.6.1.2.1.31.1.1.1.1.120 string Vlan20
.1.3.6.1.2.1.31.1.1.1.15.1 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.2 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.3 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.4 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.5 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.6 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.7 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.8 gauge 1000
.1.3.6.1.2.1.31.1.1.1.15.101 gauge 0
.1.3.6.1.2.1.31.1.1.1.15.120 gauge 0
EOF

# switch-cisco-01 bridge tables, default context — the near-empty one.
#
# This is the whole of what the reporter's switch returned however they asked for it: one learned
# MAC, reported as `count=1 complete=true`. It is not a truncated read and never was. IOS-XE
# partitions its forwarding database per VLAN and keeps almost nothing in the default context, so
# a scan that cannot name a context is reading the wrong table and being told nothing is wrong.
#
# dot1dBasePortIfIndex is served here as well as in the VLAN context, because the daemon walks it
# alongside whichever FDB it is reading and both have to resolve to the same ports.
cat > "$DATA_DIR/switch-cisco-01-bridge.txt" << 'EOF'
.1.3.6.1.2.1.17.1.4.1.2.1 integer 1
.1.3.6.1.2.1.17.1.4.1.2.2 integer 2
.1.3.6.1.2.1.17.1.4.1.2.3 integer 3
.1.3.6.1.2.1.17.1.4.1.2.4 integer 4
.1.3.6.1.2.1.17.1.4.1.2.5 integer 5
.1.3.6.1.2.1.17.1.4.1.2.6 integer 6
.1.3.6.1.2.1.17.1.4.1.2.7 integer 7
.1.3.6.1.2.1.17.1.4.1.2.8 integer 8
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.20.1 octet 00 50 56 9a 14 01
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.20.1 integer 1
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.20.1 integer 3
EOF

# switch-cisco-01 bridge tables, `vlan-20` context — the nine entries.
#
# Served by a second snmpd on $CTX_BACKEND_ADDR that exists only to be proxied. `pass` takes no context
# argument — it registers into the default context and nothing else — so a handler cannot be
# scoped to a context directly, and `proxy -Cn` in front of a second agent is the only way stock
# net-snmp serves different data per context.
cat > "$DATA_DIR/switch-cisco-01-vlan20.txt" << 'EOF'
.1.3.6.1.2.1.17.1.4.1.2.1 integer 1
.1.3.6.1.2.1.17.1.4.1.2.2 integer 2
.1.3.6.1.2.1.17.1.4.1.2.3 integer 3
.1.3.6.1.2.1.17.1.4.1.2.4 integer 4
.1.3.6.1.2.1.17.1.4.1.2.5 integer 5
.1.3.6.1.2.1.17.1.4.1.2.6 integer 6
.1.3.6.1.2.1.17.1.4.1.2.7 integer 7
.1.3.6.1.2.1.17.1.4.1.2.8 integer 8
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.32.1 octet 00 50 56 9a 20 01
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.32.2 octet 00 50 56 9a 20 02
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.32.3 octet 00 50 56 9a 20 03
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.32.4 octet 00 50 56 9a 20 04
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.32.5 octet 00 50 56 9a 20 05
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.32.6 octet 00 50 56 9a 20 06
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.32.7 octet 00 50 56 9a 20 07
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.32.8 octet 00 50 56 9a 20 08
.1.3.6.1.2.1.17.4.3.1.1.0.80.86.154.32.9 octet 00 50 56 9a 20 09
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.32.1 integer 1
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.32.2 integer 2
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.32.3 integer 2
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.32.4 integer 3
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.32.5 integer 4
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.32.6 integer 5
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.32.7 integer 6
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.32.8 integer 7
.1.3.6.1.2.1.17.4.3.1.2.0.80.86.154.32.9 integer 8
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.32.1 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.32.2 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.32.3 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.32.4 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.32.5 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.32.6 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.32.7 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.32.8 integer 3
.1.3.6.1.2.1.17.4.3.1.3.0.80.86.154.32.9 integer 3
EOF

# ── 5. Write snmpd configs ───────────────────────────────────────────
echo "Writing snmpd configs..."

D="$CONF_DIR/data"
H="$CONF_DIR/snmp-pass-handler.sh"
HU="$CONF_DIR/snmp-pass-handler-unsorted.sh"
HS="$CONF_DIR/snmp-pass-handler-stuck.sh"

cat > "$CONF_DIR/snmpd-switch-core-01.conf" << EOF
agentAddress udp:${HOSTS[0]}:161
rocommunity netdefault
sysdescr Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3
syscontact netops@example.com
sysname switch-core-01
syslocation Server Room A, Rack 1
sysobjectid .1.3.6.1.4.1.9.1.1208
sysservices 6
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-core-01-iftable.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-access-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-access-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-access-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-access-01-bridge.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/router-gw-01-iftable.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/firewall-01-iftable.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/printer-lobby-iftable.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/ap-wireless-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/ap-wireless-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/ap-wireless-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/ap-wireless-01-bridge.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/legacy-switch-01-iftable.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/secure-switch-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/secure-switch-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/secure-switch-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/secure-switch-01-bridge.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-exos-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-exos-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-exos-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-exos-01-bridge.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-voss-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-voss-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-voss-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-voss-01-bridge.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-netgear-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-netgear-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-netgear-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-netgear-01-bridge.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-aruba-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-aruba-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-aruba-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-aruba-01-bridge.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-omada-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-omada-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-omada-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-omada-01-bridge.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-omada-01-lldp.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-flaky-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-flaky-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-flaky-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-flaky-01-bridge.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-dlink-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-dlink-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-dlink-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-dlink-01-bridge.txt
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
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-tplink-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-tplink-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-tplink-01-bridge.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-tplink-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-switch-unsorted-01.conf" << EOF
agentAddress udp:${HOSTS[16]}:161
rocommunity netdefault
sysdescr PoE switch, firmware V3.3.3
syscontact netops@example.com
sysname switch-unsorted-01
syslocation Floor 1, camera room
sysobjectid .1.3.6.1.4.1.99999.1.1
sysservices 2
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-unsorted-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-unsorted-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-unsorted-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-unsorted-01-bridge.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.1 /bin/bash $HU $D/switch-unsorted-01-arp.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.2 /bin/bash $HU $D/switch-unsorted-01-arp.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.3 /bin/bash $HU $D/switch-unsorted-01-arp.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.4 /bin/bash $HU $D/switch-unsorted-01-arp.txt
EOF

cat > "$CONF_DIR/snmpd-switch-macport-01.conf" << EOF
agentAddress udp:${HOSTS[17]}:161
rocommunity netdefault
sysdescr WeOS 5.21.0 industrial ethernet switch
syscontact netops@example.com
sysname switch-macport-01
syslocation Substation B, DIN rail
sysobjectid .1.3.6.1.4.1.16177.1.1
sysservices 2
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-macport-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-macport-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-macport-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-macport-01-bridge.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-macport-01-lldp.txt
EOF

cat > "$CONF_DIR/snmpd-switch-mute-01.conf" << EOF
agentAddress udp:${HOSTS[18]}:161
rocommunity netdefault
sysdescr Mute agent, system MIB only
syscontact netops@example.com
sysname switch-mute-01
syslocation Rack 9, top
sysobjectid .1.3.6.1.4.1.99999.2.1
sysservices 2
pass -p 1 .1.3.6.1.2.1.4.20.1.1 /bin/bash $H $D/switch-mute-01-empty.txt
pass -p 1 .1.3.6.1.2.1.4.20.1.2 /bin/bash $H $D/switch-mute-01-empty.txt
pass -p 1 .1.3.6.1.2.1.4.20.1.3 /bin/bash $H $D/switch-mute-01-empty.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.1 /bin/bash $H $D/switch-mute-01-empty.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.2 /bin/bash $H $D/switch-mute-01-empty.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.3 /bin/bash $H $D/switch-mute-01-empty.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.4 /bin/bash $H $D/switch-mute-01-empty.txt
EOF

cat > "$CONF_DIR/snmpd-switch-stuck-01.conf" << EOF
agentAddress udp:${HOSTS[19]}:161
rocommunity netdefault
sysdescr Non-advancing agent, ARP table loops
syscontact netops@example.com
sysname switch-stuck-01
syslocation Rack 9, middle
sysobjectid .1.3.6.1.4.1.99999.3.1
sysservices 2
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-stuck-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-stuck-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-stuck-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-stuck-01-bridge.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.1 /bin/bash $HS $D/switch-stuck-01-arp.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.2 /bin/bash $HS $D/switch-stuck-01-arp.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.3 /bin/bash $HS $D/switch-stuck-01-arp.txt
pass -p 1 .1.3.6.1.2.1.4.22.1.4 /bin/bash $HS $D/switch-stuck-01-arp.txt
EOF

cat > "$CONF_DIR/snmpd-switch-dell-01.conf" << EOF
agentAddress udp:${HOSTS[20]}:161
rocommunity netdefault
sysdescr Dell EMC Networking OS10 Enterprise. Dell EMC Networking S4112T-ON. OS Version 10.4.3.4
syscontact netops@example.com
sysname switch-dell-01
syslocation Rack 4, breakout panel
sysobjectid .1.3.6.1.4.1.674.11000.5000.100.2.1
sysservices 2
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-dell-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-dell-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-dell-01-iftable.txt
pass .1.0.8802.1.1.2 /bin/bash $H $D/switch-dell-01-lldp.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-dell-01-bridge.txt
EOF

# switch-cisco-01 — the SNMPv3 context fixture (GH #686).
#
# v3-only, and on its own USM user: see V3_CTX_USER above for why the winner has to be
# deterministic. No rocommunity for any seeded community either, so no v2c credential can win here
# — `netdefault@20` below is reachable from the command line and is not a seeded credential.
#
# Three things make this device the thing under test:
#
#   `proxy -Cn vlan-20` routes the whole BRIDGE-MIB subtree, under that context name only, to the
#   back-end agent holding the nine-entry table. Ask without the context and you get the one-entry
#   table below, which is exactly the reporter's symptom.
#
#   `rouser ... -V all vlan-20` is what lets the v3 user name that context at all. Without it the
#   request is authorised for the default context and answered from the wrong table.
#
#   `com2sec -Cn vlan-20` maps the community `netdefault@20` onto the same context, which is how
#   Cisco exposes per-VLAN bridge data to v2c. We send the community verbatim, so that form works
#   today with no code at all — this is here so `snmp-verify` can prove it rather than assert it.
#
# ifTable, ifXTable and the system MIB stay in the default context, as they do on the real device.
# That is why the daemon scopes only its bridge and VLAN walks to the credential's context: a
# whole-session context would find none of them.
mkdir -p "$CONF_DIR/state/switch-cisco-01"
cat > "$CONF_DIR/snmpd-switch-cisco-01.conf" << EOF
agentAddress udp:${HOSTS[21]}:161
persistentDir $CONF_DIR/state/switch-cisco-01
createUser $V3_CTX_USER SHA-256 "$V3_CTX_AUTH_PASS" AES "$V3_CTX_PRIV_PASS"
rouser $V3_CTX_USER priv
rouser $V3_CTX_USER priv -V all vlan-20
com2sec -Cn vlan-20 v20sec default netdefault@20
group v20group v2c v20sec
view all included .1
access v20group vlan-20 any noauth exact all none none
sysdescr Cisco IOS Software [Fuji], Catalyst L3 Switch Software (CAT3K_CAA-UNIVERSALK9-M), Version 16.9.5
syscontact netops@example.com
sysname switch-cisco-01
syslocation Server Room B, Rack 2
sysobjectid .1.3.6.1.4.1.9.1.1745
sysservices 6
pass -p 1 .1.3.6.1.2.1.2.1 /bin/bash $H $D/switch-cisco-01-iftable.txt
pass .1.3.6.1.2.1.2.2 /bin/bash $H $D/switch-cisco-01-iftable.txt
pass .1.3.6.1.2.1.31.1.1 /bin/bash $H $D/switch-cisco-01-iftable.txt
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-cisco-01-bridge.txt
proxy -Cn vlan-20 -v2c -c $CTX_BACKEND_COMMUNITY $CTX_BACKEND_ADDR .1.3.6.1.2.1.17
EOF

# The back end the proxy points at. Loopback-only and not in SYSNAMES: it has no macvlan, no IP on
# the test subnet, and is never scanned or verified directly.
cat > "$CONF_DIR/snmpd-switch-cisco-01-vlan20.conf" << EOF
agentAddress udp:$CTX_BACKEND_ADDR
rocommunity $CTX_BACKEND_COMMUNITY 127.0.0.1
sysdescr switch-cisco-01 VLAN 20 bridge context
sysname switch-cisco-01-vlan20
sysservices 2
pass .1.3.6.1.2.1.17 /bin/bash $H $D/switch-cisco-01-vlan20.txt
EOF

# ── 5b. Derive the counts each device publishes about itself ─────────
#
# `ifNumber` and `dot1dBaseNumPorts` are what a device claims to have, as opposed to what it
# serves, and the daemon now compares the two: a switch declaring 48 bridge ports and answering
# the table with none has contradicted itself, which reads very differently to an operator than
# a walk that simply came up short.
#
# Computed from each fixture's own rows rather than hand-written, so editing an ifTable cannot
# leave a stale count behind and turn every scan of that device into a false warning. The one
# device that *should* disagree is overridden below, deliberately and in one place.
#
# Both scalars sort below every row already in their file (`.2.1.0` before `.2.2.*`, `.17.1.2.0`
# before `.17.1.4.*`), so prepending keeps the file ascending — which the GETNEXT handler needs,
# since it returns the first line numerically greater than the request.
echo "Deriving self-reported counts..."

# Give every switch the bridge-port numbering it claims to have.
#
# 15 of these fixtures set the `sysServices` datalink bit — they are modelled as switches — and
# served no BRIDGE-MIB at all, because nobody had written one. The daemon now reads that pairing
# as a device contradicting itself, and it is right to: on real hardware a managed switch that
# declares layer 2 and answers `dot1dBasePortIfIndex` with `noSuchObject` is hiding its bridge
# tables behind an SNMP view or a VLAN context, which is the GH #686 report.
#
# So the fixtures are the thing that was wrong, not the check. Bridge ports are numbered from
# each device's own ethernetCsmacd(6) interfaces, in ifIndex order, which is what an unconfigured
# managed switch reports. Devices that already have a hand-written bridge file keep it — theirs
# encode a specific shape and must not be regenerated.
for f in "$DATA_DIR"/*-iftable.txt; do
    [ -e "$f" ] || continue
    name=$(basename "$f" -iftable.txt)
    bridge="$DATA_DIR/$name-bridge.txt"
    [ -e "$bridge" ] && continue
    # Only for devices whose config actually serves the subtree, which is exactly those whose
    # `sysservices` sets the datalink bit. Reading that back from the config rather than keeping
    # a second list here means the two cannot disagree.
    grep -q '^pass \.1\.3\.6\.1\.2\.1\.17 ' "$CONF_DIR/snmpd-$name.conf" || continue
    # ifType 6 is ethernetCsmacd; VLAN (53) and loopback (24) interfaces are not bridge ports.
    physical=$(grep '^\.1\.3\.6\.1\.2\.1\.2\.2\.1\.3\..* integer 6$' "$f" \
        | sed 's/^\.1\.3\.6\.1\.2\.1\.2\.2\.1\.3\.\([0-9]*\) .*/\1/' | sort -n)
    [ -n "$physical" ] || continue
    port=0
    for if_index in $physical; do
        port=$((port + 1))
        echo ".1.3.6.1.2.1.17.1.4.1.2.$port integer $if_index"
    done > "$bridge"
done


for f in "$DATA_DIR"/*-iftable.txt; do
    [ -e "$f" ] || continue
    count=$(grep -c '^\.1\.3\.6\.1\.2\.1\.2\.2\.1\.1\.' "$f" || true)
    [ "$count" -gt 0 ] || continue
    { echo ".1.3.6.1.2.1.2.1.0 integer $count"; cat "$f"; } > "$f.tmp" && mv "$f.tmp" "$f"
done

# Keyed on what a file *serves* rather than on its name: switch-cisco-01 answers the bridge MIB
# from two files, one per SNMP context, and the VLAN-context one is no less a bridge table for
# being called `-vlan20`. A device whose two contexts disagreed about how many ports it has would
# be a fixture bug nobody could see.
for f in "$DATA_DIR"/*.txt; do
    [ -e "$f" ] || continue
    count=$(grep -c '^\.1\.3\.6\.1\.2\.1\.17\.1\.4\.1\.2\.' "$f" || true)
    [ "$count" -gt 0 ] || continue
    # Prepending only keeps the file ascending while every row already in it sorts above the
    # scalar, which holds for a bridge table (lowest row `.17.1.4.1.2.1`) and would not for a
    # file that also carried, say, `.17.1.1`. Fail rather than silently corrupt the walk order.
    if ! head -1 "$f" | grep -q '^\.1\.3\.6\.1\.2\.1\.17\.1\.4\.'; then
        echo "ERROR: $f serves bridge ports but does not start at .1.3.6.1.2.1.17.1.4 —" >&2
        echo "       prepending dot1dBaseNumPorts would break its OID ordering." >&2
        exit 1
    fi
    { echo ".1.3.6.1.2.1.17.1.2.0 integer $count"; cat "$f"; } > "$f.tmp" && mv "$f.tmp" "$f"
done

# switch-dell-01 declares more interfaces than it serves, on purpose.
#
# Every other device here agrees with itself, which proves the check stays quiet but cannot show
# it firing — and a guard nobody has watched fire is a guard nobody knows works. This is the
# GH #685 device, whose report is a switch discovering cleanly in every other respect, so the
# contradiction belongs on it: 52 declared against the 23 its ifTable serves.
dell="$DATA_DIR/switch-dell-01-iftable.txt"
{ echo ".1.3.6.1.2.1.2.1.0 integer 52"; tail -n +2 "$dell"; } > "$dell.tmp" && mv "$dell.tmp" "$dell"

# ── 5c. Check the fixture set hangs together ─────────────────────────
#
# A device with data files and no config, or a config naming a data file nobody wrote, provisions
# without complaint and then answers from net-snmp's built-in MIBs instead. That reads as a device
# behaving oddly rather than as a lab that was never assembled, and it is exactly the silent pass
# this whole environment exists to stop being fooled by — so fail the deploy here instead.
echo "Checking fixture coherence..."
coherence_errors=0
fixture_error() { echo "  ERROR: $*" >&2; coherence_errors=$((coherence_errors + 1)); }

for name in "${SYSNAMES[@]}"; do
    [ -f "$CONF_DIR/snmpd-$name.conf" ] \
        || fixture_error "$name is in SYSNAMES but has no snmpd-$name.conf"
done

# A data file nobody serves, and a config naming a file nobody wrote — the two halves of the same
# typo. Scoped to ifTables, the one file every simulated device has and no variant spares exist for
# (switch-flaky-01 keeps unserved `-lldp-*` files on purpose, to be swapped in by hand).
for f in "$DATA_DIR"/*-iftable.txt; do
    [ -e "$f" ] || continue
    name=$(basename "$f" -iftable.txt)
    [ -f "$CONF_DIR/snmpd-$name.conf" ] \
        || fixture_error "$(basename "$f") exists but no device serves it"
done
for conf in "$CONF_DIR"/snmpd-*.conf; do
    [ -e "$conf" ] || continue
    # Command substitution rather than a pipe into `while`: a pipeline's loop runs in a subshell,
    # so every error it counted would be discarded and the check would pass silently.
    for ref in $(grep -oE "$DATA_DIR/[a-z0-9._-]+\.txt" "$conf" | sort -u); do
        [ -f "$ref" ] \
            || fixture_error "$(basename "$conf") serves missing $(basename "$ref")"
    done
done

# Every device serving an ifTable must also register ifNumber. Without the `pass -p 1`, the scalar
# sits in the data file unserved while mibII/interfaces answers from the VM's own kernel state, and
# the device reports a count contradiction on every scan for a fault it does not have.
for conf in "$CONF_DIR"/snmpd-*.conf; do
    [ -e "$conf" ] || continue
    grep -q "\-iftable\.txt" "$conf" || continue
    grep -qE '^pass -p 1 \.1\.3\.6\.1\.2\.1\.2\.1 ' "$conf" \
        || fixture_error "$(basename "$conf") serves an ifTable but does not register ifNumber"
done

if [ "$coherence_errors" -gt 0 ]; then
    echo "Fixture set is incoherent ($coherence_errors problem(s)) — not deploying." >&2
    exit 1
fi
echo "  fixture set is coherent"

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

# switch-cisco-01's VLAN 20 back end, written by hand because it is not in SYSNAMES: it binds a
# loopback port rather than a macvlan, has no entry in HOSTS, and must never be verified or
# scanned as a device of its own. The front agent proxies to it, so it has to be up first —
# `Before=` rather than `After=`, or the first scan after a deploy reads an unreachable proxy.
cat > "/etc/systemd/system/snmpd-switch-cisco-01-vlan20.service" << EOF
[Unit]
Description=SNMP Test Agent — switch-cisco-01 VLAN 20 bridge context (${CTX_BACKEND_ADDR})
After=network.target
Before=snmpd-switch-cisco-01.service

[Service]
Type=simple
ExecStart=/usr/sbin/snmpd -f -Lo -I -ifTable,-ifXTable -C -c ${CONF_DIR}/snmpd-switch-cisco-01-vlan20.conf
Restart=on-failure
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF

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
# Ahead of the loop: the front agent proxies to it, and a proxy to a dead port answers nothing.
systemctl enable snmpd-switch-cisco-01-vlan20 --quiet
systemctl restart snmpd-switch-cisco-01-vlan20
printf "  %-28s started\n" "snmpd-switch-cisco-01-vlan20"
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
if systemctl is-active --quiet snmpd-switch-cisco-01-vlan20; then
    printf "  \033[0;32m✓\033[0m %-18s %-20s %s (active)\n" "$CTX_BACKEND_ADDR" "cisco vlan-20 back end" "v2c"
else
    printf "  \033[0;31m✗\033[0m %-18s %-20s (not active — journalctl -u snmpd-switch-cisco-01-vlan20)\n" "$CTX_BACKEND_ADDR" "cisco vlan-20 back end"
    all_ok=false
fi
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
