#!/bin/bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════
# Middlebox Test Environment — Debian LXC/VM setup
#
# Reproduces the FortiGate SIP session-helper report: a router that completes the
# TCP handshake for *every* address in a subnet it fronts, whether or not anything
# is there. That is what turned an empty VLAN into one phantom "SIP Server" host
# per VLAN for the customer who reported it.
#
# The mechanism here is the same one FortiOS uses, reduced to its essentials: a
# REDIRECT rule on a forwarding host sends traffic for the whole range to a local
# listener, so the SYN never reaches the wire and the handshake completes anyway.
# A packet capture on the phantom range shows nothing, exactly as the reporter's did.
#
# Run this ON the VM. See MIDDLEBOX-TEST-ENV.md for the one route the scanning
# host needs, which is the only change outside this machine.
# ══════════════════════════════════════════════════════════════════════

# The range this box pretends to route to. Nothing is on it. Nothing ever will be.
PHANTOM_SUBNET="${PHANTOM_SUBNET:-10.77.0.0/24}"

# Ports the middlebox answers for. The first six are what FortiOS ships enabled in
# `config system session-helper`; the rest are here so the guard can be exercised
# against protocols whose probes differ in shape.
INTERCEPT_TCP_PORTS="${INTERCEPT_TCP_PORTS:-5060,21,554,1720,2727,69,22,445,3389}"

# Where the intercepted connections land. Nothing listens behind it in any real
# sense: it accepts and says nothing, which is the whole point.
SINK_PORT="${SINK_PORT:-59595}"

if [ "${1:-}" = "--down" ]; then
    echo "=== Tearing down ==="
    iptables -t nat -D PREROUTING -j MIDDLEBOX 2>/dev/null || true
    iptables -t nat -D OUTPUT -j MIDDLEBOX 2>/dev/null || true
    iptables -t nat -F MIDDLEBOX 2>/dev/null || true
    iptables -t nat -X MIDDLEBOX 2>/dev/null || true
    systemctl disable --now middlebox-sink.service 2>/dev/null || true
    rm -f /etc/systemd/system/middlebox-sink.service
    systemctl daemon-reload
    ip route del "$PHANTOM_SUBNET" dev lo 2>/dev/null || true
    echo "done. ip_forward and route_localnet left as they were."
    exit 0
fi

echo "=== Middlebox Test Environment Setup ==="
echo "    phantom range : $PHANTOM_SUBNET"
echo "    intercepting  : tcp/$INTERCEPT_TCP_PORTS"

# ── 1. Packages ───────────────────────────────────────────────────────
# iproute2 for `ip route`, procps for `sysctl`. Both are present on a normal Debian
# install and absent from the slim container images, which is worth naming rather than
# discovering when the route step fails silently.
if ! command -v socat &>/dev/null || ! command -v iptables &>/dev/null; then
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y socat iptables iproute2 procps tcpdump
fi

# ── 2. Forwarding, and a route for a range with nothing on it ─────────
# The route matters: without it the kernel has nowhere to send traffic for the
# range and rejects it before the REDIRECT rule is consulted. Pointing it at the
# loopback is what lets this box claim to be the way to a subnet that does not exist.
sysctl -w net.ipv4.ip_forward=1 >/dev/null
sysctl -w net.ipv4.conf.all.route_localnet=1 >/dev/null
ip route replace "$PHANTOM_SUBNET" dev lo 2>/dev/null || true

# ── 3. The sink ───────────────────────────────────────────────────────
# `/dev/null` rather than a protocol responder, deliberately. A middlebox that
# answers the protocol is a different (and much rarer) case; this one completes the
# handshake and stays silent, which is what a session helper does for a destination
# that does not exist.
#
# Bound to 0.0.0.0, which is load bearing. `REDIRECT` rewrites the destination to the
# address of the interface the packet arrived on: 127.0.0.1 for traffic this box
# originates, but the LAN address for traffic it forwards. A sink bound to loopback
# passes a local test and then silently drops every packet from the scanner, which is
# the one case the lab exists for.
cat > /etc/systemd/system/middlebox-sink.service <<EOF
[Unit]
Description=Middlebox sink: accepts TCP and says nothing
After=network.target

[Service]
ExecStart=/usr/bin/socat TCP-LISTEN:${SINK_PORT},fork,reuseaddr /dev/null
Restart=always

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable --now middlebox-sink.service

# ── 4. The interception ───────────────────────────────────────────────
# One rule per port, matching on destination *range* rather than address, which is
# what makes every address in it answer. `-j REDIRECT` rewrites the destination to
# this box, so the packet is consumed here and never forwarded.
iptables -t nat -N MIDDLEBOX 2>/dev/null || iptables -t nat -F MIDDLEBOX
iptables -t nat -C PREROUTING -j MIDDLEBOX 2>/dev/null || iptables -t nat -A PREROUTING -j MIDDLEBOX
iptables -t nat -C OUTPUT -j MIDDLEBOX 2>/dev/null || iptables -t nat -A OUTPUT -j MIDDLEBOX

IFS=',' read -ra PORTS <<< "$INTERCEPT_TCP_PORTS"
for port in "${PORTS[@]}"; do
    iptables -t nat -A MIDDLEBOX -p tcp -d "$PHANTOM_SUBNET" --dport "$port" \
        -j REDIRECT --to-ports "$SINK_PORT"
done

# ── 5. Verify ─────────────────────────────────────────────────────────
echo
echo "=== Verification ==="
FIRST_PORT="${PORTS[0]}"
for addr in "${PHANTOM_SUBNET%.*/*}.7" "${PHANTOM_SUBNET%.*/*}.99" "${PHANTOM_SUBNET%.*/*}.201"; do
    if timeout 3 bash -c "echo > /dev/tcp/$addr/$FIRST_PORT" 2>/dev/null; then
        echo "  $addr:$FIRST_PORT  handshake completed (nothing is there)"
    else
        echo "  $addr:$FIRST_PORT  NO ANSWER — the interception is not working"
    fi
done

echo
echo "Next: add a route on the scanning host so it sends this range here."
echo "  see MIDDLEBOX-TEST-ENV.md"
echo
echo "To tear down:  $0 --down"
