# SNMP Test Environment

13 simulated network devices running on a Proxmox VM, each on port 161. Most speak SNMPv2c; `.236`/`.237` are version-locked to exercise the SNMPv1 and SNMPv3 paths (#557); `.238`/`.239` are Extreme switches that exercise the LLDP local-port remap (Issue 2, July 2026); `.240`–`.242` reproduce the L2-topology failures from #664, #649 and #614.

| IP | Host | Version | Credential | Device |
|---|---|---|---|---|
| 192.168.7.230 | switch-core-01 | v2c | community `netdefault` | Cisco C2960 |
| 192.168.7.231 | switch-access-01 | v2c | community `netdefault` | Cisco C3750 |
| 192.168.7.232 | router-gw-01 | v2c | community `secret42` | Juniper MX204 |
| 192.168.7.233 | firewall-01 | v2c | community `secret42` | FortiGate 60F |
| 192.168.7.234 | printer-lobby | v2c | community `public` | HP LaserJet M428 |
| 192.168.7.235 | ap-wireless-01 | v2c | community `netdefault` | Ubiquiti UniFi AP |
| 192.168.7.236 | legacy-switch-01 | **v1 only** | community `legacyv1` | Cisco C2950 |
| 192.168.7.237 | secure-switch-01 | **v3 only** | user `scanopyv3` (see below) | Huawei S5000 |
| 192.168.7.238 | switch-exos-01 | v2c | community `netdefault` | Extreme X435 (EXOS) |
| 192.168.7.239 | switch-voss-01 | v2c | community `netdefault` | Extreme VSP-7400 (VOSS) |
| 192.168.7.240 | switch-netgear-01 | v2c | community `netdefault` | Netgear GS724Tv3 |
| 192.168.7.241 | switch-aruba-01 | v2c | community `netdefault` | HP/Aruba ProCurve 2910al |
| 192.168.7.242 | switch (Omada) | v2c | community `public` | TP-Link Omada TL-SG3216 |

**LLDP local-port remap (`.238`/`.239`).** ExtremeXOS reports its `lldpRemTable` local-port index as an `lldpLocPortNum` (1..N) that is a **separate namespace from `ifIndex`** (switch-exos-01 uses ifIndex 1001+, ifName `1:N`), so neighbours only resolve if the daemon walks `lldpLocPortTable` (`1.0.8802.1.1.2.1.3.7`) and suffix-matches `lldpLocPortId` against `ifName`. Before the Issue 2 fix, switch-exos-01 yields **zero** LLDP neighbours. Extreme VOSS (switch-voss-01) reports local-port == ifIndex with `lldpLocPortId` matching `ifName` exactly, so it stays correct on both old and new code — the regression guard for the fix.

**L2 neighbour resolution (`.240`/`.241`).** These two are cabled to each other in the fixture data — `switch-netgear-01 g1 ↔ switch-aruba-01 port 41` and `g2 ↔ A5` — and between them cover both halves of a physical link:

- **Chassis MAC that is on no port (#664).** switch-netgear-01's LLDP chassis id is `00:1a:2b:3c:4d:63`, while its ports report `…:65/:66/:67` and it bears no IP with that MAC. switch-aruba-01's neighbour entries advertise that chassis MAC, so the remote host is identifiable **only** through the `chassis_id` recorded from switch-netgear-01's own LLDP local identity. Matching MACs against interfaces and IPs alone yields `hosts_resolved=0` and an empty L2 Physical view.
- **Locally-assigned port ids (#649).** switch-netgear-01's neighbour entries use port-ID subtype 7 with values `41` (which is switch-aruba-01's `ifDescr`) and `197` (which matches only its `ifIndex` — that port is labelled `A5`). Both shapes occur on real Aruba/HP gear. Treating subtype 7 as unresolvable stops resolution at the host, and a host-only neighbour draws **no edge at all**, so the switch is missing from L2 Physical entirely.

Both links should render in L2 Physical, and the server's `LLDP/CDP link resolution complete` line should report `ports_resolved` covering all four neighbour records (two per device).

**High-ifIndex interface persistence (`.242`).** The Omada TL-SG3216 puts its 16 physical ports at ifIndex 49153–49168, reports **no** ifXTable `ifName` for any of them, and returns the same chassis `ifPhysAddress` on every port; only ifIndex 1 (`Vlan-interface1`) carries a name and an IP. All 17 must persist as distinct interfaces. It advertises no LLDP neighbours at all — deliberately, so it exercises the interface-persistence path in isolation. Note its `sysName` is the literal `switch`, matching the reporter's device.

> **MAC octet padding.** Every fixture wrote MACs abbreviated (`0:1a:2b:0:10:0`) until 2026-07-27, and the daemon's string-parsing fallback rejected that form outright — so no LLDP data persisted for *any* sim device and no host ever got a `chassis_id`. Silently: an unparseable chassis id discards the whole neighbour record, which is indistinguishable from a switch that advertises none. The daemon now accepts both forms, and the fixtures are padded **except switch-exos-01's own chassis id**, deliberately left abbreviated as the standing guard for that tolerance (ExtremeXOS is one of the two vendors known to send this identifier as a string rather than octets).

**What a scan exercises (session-reuse + getbulk).** Every device is scanned with a single reused SNMP session across all ~11 queries (one v3 engine discovery instead of ~12), and each table is walked with `getbulk` (v1 falls back to `getnext`). To make the getbulk walks land on real data for the subtrees stock `snmpd` does **not** implement:
- **switch-core-01** additionally serves BRIDGE-MIB / Q-BRIDGE (`dot1dBasePortIfIndex`, `dot1qVlanStaticName` → VLANs "DATA"/"VOICE", `dot1qPvid`), ENTITY-MIB (chassis inventory) and CDP (a `router-gw-01` neighbour) — exercising those getbulk walks end-to-end.
- **legacy-switch-01 (v1-only)** additionally serves a small bridge table, so the **getbulk → getnext fallback** is exercised on a non-ifTable walk, not just ifTable/LLDP.
- `ipAddrTable` and `ipNetToMedia` (ARP) are answered by snmpd's built-in IP module, so those walks run on every device already. (net-snmp `pass` can't emit binary MAC octet-strings, so FDB/ARP MAC *rows* aren't simulated — the daemon still walks those subtrees and terminates cleanly.)
- **ap-wireless-01** is the one exception: it serves its own `ipAddrTable` so it can advertise a second subnet (see below).

**Access-point guest subnet (`.235`) — #663.** The built-in IP module answers `ipAddrTable` from the VM's real kernel state, so every other agent only ever reports addresses inside the scanned `192.168.4.0/22`. `ap-wireless-01` displaces that module (`-I …,-ipAddr` in its systemd unit) and serves the table from `ap-wireless-01-ipaddr.txt`, advertising **172.30.10.1/24 on ifIndex 4**, whose `ifName` is **`br-guest`** — the built-in NAT guest network of a real access point.

That combination is what issue #663 reported: a `br-` prefixed `ifName` on a remote device used to be classified as a Docker bridge, so the AP's guest subnet rendered as "Docker @ *AP*" in Topology. A scan of `.235` should now discover `172.30.10.0/24` as a **Guest** subnet, with no Docker/container label anywhere.

Because `.235` is the only agent serving its own `ipAddrTable`, it is also the only one that can fail *silently* — if the module displacement doesn't take, the `pass` directive loses the duplicate registration and the agent quietly reports just the scanned subnet. `make snmp-verify` checks this fixture explicitly for that reason; don't run a scan against it until that check passes.

The two version-locked hosts use net-snmp VACM/USM so the other protocol versions are genuinely refused (a plain `rocommunity` answers both v1 and v2c, which wouldn't prove version negotiation):

- **legacy-switch-01 (v1 only):** VACM grants access only via the v1 security model — v2c/v3 are denied.
- **secure-switch-01 (v3 only):** USM user `scanopyv3`, AuthPriv, **SHA-256 / AES-128**, auth password `authpass12345`, priv password `privpass12345`. No `rocommunity`, so v1/v2c are denied.

> **AES-256 note:** the v3 host uses AES-128, which stock Debian/Ubuntu net-snmp supports out of the box. AES-256 (`createUser … AES-256`) requires net-snmp built with Blumenthal AES (`--enable-blumenthal-aes`); change `createUser`/the verify command in `lxc/setup.sh` only if your build supports it.

## Credentials

The devices deliberately span five credentials so a scan exercises credential selection, the v1/v2c/v3 negotiation paths, and the "try the next credential" fallback rather than one community answering everything. Seed all five into the dev database with:

```bash
make snmp-seed-credentials
```

It assigns each one to **every network in the database** (Broadcast scope — the only option that works before a scan, since PerHost assignment needs hosts that don't exist yet), and is idempotent: re-running updates the existing rows rather than accumulating duplicates. If it reports `networks | 0`, create a network first — nothing was seeded.

The credential values live in `backend/scripts/seed-snmp-credentials.sql` and must stay in step with the community strings in `lxc/setup.sh`.

## Setup

Paste the contents of `tools/snmp/lxc/setup.sh` into a root shell on a Debian/Ubuntu VM with primary IP 192.168.7.230/22.

Before pasting, verify:
- Interface is `eth0` (`ip link`) — edit `IFACE=` if different
- Primary IP is 192.168.7.230 — edit `HOSTS=()` if different

## Patch: migrate secondary IPs to macvlan (unique MACs)

If each device shares the host's MAC (secondary IPs on eth0), run on the VM:

```bash
IFACE=eth0; CIDR=22; HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235 192.168.7.236 192.168.7.237 192.168.7.238 192.168.7.239 192.168.7.240 192.168.7.241 192.168.7.242); for i in "${!HOSTS[@]}"; do ip addr del "${HOSTS[$i]}/$CIDR" dev "$IFACE" 2>/dev/null; ip link del "mv-snmp${i}" 2>/dev/null; ip link add "mv-snmp${i}" link "$IFACE" type macvlan mode bridge; ip addr add "${HOSTS[$i]}/$CIDR" dev "mv-snmp${i}"; ip link set "mv-snmp${i}" up; done && sysctl -w net.ipv4.conf.all.arp_ignore=1 net.ipv4.conf.all.arp_announce=2 && for i in "${!HOSTS[@]}"; do sysctl -w net.ipv4.conf.mv-snmp${i}.arp_ignore=1 net.ipv4.conf.mv-snmp${i}.arp_announce=2; done && sysctl -w net.ipv4.conf.${IFACE}.arp_ignore=1 net.ipv4.conf.${IFACE}.arp_announce=2
```

Then flush the ARP cache on the scanning host (`sudo arp -a -d` on macOS).

## Patch: fix duplicate MIB registration

If snmpd logs show `duplicate registration: MIB modules ifTable and pass`, run:

```bash
for f in /etc/systemd/system/snmpd-*.service; do sed -i 's|snmpd -f -Lo -C|snmpd -f -Lo -I -ifTable,-ifXTable -C|' "$f"; done && systemctl daemon-reload && for f in /etc/systemd/system/snmpd-*.service; do systemctl restart "$(basename "$f" .service)"; done
```

## Updating an already-running VM

`lxc/setup.sh` is idempotent — existing macvlan interfaces are left alone, while MIB data files, snmpd configs and systemd units are rewritten and every agent is restarted. So a full re-run is always the update path; there is no separate partial script.

```bash
# from your Mac
scp -r tools/snmp root@192.168.7.230:/root/snmp-test
# on the VM (root shell)
bash /root/snmp-test/lxc/setup.sh
```

Hosts that gained nothing are effectively no-ops; anything whose data file, config or unit changed comes back with the new content.

Afterwards, flush the scanning host's ARP cache (`sudo arp -a -d` on macOS) so any new MACs are learned, then run `make snmp-verify` from your Mac.

> Re-running is required after any change to the MIB data or a systemd unit — including the `ap-wireless-01` guest-subnet fixture (#663), which changes both its `ipAddrTable` data and its `ExecStart` module exclusions.

## Verify

**Verify from an external host (e.g. your Mac), not the VM itself.** The agents bind to macvlan interfaces, and the Linux kernel won't let the VM reach its own macvlan child interfaces — so `snmpget` from the VM to `192.168.7.x` always fails even when everything is healthy. `setup.sh` therefore only checks systemd service health locally and prints a reminder to verify externally.

From your Mac:

```bash
make snmp-verify
```

Or manually — note the per-version flags:

```bash
# v2c
snmpget -v2c -c secret42 -t 2 -r 1 192.168.7.232 sysName.0
# v1 (legacy-switch-01)
snmpget -v1 -c legacyv1 -t 2 -r 1 192.168.7.236 sysName.0
# v3 (secure-switch-01) — SHA-256 / AES-128 AuthPriv
snmpget -v3 -l authPriv -u scanopyv3 -a SHA-256 -A authpass12345 -x AES -X privpass12345 -t 2 -r 1 192.168.7.237 sysName.0
```

To prove the version lock, confirm the wrong version is refused:

```bash
snmpget -v2c -c legacyv1 192.168.7.236 sysName.0   # should time out (v1-only)
snmpget -v2c -c public   192.168.7.237 sysName.0   # should time out (v3-only)
```

## Manage services

```bash
# On the VM
systemctl status snmpd-router-gw-01
journalctl -u snmpd-router-gw-01 --no-pager -n 20
systemctl restart snmpd-router-gw-01
```
