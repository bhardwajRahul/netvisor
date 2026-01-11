use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::{DiscoveryMetadata, EntitySource};
use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
use crate::server::subnets::r#impl::types::SubnetType;
use anyhow::Error;
use anyhow::anyhow;
use async_trait::async_trait;
use bollard::query_parameters::ListNetworksOptions;
use bollard::{API_DEFAULT_VERSION, Docker};
use cidr::IpCidr;
use local_ip_address::local_ip;
use mac_address::MacAddress;
use net_route::Handle;
use pnet::ipnetwork::IpNetwork;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

pub const SCAN_TIMEOUT: Duration = Duration::from_millis(800);

/// Describes concurrent operations in the discovery pipeline that consume file descriptors.
/// Used to calculate optimal deep scan concurrency.
#[derive(Debug, Clone, Default)]
pub struct ConcurrentPipelineOps {
    /// Number of ARP datalink channels open (2 FDs each: tx + rx)
    pub arp_subnet_count: usize,
    /// Number of concurrent hosts in non-interfaced discovery port scan
    pub non_interfaced_scan_concurrency: usize,
    /// Number of discovery ports being scanned per non-interfaced host
    pub discovery_ports_count: usize,
    /// Batch size for non-interfaced port scanning
    pub port_scan_batch_size: usize,
}

impl ConcurrentPipelineOps {
    /// Calculate total FDs consumed by concurrent pipeline operations
    pub fn estimated_fd_usage(&self) -> usize {
        // ARP channels: 2 FDs per subnet (tx + rx)
        let arp_fds = self.arp_subnet_count * 2;

        // Non-interfaced discovery scan: concurrent hosts * min(batch_size, discovery_ports)
        let ports_per_host = self.port_scan_batch_size.min(self.discovery_ports_count);
        let non_interfaced_fds = self.non_interfaced_scan_concurrency * ports_per_host;

        arp_fds + non_interfaced_fds
    }
}

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
                tracing::info!(ip = %ip, "Detected local IP address");
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
        discovery_type: DiscoveryType,
        daemon_id: Uuid,
        network_id: Uuid,
        interface_filter: &[String],
    ) -> Result<
        (
            Vec<Interface>,
            Vec<Subnet>,
            HashMap<IpCidr, Option<MacAddress>>,
        ),
        Error,
    > {
        let all_interfaces = pnet::datalink::interfaces();

        // Apply interface filter if specified
        let interfaces: Vec<_> = if interface_filter.is_empty() {
            all_interfaces
        } else {
            let filtered: Vec<_> = all_interfaces
                .into_iter()
                .filter(|iface| interface_filter.iter().any(|f| f == &iface.name))
                .collect();

            if filtered.is_empty() {
                tracing::warn!(
                    filter = ?interface_filter,
                    "No interfaces matched the filter. Check --interface argument."
                );
            } else {
                tracing::debug!(
                    filter = ?interface_filter,
                    matched = filtered.len(),
                    "Filtered interfaces by --interfaces argument"
                );
            }

            filtered
        };

        tracing::debug!(
            interface_count = interfaces.len(),
            "Enumerating network interfaces"
        );

        for interface in &interfaces {
            tracing::debug!(
                name = %interface.name,
                index = interface.index,
                is_up = interface.is_up(),
                is_loopback = interface.is_loopback(),
                mac = ?interface.mac,
                ips = ?interface.ips,
                flags = interface.flags,
                "Found interface"
            );
        }

        // First pass: collect all interface data and potential subnets
        let mut potential_subnets: Vec<(String, IpNetwork)> = Vec::new();
        let mut interface_data: Vec<(String, IpAddr, Option<MacAddress>)> = Vec::new();

        for interface in interfaces.into_iter().filter(|i| !i.is_loopback()) {
            let name = interface.name.clone();
            let mac_address = match interface.mac {
                Some(mac) if !mac.octets().iter().all(|o| *o == 0) => {
                    Some(MacAddress::new(mac.octets()))
                }
                _ => None,
            };

            for ip in interface.ips.iter() {
                interface_data.push((name.clone(), ip.ip(), mac_address));
                potential_subnets.push((name.clone(), *ip));
            }
        }

        // Second pass: create unique subnets from valid networks
        let mut subnet_map: HashMap<IpCidr, Subnet> = HashMap::new();

        for (interface_name, ip_network) in potential_subnets {
            if let Some(subnet) = Subnet::from_discovery(
                interface_name,
                &ip_network,
                daemon_id,
                &discovery_type,
                network_id,
            ) {
                subnet_map.entry(subnet.base.cidr).or_insert(subnet);
            }
        }

        // Third pass: assign all interfaces to appropriate subnets
        let mut interfaces = Vec::new();
        let mut cidr_to_mac = HashMap::new();

        for (interface_name, ip_addr, mac_address) in interface_data {
            // Find which subnet this IP belongs to
            if let Some(subnet) = subnet_map.values().find(|s| s.base.cidr.contains(&ip_addr)) {
                cidr_to_mac.insert(subnet.base.cidr, mac_address);

                interfaces.push(Interface::new(InterfaceBase {
                    network_id: subnet.base.network_id,
                    host_id: Uuid::nil(), // Placeholder - server will set correct host_id
                    name: Some(interface_name),
                    subnet_id: subnet.id,
                    ip_address: ip_addr,
                    mac_address,
                    position: interfaces.len() as i32,
                }));
            }
        }

        let subnets: Vec<Subnet> = subnet_map.into_values().collect();

        Ok((interfaces, subnets, cidr_to_mac))
    }

    async fn new_local_docker_client(
        &self,
        docker_proxy: Result<Option<String>, Error>,
        docker_proxy_ssl_info: Result<Option<(String, String, String)>, Error>,
    ) -> Result<Docker, Error> {
        use tokio::time::timeout;

        const DOCKER_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

        tracing::debug!("Creating Docker client connection");
        let start = std::time::Instant::now();

        let client = if let Ok(Some(docker_proxy)) = docker_proxy {
            tracing::debug!(proxy = %docker_proxy, "Using Docker proxy");
            if docker_proxy.contains("https://")
                && let Ok(Some((key, cert, chain))) = docker_proxy_ssl_info
            {
                let key_path = PathBuf::from(key);
                let cert_path = PathBuf::from(cert);
                let chain_path = PathBuf::from(chain);

                Docker::connect_with_ssl(
                    &docker_proxy,
                    &key_path,
                    &cert_path,
                    &chain_path,
                    4,
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

        // Add timeout to Docker ping to prevent indefinite blocking
        tracing::debug!(
            "Pinging Docker daemon (timeout: {:?})",
            DOCKER_CONNECT_TIMEOUT
        );
        match timeout(DOCKER_CONNECT_TIMEOUT, client.ping()).await {
            Ok(Ok(_)) => {
                tracing::info!(
                    elapsed_ms = start.elapsed().as_millis(),
                    "Docker client connected successfully"
                );
                Ok(client)
            }
            Ok(Err(e)) => {
                tracing::warn!(
                    elapsed_ms = start.elapsed().as_millis(),
                    error = %e,
                    "Docker ping failed"
                );
                Err(anyhow::anyhow!("Docker ping failed: {}", e))
            }
            Err(_) => {
                tracing::warn!(
                    elapsed_ms = start.elapsed().as_millis(),
                    "Docker ping timed out after {:?}",
                    DOCKER_CONNECT_TIMEOUT
                );
                Err(anyhow::anyhow!(
                    "Docker connection timed out after {:?}",
                    DOCKER_CONNECT_TIMEOUT
                ))
            }
        }
    }

    async fn get_subnets_from_docker_networks(
        &self,
        daemon_id: Uuid,
        network_id: Uuid,
        client: &Docker,
        discovery_type: DiscoveryType,
    ) -> Result<Vec<Subnet>, Error> {
        let subnets: Vec<Subnet> = client
            .list_networks(None::<ListNetworksOptions>)
            .await?
            .into_iter()
            .filter_map(|n| {
                let driver = n.driver.as_deref().unwrap_or("bridge");

                // Include Docker networks that can be scanned
                // Skip: host (no separate CIDR), none (no networking), null (invalid)
                let subnet_type = match driver {
                    "bridge" | "overlay" => SubnetType::DockerBridge,
                    "macvlan" => SubnetType::MacVlan,
                    "ipvlan" => SubnetType::IpVlan,
                    _ => {
                        tracing::trace!(
                            network_name = ?n.name,
                            driver = driver,
                            "Skipping unsupported Docker network driver"
                        );
                        return None;
                    }
                };

                let network_name = n.name.clone().unwrap_or("Unknown Network".to_string());
                n.ipam.clone().map(|ipam| (network_name, ipam, subnet_type))
            })
            .filter_map(|(network_name, ipam, subnet_type)| {
                ipam.config
                    .map(|config| (network_name, config, subnet_type))
            })
            .flat_map(|(network_name, configs, subnet_type)| {
                configs
                    .iter()
                    .filter_map(|c| {
                        if let Some(cidr) = &c.subnet {
                            return Some(Subnet::new(SubnetBase {
                                cidr: IpCidr::from_str(cidr).ok()?,
                                description: None,
                                tags: Vec::new(),
                                network_id,
                                name: network_name.clone(),
                                subnet_type,
                                source: EntitySource::Discovery {
                                    metadata: vec![DiscoveryMetadata::new(
                                        discovery_type.clone(),
                                        daemon_id,
                                    )],
                                },
                            }));
                        }
                        None
                    })
                    .collect::<Vec<Subnet>>()
            })
            .collect();

        Ok(subnets)
    }

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

    /// Get optimal concurrency for ARP scanning (OS-specific due to BPF limits on macOS)
    fn get_optimal_arp_concurrency(&self) -> Result<usize, Error>;

    /// Get optimal concurrency for deep port scanning.
    ///
    /// # Arguments
    /// * `port_batch_size` - Number of ports scanned concurrently per host in deep scan
    /// * `concurrent_ops` - Description of other concurrent operations consuming FDs
    fn get_optimal_deep_scan_concurrency(
        &self,
        port_batch_size: usize,
        concurrent_ops: ConcurrentPipelineOps,
    ) -> Result<usize, Error>;

    async fn get_optimal_port_batch_size(&self) -> Result<usize, Error> {
        let fd_limit = Self::get_fd_limit()?;

        // Reserve file descriptors for:
        // - stdin, stdout, stderr (3)
        // - HTTP client connections for endpoints (50)
        // - Docker socket and other daemon operations (50)
        // - Buffer for safety (100)
        let reserved = 203;

        let available = fd_limit.saturating_sub(reserved);

        // Calculate optimal batch size
        let optimal = if available < 50 {
            // Very constrained system (like macOS default of 256)
            tracing::warn!(
                "Low file descriptor limit detected ({}). Using minimal batch size of 20. \
                Consider increasing limit for better performance.",
                fd_limit
            );
            20
        } else if available < 500 {
            // Moderate system
            available / 2 // Use half of available to be conservative
        } else {
            // High-limit system (Linux default ~8000+)
            // Cap at 1000 for reasonable performance without overwhelming target
            std::cmp::min(available, 1000)
        };

        tracing::trace!(
            "File descriptor limit: {}, reserved: {}, available: {}, port batch size: {}",
            fd_limit,
            reserved,
            available,
            optimal
        );

        Ok(optimal)
    }

    /// Get optimal number of concurrent host scans
    /// Host-prioritized: maximize concurrent hosts, then optimize port batch
    async fn get_optimal_concurrent_scans(
        &self,
        concurrency_config_value: usize,
    ) -> Result<usize, Error> {
        let fd_limit = Self::get_fd_limit()?;

        // Reserve FDs for daemon operations
        let reserved = 203;
        let available = fd_limit.saturating_sub(reserved);

        // Target concurrent host scans (prefer more hosts)
        let target_concurrent_hosts = if available < 500 {
            5 // Very constrained
        } else if available < 2000 {
            15 // Moderate
        } else if available < 5000 {
            30 // Good
        } else {
            50 // Excellent
        };

        // Calculate FD usage per host
        let endpoint_fds_per_host = 25;
        let overhead_per_host = 20;

        // Calculate what port batch size we can afford with target concurrent hosts
        let available_per_host = available / target_concurrent_hosts;
        let port_batch_per_host =
            available_per_host.saturating_sub(endpoint_fds_per_host + overhead_per_host);

        // Ensure port batch is reasonable (min 10, max 200)
        let port_batch_bounded = port_batch_per_host.clamp(10, 200);

        // Recalculate actual concurrent hosts we can support with this port batch
        let fds_per_host = port_batch_bounded + endpoint_fds_per_host + overhead_per_host;
        let actual_concurrent = available / fds_per_host;

        // Bound concurrent hosts (min 1, max 50)
        let optimal_concurrent = actual_concurrent.clamp(1, 50);

        let result = if concurrency_config_value != 15 {
            // User override - respect it
            tracing::info!(
                "Using configured concurrent_scans={} (automatic would be {}, \
                 with port_batch={})",
                concurrency_config_value,
                optimal_concurrent,
                port_batch_bounded
            );
            concurrency_config_value
        } else {
            // Use automatic
            tracing::info!(
                concurrent_scans = %optimal_concurrent,
                port_batch = %port_batch_bounded,
                fd_limit = %fd_limit,
                fd_available = %available,
                fds_per_host = %fds_per_host,
                "Using automatic concurrent_scans",
            );
            optimal_concurrent
        };

        if result < 5 {
            tracing::warn!(
                fd_limit = %fd_limit,
                "Very low concurrency. Consider increasing for better performance.",
            );
        }

        Ok(result)
    }
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

pub fn create_system_utils() -> PlatformDaemonUtils {
    PlatformDaemonUtils::new()
}
