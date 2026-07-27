use crate::daemon::discovery::integration::container::ContainerRuntime;
use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::subnets::r#impl::virtualization::SubnetVirtualization;
use anyhow::Error;
use anyhow::anyhow;
use async_trait::async_trait;
use bollard::query_parameters::ListNetworksOptions;
use bollard::{API_DEFAULT_VERSION, Docker};
use cidr::IpCidr;
use local_ip_address::local_ip;
use mac_address::MacAddress;
// net-route has no BSD support; FreeBSD/OpenBSD use netdev instead (see
// `get_own_routing_table_gateway_ips`). Keep the import off BSD so net-route is never compiled there.
#[cfg(not(any(target_os = "freebsd", target_os = "openbsd")))]
use net_route::Handle;
use pnet::ipnetwork::IpNetwork;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

/// Negotiate the container daemon's API version after a successful ping.
///
/// bollard pins `API_DEFAULT_VERSION` and never negotiates, so it talks to a daemon's
/// Docker-compatible API using its newest response models. Podman's compat layer advertises
/// an older Docker API level, so those newer models can fail to deserialize — surfacing as
/// "no containers found." Negotiating downgrades the client to the daemon's advertised
/// version. `Docker` is cheaply cloneable (Arc internally), so we clone before negotiating
/// and fall back to the original default-version client if the `/version` call fails.
///
/// Best-effort and capped by a short timeout: the client is fully usable on the default
/// version, so a slow/unresponsive `/version` (e.g. a sluggish `podman machine` VM) must not
/// stall discovery. Without this cap the call inherits bollard's connect timeout (up to 120s),
/// which blocks every scan for two minutes before falling back.
const NEGOTIATE_VERSION_TIMEOUT: Duration = Duration::from_secs(5);

async fn negotiate_container_api_version(client: Docker) -> Docker {
    match tokio::time::timeout(
        NEGOTIATE_VERSION_TIMEOUT,
        client.clone().negotiate_version(),
    )
    .await
    {
        Ok(Ok(negotiated)) => negotiated,
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "Container API version negotiation failed; using default version");
            client
        }
        Err(_) => {
            tracing::warn!(
                timeout_secs = NEGOTIATE_VERSION_TIMEOUT.as_secs(),
                "Container API version negotiation timed out; using default version"
            );
            client
        }
    }
}

pub const SCAN_TIMEOUT: Duration = Duration::from_millis(800);

/// Cross-platform system utilities trait
#[async_trait]
pub trait DaemonUtils {
    fn new() -> Self;

    /// Get MAC address for an IP from ARP table
    async fn get_mac_address_for_ip(&self, ip: IpAddr) -> Result<Option<MacAddress>, Error>;

    fn get_fd_limit() -> Result<usize, Error>;

    fn get_own_ip_address(&self) -> Result<IpAddr, Error> {
        match local_ip() {
            Ok(ip) => {
                tracing::debug!(ip = %ip, "Detected local IP address");
                Ok(ip)
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to detect local IP address. This may occur in MACVLAN containers \
                     or environments without a default route."
                );
                Err(anyhow!("Failed to get local IP address: {}", e))
            }
        }
    }

    fn get_own_mac_address(&self) -> Result<Option<MacAddress>, Error> {
        mac_address::get_mac_address().map_err(|e| anyhow!("Failed to get own MAC address: {}", e))
    }

    fn get_own_hostname(&self) -> Option<String> {
        hostname::get()
            .ok()
            .map(|os_str| os_str.to_string_lossy().into_owned())
    }

    async fn get_own_interfaces(
        &self,
        network_id: Uuid,
        interface_filter: &[String],
    ) -> Result<
        (
            Vec<IPAddress>,
            Vec<Subnet>,
            HashMap<IpCidr, Option<MacAddress>>,
        ),
        Error,
    > {
        let all_interfaces = pnet::datalink::interfaces();

        // Apply interface filter if specified
        let ip_addresses: Vec<_> = if interface_filter.is_empty() {
            all_interfaces
        } else {
            let filtered: Vec<_> = all_interfaces
                .into_iter()
                .filter(|iface| interface_filter.iter().any(|f| f == &iface.name))
                .collect();

            if filtered.is_empty() {
                tracing::warn!(
                    filter = ?interface_filter,
                    "No ip_addresses matched the filter. Check --ip_address argument."
                );
            } else {
                tracing::debug!(
                    filter = ?interface_filter,
                    matched = filtered.len(),
                    "Filtered ip_addresses by --ip_addresses argument"
                );
            }

            filtered
        };

        tracing::debug!(
            interface_count = ip_addresses.len(),
            "Enumerating network ip_addresses"
        );

        for ip_address in &ip_addresses {
            tracing::debug!(
                name = %ip_address.name,
                index = ip_address.index,
                is_up = ip_address.is_up(),
                is_loopback = ip_address.is_loopback(),
                mac = ?ip_address.mac,
                ips = ?ip_address.ips,
                flags = ip_address.flags,
                "Found ip_address"
            );
        }

        // First pass: collect all interface data and potential subnets
        let mut potential_subnets: Vec<(String, IpNetwork)> = Vec::new();
        let mut interface_data: Vec<(String, IpAddr, Option<MacAddress>)> = Vec::new();

        for ip_address in ip_addresses.into_iter() {
            let name = ip_address.name.clone();

            // Container bridges on this host belong to the container integration,
            // which types them from the runtime API. Skip them here so an
            // unreachable runtime can't leave them to be created and scanned as
            // ordinary subnets (and so the heartbeat never reports them at all).
            if ContainerRuntime::reserves_host_interface_name(&name) {
                tracing::debug!(
                    interface = %name,
                    "Skipping container-runtime bridge interface"
                );
                continue;
            }

            let mac_address = match ip_address.mac {
                Some(mac) if !mac.octets().iter().all(|o| *o == 0) => {
                    Some(MacAddress::new(mac.octets()))
                }
                _ => None,
            };

            for ip in ip_address.ips.iter() {
                // APIPA (169.254.x.x) is defined as exactly /16 by RFC 3927.
                // Windows can report bogus prefixes (e.g. /0) via pnet — correct them.
                let ip = match ip {
                    IpNetwork::V4(v4)
                        if v4.ip().octets()[0] == 169
                            && v4.ip().octets()[1] == 254
                            && v4.prefix() != 16 =>
                    {
                        tracing::warn!(
                            ip = %v4.ip(),
                            reported_prefix = v4.prefix(),
                            "Correcting APIPA ip_address prefix to /16"
                        );
                        IpNetwork::V4(pnet::ipnetwork::Ipv4Network::new(v4.ip(), 16).unwrap_or(*v4))
                    }
                    other => *other,
                };
                interface_data.push((name.clone(), ip.ip(), mac_address));
                potential_subnets.push((name.clone(), ip));
            }
        }

        // Second pass: create unique subnets from valid networks
        let mut subnet_map: HashMap<IpCidr, Subnet> = HashMap::new();

        for (interface_name, ip_network) in potential_subnets {
            if let Some(subnet) = Subnet::from_discovery(interface_name, &ip_network, network_id) {
                subnet_map.entry(subnet.base.cidr).or_insert(subnet);
            }
        }

        // Third pass: assign all ip_addresses to appropriate subnets
        let mut ip_addresses = Vec::new();
        let mut cidr_to_mac = HashMap::new();

        for (interface_name, ip_addr, mac_address) in interface_data {
            // Find which subnet this IP belongs to
            if let Some(subnet) = subnet_map
                .values()
                .filter(|s| s.base.cidr.contains(&ip_addr))
                .max_by_key(|s| s.base.cidr.network_length())
            {
                cidr_to_mac
                    .entry(subnet.base.cidr)
                    .and_modify(|existing: &mut Option<MacAddress>| {
                        // Prefer a valid MAC over None
                        if existing.is_none() && mac_address.is_some() {
                            *existing = mac_address;
                        }
                    })
                    .or_insert(mac_address);

                ip_addresses.push(IPAddress::new(IPAddressBase {
                    network_id: subnet.base.network_id,
                    host_id: Uuid::nil(), // Placeholder - server will set correct host_id
                    name: Some(interface_name),
                    subnet_id: subnet.id,
                    ip_address: ip_addr,
                    mac_address,
                    position: ip_addresses.len() as i32,
                }));
            }
        }

        let subnets: Vec<Subnet> = subnet_map.into_values().collect();

        Ok((ip_addresses, subnets, cidr_to_mac))
    }

    async fn new_docker_client(
        &self,
        docker_proxy: Result<Option<String>, Error>,
        docker_proxy_ssl_info: Result<Option<(String, String, String)>, Error>,
    ) -> Result<Docker, Error> {
        use tokio::time::timeout;

        const DOCKER_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

        tracing::debug!("Creating Docker client connection");
        let start = std::time::Instant::now();

        let client = if let Ok(Some(docker_proxy)) = docker_proxy {
            tracing::debug!(proxy = %docker_proxy, "Using Docker proxy");
            if docker_proxy.contains("https://")
                && let Ok(Some((cert, key, chain))) = docker_proxy_ssl_info
            {
                let cert_path = PathBuf::from(cert);
                let key_path = PathBuf::from(key);
                let chain_path = PathBuf::from(chain);

                Docker::connect_with_ssl(
                    &docker_proxy,
                    &key_path,
                    &cert_path,
                    &chain_path,
                    15,
                    API_DEFAULT_VERSION,
                )
                .map_err(|e| anyhow::anyhow!("Failed to connect to Docker: {}", e))?
            } else {
                Docker::connect_with_http(&docker_proxy, 4, API_DEFAULT_VERSION)
                    .map_err(|e| anyhow::anyhow!("Failed to connect to Docker: {}", e))?
            }
        } else {
            tracing::debug!("Using Docker local defaults");
            Docker::connect_with_local_defaults()
                .map_err(|e| anyhow::anyhow!("Failed to connect to Docker: {}", e))?
        };

        // Ping Docker with retry and exponential backoff
        const MAX_PING_ATTEMPTS: u32 = 3;
        let mut last_error = None;
        for attempt in 1..=MAX_PING_ATTEMPTS {
            match timeout(DOCKER_CONNECT_TIMEOUT, client.ping()).await {
                Ok(Ok(_)) => {
                    tracing::debug!(
                        elapsed_ms = start.elapsed().as_millis(),
                        attempt,
                        "Docker client connected successfully"
                    );
                    return Ok(negotiate_container_api_version(client).await);
                }
                Ok(Err(e)) => {
                    last_error = Some(format!("Docker ping failed: {}", e));
                    if attempt < MAX_PING_ATTEMPTS {
                        let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                        tracing::warn!(
                            attempt,
                            backoff_ms = backoff.as_millis(),
                            error = %e,
                            "Docker ping failed, retrying"
                        );
                        tokio::time::sleep(backoff).await;
                    }
                }
                Err(_) => {
                    last_error = Some(format!(
                        "Docker connection timed out after {:?}",
                        DOCKER_CONNECT_TIMEOUT
                    ));
                    if attempt < MAX_PING_ATTEMPTS {
                        let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                        tracing::warn!(
                            attempt,
                            backoff_ms = backoff.as_millis(),
                            "Docker ping timed out, retrying"
                        );
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }
        tracing::warn!(
            elapsed_ms = start.elapsed().as_millis(),
            "Docker ping failed after {} attempts",
            MAX_PING_ATTEMPTS
        );
        Err(anyhow::anyhow!(
            last_error.unwrap_or_else(|| "Docker connection failed".to_string())
        ))
    }

    /// Connect to a container runtime's local Unix socket via bollard's
    /// Docker-compatible client. `Some(path)` connects to an explicit socket.
    /// `None` falls back to bollard's Docker defaults (`DOCKER_HOST` /
    /// `/var/run/docker.sock`) — valid ONLY for `ContainerRuntime::Docker`. For
    /// `Podman`, a `None` path is an error: Podman must never silently connect to
    /// the Docker socket (which would discover Docker containers as Podman). Pings
    /// to verify before returning.
    async fn new_container_socket_client(
        &self,
        runtime: ContainerRuntime,
        socket_path: Option<String>,
    ) -> Result<Docker, Error> {
        use tokio::time::timeout;

        const DOCKER_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
        const MAX_PING_ATTEMPTS: u32 = 3;

        let start = std::time::Instant::now();
        let client = match (&socket_path, runtime) {
            (Some(path), _) => {
                tracing::debug!(socket = %path, runtime = runtime.label(), "Connecting to container socket");
                Docker::connect_with_socket(path, 120, API_DEFAULT_VERSION).map_err(|e| {
                    anyhow::anyhow!("Failed to connect to container socket {}: {}", path, e)
                })?
            }
            // No explicit path: only Docker may use bollard's local defaults.
            (None, ContainerRuntime::Docker) => {
                tracing::debug!("Using Docker local defaults");
                Docker::connect_with_local_defaults()
                    .map_err(|e| anyhow::anyhow!("Failed to connect to Docker: {}", e))?
            }
            // Podman with no resolved socket: fail cleanly rather than fall back to
            // the Docker default socket.
            (None, ContainerRuntime::Podman) => {
                return Err(anyhow::anyhow!(
                    "No Podman socket found — checked CONTAINER_HOST, /run/podman/podman.sock, \
                     and $XDG_RUNTIME_DIR/podman/podman.sock. Set the credential's socket_path \
                     or CONTAINER_HOST to the Podman socket."
                ));
            }
        };

        let mut last_error = None;
        for attempt in 1..=MAX_PING_ATTEMPTS {
            match timeout(DOCKER_CONNECT_TIMEOUT, client.ping()).await {
                Ok(Ok(_)) => return Ok(negotiate_container_api_version(client).await),
                Ok(Err(e)) => {
                    last_error = Some(format!("Container socket ping failed: {}", e));
                }
                Err(_) => {
                    last_error = Some(format!(
                        "Container socket connection timed out after {:?}",
                        DOCKER_CONNECT_TIMEOUT
                    ));
                }
            }
            if attempt < MAX_PING_ATTEMPTS {
                let backoff = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                tokio::time::sleep(backoff).await;
            }
        }
        tracing::debug!(
            elapsed_ms = start.elapsed().as_millis(),
            "Container socket ping failed after {} attempts",
            MAX_PING_ATTEMPTS
        );
        Err(anyhow::anyhow!(last_error.unwrap_or_else(|| {
            "Container socket connection failed".to_string()
        })))
    }

    async fn get_subnets_from_docker_networks(
        &self,
        network_id: Uuid,
        client: &Docker,
        runtime: ContainerRuntime,
        runtime_service_id: Uuid,
    ) -> Result<Vec<Subnet>, Error> {
        let subnets: Vec<Subnet> = client
            .list_networks(None::<ListNetworksOptions>)
            .await?
            .into_iter()
            .filter_map(|n| {
                let driver = n.driver.clone().unwrap_or_else(|| "bridge".to_string());
                let network_name = n.name.clone().unwrap_or("Unknown Network".to_string());
                let configs = n.ipam.and_then(|ipam| ipam.config)?;
                Some((network_name, driver, configs))
            })
            .flat_map(|(network_name, driver, configs)| {
                configs
                    .iter()
                    .filter_map(|c| {
                        let cidr = IpCidr::from_str(c.subnet.as_ref()?).ok()?;
                        runtime.subnet_from_network(
                            network_id,
                            cidr,
                            network_name.clone(),
                            &driver,
                            runtime_service_id,
                        )
                    })
                    .collect::<Vec<Subnet>>()
            })
            .collect();

        Ok(subnets)
    }

    /// Collect all gateway IPs from the host routing table.
    ///
    /// Uses `net-route` on linux/macos/windows and `netdev` on FreeBSD/OpenBSD (net-route has no
    /// BSD cfg arm). Both arms return the same shape — every distinct routing-table gateway — so
    /// callers are platform-agnostic.
    #[cfg(not(any(target_os = "freebsd", target_os = "openbsd")))]
    async fn get_own_routing_table_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        let routing_handle = Handle::new()?;
        let routes = routing_handle.list().await?;

        Ok(routes
            .into_iter()
            .filter_map(|r| match r.gateway {
                Some(gateway) if gateway != r.destination => Some(gateway),
                _ => None,
            })
            .collect())
    }

    /// BSD variant: net-route lacks BSD cfg arms, so use netdev, which does BSD gateway discovery
    /// via `libc`/sysctl in-code. Each interface exposes its gateway's v4/v6 addresses; the union
    /// reproduces the "all routing-table gateways" set the net-route arm returns.
    ///
    /// Hardware-validation note: confirm on a multi-homed FreeBSD host that this matches the full
    /// gateway set; if netdev under-reports, fall back to a `sysctl(NET_RT_DUMP)` route reader
    /// isolated to this cfg arm.
    #[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
    async fn get_own_routing_table_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        let gateways = netdev::get_interfaces()
            .into_iter()
            .filter_map(|iface| iface.gateway)
            .flat_map(|gw| {
                gw.ipv4
                    .into_iter()
                    .map(IpAddr::V4)
                    .chain(gw.ipv6.into_iter().map(IpAddr::V6))
                    .collect::<Vec<_>>()
            })
            .collect();

        Ok(gateways)
    }

    /// Get optimal concurrency for ARP scanning (OS-specific due to BPF limits on macOS)
    fn get_optimal_arp_concurrency(&self) -> Result<usize, Error>;

    /// Get optimal concurrency for deep port scanning.
    ///
    /// # Arguments
    /// * `port_batch_size` - Number of ports scanned concurrently per host in deep scan
    /// * `arp_subnet_count` - Number of ARP datalink channels currently open (2 FDs each)
    fn get_optimal_deep_scan_concurrency(
        &self,
        port_batch_size: usize,
        arp_subnet_count: usize,
    ) -> Result<usize, Error>;
}

/// Merge host (physical) and Docker subnets, giving host subnets precedence.
/// - Host subnets are always kept
/// - Docker subnets with CIDRs matching host subnets are dropped (host wins),
///   but if the dropped one was a container *bridge*, the host record adopts its
///   `subnet_type` and `virtualization`
///
/// The host record wins on identity (its CIDR and interface-derived name are
/// what the rest of discovery keys off), but its type is only a guess from an
/// interface name, whereas the Docker record's came from the runtime API. For a
/// bridge — a distinct L3 network the runtime owns — that authoritative
/// classification carries over, or the bridge would keep the guess and lose the
/// `virtualization` that scopes it to its owning runtime service.
///
/// MacVLAN/IpVLAN are deliberately excluded: they are not separate networks but
/// overlays on the physical LAN the host is already on, so the host's own
/// classification is the correct one and adopting theirs would relabel a real
/// LAN as a container network.
///
/// Callers that don't want DockerBridge subnets (e.g., self-report) should
/// filter them separately after calling this function.
pub fn merge_host_and_docker_subnets(
    host_subnets: Vec<Subnet>,
    docker_subnets: Vec<Subnet>,
) -> Vec<Subnet> {
    let host_cidrs: HashSet<IpCidr> = host_subnets.iter().map(|s| s.base.cidr).collect();

    let mut authoritative: HashMap<IpCidr, (SubnetType, Option<SubnetVirtualization>)> =
        HashMap::new();

    let filtered_docker: Vec<Subnet> = docker_subnets
        .into_iter()
        .filter(|s| {
            let dominated_by_host = host_cidrs.contains(&s.base.cidr);
            if dominated_by_host {
                tracing::debug!(
                    cidr = %s.base.cidr,
                    subnet_type = ?s.base.subnet_type,
                    "Filtering out Docker subnet (host takes precedence)"
                );
                if s.base.subnet_type.is_container_bridge() {
                    authoritative.insert(
                        s.base.cidr,
                        (s.base.subnet_type, s.base.virtualization.clone()),
                    );
                }
            }
            !dominated_by_host
        })
        .collect();

    let host_subnets: Vec<Subnet> = host_subnets
        .into_iter()
        .map(|mut s| {
            if let Some((subnet_type, virtualization)) = authoritative.remove(&s.base.cidr) {
                s.base.subnet_type = subnet_type;
                s.base.virtualization = virtualization;
            }
            s
        })
        .collect();

    [host_subnets, filtered_docker].concat()
}

#[cfg(target_os = "linux")]
use crate::daemon::utils::linux::LinuxDaemonUtils;
#[cfg(target_os = "linux")]
pub type PlatformDaemonUtils = LinuxDaemonUtils;

#[cfg(target_os = "macos")]
use crate::daemon::utils::macos::MacOsDaemonUtils;
#[cfg(target_os = "macos")]
pub type PlatformDaemonUtils = MacOsDaemonUtils;

#[cfg(target_family = "windows")]
use crate::daemon::utils::windows::WindowsDaemonUtils;
#[cfg(target_family = "windows")]
pub type PlatformDaemonUtils = WindowsDaemonUtils;

#[cfg(target_os = "freebsd")]
use crate::daemon::utils::bsd::BsdDaemonUtils;
#[cfg(target_os = "freebsd")]
pub type PlatformDaemonUtils = BsdDaemonUtils;

pub fn create_system_utils() -> PlatformDaemonUtils {
    PlatformDaemonUtils::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::shared::types::entities::EntitySource;
    use crate::server::subnets::r#impl::base::SubnetBase;
    use std::str::FromStr;

    fn make_subnet(cidr: &str, subnet_type: SubnetType) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: IpCidr::from_str(cidr).unwrap(),
            network_id: Uuid::nil(),
            name: String::new(),
            description: None,
            subnet_type,
            virtualization: None,
            source: EntitySource::Manual,
            tags: Vec::new(),
        })
    }

    #[test]
    fn macvlan_overlap_keeps_physical_only() {
        let host = vec![make_subnet("192.168.1.0/24", SubnetType::Lan)];
        let docker = vec![make_subnet("192.168.1.0/24", SubnetType::MacVlan)];

        let result = merge_host_and_docker_subnets(host, docker);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].base.subnet_type, SubnetType::Lan);
    }

    #[test]
    fn docker_bridge_kept_when_no_overlap() {
        let host = vec![];
        let docker = vec![make_subnet("172.17.0.0/16", SubnetType::DockerBridge)];

        let result = merge_host_and_docker_subnets(host, docker);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].base.subnet_type, SubnetType::DockerBridge);
    }

    /// A bridge the host is attached to appears twice: once from the host's own
    /// interface (typed by name, no virtualization) and once from the runtime
    /// API. The host row wins on identity, but must not keep the name guess —
    /// losing the API's type and virtualization would leave the bridge
    /// unrecognised and unscoped to its owning runtime service.
    #[test]
    fn bridge_overlap_adopts_authoritative_classification() {
        let service_id = Uuid::new_v4();
        let host = vec![make_subnet("172.17.0.0/16", SubnetType::Lan)];
        let mut bridge = make_subnet("172.17.0.0/16", SubnetType::DockerBridge);
        bridge.base.virtualization =
            Some(ContainerRuntime::Docker.subnet_virtualization(service_id));

        let result = merge_host_and_docker_subnets(host, vec![bridge]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].base.subnet_type, SubnetType::DockerBridge);
        assert_eq!(
            result[0]
                .base
                .virtualization
                .as_ref()
                .and_then(|v| v.service_id()),
            Some(service_id)
        );
    }

    #[test]
    fn no_overlap_keeps_both() {
        let host = vec![make_subnet("192.168.1.0/24", SubnetType::Lan)];
        let docker = vec![make_subnet("10.0.0.0/8", SubnetType::IpVlan)];

        let result = merge_host_and_docker_subnets(host, docker);
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn podman_socket_without_path_errors_never_uses_docker_defaults() {
        // Regression: a Podman socket credential with no resolvable socket path must
        // fail cleanly rather than fall back to Docker's local socket (which would
        // discover Docker containers stamped as Podman). The None+Podman arm returns
        // before any connection attempt, so this is deterministic with no I/O.
        let utils = create_system_utils();
        let result = utils
            .new_container_socket_client(ContainerRuntime::Podman, None)
            .await;
        assert!(result.is_err(), "Podman + no socket path must error");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Podman socket"),
            "error should name the missing Podman socket, got: {msg}"
        );
    }

    #[test]
    fn mixed_filters_only_overlapping_docker() {
        let host = vec![make_subnet("192.168.1.0/24", SubnetType::Lan)];
        let docker = vec![
            make_subnet("192.168.1.0/24", SubnetType::MacVlan),
            make_subnet("172.17.0.0/16", SubnetType::DockerBridge),
            make_subnet("10.0.0.0/8", SubnetType::IpVlan),
        ];

        let result = merge_host_and_docker_subnets(host, docker);
        // MacVlan dropped (overlaps host), Bridge + IpVlan kept (no overlap)
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].base.subnet_type, SubnetType::Lan);
        assert_eq!(result[1].base.subnet_type, SubnetType::DockerBridge);
        assert_eq!(result[2].base.subnet_type, SubnetType::IpVlan);
    }

    #[test]
    fn apipa_with_bogus_prefix_corrected_to_16() {
        use pnet::ipnetwork::{IpNetwork, Ipv4Network};
        use std::net::Ipv4Addr;

        // Simulate a Windows APIPA interface reporting /0
        let bogus = IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(169, 254, 1, 100), 0).unwrap());

        let corrected = match bogus {
            IpNetwork::V4(v4)
                if v4.ip().octets()[0] == 169
                    && v4.ip().octets()[1] == 254
                    && v4.prefix() != 16 =>
            {
                IpNetwork::V4(Ipv4Network::new(v4.ip(), 16).unwrap_or(v4))
            }
            other => other,
        };

        match corrected {
            IpNetwork::V4(v4) => assert_eq!(v4.prefix(), 16),
            _ => panic!("Expected V4"),
        }
    }

    #[test]
    fn apipa_with_correct_prefix_unchanged() {
        use pnet::ipnetwork::{IpNetwork, Ipv4Network};
        use std::net::Ipv4Addr;

        let correct = IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(169, 254, 1, 100), 16).unwrap());

        let result = match correct {
            IpNetwork::V4(v4)
                if v4.ip().octets()[0] == 169
                    && v4.ip().octets()[1] == 254
                    && v4.prefix() != 16 =>
            {
                IpNetwork::V4(Ipv4Network::new(v4.ip(), 16).unwrap_or(v4))
            }
            other => other,
        };

        match result {
            IpNetwork::V4(v4) => assert_eq!(v4.prefix(), 16),
            _ => panic!("Expected V4"),
        }
    }

    #[test]
    fn non_apipa_with_bogus_prefix_not_corrected() {
        use pnet::ipnetwork::{IpNetwork, Ipv4Network};
        use std::net::Ipv4Addr;

        // 10.0.0.1/8 should NOT be corrected (not APIPA)
        let normal = IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(10, 0, 0, 1), 8).unwrap());

        let result = match normal {
            IpNetwork::V4(v4)
                if v4.ip().octets()[0] == 169
                    && v4.ip().octets()[1] == 254
                    && v4.prefix() != 16 =>
            {
                IpNetwork::V4(Ipv4Network::new(v4.ip(), 16).unwrap_or(v4))
            }
            other => other,
        };

        match result {
            IpNetwork::V4(v4) => assert_eq!(v4.prefix(), 8),
            _ => panic!("Expected V4"),
        }
    }
}
