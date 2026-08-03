# SNMP Test Environment

16 simulated network devices running on a Proxmox VM, each on port 161. Most speak SNMPv2c; `.236`/`.237` are version-locked to exercise the SNMPv1 and SNMPv3 paths (#557); `.238`/`.239` are Extreme switches that exercise the LLDP local-port remap (Issue 2, July 2026); `.240`–`.242` reproduce the L2-topology failures from #664, #649 and #614; `.243` serves a deliberately malformed neighbour record; `.244` serves the port-id shapes from #668; `.245` serves that report's last device, whose neighbour table is indexed one sub-id short.

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
| 192.168.7.243 | switch-flaky-01 | v2c | community `netdefault` | Malformed-LLDP profile (see below) |
| 192.168.7.244 | switch-dlink-01 | v2c | community `netdefault` | D-Link DGS-1210-48 (see below) |
| 192.168.7.245 | switch-tplink-01 | v2c | community `netdefault` | TP-Link TL-SX3016F (see below) |

**LLDP local-port remap (`.238`/`.239`).** ExtremeXOS reports its `lldpRemTable` local-port index as an `lldpLocPortNum` (1..N) that is a **separate namespace from `ifIndex`** (switch-exos-01 uses ifIndex 1001+, ifName `1:N`), so neighbours only resolve if the daemon walks `lldpLocPortTable` (`1.0.8802.1.1.2.1.3.7`) and suffix-matches `lldpLocPortId` against `ifName`. Before the Issue 2 fix, switch-exos-01 yields **zero** LLDP neighbours. Extreme VOSS (switch-voss-01) reports local-port == ifIndex with `lldpLocPortId` matching `ifName` exactly, so it stays correct on both old and new code — the regression guard for the fix.

**L2 neighbour resolution (`.240`/`.241`).** These two are cabled to each other in the fixture data — `switch-netgear-01 g1 ↔ switch-aruba-01 port 41` and `g2 ↔ A5` — and between them cover both halves of a physical link:

- **Chassis MAC that is on no port (#664).** switch-netgear-01's LLDP chassis id is `00:1a:2b:3c:4d:63`, while its ports report `…:65/:66/:67` and it bears no IP with that MAC. switch-aruba-01's neighbour entries advertise that chassis MAC, so the remote host is identifiable **only** through the `chassis_id` recorded from switch-netgear-01's own LLDP local identity. Matching MACs against interfaces and IPs alone yields `hosts_resolved=0` and an empty L2 Physical view.
- **Locally-assigned port ids (#649).** switch-netgear-01's neighbour entries use port-ID subtype 7 with values `41` (which is switch-aruba-01's `ifDescr`) and `197` (which matches only its `ifIndex` — that port is labelled `A5`). Both shapes occur on real Aruba/HP gear. Treating subtype 7 as unresolvable stops resolution at the host, and a host-only neighbour draws **no edge at all**, so the switch is missing from L2 Physical entirely.

Both links should render in L2 Physical, and the server's `LLDP/CDP link resolution complete` line should report `ports_resolved` covering all four neighbour records (two per device).

**High-ifIndex interface persistence (`.242`).** The Omada TL-SG3216 puts its 16 physical ports at ifIndex 49153–49168, reports **no** ifXTable `ifName` for any of them, and returns the same chassis `ifPhysAddress` on every port; only ifIndex 1 (`Vlan-interface1`) carries a name and an IP. All 17 must persist as distinct interfaces. It advertises no LLDP neighbours at all — deliberately, so it exercises the interface-persistence path in isolation. Note its `sysName` is the literal `switch`, matching the reporter's device.

> **MAC octet padding.** Every fixture wrote MACs abbreviated (`0:1a:2b:0:10:0`) until 2026-07-27, and the daemon's string-parsing fallback rejected that form outright — so no LLDP data persisted for *any* sim device and no host ever got a `chassis_id`. Silently: an unparseable chassis id discards the whole neighbour record, which is indistinguishable from a switch that advertises none. The daemon now accepts both forms, and the fixtures are padded **except switch-exos-01's own chassis id**, deliberately left abbreviated as the standing guard for that tolerance (ExtremeXOS is one of the two vendors known to send this identifier as a string rather than octets).

**Malformed neighbour records (`.243`).** A truncated `lldpRemChassisId` column and a device that simply serves no chassis ID are indistinguishable to the daemon — both yield a neighbour carrying a port ID and a system name but no chassis ID. The first is a transient nobody can schedule; the second is static and reproduces on every scan, which is what makes this path testable at all.

Taken at face value that record is destructive: the chassis ID is a mandatory TLV (IEEE 802.1AB), so it is malformed, but writing it through overwrites a good chassis ID with NULL — and a row without one is excluded from L2 resolution entirely, freezing the link at whatever it last resolved to with no way back. That is what stranded `router-gw-01` in July 2026.

`switch-flaky-01` links to `switch-core-01`'s `Gi0/3` (the one port on that switch with no other neighbour) and ships five LLDP variants. The agent serves whichever is copied over `-lldp-active.txt`, and the `pass` handler re-reads its file per request, so swapping takes effect immediately with **no snmpd restart**:

```bash
# on the VM — serve the chassis-less record
cp /etc/snmp-test/data/switch-flaky-01-lldp-nochassis.txt \
   /etc/snmp-test/data/switch-flaky-01-lldp-active.txt

# ...and restore the well-formed one
cp /etc/snmp-test/data/switch-flaky-01-lldp-complete.txt \
   /etc/snmp-test/data/switch-flaky-01-lldp-active.txt
```

The other variants separate the causes that used to arrive as one number (#668). All of them discard the record; the daemon's warning now says which happened — and, decisively, whether a rescan is worth the operator's time — and the counters are what these files exercise:

| Variant file | Serves | Counter it drives | Reported cause |
|---|---|---|---|
| `-lldp-nochassis.txt` | neither `.4` nor `.5`, for the only neighbour | `ghost_rows`, `kept=0` | `GhostRows` — device contributes nothing |
| `-lldp-ghost.txt` | local port 2 in `.6`–`.10` only, port 1 complete | `ghost_rows`, `kept=1` | `GhostRows` — device is present with holes |
| `-lldp-nosubtype.txt` | `.5` only | `missing_subtype` | `IncompleteRecords` |
| `-lldp-badsubtype.txt` | `.4` as a string, `.5` fine | `unexpected_subtype_type="OctetString"` | `UnexpectedType` |

Note what the first two have in common: a chassis column that lists **none** of a row's positions is indistinguishable from one that never had them, so both read as ghost rows. They differ in `kept`, which is what decides whether the warning says the device contributes no physical links at all or only that some are missing — a different place on the map, so they must not read the same.

`-lldp-badsubtype.txt` matters most: it reads as a *complete* walk — no truncation signal anywhere — so before the per-cause counters the only evidence was the record silently going missing.

`-lldp-ghost.txt` is the sparse-chassis-column shape. Until August 2026 it had no fixture at all and was reachable only in unit tests, so the classification that separates it from a cut-short read had never been checked against a real agent.

**No variant serves `WalkCutShort`**, and that is not an omission: it needs a column to stop *mid-walk*, which a static data file cannot stage. It is the one cause where a rescan is the remedy rather than a waste, so it is covered by a unit test that fails the transport partway (`a_cut_short_chassis_column_is_not_reported_as_a_firmware_defect`). The sim env produces it by accident anyway — `pass` forks per request and the agents fall behind under load, which is the noise the header of `lxc/setup.sh` warns about.

Re-running `lxc/setup.sh` also resets it, which is the simplest way to undo a test that left the device broken.

**Port ids that name-only matching cannot resolve (`.244`).** Modelled on the D-Link DGS-1210-48 from #668. Its own ifTable uses the D-Link shape — `ifDescr` = `D-Link DGS-1210-48 Rev.GX/7.20.003 Port N`, `ifName` = `Slot0/N`, `ifIndex` = N — and its two neighbour records, both pointing at `switch-core-01`, each need a different fallback:

- **Subtype 5 carrying a bare port number.** `lldpRemPortId` = `2` with `lldpRemPortIdSubtype` = 5 (`interfaceName`), while switch-core-01's ifIndex 2 is named `GigabitEthernet0/2`/`Gi0/2`. Subtype 5 used to get a name lookup and nothing else, so this resolved to the host and stopped — and a host-only neighbour draws no edge. It now falls through to `ifIndex`, the same ladder subtypes 2/6/7 already had.
- **A port id that matches nothing, and a port description that does.** `lldpRemPortId` = `ethernet1/0/44` is neither a name on that device nor a number, so the id is a dead end; `lldpRemPortDesc` = `GigabitEthernet0/1` is byte-identical to switch-core-01's `ifDescr` for ifIndex 1. That field was stored and never matched on.

Both should resolve to `Neighbor::Interface` and draw edges in L2 Physical. The records deliberately share `Gi0/1`/`Gi0/2` with links other sim devices also claim — the profile exercises port-id resolution, which runs per interface row, not a physically consistent lab.

**A neighbour table indexed without `lldpRemTimeMark` (`.245`).** Modelled on the TP-Link TL-SX3016F from #668, from the reporter's own `snmpwalk`. The MIB indexes `lldpRemEntry` as `lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex`; this firmware omits the time mark and indexes on the remaining two, so every neighbour row arrives one sub-id shorter than on every other device here:

```
.1.0.8802.1.1.2.1.4.1.1.4.1.1 = INTEGER: 4                  # local port 1, remIndex 1
.1.0.8802.1.1.2.1.4.1.1.5.1.1 = STRING: "00:1A:2B:00:10:00"
```

This is the shape that made the device vanish without evidence. A parser requiring three sub-ids built no record, so nothing reached the discard counters, the walk still reported itself complete, and an empty result from a sixteen-port switch was then treated as the device authoritatively reporting no neighbours — clearing the links the server already held. It was the only failure in this query that raised **no warning of any kind**, which is why the reporter's completed scan named every other problem device and not this one. Verify with:

```bash
snmpwalk -v2c -c netdefault 192.168.7.245 1.0.8802.1.1.2.1.4.1.1.4   # two-element index
```

Two further quirks from the same device are kept deliberately, because they decide whether a row that now survives can actually resolve: chassis ids are subtype 4 carrying an **uppercase ASCII MAC** rather than six raw octets, and ports are `ifDescr` `ten-gigabitEthernet 1/0/N` with **no `ifName`** (there is no ifXTable `pass` in its config), alongside a `Vlan-interface1`. Its three neighbours point at `switch-core-01`, `switch-dlink-01` and `switch-netgear-01`, so all three should resolve to real hosts and draw edges in L2 Physical. Its `lldpLocPortNum` equals `ifIndex`, so the local-port remap is the identity mapping and cannot mask the index parse under test.

> **The NUL half of #668 is not reproducible here.** The same D-Links NUL-terminate their port ids (`lldpRemPortId` arrives as `31 00`, i.e. `"1\0"`), which used to fail the write of the entire host. net-snmp's `pass` protocol is line-based — the handler prints OID, type and value as three lines — so an embedded `0x00` cannot survive the transport and no data file can express it. That half is covered by unit tests instead: `value_to_string`, `LldpPortId::from_snmp`, and `PgText`/`PgJson` in `server/shared/storage/pg_value.rs`.

## Expect truncation warnings — the simulator races itself

A scan of this environment normally reports several incomplete SNMP walks. **That is the simulator, not the product under test.**

`snmpd` forks the `pass` handler — a bash script that then forks awk — once per SNMP request. With 14 agents on one VM and ~17 column walks per host, a single scan is hundreds of concurrent forks, and under that load the agents answer some requests with the *wrong* OID: one belonging to a request the daemon made earlier.

Measured 2026-07-27, walking all 12 v2c devices from a single client:

| | truncated |
|---|---|
| serial | **0** of 12 |
| concurrent | **4–5** of 12, a *different* set of devices each run |

Every truncation was a stale response — an in-subtree walk answered with an OID *lower* than the one requested (asking for `lldpRemChassisId` and getting `lldpRemChassisIdSubtype`; asking within ifXTable and being handed an LLDP OID that sorts below the entire subtree). A correct agent walking forward cannot produce that, and it is not the client desyncing: the responses pass request-id and community validation, each session owns its own connected socket and request-id range, and the identical walks are clean run serially.

### The daemon now recovers from it (2026-07-31)

Re-measured after the walk gained a bounded re-ask on a misdirected answer:

| | truncated | re-asks |
|---|---|---|
| concurrent, 2 scans | **0** | **13**, every one `StaleResponse` |

Re-asking is safe because a wrong-OID response is rejected before it reaches the row callback, so nothing from it was collected and the retry cannot duplicate rows. Bounded at two attempts, so an agent answering persistently out of step still reports as truncated rather than spinning the scan.

**The simulator is still doing exactly what it did** — 13 misdirected answers across two scans, on a different set of devices each time. What changed is that the daemon no longer converts them into lost columns. Judge future changes against *0 truncated*, and treat a rise in the re-ask count as the simulator being busier rather than the product regressing.

Note this arrived by way of a null result worth remembering: an earlier retry keyed on `snmp2::Error::RequestIdMismatch` — a transport error where the session rejects the datagram — and moved these numbers not at all, because the simulator never produces that. A customer's switches did. Two environments, two different faults; the first fix was aimed at the wrong one.

The remaining `set_complete=false` results here are a *different* simulator misbehaviour: columns answering for ifIndexes the device never listed (`foreign_rows=1, 7, 19` in one two-scan window). Those rows are discarded and reported separately, so warning count is not a clean proxy for truncation — count `SNMP walk truncated` lines instead.

### Control: a correct agent under load (2026-07-31)

The agents here are deliberately incorrect, so they cannot answer "is the *scanner* losing data?". A control agent settles it: one `snmpd` with **no `pass` directives at all**, serving the built-in MIBs from real kernel state, on its own macvlan behind `tc netem`.

| impairment | rescans | truncations | re-asks | ifTable |
|---|---|---|---|---|
| none | 5 | 0 | 0 | 17/17, complete |
| 200 ms delay, 2% loss | 6 | 0 | 0 | 17/17, complete |
| 400 ms delay, 15% loss | 6 | 0 | 0 | 17/17, complete |

Against an agent that is correct by construction, the scanner loses nothing — including at loss rates well past anything a LAN produces. (At 800 ms / 55% loss only 1 of 3 rescans got past the probe at all, so that arm proves little either way.)

To rebuild it: a macvlan on the parent interface, a conf file with `agentAddress`, `rocommunity` and the `sys*` values and **nothing else**, a systemd unit running `snmpd -f -Lo -C -c <conf>` — note *without* the `-I -ifTable,-ifXTable` the other units carry, since here the built-in implementations are the point — then `tc qdisc replace dev <iface> root netem delay Xms loss Y%`. Remove the qdisc, unit, conf and macvlan afterwards.

**How to read a scan.** Judge a change by whether *data* was lost — interfaces pruned, neighbours wiped, links frozen — not by whether warnings appeared. A warning saying previously discovered values "were kept rather than overwritten" is the daemon handling the chaos correctly.

**Worth keeping.** This free adversarial agent surfaced three real defects in July 2026: a foreign interface appearing on a switch, a chassis ID overwritten with NULL leaving a link permanently unresolvable, and a truncated column reported as authoritative. If the noise ever needs quieting, `pass_persist` replaces the fork-per-request with one long-lived handler per agent — but leave a device or two on `pass` deliberately, or the environment loses the property that found those bugs.

**What a scan exercises (session-reuse + getbulk).** Every device is scanned with a single reused SNMP session across all ~11 queries (one v3 engine discovery instead of ~12), and each table is walked with `getbulk` (v1 falls back to `getnext`). To make the getbulk walks land on real data for the subtrees stock `snmpd` does **not** implement:
- **switch-core-01** additionally serves BRIDGE-MIB / Q-BRIDGE (`dot1dBasePortIfIndex`, `dot1qVlanStaticName` → VLANs "DATA"/"VOICE", `dot1qPvid`), ENTITY-MIB (chassis inventory) and CDP (a `router-gw-01` neighbour) — exercising those getbulk walks end-to-end.
- **legacy-switch-01 (v1-only)** additionally serves a small bridge table, so the **getbulk → getnext fallback** is exercised on a non-ifTable walk, not just ifTable/LLDP.
- `ipAddrTable` and `ipNetToMedia` (ARP) are answered by snmpd's built-in IP module, so those walks run on every device already. (net-snmp `pass` can't emit binary MAC octet-strings, so FDB/ARP MAC *rows* aren't simulated — the daemon still walks those subtrees and terminates cleanly.)
- **ap-wireless-01** is the one exception: it serves its own `ipAddrTable` so it can advertise a second subnet (see below).

**Access-point guest subnet (`.235`) — #663.** The built-in IP module answers `ipAddrTable` from the VM's real kernel state, so every other agent only ever reports addresses inside the scanned `192.168.4.0/22`. `ap-wireless-01` overrides it and serves the table from `ap-wireless-01-ipaddr.txt`, advertising **172.30.10.1/24 on ifIndex 4**, whose `ifName` is **`br-guest`** — the built-in NAT guest network of a real access point.

> Unlike ifTable/ifXTable, this subtree **cannot** be freed by disabling its module: `-I -ipaddr` (or `-ipAddr`) does not stop `mibII/ipaddr` registering. It also registers per *column* (`.4.20.1.1`…`.4.20.1.5`), so a single `pass` at the `.4.20` root always loses on specificity, whatever priority it carries. The override therefore registers one `pass -p 1` per column — matching granularity and beating the default priority of 255. Confirm what owns a subtree with `snmpd -Dregister_mib -C -c <conf>`.

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
ssh -i ~/.ssh/snmp-test-vm root@192.168.7.230 'rm -rf /root/snmp-test' \
  && scp -i ~/.ssh/snmp-test-vm -r tools/snmp root@192.168.7.230:/root/snmp-test \
  && ssh -i ~/.ssh/snmp-test-vm root@192.168.7.230 'bash /root/snmp-test/lxc/setup.sh'
```

Hosts that gained nothing are effectively no-ops; anything whose data file, config or unit changed comes back with the new content.

> **The `rm -rf` is required, not tidiness.** `scp -r tools/snmp <host>:/root/snmp-test` only lands at that path the *first* time. Once `/root/snmp-test` exists, scp copies *into* it — the new tree lands at `/root/snmp-test/snmp/` while `bash /root/snmp-test/lxc/setup.sh` re-runs the **stale** copy. Every agent restarts and the run reports success, so this fails silently and looks like a broken fixture rather than a stale deploy. Sanity-check with `grep -c br-guest /root/snmp-test/lxc/setup.sh` before running it.

> **SSH key.** The VM accepts publickey only (password auth is disabled) and there is no `~/.ssh/config` entry, so `-i ~/.ssh/snmp-test-vm` is required or you get `Permission denied (publickey)`. Add a `Host 192.168.7.2*` / `IdentityFile ~/.ssh/snmp-test-vm` block to `~/.ssh/config` to drop the flag.

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
