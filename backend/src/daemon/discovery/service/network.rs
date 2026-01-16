use crate::daemon::discovery::service::base::{
    CreatesDiscoveredEntities, DiscoversNetworkedEntities, DiscoveryRunner, RunsDiscovery,
};
use crate::daemon::discovery::types::base::{DiscoveryCriticalError, DiscoverySessionUpdate};
use crate::daemon::utils::arp::{self, ArpScanResult};
use crate::daemon::utils::base::ConcurrentPipelineOps;
use crate::daemon::utils::scanner::{can_arp_scan, scan_endpoints, scan_tcp_ports, scan_udp_ports};
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback};
use crate::server::interfaces::r#impl::base::{Interface, InterfaceBase};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::{Service, ServiceMatchBaselineParams};
use crate::server::subnets::r#impl::types::SubnetTypeDiscriminants;
use crate::{
    daemon::utils::base::DaemonUtils,
    server::{
        daemons::r#impl::api::DaemonDiscoveryRequest, hosts::r#impl::base::Host,
        subnets::r#impl::base::Subnet,
    },
};
use anyhow::Error;
use async_trait::async_trait;
use cidr::IpCidr;
use futures::{
    future::try_join_all,
    stream::{self, StreamExt},
};
use mac_address::MacAddress;
use pnet::datalink;
use std::collections::{HashMap, HashSet};
use std::result::Result::Ok;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{net::IpAddr, sync::Arc};
use strum::IntoDiscriminant;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Grace period to wait for late ARP arrivals after the last deep scan completes
const LATE_ARRIVAL_GRACE_PERIOD: Duration = Duration::from_secs(30);

/// Maximum interval between progress reports (heartbeat even if progress unchanged)
const MAX_PROGRESS_REPORT_INTERVAL: Duration = Duration::from_secs(30);

// Progress phase weights (must sum to 100)
const PROGRESS_ARP_PHASE: u8 = 30; // 0-30%: ARP discovery
const PROGRESS_DEEP_SCAN_PHASE: u8 = 65; // 30-95%: Deep scanning
const PROGRESS_GRACE_PHASE: u8 = 5; // 95-100%: Grace period

#[derive(Default)]
pub struct NetworkScanDiscovery {
    subnet_ids: Option<Vec<Uuid>>,
    host_naming_fallback: HostNamingFallback,
}

impl NetworkScanDiscovery {
    pub fn new(subnet_ids: Option<Vec<Uuid>>, host_naming_fallback: HostNamingFallback) -> Self {
        Self {
            subnet_ids,
            host_naming_fallback,
        }
    }
}

pub struct DeepScanParams<'a> {
    ip: IpAddr,
    subnet: &'a Subnet,
    mac: Option<MacAddress>,
    phase1_ports: Vec<PortType>,
    cancel: CancellationToken,
    port_scan_batch_size: usize,
    gateway_ips: &'a [IpAddr],
    /// Optional counter for batch-level progress tracking
    batches_completed: Option<&'a Arc<AtomicUsize>>,
}

impl CreatesDiscoveredEntities for DiscoveryRunner<NetworkScanDiscovery> {}

#[async_trait]
impl RunsDiscovery for DiscoveryRunner<NetworkScanDiscovery> {
    fn discovery_type(&self) -> DiscoveryType {
        DiscoveryType::Network {
            subnet_ids: self.domain.subnet_ids.clone(),
            host_naming_fallback: self.domain.host_naming_fallback,
        }
    }

    async fn discover(
        &self,
        request: DaemonDiscoveryRequest,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        // Ignore docker bridge subnets, they are discovered through Docker Discovery
        let subnets: Vec<Subnet> = self.discover_create_subnets().await?;

        self.start_discovery(request).await?;

        let discovery_result = self
            .scan_and_process_hosts(subnets, cancel.clone())
            .await
            .map(|_| ());

        self.finish_discovery(discovery_result, cancel.clone())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl DiscoversNetworkedEntities for DiscoveryRunner<NetworkScanDiscovery> {
    async fn get_gateway_ips(&self) -> Result<Vec<IpAddr>, Error> {
        self.as_ref()
            .utils
            .get_own_routing_table_gateway_ips()
            .await
    }

    async fn discover_create_subnets(&self) -> Result<Vec<Subnet>, Error> {
        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        // Target specific subnets if provided in discovery type
        let subnets = if let Some(subnet_ids) = &self.domain.subnet_ids {
            let all_subnets = self.get_subnets().await?;
            all_subnets
                .into_iter()
                .filter(|s| subnet_ids.contains(&s.id))
                .collect()

        // Target all interfaced subnets if not
        } else {
            let interface_filter = self.as_ref().config_store.get_interfaces().await?;
            let (_, subnets, _) = self
                .as_ref()
                .utils
                .get_own_interfaces(
                    self.discovery_type(),
                    daemon_id,
                    network_id,
                    &interface_filter,
                )
                .await?;

            // Filter out docker bridge subnets, those are handled in docker discovery
            // Filter out subnets with
            let subnets: Vec<Subnet> = subnets
                .into_iter()
                .filter(|s| {

                    if s.base.cidr.network_length() < 10 {
                        tracing::warn!("Skipping {} with CIDR {}, scanning would take too long", s.base.name, s.base.cidr);
                        return false
                    }

                    if s.base.subnet_type.discriminant() == SubnetTypeDiscriminants::DockerBridge {
                        tracing::warn!("Skipping {} with CIDR {}, docker bridge subnets are scanned in docker discovery", s.base.name, s.base.cidr);
                        return false
                    }

                    true
                })
                .collect();
            let subnet_futures = subnets.iter().map(|subnet| self.create_subnet(subnet));
            try_join_all(subnet_futures).await?
        };

        Ok(subnets)
    }
}

impl DiscoveryRunner<NetworkScanDiscovery> {
    async fn scan_and_process_hosts(
        &self,
        subnets: Vec<Subnet>,
        cancel: CancellationToken,
    ) -> Result<Vec<Host>, Error> {
        let session = self.as_ref().get_session().await?;

        let interface_filter = self.as_ref().config_store.get_interfaces().await?;
        let (_, _, subnet_cidr_to_mac) = self
            .as_ref()
            .utils
            .get_own_interfaces(
                self.discovery_type(),
                session.info.daemon_id,
                session.info.network_id,
                &interface_filter,
            )
            .await?;

        let all_ips_with_subnets: Vec<(IpAddr, Subnet)> = subnets
            .iter()
            .flat_map(|subnet| {
                self.determine_scan_order(&subnet.base.cidr)
                    .map(move |ip| (ip, subnet.clone()))
            })
            .collect();

        let total_ips = all_ips_with_subnets.len();

        // Pre-compute values used in streams
        let port_scan_batch_size = self.as_ref().utils.get_optimal_port_batch_size().await?;
        let discovery_ports: Vec<u16> = Service::all_discovery_ports()
            .iter()
            .filter(|p| p.is_tcp())
            .map(|p| p.number())
            .collect();

        // Get ARP config
        let use_npcap = self.as_ref().config_store.get_use_npcap_arp().await?;
        let arp_retries = self.as_ref().config_store.get_arp_retries().await?;
        let arp_rate_pps = self.as_ref().config_store.get_arp_rate_pps().await?;

        // Check ARP capability once before partitioning
        let arp_available = can_arp_scan(use_npcap);

        // Partition IPs - only use ARP path if we have capability
        let (interfaced_ips, non_interfaced_ips): (Vec<_>, Vec<_>) = if arp_available {
            all_ips_with_subnets.into_iter().partition(|(_, subnet)| {
                subnet_cidr_to_mac
                    .get(&subnet.base.cidr)
                    .and_then(|m| *m)
                    .is_some()
            })
        } else {
            // No ARP capability - treat all as non-interfaced (port scan only)
            (Vec::new(), all_ips_with_subnets)
        };

        // Calculate estimated ARP duration for progress reporting
        let arp_target_count = interfaced_ips.len() as u64;
        let total_rounds = 1 + arp_retries as u64;
        let send_time_per_round_secs = arp_target_count / arp_rate_pps.max(1) as u64;
        let estimated_arp_duration = Duration::from_secs(
            total_rounds * (send_time_per_round_secs + arp::ROUND_WAIT.as_secs())
                + arp::POST_SCAN_RECEIVE.as_secs(),
        );
        let pipeline_start = Instant::now();

        tracing::info!(
            total_ips = total_ips,
            interfaced_ips = interfaced_ips.len(),
            non_interfaced_ips = non_interfaced_ips.len(),
            estimated_arp_secs = estimated_arp_duration.as_secs(),
            arp_method = if cfg!(target_family = "windows") && !use_npcap {
                "SendARP"
            } else {
                "Broadcast"
            },
            "Starting continuous discovery pipeline"
        );

        self.report_discovery_update(DiscoverySessionUpdate::scanning(0))
            .await?;

        // Count unique subnets that will have ARP channels open
        let arp_subnet_count: usize = {
            let unique_cidrs: std::collections::HashSet<_> = interfaced_ips
                .iter()
                .map(|(_, subnet)| subnet.base.cidr)
                .collect();
            unique_cidrs.len()
        };

        // Pre-compute non-interfaced port concurrency if needed
        let non_interfaced_scan_concurrency = if !non_interfaced_ips.is_empty() {
            let configured = self.as_ref().config_store.get_concurrent_scans().await?;
            self.as_ref()
                .utils
                .get_optimal_concurrent_scans(configured)
                .await?
        } else {
            0
        };

        // Get deep scan parameters with precise FD budget
        let ports_per_host_batch = 200;
        let concurrent_ops = ConcurrentPipelineOps {
            arp_subnet_count,
            non_interfaced_scan_concurrency,
            discovery_ports_count: discovery_ports.len(),
            port_scan_batch_size,
        };
        let deep_scan_concurrency = self
            .as_ref()
            .utils
            .get_optimal_deep_scan_concurrency(ports_per_host_batch, concurrent_ops)?;

        let gateway_ips = self
            .as_ref()
            .utils
            .get_own_routing_table_gateway_ips()
            .await?;

        // Create async channel for discovered hosts
        // Buffer size allows ARP to run ahead while deep scanning catches up
        let (host_tx, mut host_rx) =
            tokio_mpsc::channel::<(IpAddr, Subnet, Option<MacAddress>)>(256);

        // Track active ARP forwarders
        let arp_forwarders_active = Arc::new(AtomicUsize::new(0));

        // Start ARP scanning for interfaced subnets
        if !interfaced_ips.is_empty() {
            // Group IPs by subnet for batch scanning
            let mut subnet_to_ips: HashMap<IpCidr, (Subnet, Vec<std::net::Ipv4Addr>)> =
                HashMap::new();
            for (ip, subnet) in &interfaced_ips {
                if let IpAddr::V4(ipv4) = ip {
                    subnet_to_ips
                        .entry(subnet.base.cidr)
                        .or_insert_with(|| (subnet.clone(), Vec::new()))
                        .1
                        .push(*ipv4);
                }
            }

            tracing::info!(
                subnets = subnet_to_ips.len(),
                total_ips = interfaced_ips.len(),
                arp_retries,
                arp_rate_pps,
                "Starting ARP discovery"
            );

            // Start ARP scan for each subnet and forward results to async channel
            for (cidr, (subnet, target_ips)) in subnet_to_ips {
                if cancel.is_cancelled() {
                    return Err(Error::msg("Discovery session was cancelled"));
                }

                let subnet_mac = subnet_cidr_to_mac.get(&cidr).and_then(|m| *m);

                let Some(source_mac) = subnet_mac else {
                    tracing::warn!(cidr = %cidr, "No MAC address found for subnet, skipping ARP scan");
                    continue;
                };

                // Find the network interface for this subnet
                // Match by both MAC and having an IP in the target subnet to handle
                // bridge setups where physical and bridge interfaces share the same MAC
                let pnet_source_mac = pnet::util::MacAddr::from(source_mac.bytes());
                let interface = datalink::interfaces().into_iter().find(|iface| {
                    iface.mac.unwrap_or_default() == pnet_source_mac
                        && iface.ips.iter().any(|ip| cidr.contains(&ip.ip()))
                });

                let Some(interface) = interface else {
                    tracing::warn!(mac = %source_mac, "No interface found for MAC, skipping ARP scan");
                    continue;
                };

                // Get an IPv4 address from this interface (prefer one on the target subnet)
                let source_ipv4 = interface
                    .ips
                    .iter()
                    .filter_map(|ip_net| match ip_net.ip() {
                        IpAddr::V4(ip) => Some(ip),
                        IpAddr::V6(_) => None,
                    })
                    .find(|ip| cidr.contains(&IpAddr::V4(*ip)))
                    .or_else(|| {
                        interface.ips.iter().find_map(|ip_net| match ip_net.ip() {
                            IpAddr::V4(ip) => Some(ip),
                            IpAddr::V6(_) => None,
                        })
                    });

                let Some(source_ipv4) = source_ipv4 else {
                    tracing::warn!(
                        interface = %interface.name,
                        cidr = %cidr,
                        "No IPv4 address found on interface, skipping ARP scan"
                    );
                    continue;
                };

                let target_count = target_ips.len();
                tracing::debug!(
                    cidr = %cidr,
                    interface = %interface.name,
                    source_ip = %source_ipv4,
                    source_mac = %source_mac,
                    targets = target_count,
                    arp_rate_pps,
                    "Starting ARP scan"
                );

                match arp::scan_subnet(
                    &interface,
                    source_ipv4,
                    source_mac,
                    target_ips,
                    use_npcap,
                    arp_retries,
                    arp_rate_pps,
                ) {
                    Ok(arp_rx) => {
                        // Spawn a task to forward ARP results to the async channel
                        // Use spawn_blocking since std::sync::mpsc::recv_timeout is blocking
                        let host_tx = host_tx.clone();
                        let subnet = subnet.clone();
                        let forwarders = arp_forwarders_active.clone();
                        forwarders.fetch_add(1, Ordering::SeqCst);

                        // Use a background thread for the blocking recv, forward via channel
                        std::thread::spawn(move || {
                            let mut forwarded = 0u64;
                            loop {
                                match arp_rx.recv_timeout(Duration::from_millis(100)) {
                                    Ok(ArpScanResult { ip, mac }) => {
                                        // Use blocking_send since we're in a std thread
                                        if host_tx
                                            .blocking_send((
                                                IpAddr::V4(ip),
                                                subnet.clone(),
                                                Some(mac),
                                            ))
                                            .is_err()
                                        {
                                            // Receiver dropped, stop forwarding
                                            break;
                                        }
                                        forwarded += 1;
                                    }
                                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                                }
                            }
                            tracing::debug!(
                                cidr = %cidr,
                                forwarded,
                                "ARP forwarder completed"
                            );
                            forwarders.fetch_sub(1, Ordering::SeqCst);
                        });
                    }
                    Err(e) => {
                        if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                            tracing::error!(cidr = %cidr, error = %e, "Critical error starting ARP scan");
                        } else {
                            tracing::warn!(cidr = %cidr, error = %e, "ARP scan failed to start");
                        }
                    }
                }
            }
        }

        // Process non-interfaced subnets with port scanning (send to same channel)
        if !non_interfaced_ips.is_empty() {
            // Use pre-computed concurrency (calculated earlier for FD budget)
            let port_concurrency = non_interfaced_scan_concurrency;

            tracing::info!(
                count = non_interfaced_ips.len(),
                concurrency = port_concurrency,
                "Port scanning non-interfaced subnets in parallel to ARP"
            );

            let host_tx = host_tx.clone();
            let discovery_ports = discovery_ports.clone();
            let cancel = cancel.clone();

            // Spawn port scanning as a parallel task
            tokio::spawn(async move {
                let results: Vec<_> = stream::iter(non_interfaced_ips)
                    .map(|(ip, subnet)| {
                        let cancel = cancel.clone();
                        let discovery_ports = discovery_ports.clone();

                        async move {
                            let result = scan_tcp_ports(
                                ip,
                                cancel,
                                port_scan_batch_size,
                                discovery_ports,
                            )
                            .await;

                            match result {
                                Ok(open_ports) if !open_ports.is_empty() => {
                                    tracing::debug!(ip = %ip, ports = open_ports.len(), "Host responsive (TCP)");
                                    Some((ip, subnet))
                                }
                                _ => None,
                            }
                        }
                    })
                    .buffer_unordered(port_concurrency)
                    .filter_map(|x| async { x })
                    .collect()
                    .await;

                for (ip, subnet) in results {
                    let _ = host_tx.send((ip, subnet, None)).await;
                }
            });
        }

        // Drop our copy of the sender so the channel closes when all forwarders are done
        drop(host_tx);

        // =============================================================
        // CONTINUOUS PIPELINE: Deep scan hosts as they arrive
        // =============================================================
        tracing::info!(
            deep_scan_concurrency,
            grace_period_secs = LATE_ARRIVAL_GRACE_PERIOD.as_secs(),
            "Deep scanning hosts as they are discovered"
        );

        let hosts_discovered = Arc::new(AtomicUsize::new(0));
        let hosts_scanned = Arc::new(AtomicUsize::new(0));
        let last_activity = Arc::new(std::sync::Mutex::new(Instant::now()));
        let mut results: Vec<Host> = Vec::new();

        // Batch-level progress tracking for smoother UX
        // TCP port scanning is the bulk of deep scan work (~328 batches per host for 65535 ports)
        let batches_per_host = 65535_usize.div_ceil(ports_per_host_batch);
        let total_batches = Arc::new(AtomicUsize::new(0));
        let batches_completed = Arc::new(AtomicUsize::new(0));

        // Collect hosts into a stream and process with concurrency limit
        // Use trait objects to allow spawning from different code paths
        let mut pending_scans: futures::stream::FuturesUnordered<
            std::pin::Pin<Box<dyn std::future::Future<Output = Option<Host>> + Send>>,
        > = futures::stream::FuturesUnordered::new();
        let mut channel_closed = false;
        let mut last_progress_report = 0u8;
        let mut last_progress_time = Instant::now();

        // Buffer for hosts waiting to be scanned when at concurrency limit
        let mut pending_hosts: Vec<(IpAddr, Subnet, Option<MacAddress>)> = Vec::new();

        // Use interval instead of sleep - interval persists across select iterations
        // whereas sleep creates a new future each time and gets dropped when other branches fire
        let mut progress_ticker = tokio::time::interval(Duration::from_secs(1));

        // Helper to calculate phase-weighted progress
        // Note: counters passed by value to avoid borrowing issues in closure
        let calculate_progress = |channel_closed: bool,
                                  has_pending_scans: bool,
                                  grace_elapsed: Duration,
                                  total_batches_val: usize,
                                  batches_completed_val: usize|
         -> u8 {
            if !channel_closed {
                // ARP phase (0-30%): Based on elapsed time vs estimated duration
                let arp_elapsed = pipeline_start.elapsed();
                let arp_progress = if estimated_arp_duration.as_secs() > 0 {
                    (arp_elapsed.as_secs_f64() / estimated_arp_duration.as_secs_f64()).min(1.0)
                } else {
                    1.0
                };
                (arp_progress * PROGRESS_ARP_PHASE as f64) as u8
            } else if total_batches_val > 0
                && (batches_completed_val < total_batches_val || has_pending_scans)
            {
                // Deep scan phase (30-95%): Based on batch completion ratio for smooth progress
                let scan_progress = batches_completed_val as f64 / total_batches_val as f64;
                PROGRESS_ARP_PHASE + (scan_progress * PROGRESS_DEEP_SCAN_PHASE as f64) as u8
            } else if has_pending_scans {
                // Deep scan with no batch info yet - show minimal progress
                PROGRESS_ARP_PHASE
            } else {
                // Grace period phase (95-100%): Based on grace period elapsed
                let grace_progress = (grace_elapsed.as_secs_f64()
                    / LATE_ARRIVAL_GRACE_PERIOD.as_secs_f64())
                .min(1.0);
                PROGRESS_ARP_PHASE
                    + PROGRESS_DEEP_SCAN_PHASE
                    + (grace_progress * PROGRESS_GRACE_PHASE as f64) as u8
            }
        };

        loop {
            tokio::select! {
                // Try to receive new hosts from the channel
                host = host_rx.recv(), if !channel_closed => {
                    match host {
                        Some((ip, subnet, mac)) => {
                            hosts_discovered.fetch_add(1, Ordering::Relaxed);
                            *last_activity.lock().unwrap() = Instant::now();

                            // Spawn deep scan if under concurrency limit, otherwise buffer
                            if pending_scans.len() < deep_scan_concurrency {
                                let cancel = cancel.clone();
                                let gateway_ips = gateway_ips.clone();
                                let hosts_scanned = hosts_scanned.clone();
                                let last_activity = last_activity.clone();
                                let batches_completed = batches_completed.clone();

                                total_batches.fetch_add(batches_per_host, Ordering::Relaxed);
                                pending_scans.push(Box::pin(async move {
                                    let result = self
                                        .deep_scan_host(DeepScanParams {
                                            ip,
                                            subnet: &subnet,
                                            mac,
                                            phase1_ports: Vec::new(),
                                            cancel,
                                            port_scan_batch_size: ports_per_host_batch,
                                            gateway_ips: &gateway_ips,
                                            batches_completed: Some(&batches_completed),
                                        })
                                        .await;

                                    hosts_scanned.fetch_add(1, Ordering::Relaxed);
                                    *last_activity.lock().unwrap() = Instant::now();

                                    match result {
                                        Ok(Some(host)) => Some(host),
                                        Ok(None) => None,
                                        Err(e) => {
                                            if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                                                tracing::error!(ip = %ip, error = %e, "Critical error in deep scan");
                                            } else {
                                                tracing::warn!(ip = %ip, error = %e, "Deep scan failed");
                                            }
                                            None
                                        }
                                    }
                                }));
                            } else {
                                // Count batches upfront so progress doesn't regress when pulled from buffer
                                total_batches.fetch_add(batches_per_host, Ordering::Relaxed);
                                pending_hosts.push((ip, subnet, mac));
                            }
                        }
                        None => {
                            channel_closed = true;
                            tracing::debug!("Host discovery channel closed");
                        }
                    }
                }

                // Collect completed deep scans and spawn buffered hosts
                Some(result) = pending_scans.next(), if !pending_scans.is_empty() => {
                    if let Some(host) = result {
                        results.push(host);
                    }

                    // Spawn next buffered host if available (batches already counted when buffered)
                    if let Some((ip, subnet, mac)) = pending_hosts.pop() {
                        let cancel = cancel.clone();
                        let gateway_ips = gateway_ips.clone();
                        let hosts_scanned = hosts_scanned.clone();
                        let last_activity = last_activity.clone();
                        let batches_completed = batches_completed.clone();

                        pending_scans.push(Box::pin(async move {
                            let result = self
                                .deep_scan_host(DeepScanParams {
                                    ip,
                                    subnet: &subnet,
                                    mac,
                                    phase1_ports: Vec::new(),
                                    cancel,
                                    port_scan_batch_size: ports_per_host_batch,
                                    gateway_ips: &gateway_ips,
                                    batches_completed: Some(&batches_completed),
                                })
                                .await;

                            hosts_scanned.fetch_add(1, Ordering::Relaxed);
                            *last_activity.lock().unwrap() = Instant::now();

                            match result {
                                Ok(Some(host)) => Some(host),
                                Ok(None) => None,
                                Err(e) => {
                                    if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                                        tracing::error!(ip = %ip, error = %e, "Critical error in deep scan");
                                    } else {
                                        tracing::warn!(ip = %ip, error = %e, "Deep scan failed");
                                    }
                                    None
                                }
                            }
                        }));
                    }
                }

                // Periodic progress update and grace period check
                _ = progress_ticker.tick() => {
                    let has_pending = !pending_scans.is_empty() || !pending_hosts.is_empty();
                    let grace_elapsed = last_activity.lock().unwrap().elapsed();
                    let pipeline_elapsed = pipeline_start.elapsed();
                    let total_batches_val = total_batches.load(Ordering::Relaxed);
                    let batches_completed_val = batches_completed.load(Ordering::Relaxed);

                    // Calculate and report progress (only if changed)
                    let progress = calculate_progress(
                        channel_closed,
                        has_pending,
                        grace_elapsed,
                        total_batches_val,
                        batches_completed_val,
                    );

                    let phase = if !channel_closed {
                        "arp"
                    } else if total_batches_val > 0 && (batches_completed_val < total_batches_val || has_pending) {
                        "deep_scan"
                    } else {
                        "grace"
                    };

                    tracing::trace!(
                        phase,
                        progress,
                        channel_closed,
                        total_batches = total_batches_val,
                        batches_completed = batches_completed_val,
                        has_pending,
                        pipeline_elapsed_secs = pipeline_elapsed.as_secs(),
                        estimated_arp_secs = estimated_arp_duration.as_secs(),
                        grace_elapsed_secs = grace_elapsed.as_secs(),
                        "Progress calculation"
                    );

                    // Report progress if it changed OR if enough time has passed (heartbeat)
                    let time_since_last_report = last_progress_time.elapsed();
                    if progress != last_progress_report || time_since_last_report >= MAX_PROGRESS_REPORT_INTERVAL {
                        last_progress_report = progress;
                        last_progress_time = Instant::now();
                        let _ = self.report_scanning_progress(progress.min(99)).await;
                    }

                    // Check grace period expiry
                    if channel_closed && !has_pending && grace_elapsed >= LATE_ARRIVAL_GRACE_PERIOD {
                            tracing::debug!(
                                elapsed_secs = grace_elapsed.as_secs(),
                                "Grace period expired, ending discovery"
                            );
                            break;
                    }
                }
            }

            // Check for cancellation
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery session was cancelled"));
            }

            // Exit when channel closed, no pending scans/hosts, and grace period expired
            if channel_closed && pending_scans.is_empty() && pending_hosts.is_empty() {
                let elapsed = last_activity.lock().unwrap().elapsed();

                if elapsed >= LATE_ARRIVAL_GRACE_PERIOD {
                    break;
                }

                // Log status while waiting
                let discovered = hosts_discovered.load(Ordering::Relaxed);
                if discovered > 0 {
                    tracing::debug!(
                        discovered,
                        scanned = hosts_scanned.load(Ordering::Relaxed),
                        results = results.len(),
                        grace_remaining_secs = (LATE_ARRIVAL_GRACE_PERIOD - elapsed).as_secs(),
                        "Waiting for late arrivals"
                    );
                }
            }
        }

        self.report_discovery_update(DiscoverySessionUpdate::scanning(100))
            .await?;

        let discovered = hosts_discovered.load(Ordering::Relaxed);
        tracing::info!(
            hosts_discovered = discovered,
            hosts_scanned = hosts_scanned.load(Ordering::Relaxed),
            results = results.len(),
            "Discovery pipeline complete"
        );

        Ok(results)
    }

    async fn deep_scan_host(&self, params: DeepScanParams<'_>) -> Result<Option<Host>, Error> {
        let DeepScanParams {
            ip,
            subnet,
            mac,
            phase1_ports,
            cancel,
            port_scan_batch_size,
            gateway_ips,
            batches_completed,
        } = params;

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        let phase1_port_nums: HashSet<u16> = phase1_ports.iter().map(|p| p.number()).collect();
        let remaining_tcp_ports: Vec<u16> = (1..=65535)
            .filter(|p| !phase1_port_nums.contains(p))
            .collect();

        tracing::debug!(
            ip = %ip,
            phase1_ports = phase1_ports.len(),
            remaining_ports = remaining_tcp_ports.len(),
            "Starting deep scan"
        );

        // Scan in batches
        let mut all_tcp_ports = Vec::new();
        for chunk in remaining_tcp_ports.chunks(port_scan_batch_size) {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            let open_ports =
                scan_tcp_ports(ip, cancel.clone(), port_scan_batch_size, chunk.to_vec()).await?;
            all_tcp_ports.extend(open_ports);

            // Update batch-level progress
            if let Some(counter) = batches_completed {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        let use_https_ports: HashMap<u16, bool> = all_tcp_ports
            .iter()
            .map(|(p, h)| (p.number(), *h))
            .collect();
        let mut open_ports: Vec<PortType> = all_tcp_ports.iter().map(|(p, _)| *p).collect();

        // Merge phase 1 discovered ports
        open_ports.extend(phase1_ports);
        open_ports.sort_by_key(|p| (p.number(), p.protocol()));
        open_ports.dedup();

        // UDP and endpoint scanning
        let udp_ports = scan_udp_ports(
            ip,
            cancel.clone(),
            port_scan_batch_size,
            subnet.base.cidr,
            gateway_ips.to_vec(),
        )
        .await?;
        open_ports.extend(udp_ports);

        let mut ports_to_check = open_ports.clone();
        let endpoint_only_ports = Service::endpoint_only_ports();
        ports_to_check.extend(endpoint_only_ports);
        ports_to_check.sort_by_key(|p| (p.number(), p.protocol()));
        ports_to_check.dedup();

        let endpoint_responses = scan_endpoints(
            ip,
            cancel.clone(),
            Some(ports_to_check),
            Some(use_https_ports),
            port_scan_batch_size,
        )
        .await?;

        for endpoint_response in &endpoint_responses {
            let port = endpoint_response.endpoint.port_type;
            if !open_ports.contains(&port) {
                open_ports.push(port);
            }
        }

        open_ports.sort_by_key(|p| (p.number(), p.protocol()));
        open_ports.dedup();

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        tracing::info!(
            ip = %ip,
            open_ports = open_ports.len(),
            endpoints = endpoint_responses.len(),
            "Deep scan complete"
        );

        let hostname = self.get_hostname_for_ip(ip).await?;

        let interface = Interface::new(InterfaceBase {
            network_id: subnet.base.network_id,
            host_id: Uuid::nil(), // Placeholder - server will set correct host_id
            name: None,
            subnet_id: subnet.id,
            ip_address: ip,
            mac_address: mac,
            position: 0,
        });

        if let Ok(Some((host, interfaces, ports, services))) = self
            .process_host(
                ServiceMatchBaselineParams {
                    subnet,
                    interface: &interface,
                    all_ports: &open_ports,
                    endpoint_responses: &endpoint_responses,
                    virtualization: &None,
                },
                hostname,
                self.domain.host_naming_fallback,
            )
            .await
        {
            let services_count = services.len();

            if let Ok(host_response) = self.create_host(host, interfaces, ports, services).await {
                tracing::info!(
                    ip = %ip,
                    services = services_count,
                    "Host created"
                );
                return Ok(Some(host_response.to_host()));
            } else {
                tracing::warn!(ip = %ip, "Host creation failed");
            }
        } else {
            tracing::debug!(ip = %ip, "Host processing returned None");
        }

        Ok(None)
    }

    async fn get_hostname_for_ip(&self, ip: IpAddr) -> Result<Option<String>, Error> {
        match timeout(Duration::from_millis(800), async {
            tokio::task::spawn_blocking(move || dns_lookup::lookup_addr(&ip)).await?
        })
        .await
        {
            Ok(Ok(hostname)) => Ok(Some(hostname)),
            _ => Ok(None),
        }
    }

    /// Figure out what order to scan IPs in given allocation patterns
    fn determine_scan_order(&self, subnet: &IpCidr) -> impl Iterator<Item = IpAddr> {
        let mut ips: Vec<IpAddr> = subnet.iter().map(|ip| ip.address()).collect();

        // Sort by likelihood of being active hosts - highest probability first
        ips.sort_by_key(|ip| {
            let last_octet = match ip {
                IpAddr::V4(ipv4) => ipv4.octets()[3],
                IpAddr::V6(_) => return 9999, // IPv6 gets lowest priority for now
            };

            match last_octet {
                // Tier 1: Almost guaranteed to be active infrastructure
                1 => 1,   // Default gateway (.1)
                254 => 2, // Alternative gateway (.254)

                // Tier 2: Very common infrastructure and static assignments
                2 => 10,   // Secondary router/switch
                3 => 11,   // Tertiary infrastructure
                10 => 12,  // Common DHCP start
                100 => 13, // Common DHCP end
                253 => 14, // Alt gateway range
                252 => 15, // Alt gateway range

                // Tier 3: Common static device ranges
                4..=9 => 20 + last_octet as u16, // Infrastructure devices
                11..=20 => 30 + last_octet as u16, // Servers, printers
                21..=30 => 50 + last_octet as u16, // Network devices

                // Tier 4: Active DHCP ranges (most devices live here)
                31..=50 => 100 + last_octet as u16, // Early DHCP range
                51..=100 => 200 + last_octet as u16, // Mid DHCP range
                101..=150 => 400 + last_octet as u16, // Late DHCP range

                // Tier 5: Less common but still viable
                151..=200 => 600 + last_octet as u16, // Extended DHCP
                201..=251 => 800 + last_octet as u16, // High static range

                // Skip entirely - reserved addresses
                0 | 255 => 9998, // Network/broadcast addresses
            }
        });

        ips.into_iter()
    }

    async fn get_subnets(&self) -> Result<Vec<Subnet>, Error> {
        self.as_ref()
            .api_client
            .get("/api/v1/subnets", "Failed to get subnets")
            .await
    }
}
