#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# SNMP Test Environment — Proxmox VM setup
#
# The host harness only: interfaces, the three `pass` handlers, systemd units and
# verification. The devices themselves — every data file and every agent config —
# are generated from the typed definitions in
#   backend/src/daemon/discovery/integration/snmp/sim/
# and shipped alongside this script by `make snmp-deploy`. Nothing here describes
# a device, so nothing here can disagree with the structs about what one is.
#
# To add or change a device, edit the struct and run `make snmp-deploy`. See
# "Adding a device" in ../SNMP-TEST-ENV.md.
#
# EXPECT TRUNCATION WARNINGS. See "Known chaos" at section 3 — a scan of this
# environment normally reports several incomplete SNMP walks, and that is the
# simulator, not the product under test.
# ══════════════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATED="$SCRIPT_DIR/generated"

if [ ! -d "$GENERATED" ]; then
    echo "ERROR: $GENERATED is missing." >&2
    echo "       Deploy with 'make snmp-deploy', which generates it from the device" >&2
    echo "       definitions before copying this tree to the VM." >&2
    exit 1
fi

# The device list, generated alongside the configs so it cannot disagree with them.
# shellcheck source=/dev/null
. "$GENERATED/lab.env"

CIDR="22"
IFACE="eth0"

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

# A fourth handler: one that answers correctly and too slowly to be asked in bulk.
#
# KNOWN NOT TO WORK YET — see Handler::Slow in sim/transport.rs. snmpd drives `pass` serially
# (measured: 3 calls, 3s sleep, 9.03s total), so a GETBULK of 20 blocks the agent for 20*sleep
# while the client gives up after 5s. The GETNEXT the walk sends next queues behind that bulk and
# times out as well, so the device fails both ways instead of failing one and answering the other.
# The script is left in place because the fix is a different refusal mechanism, not a different
# sleep. switch-slowbulk-01 does still register it, so that device does not currently reproduce the
# defect on the VM: its LLDP walk comes back empty and the scan reports SnmpWalkNoAnswer for it.
#
# Measured 2026-08-27, second lab scan: a full GETNEXT walk of that device's remote tables takes
# 66.6s against the daemon's 60s SNMP_WALK_TIMEOUT, so even reaching the fallback is not enough.
#
# snmpd assembles a GETBULK of n over a `pass` script by invoking it n times, so a handler that
# takes t to answer costs n*t for a bulk page and t for a single GETNEXT. At 3s against the
# daemon's 5s SNMP_TIMEOUT that splits the two cleanly: every bulk page the walk asks for — 20,
# 10, 5, 2 — exceeds the timeout, and one GETNEXT returns with two seconds to spare.
#
# That is the GH #668 device. Its columns are readable and were being reported empty, because the
# walk halved its page size on each timeout and ran out of retries three shrinks before halving
# would have reached GETNEXT. Registered on one subtree only, so the device serves the rest of its
# tables at full speed — the reporter's switch answered its ifTable and ARP table fine.
#
# `sleep` is the whole difference from snmp-pass-handler.sh; the answer is identical.
cat > "$CONF_DIR/snmp-pass-handler-slow.sh" << 'PASSEOF'
#!/bin/bash
DATA_FILE="$1"
REQUEST="$2"
OID="$3"

# Long enough that no multi-varbind bulk page fits in the client's 5s, short enough that a single
# GETNEXT does. Raising this past 5 makes the device unreadable rather than bulk-unreadable, which
# is a different fixture; lowering it past 2.5 lets a 2-repetition page through and the device
# stops reproducing the defect.
sleep 3

if [ ! -f "$DATA_FILE" ]; then
    echo "NONE"
    exit 0
fi

case "$REQUEST" in
    -g)
        LINE=$(awk -v oid="$OID" '$1 == oid { print; exit }' "$DATA_FILE")
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
        ;;
    *)  echo "NONE"; exit 0 ;;
esac

if [ -z "$LINE" ]; then
    echo "NONE"
    exit 0
fi
echo "$LINE" | awk '{ print $1; print $2; $1=""; $2=""; sub(/^  */, ""); print }'
PASSEOF
chmod +x "$CONF_DIR/snmp-pass-handler-slow.sh"


# ── 4. Install the generated devices ─────────────────────────────────
#
# Every data file and every agent config comes from the typed definitions. This step is a copy,
# deliberately: the checks that used to live here — a data file nobody serves, a config naming a
# file nobody wrote, a name with no config, an ifTable served without its ifNumber registration —
# are now properties of the model rather than greps run at deploy time, and are covered by unit
# tests in `snmp::sim`.
echo "Installing generated device definitions..."
rm -rf "$DATA_DIR"
mkdir -p "$DATA_DIR"
cp "$GENERATED"/data/*.txt "$DATA_DIR/"
cp "$GENERATED"/snmpd-*.conf "$CONF_DIR/"

# The v3 agents keep USM state here; snmpd creates the file but not the directory.
for name in "${UNITS[@]}"; do
    mkdir -p "$CONF_DIR/state/$name"
done

printf "  %d data file(s), %d agent config(s)\n" \
    "$(find "$DATA_DIR" -name '*.txt' | wc -l)" \
    "$(find "$CONF_DIR" -maxdepth 1 -name 'snmpd-*.conf' | wc -l)"


# ── 5. Create systemd services ───────────────────────────────────────
#
# One unit per device, plus any context back end. `-I -ifTable,-ifXTable` stops net-snmp answering
# the interface tables from the VM's own kernel; the subtrees a `pass` cannot displace that way are
# registered at priority 1 in the generated configs instead.
echo "Creating systemd services..."
make_unit() {
    local name="$1" description="$2" extra="${3:-}"
    cat > "/etc/systemd/system/snmpd-${name}.service" << UNIT
[Unit]
Description=SNMP Test Agent — ${description}
After=network.target
${extra}

[Service]
Type=simple
ExecStart=/usr/sbin/snmpd -f -Lo -I -ifTable,-ifXTable -C -c ${CONF_DIR}/snmpd-${name}.conf
Restart=on-failure
RestartSec=2

[Install]
WantedBy=multi-user.target
UNIT
}

for i in "${!UNITS[@]}"; do
    make_unit "${UNITS[$i]}" "${UNITS[$i]} (${HOSTS[$i]})"
done

# Any context back end binds loopback rather than a macvlan, has no entry in HOSTS, and must never
# be scanned as a device of its own. The front agent proxies to it, so it has to be up first —
# `Before=`, or the first scan after a deploy reads an unreachable proxy.
CONTEXT_UNITS=()
for conf in "$CONF_DIR"/snmpd-*-vlan20.conf; do
    [ -e "$conf" ] || continue
    name=$(basename "$conf" .conf); name=${name#snmpd-}
    CONTEXT_UNITS+=("$name")
    make_unit "$name" "${name} bridge context (loopback)" "Before=snmpd-${name%-vlan20}.service"
done

# ── 6. Persist macvlan interfaces ────────────────────────────────────
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

# ── 7. Start everything ──────────────────────────────────────────────
echo "Starting SNMP agents..."
systemctl daemon-reload
# Ahead of the loop: a front agent proxies to these, and a proxy to a dead port answers nothing.
for name in "${CONTEXT_UNITS[@]}"; do
    systemctl enable "snmpd-${name}" --quiet
    systemctl restart "snmpd-${name}"
    printf "  %-28s started\n" "snmpd-${name}"
done
for name in "${UNITS[@]}"; do
    systemctl enable "snmpd-${name}" --quiet
    systemctl restart "snmpd-${name}"
    printf "  %-28s started\n" "snmpd-${name}"
done


# ── 8. Verify ─────────────────────────────────────────────────────────
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
for name in "${CONTEXT_UNITS[@]}"; do
    if systemctl is-active --quiet "snmpd-${name}"; then
        printf "  \033[0;32m✓\033[0m %-18s %-20s (active)\n" "loopback" "$name"
    else
        printf "  \033[0;31m✗\033[0m %-18s %-20s (not active — journalctl -u snmpd-%s)\n" "loopback" "$name" "$name"
        all_ok=false
    fi
done
for i in "${!UNITS[@]}"; do
    name="${UNITS[$i]}"
    if systemctl is-active --quiet "snmpd-${name}"; then
        printf "  \033[0;32m✓\033[0m %-18s %-20s %s (active)\n" "${HOSTS[$i]}" "$name" "${VERSIONS[$i]}"
    else
        printf "  \033[0;31m✗\033[0m %-18s %-20s %s (not active — journalctl -u snmpd-%s)\n" "${HOSTS[$i]}" "$name" "${VERSIONS[$i]}" "$name"
        all_ok=false
    fi
done

echo ""
if $all_ok; then
    printf "\033[0;32mAll %d SNMP agents are active.\033[0m\n" "${#UNITS[@]}"
    echo ""
    echo "macvlan blocks queries from this VM. Verify reachability from an"
    echo "external host (e.g. your Mac) with: make snmp-verify"
else
    echo "Some agents are not active. Check: journalctl -u snmpd-<name>"
fi
