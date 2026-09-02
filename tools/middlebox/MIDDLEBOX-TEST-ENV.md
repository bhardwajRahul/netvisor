# Middlebox Test Environment

Reproduces the report this branch came from: a router that completes the TCP handshake for **every**
address in a subnet it fronts, whether or not anything is there. A customer scanning remote VLANs
through a FortiGate got one phantom "SIP Server" host per VLAN, none with a MAC, none corresponding
to a device. Their packet capture on the destination VLAN showed zero packets on the wire.

FortiOS ships `config system session-helper` with a `sip` entry enabled by default, and it answers on
behalf of any destination routed through the firewall. `setup.sh` reduces that to its essentials: a
`REDIRECT` rule on a forwarding host sends traffic for a whole range to a local listener that accepts
and says nothing.

## Why the subnet has to be routed

The daemon only invents a host from a bare TCP connect when it has nothing else. On a subnet it has
an interface on, ARP answers first and the address carries a MAC. The failure needs a subnet the
daemon reaches **through** something, so every address arrives as `LivenessEvidence::Enumerated` and
the connect is the only thing between it and a host record.

The daemon must therefore not be on the phantom range's segment. That constraint is the whole setup.

## Currently deployed

Running on the SNMP lab VM (`192.168.4.21`, reachable as `root@192.168.7.230` with
`~/.ssh/snmp-test-vm`), with the route already added on the daemon Mac. Verified end to end: every
address in `10.77.0.0/24` completes a handshake on the intercepted ports from the scanning host, and
a port outside the set is refused.

It shares that VM with the SNMP simulator and does not disturb it — the NAT rules match only
`-d 10.77.0.0/24`, so no `192.168.7.x` traffic touches them. Confirmed after deployment: 26 `snmpd`
units still active, 25 macvlans present, `switch-core-01` still answering `snmpget`.

Teardown:

```
ssh -i ~/.ssh/snmp-test-vm root@192.168.7.230 '/root/middlebox-setup.sh --down'
sudo route -n delete -net 10.77.0.0/24
```

## Setting it up elsewhere

A Debian LXC or VM the scanning daemon can reach. It needs one core and 512 MB.

```
pct create 300 local:vztmpl/debian-12-standard_12.7-1_amd64.tar.zst \
    --hostname middlebox --net0 name=eth0,bridge=vmbr0,ip=dhcp --unprivileged 0
pct start 300 && pct enter 300
```

It has to be **privileged** (`--unprivileged 0`) — the setup loads `iptables` NAT rules, which an
unprivileged container cannot.

Then copy `setup.sh` across and run it:

```
apt-get update && apt-get install -y socat iptables
./setup.sh
```

It prints the three addresses it verified, each completing a handshake with nothing behind it.

Those checks run **on the middlebox itself**, which is a weaker test than it looks: locally
originated traffic is redirected to loopback, while traffic forwarded from the scanner is redirected
to the LAN address. A sink bound to `127.0.0.1` passes the local check and drops every packet from
the scanner. Always confirm from the scanning host as well, which is what the next section is for.

## The one change outside that box

The scanning host has to know the range is reachable through the middlebox:

```
# Linux
ip route add 10.77.0.0/24 via <middlebox-ip>
# macOS
sudo route -n add 10.77.0.0/24 <middlebox-ip>
```

Undo with `ip route del 10.77.0.0/24` or `sudo route -n delete 10.77.0.0/24`.

If the daemon runs in Docker, add the route inside the container (needs `--cap-add=NET_ADMIN`), or
run the daemon on the host for this test.

Then confirm from the scanning host, not just from the middlebox:

```
for a in 10.77.0.7 10.77.0.99 10.77.0.201; do
    timeout 4 bash -c "echo > /dev/tcp/$a/5060" && echo "$a completed" || echo "$a no answer"
done
timeout 4 bash -c "echo > /dev/tcp/10.77.0.7/8080" && echo "8080 answered (wrong)" || echo "8080 refused"
```

## Testing it

1. Add `10.77.0.0/24` as a subnet on the network and include it in a scan.
2. Run the scan from a daemon that is **not** on that range.

**Before this branch:** a host appears at every address the middlebox answers for, each carrying a
SIP Server — and an FTP Server, an RTSP Camera and so on for the other intercepted ports — none with
a MAC address.

**After:** no hosts on that range at all. Every port the middlebox answers for is one Scanopy knows
how to interrogate, none answered its protocol, and an address with no other evidence is not
recorded.

### Confirming the guard discriminates rather than refusing the whole range

Put something real on the range. On the middlebox host:

```
ip addr add 10.77.0.50/32 dev lo
python3 -m http.server 8080 --bind 10.77.0.50
```

This address is the interesting one, because it is **both**: it serves real HTTP on 8080 *and* is
intercepted on 5060 along with every other address in the range. That is what a real host behind the
customer's FortiGate looks like — the helper answers on its behalf too.

So it should appear as a host, with its web service, and **without** a SIP Server. Three failures to
watch for:

- The host is missing entirely — the guard is over-suppressing, and that is a worse bug than the one
  it fixes.
- The host appears with a SIP Server on it — a probe is accepting a completed handshake as evidence.
- The phantom addresses appear at all — the guard is not running.

## Tuning

Both are environment variables read by `setup.sh`:

- `PHANTOM_SUBNET` — the range with nothing on it. Default `10.77.0.0/24`.
- `INTERCEPT_TCP_PORTS` — what the middlebox answers for. Default is the six ports FortiOS ships
  session helpers for (5060, 21, 554, 1720, 2727, 69) plus 22, 445 and 3389, which exercise probes of
  a different shape.

## What this does not reproduce

The FortiGate also mangles real SIP signalling and installs expectation sessions. None of that
reaches the detection path and none of it is simulated. What is reproduced is the only part Scanopy
can observe: a handshake that completes for an address holding no device.
