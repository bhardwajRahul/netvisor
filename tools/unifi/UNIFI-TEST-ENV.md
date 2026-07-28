# UniFi Controller Test Environment

A self-hosted UniFi controller for developing and validating the UniFi integration
(`backend/src/daemon/discovery/integration/unifi/`).

## What this validates — and what it does not

**Read this before trusting a green run.**

| | Validated here | Needs real hardware |
|---|---|---|
| API-key auth (`X-API-KEY`) | ✅ | |
| Local-admin login + session cookie | ✅ | |
| UniFi OS vs legacy path detection | ✅ | |
| `{"meta":…,"data":[…]}` envelope | ✅ | |
| Site scoping, 401 vs 404 error shapes | ✅ | |
| Self-signed TLS handling | ✅ | |
| `port_table` → interfaces | | ❌ |
| `lldp_table` → LLDP neighbors | | ❌ |
| `mac_table` → bridge FDB | | ❌ |
| `uplink` / `downlink_table` → topology edges | | ❌ |

A controller with no adopted devices returns `"data": []` from `stat/device`. That confirms the
envelope and proves **nothing** about the device sub-table shapes — which are precisely the
fields Ubiquiti does not document and that we inferred from the unpoller Go structs. Those stay
unvalidated until either real hardware is adopted here or the customer's captured `stat/device`
arrives. This is why the credential type ships as `stability: Beta`.

## Which controller to install

API-key support is **UniFi OS only**:

| Port | Controller | API key | Local admin |
|---|---|---|---|
| 443 | UniFi OS console (UDM / Cloud Key / Cloud Gateway) | ✅ | ✅ |
| 11443 | **UniFi OS Server** (self-hosted) | ✅ | ✅ |
| 8443 | legacy self-hosted Network Application | ❌ **unsupported** | ✅ |

Install **UniFi OS Server** — it is the only self-hostable option that exercises the API-key
transport. To also exercise the legacy path, additionally run a Network Application container
(see "Legacy controller" below).

## Provisioning UniFi OS Server (Proxmox VM)

Requirements per Ubiquiti:

- Ubuntu 24.04+ or Debian 13+ (a Proxmox VM is fine; Hyper-V guests are explicitly unsupported)
- Podman ≥ 4.3.1 and slirp4netns ≥ 1.2
- ≥ 20 GB free disk
- Ports: 3478, 5005, 5514, 6789, 8080, 8444, 8880, 8881, 8882, 9543, 10003, **11443**

**Networking first.** If the VM was installed without a network, it will come up with only
`lo` and no default route, and every later step fails in a confusing way (`apt` reports packages
as "not available" when it really has no package lists at all). Verify before anything else:

```bash
ip -4 addr show && ip route      # expect an address on ens18 and a default route
ping -c1 archive.ubuntu.com
```

If `ens18` has no address, write `/etc/netplan/01-netcfg.yaml` with `dhcp4: true` (or a static
`addresses:` / `routes:` / `nameservers:` block), `sudo netplan apply`, and re-check. If the link
shows `NO-CARRIER`, it is the hypervisor side — check the VM's NIC is on the right bridge/VLAN.

```bash
sudo apt-get update
sudo apt-get install -y podman slirp4netns curl
podman --version                 # must be >= 4.3.1; Docker is NOT a supported substitute
```

`podman` lives in `universe`; if it is missing, `sudo add-apt-repository universe` first.

**Then download the installer** — it is not bundled with anything and the URL is
version-specific, so it has to be copied from Ubiquiti's page each time:

1. Open <https://ui.com/download/software/unifi-os-server> in a browser.
2. Pick the Linux build matching the VM's architecture (`uname -m` → `x86_64` or `aarch64`).
3. Right-click the download button → **Copy Link Address** (a
   `https://fw-download.ubnt.com/data/unifi-os-server/…` URL). These expire, so fetch a fresh
   one rather than reusing an old link.

```bash
mkdir -p ~/uos && cd ~/uos
wget '<paste-the-URL>'           # quote it: the URL contains & and ?
chmod +x unifi-os-server*
sudo ./unifi-os-server*
```

Then, in a browser at `https://<vm-ip>:11443`:

1. Complete first-run setup and create the console owner account.
2. **Create a local-only admin** for the integration: Settings → Admins & Users → Add Admin →
   *Restrict to Local Access*. A local-only account avoids the MFA prompt that blocks
   programmatic login on cloud-linked accounts.
3. **Create an API key**: Settings → Control Plane → Integrations → Create API Key. Copy it
   immediately; it is shown once.
4. Note the **internal site name** from the URL when viewing the site:
   `/manage/site/<name>` — this is what the credential's `site` field wants, *not* the site's
   display name. A fresh install is `default`.

### Legacy controller (optional, for the 8443 path)

```bash
podman run -d --name unifi-legacy --network host \
  -e TZ=UTC \
  -v unifi-legacy-config:/config \
  lscr.io/linuxserver/unifi-network-application:latest
```

Reach it at `https://<host>:8443`. Use it to confirm the legacy `/api/login` path and that an
API key really is rejected there — the integration surfaces a specific error message for that
case, and it should be checked rather than assumed.

## Running the checks

```bash
export UNIFI_HOST=192.168.7.240
export UNIFI_PORT=11443
export UNIFI_SITE=default
export UNIFI_API_KEY='...'
export UNIFI_USERNAME='scanopy'
export UNIFI_PASSWORD='...'

make unifi-status     # authenticate over both transports, detect the API layout
make unifi-capture    # write stat/sysinfo + stat/device to tools/unifi/captures/
```

`make unifi-status` reports each transport independently, so a controller that supports only
one still gives a useful result.

## Using captures as fixtures

`tools/unifi/captures/` is gitignored — captures contain MACs, IPs and device names.

To promote a capture into the test suite, copy it to `backend/src/tests/unifi/` and reference it
from the test module in `.../integration/unifi/mapping.rs`. **Update that module's provenance
comment**: the existing fixtures are explicitly labelled as hand-authored from unpoller structs,
and a captured payload must be labelled as captured. The distinction is the difference between
"our mapping rules are self-consistent" and "we parse real hardware correctly" — do not blur it.

## End-to-end against Scanopy

1. Create a **UniFi API Key** (or **UniFi Local Admin**) credential in the UI. It carries a
   **Beta** tag — that is expected.
2. Target it at the controller's host, or at the daemon host if the controller runs there.
3. Run a discovery covering the controller's IP.
4. Expect: the controller host gains a *UniFi Controller* service; each adopted device becomes a
   host with a UniFi Switch / Access Point / Gateway service (matched from the controller's
   reported device type, not stamped); switch ports appear as interfaces; and LLDP neighbors
   resolve into L2 Physical topology edges.

With no adopted devices, only step 4's first clause is observable.
