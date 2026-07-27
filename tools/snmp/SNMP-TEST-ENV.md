# SNMP Test Environment

10 simulated network devices running on a Proxmox VM, each on port 161. Most speak SNMPv2c; `.236`/`.237` are version-locked to exercise the SNMPv1 and SNMPv3 paths (#557); `.238`/`.239` are Extreme switches that exercise the LLDP local-port remap (Issue 2, July 2026).

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

**LLDP local-port remap (`.238`/`.239`).** ExtremeXOS reports its `lldpRemTable` local-port index as an `lldpLocPortNum` (1..N) that is a **separate namespace from `ifIndex`** (switch-exos-01 uses ifIndex 1001+, ifName `1:N`), so neighbours only resolve if the daemon walks `lldpLocPortTable` (`1.0.8802.1.1.2.1.3.7`) and suffix-matches `lldpLocPortId` against `ifName`. Before the Issue 2 fix, switch-exos-01 yields **zero** LLDP neighbours. Extreme VOSS (switch-voss-01) reports local-port == ifIndex with `lldpLocPortId` matching `ifName` exactly, so it stays correct on both old and new code — the regression guard for the fix.

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

## Setup

Paste the contents of `tools/snmp/lxc/setup.sh` into a root shell on a Debian/Ubuntu VM with primary IP 192.168.7.230/22.

Before pasting, verify:
- Interface is `eth0` (`ip link`) — edit `IFACE=` if different
- Primary IP is 192.168.7.230 — edit `HOSTS=()` if different

## Patch: migrate secondary IPs to macvlan (unique MACs)

If each device shares the host's MAC (secondary IPs on eth0), run on the VM:

```bash
IFACE=eth0; CIDR=22; HOSTS=(192.168.7.230 192.168.7.231 192.168.7.232 192.168.7.233 192.168.7.234 192.168.7.235 192.168.7.236 192.168.7.237); for i in "${!HOSTS[@]}"; do ip addr del "${HOSTS[$i]}/$CIDR" dev "$IFACE" 2>/dev/null; ip link del "mv-snmp${i}" 2>/dev/null; ip link add "mv-snmp${i}" link "$IFACE" type macvlan mode bridge; ip addr add "${HOSTS[$i]}/$CIDR" dev "mv-snmp${i}"; ip link set "mv-snmp${i}" up; done && sysctl -w net.ipv4.conf.all.arp_ignore=1 net.ipv4.conf.all.arp_announce=2 && for iface in mv-snmp0 mv-snmp1 mv-snmp2 mv-snmp3 mv-snmp4 mv-snmp5 mv-snmp6 mv-snmp7 eth0; do sysctl -w net.ipv4.conf.${iface}.arp_ignore=1 net.ipv4.conf.${iface}.arp_announce=2; done
```

Then flush the ARP cache on the scanning host (`sudo arp -a -d` on macOS).

## Patch: fix duplicate MIB registration

If snmpd logs show `duplicate registration: MIB modules ifTable and pass`, run:

```bash
for f in /etc/systemd/system/snmpd-*.service; do sed -i 's|snmpd -f -Lo -C|snmpd -f -Lo -I -ifTable,-ifXTable -C|' "$f"; done && systemctl daemon-reload && for name in switch-core-01 switch-access-01 router-gw-01 firewall-01 printer-lobby ap-wireless-01 legacy-switch-01 secure-switch-01; do systemctl restart "snmpd-${name}"; done
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
