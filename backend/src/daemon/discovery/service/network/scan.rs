use super::arp::{self, ArpScanResult};
use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::discovery::service::warnings::{CredentialIssue, CredentialIssueReason};
use crate::daemon::discovery::types::base::DiscoveryCriticalError;
use crate::daemon::utils::base::{DaemonUtils, PlatformDaemonUtils};
use crate::daemon::utils::scanner::{
    ScanConcurrencyController, can_arp_scan, scan_endpoints, scan_tcp_ports, scan_udp_ports,
};
use crate::server::credentials::r#impl::mapping::{CredentialMapping, CredentialQueryPayload};
use crate::server::discovery::r#impl::scan_settings::defaults;
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::{Service, ServiceMatchBaselineParams};
use crate::server::shared::types::entities::EntitySource;
use crate::server::{
    daemons::r#impl::base::DaemonMode,
    hosts::r#impl::{
        api::{DiscoveryHostRequest, HostResponse},
        base::{Host, HostBase},
        name::HostName,
    },
    subnets::r#impl::base::Subnet,
};
use anyhow::Error;
use cidr::IpCidr;
use futures::StreamExt;
use mac_address::MacAddress;
use pnet::datalink;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use std::{net::IpAddr, sync::Arc};
use tokio::sync::mpsc as tokio_mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::{
    DeepScanParams, DiscoveredHostData, FULL_SCAN_COST_CS, LATE_ARRIVAL_GRACE_PERIOD,
    LIGHT_SCAN_COST_CS, MAX_PROGRESS_REPORT_INTERVAL, NetworkScan, PROGRESS_ARP_PHASE,
    PROGRESS_DEEP_SCAN_PHASE, PROGRESS_GRACE_PHASE, RESPONSIVENESS_COST_CS,
    integration_cost_for_ip,
};

impl NetworkScan {
    pub async fn scan_and_process_hosts(
        &self,
        subnets: Vec<Subnet>,
        network_subnets: Vec<Subnet>,
        target_ips: Option<HashSet<IpAddr>>,
        cancel: CancellationToken,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
    ) -> Result<Vec<(IpAddr, Host, DiscoveredHostData)>, Error> {
        let session = ops.get_session().await?;

        let interface_filter = ops.config_store.get_interfaces().await?;
        let (_, _, subnet_cidr_to_mac) = utils
            .get_own_interfaces(session.info.network_id, &interface_filter)
            .await?;

        // Filter out loopback subnets — they are not scannable
        let subnets: Vec<Subnet> = subnets
            .into_iter()
            .filter(|s| !s.base.subnet_type.is_loopback())
            .collect();

        // A targeted rescan narrows enumeration to specific addresses within the
        // subnets above. `None` means sweep them entirely.
        let is_targeted = |ip: &IpAddr| target_ips.as_ref().is_none_or(|t| t.contains(ip));

        // Credentials resolve per-IP during the scan of that IP, and the enumeration above is
        // the only source of addresses — `target_ips` narrows it and never adds to it. So a
        // credential assigned to a host outside these subnets is skipped in complete silence,
        // which is exactly how a correctly configured integration can produce no traffic and no
        // error. Say so before the scan starts, while the reason is still knowable.
        let unreachable = unreachable_credential_targets(&self.credential_mappings, &subnets);
        if !unreachable.is_empty()
            && let Ok(session_state) = ops.get_session().await
            && let Ok(mut issues) = session_state.credential_issues.lock()
        {
            issues.extend(unreachable);
        }

        // Get scan settings from discovery request, falling back to defaults
        let use_npcap = self.scan_settings.use_npcap_arp;
        let arp_retries = self
            .scan_settings
            .arp_retries
            .unwrap_or(defaults::arp_retries());
        let arp_rate_pps = self
            .scan_settings
            .arp_rate_pps
            .unwrap_or(defaults::arp_rate_pps());
        let scan_rate_pps = self
            .scan_settings
            .scan_rate_pps
            .unwrap_or(defaults::scan_rate_pps());
        let port_scan_batch_size = self
            .scan_settings
            .port_scan_batch_size
            .unwrap_or(defaults::port_scan_batch_size())
            .clamp(16, 1000);

        // Check ARP capability once before partitioning
        let arp_available = can_arp_scan(use_npcap);

        // Scan scope, kept for the post-scan credential-reachability check: a credential
        // targeting an address outside these was never contacted, and saying so is the point.
        let scanned_subnets = subnets.clone();

        // Partition subnets (not IPs) into interfaced vs non-interfaced.
        // IPs are generated per-subnet at point of use to avoid allocating a
        // single Vec with every IP across all subnets (which OOMs on bogus CIDRs).
        let (interfaced_subnets, non_interfaced_subnets): (Vec<_>, Vec<_>) = if arp_available {
            subnets.into_iter().partition(|s: &Subnet| {
                subnet_cidr_to_mac
                    .get(&s.base.cidr)
                    .and_then(|m| *m)
                    .is_some()
            })
        } else {
            (Vec::new(), subnets)
        };

        let count_ips =
            |subnets: &[Subnet]| -> u64 { count_scan_ips(subnets, target_ips.as_ref()) };
        let interfaced_ip_count = count_ips(&interfaced_subnets);
        let non_interfaced_ip_count = count_ips(&non_interfaced_subnets);
        let total_ips = interfaced_ip_count + non_interfaced_ip_count;

        // Calculate estimated ARP duration for progress reporting
        let arp_target_count = interfaced_ip_count;
        let total_rounds = 1 + arp_retries as u64;
        let send_time_per_round_secs = arp_target_count / arp_rate_pps.max(1) as u64;
        let estimated_arp_duration = Duration::from_secs(
            total_rounds * (send_time_per_round_secs + arp::ROUND_WAIT.as_secs())
                + arp::POST_SCAN_RECEIVE.as_secs(),
        );
        let pipeline_start = Instant::now();

        tracing::info!(
            total_ips,
            interfaced_ips = interfaced_ip_count,
            non_interfaced_ips = non_interfaced_ip_count,
            estimated_arp_secs = estimated_arp_duration.as_secs(),
            arp_method = if cfg!(target_family = "windows") && !use_npcap {
                "SendARP"
            } else {
                "Broadcast"
            },
            "Starting continuous discovery pipeline"
        );

        ops.report_progress(0).await?;

        let arp_subnet_count = interfaced_subnets.len();

        // Use the port batch size from the coordinated calculation
        let effective_batch_size = port_scan_batch_size;

        // Calculate deep scan concurrency based on FDs available after ARP
        let mut deep_scan_concurrency =
            utils.get_optimal_deep_scan_concurrency(effective_batch_size, arp_subnet_count)?;

        // Create shared concurrency controller for graceful degradation
        let scan_controller = ScanConcurrencyController::new(effective_batch_size);

        let gateway_ips = utils.get_own_routing_table_gateway_ips().await?;

        // Create async channel for discovered hosts
        // Buffer size allows ARP to run ahead while deep scanning catches up
        let (host_tx, mut host_rx) =
            tokio_mpsc::channel::<(IpAddr, Subnet, Option<MacAddress>)>(256);

        // Start ARP scanning for interfaced subnets — build target IPs per-subnet.
        // ARP needs all targets upfront (multi-round retries), so a Vec is required.
        // Cap per-subnet based on the configurable arp_scan_cutoff prefix.
        let arp_cutoff = self
            .scan_settings
            .arp_scan_cutoff
            .unwrap_or(defaults::arp_scan_cutoff());
        let max_arp_targets: usize = 1usize << (32 - arp_cutoff as u32);

        // Work-based ARP progress signal (B): `arp_packets_sent` is incremented per ARP
        // request across every subnet/round; `arp_packets_total` is the upper bound of
        // requests this scan will send (targets × rounds). Progress/ETA derive from real
        // send throughput so a wrong rate estimate can't pin the bar or lie about the ETA.
        let arp_packets_sent = Arc::new(AtomicU64::new(0));
        let mut arp_packets_total: u64 = 0;
        let arp_total_rounds = 1 + arp_retries as u64;

        if !interfaced_subnets.is_empty() {
            let mut subnet_to_ips: HashMap<IpCidr, (Subnet, Vec<std::net::Ipv4Addr>)> =
                HashMap::new();
            for subnet in &interfaced_subnets {
                let entry = subnet_to_ips
                    .entry(subnet.base.cidr)
                    .or_insert_with(|| (subnet.clone(), Vec::new()));
                for addr in subnet.base.cidr.iter().map(|a| a.address()) {
                    if !is_targeted(&addr) {
                        continue;
                    }
                    if let IpAddr::V4(ipv4) = addr {
                        entry.1.push(ipv4);
                        if entry.1.len() >= max_arp_targets {
                            tracing::warn!(
                                cidr = %subnet.base.cidr,
                                cutoff = format!("/{}", arp_cutoff),
                                max_ips = max_arp_targets,
                                "ARP target list truncated to /{} cutoff",
                                arp_cutoff
                            );
                            break;
                        }
                    }
                }
            }

            let actual_arp_targets: usize = subnet_to_ips.values().map(|(_, ips)| ips.len()).sum();
            tracing::info!(
                subnets = subnet_to_ips.len(),
                total_ips = actual_arp_targets,
                arp_retries,
                arp_rate_pps,
                cidrs = ?subnet_to_ips.keys().map(|c| c.to_string()).collect::<Vec<_>>(),
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
                arp_packets_total += target_count as u64 * arp_total_rounds;

                match arp::scan_subnet(
                    &interface,
                    source_ipv4,
                    source_mac,
                    target_ips,
                    use_npcap,
                    arp_retries,
                    arp_rate_pps,
                    arp_packets_sent.clone(),
                ) {
                    Ok(arp_rx) => {
                        // Spawn a task to forward ARP results to the async channel
                        // Use spawn_blocking since std::sync::mpsc::recv_timeout is blocking
                        let host_tx = host_tx.clone();
                        let subnet = subnet.clone();

                        // Use a background thread for the blocking recv, forward via channel.
                        // Hard timeout prevents infinite hangs if the ARP receiver thread
                        // gets stuck (e.g., on bridge ip_addresses with continuous traffic).
                        std::thread::spawn(move || {
                            let forwarder_start = Instant::now();
                            let forwarder_timeout = Duration::from_secs(600); // 10 minutes
                            let mut forwarded = 0u64;
                            loop {
                                if forwarder_start.elapsed() >= forwarder_timeout {
                                    tracing::warn!(
                                        cidr = %cidr,
                                        forwarded,
                                        elapsed_secs = forwarder_start.elapsed().as_secs(),
                                        "ARP forwarder hit timeout, forcing exit"
                                    );
                                    break;
                                }

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

        // Send all non-interfaced IPs directly to deep scanner (no discovery phase).
        // Key insight: ARP filters to responsive hosts before expensive port scanning.
        // For non-interfaced subnets where ARP isn't possible, just deep scan all IPs
        // directly - we're going to port scan them anyway.
        if !non_interfaced_subnets.is_empty() {
            tracing::info!(
                count = non_interfaced_ip_count,
                "Queuing non-interfaced IPs for deep scan (no ARP available)"
            );

            // Stream IPs directly from CIDR iterators — zero allocation.
            // Each IP is generated on-the-fly and sent through the channel.
            let host_tx = host_tx.clone();
            let stream_targets = target_ips.clone();
            tokio::spawn(async move {
                for subnet in non_interfaced_subnets {
                    for addr in subnet.base.cidr.iter().map(|a| a.address()) {
                        if stream_targets.as_ref().is_some_and(|t| !t.contains(&addr)) {
                            continue;
                        }
                        if host_tx.send((addr, subnet.clone(), None)).await.is_err() {
                            return; // Receiver dropped
                        }
                    }
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
        let mut results: Vec<(IpAddr, Host, DiscoveredHostData)> = Vec::new();

        // Server-configurable hard ceiling for this run (default = the historical 6h).
        let max_discovery_duration = Duration::from_secs(
            self.scan_settings
                .max_discovery_duration
                .unwrap_or(defaults::max_discovery_duration()) as u64,
        );

        // Batch-level progress tracking for smoother UX
        // TCP port scanning is the bulk of deep scan work
        let is_full_scan = self.scan_settings.is_full_scan;
        let scan_port_count = if is_full_scan {
            65535_usize
        } else {
            self.light_scan_ports.len()
        };
        let batches_per_host = scan_port_count.div_ceil(effective_batch_size);
        let scan_cost_cs = if is_full_scan {
            FULL_SCAN_COST_CS
        } else {
            LIGHT_SCAN_COST_CS
        };
        let total_cost = Arc::new(AtomicUsize::new(0));
        let completed_cost = Arc::new(AtomicUsize::new(0));
        // Seed total_cost with the responsiveness-check work for every non-interfaced IP
        // (checked whether or not it responds), so progress/ETA account for draining that
        // range instead of pinning at ~95% while it's still being scanned.
        total_cost.fetch_add(
            non_interfaced_ip_count as usize * RESPONSIVENESS_COST_CS,
            Ordering::Relaxed,
        );

        // Collect hosts into a stream and process with concurrency limit
        // Use trait objects to allow spawning from different code paths
        #[allow(clippy::type_complexity)]
        let mut pending_scans: futures::stream::FuturesUnordered<
            std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Option<(IpAddr, Host, DiscoveredHostData)>>
                        + Send,
                >,
            >,
        > = futures::stream::FuturesUnordered::new();
        let mut channel_closed = false;
        let mut last_progress_report = 0u8;
        let mut last_progress_time = Instant::now();
        let mut deep_scan_started_at: Option<Instant> = None;

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
                                  total_cost_val: usize,
                                  completed_cost_val: usize,
                                  hosts_discovered_val: usize,
                                  hosts_scanned_val: usize,
                                  arp_packets_sent_val: u64,
                                  arp_packets_total_val: u64|
         -> u8 {
            if !channel_closed {
                // ARP + deep scan run concurrently, so blend both so the bar keeps moving
                // through the ARP tail instead of pinning at 30% then jumping when the
                // channel closes.
                //
                // ARP fraction (0-30%): prefer real packets-sent throughput (self-correcting
                // when the send rate differs from the estimate); fall back to the time
                // estimate before any packet is sent, or on paths that can't report sends
                // (Windows SendARP).
                let arp_frac = if arp_packets_total_val > 0 && arp_packets_sent_val > 0 {
                    (arp_packets_sent_val as f64 / arp_packets_total_val as f64).min(1.0)
                } else if estimated_arp_duration.as_secs() > 0 {
                    (pipeline_start.elapsed().as_secs_f64() / estimated_arp_duration.as_secs_f64())
                        .min(1.0)
                } else {
                    1.0
                };
                // Deep-scan fraction of the work discovered so far. Its denominator grows
                // as ARP keeps finding hosts, so a couple of early completions can spike it
                // (e.g. 2 of an eventual 34 hosts done = 100%). Weighting the deep-scan
                // contribution by `arp_frac` below discounts that until discovery is
                // actually complete, so the bar can't jump ahead then stall.
                let deep_frac = if total_cost_val > 0 {
                    (completed_cost_val as f64 / total_cost_val as f64).min(1.0)
                } else if hosts_discovered_val > 0 {
                    (hosts_scanned_val as f64 / hosts_discovered_val as f64).min(1.0)
                } else {
                    0.0
                };
                // blended = arp_frac * (30 + deep_frac*65). At arp_frac→1 this converges to
                // the post-channel-close formula (30 + deep_frac*65), a seamless handoff.
                let blended = arp_frac
                    * (PROGRESS_ARP_PHASE as f64 + deep_frac * PROGRESS_DEEP_SCAN_PHASE as f64);
                // Cap below the grace band — the channel must close before we reach 95%.
                (blended as u8).min(PROGRESS_ARP_PHASE + PROGRESS_DEEP_SCAN_PHASE - 1)
            } else if total_cost_val > 0
                && (completed_cost_val < total_cost_val || has_pending_scans)
            {
                // Deep scan phase (30-95%): Based on batch completion ratio for smooth progress
                let scan_progress = completed_cost_val as f64 / total_cost_val as f64;
                PROGRESS_ARP_PHASE + (scan_progress * PROGRESS_DEEP_SCAN_PHASE as f64) as u8
            } else if has_pending_scans && total_cost_val == 0 && hosts_discovered_val > 0 {
                // Channel closed but no batch info yet - use host-level progress
                // to avoid getting stuck at 30% when batches haven't been registered
                let host_progress =
                    (hosts_scanned_val as f64 / hosts_discovered_val as f64).min(1.0);
                PROGRESS_ARP_PHASE + (host_progress * PROGRESS_DEEP_SCAN_PHASE as f64) as u8
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

        let mut early_reported_hosts: HashMap<
            IpAddr,
            tokio::task::JoinHandle<Result<Uuid, Error>>,
        > = HashMap::new();

        loop {
            tokio::select! {
                // Try to receive new hosts from the channel
                host = host_rx.recv(), if !channel_closed => {
                    match host {
                        Some((ip, subnet, mac)) => {
                            // Only count ARP-confirmed hosts immediately.
                            // Non-interfaced hosts are counted after responsiveness
                            // check passes in deep_scan_host().
                            if mac.is_some() {
                                hosts_discovered.fetch_add(1, Ordering::Relaxed);
                            }
                            *last_activity.lock().unwrap() = Instant::now();

                            // Early-report a minimal host so the UI shows it immediately.
                            // Only for interfaced hosts (mac.is_some()) — they're confirmed
                            // responsive via ARP. Non-interfaced hosts must pass the TCP
                            // responsiveness check in deep_scan_host() first.
                            if mac.is_some()
                                && let std::collections::hash_map::Entry::Vacant(e) = early_reported_hosts.entry(ip)
                            {
                                let early_subnet = subnet.clone();
                                let early_cancel = cancel.clone();
                                let early_entity_buffer = ops.entity_buffer.clone();
                                let early_config_store = ops.config_store.clone();
                                let early_api_client = ops.api_client.clone();
                                let early_handle = tokio::spawn(async move {
                                    // Through the ladder, not by assigning `name`: a stub that
                                    // does not declare its rung enters as `Unspecified` and can
                                    // never refresh the address-derived name it wrote last scan,
                                    // so a host whose DHCP lease moved keeps showing the old one.
                                    let mut host = Host::new(HostBase {
                                        network_id: early_subnet.base.network_id,
                                        source: EntitySource::Discovery,
                                        ..Default::default()
                                    });
                                    host.base.apply_name(HostName::Ip(ip));
                                    let host_id = host.id;
                                    let ip_address = IPAddress::new(IPAddressBase {
                                        network_id: early_subnet.base.network_id,
                                        host_id: Uuid::nil(),
                                        name: None,
                                        subnet_id: early_subnet.id,
                                        ip_address: ip,
                                        mac_address: mac,
                                        position: 0,
                                    });
                                    let request = DiscoveryHostRequest {
                                        host,
                                        ip_addresses: vec![ip_address],
                                        ports: vec![],
                                        services: vec![],
                                        interfaces: vec![],
                                        subnets: vec![],
                                        // Early host stub carries no ifTable; nothing to prune against.
                                        interfaces_complete: true,
                                        // ...and no neighbour data either, so there is nothing
                                        // for it to overwrite.
                                        interface_data_complete: InterfaceDataComplete::default(),
                                    };
                                    early_entity_buffer.push_host(request.clone()).await;
                                    let mode = early_config_store.get_mode().await?;
                                    match mode {
                                        DaemonMode::DaemonPoll => {
                                            let _response: HostResponse = early_api_client
                                                .post("/api/v1/hosts/discovery", &request, "Failed to create early host")
                                                .await?;
                                            Ok(host_id)
                                        }
                                        DaemonMode::ServerPoll => {
                                            let _actual = early_entity_buffer
                                                .await_host(&host_id, Duration::from_secs(120), &early_cancel)
                                                .await
                                                .ok_or_else(|| anyhow::anyhow!("Timeout waiting for early host creation"))?;
                                            Ok(host_id)
                                        }
                                    }
                                });
                                e.insert(early_handle);
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }

                            // Spawn deep scan if under concurrency limit, otherwise buffer
                            if pending_scans.len() < deep_scan_concurrency {
                                let cancel = cancel.clone();
                                let gateway_ips = gateway_ips.clone();
                                let hosts_scanned = hosts_scanned.clone();
                                let last_activity = last_activity.clone();
                                let completed_cost = completed_cost.clone();
                                let total_cost = total_cost.clone();
                                let hosts_discovered = hosts_discovered.clone();
                                let scan_controller = scan_controller.clone();

                                // Only count batches for hosts with MAC (known responsive from ARP).
                                // Non-interfaced hosts will have batches counted AFTER responsiveness check.
                                if mac.is_some() {
                                    let integration_cost = self.compute_integration_cost_for_ip(ip);
                                    total_cost.fetch_add(scan_cost_cs + integration_cost, Ordering::Relaxed);
                                }
                                let probe_raw_socket_ports = self.scan_settings.probe_raw_socket_ports;
                                let light_scan_ports = self.light_scan_ports.clone();
                                let network_subnets_ref = network_subnets.clone();
                                let early_host_handle = early_reported_hosts.remove(&ip);
                                pending_scans.push(Box::pin(async move {
                                    let early_host_id = match early_host_handle {
                                        Some(handle) => match handle.await {
                                            Ok(Ok(id)) => id,
                                            _ => Uuid::new_v4(),
                                        },
                                        None => Uuid::new_v4(),
                                    };

                                    let result = self
                                        .deep_scan_host(DeepScanParams {
                                            ip,
                                            subnet: &subnet,
                                            mac,
                                            cancel,
                                            scan_rate_pps,
                                            port_scan_batch_size: effective_batch_size,
                                            gateway_ips: &gateway_ips,
                                            completed_cost: Some(&completed_cost),
                                            total_cost: Some(&total_cost),
                                            hosts_discovered: Some(&hosts_discovered),
                                            batches_per_host,
                                            scan_cost_cs,
                                            scan_controller,
                                            probe_raw_socket_ports,
                                            early_host_id,
                                            is_full_scan,
                                            light_scan_ports: &light_scan_ports,
                                            credential_mappings: &self.credential_mappings,
                                            known_subnets: network_subnets_ref,
                                        }, ops, utils)
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
                                // Only count batches for hosts with MAC (known responsive from ARP).
                                // Non-interfaced hosts will have batches counted AFTER responsiveness check.
                                if mac.is_some() {
                                    let integration_cost = self.compute_integration_cost_for_ip(ip);
                                    total_cost.fetch_add(scan_cost_cs + integration_cost, Ordering::Relaxed);
                                }
                                pending_hosts.push((ip, subnet, mac));
                            }
                        }
                        None => {
                            channel_closed = true;

                            tracing::info!(
                                hosts_discovered = hosts_discovered.load(Ordering::Relaxed),
                                pending_scans = pending_scans.len(),
                                pending_hosts = pending_hosts.len(),
                                total_cost = total_cost.load(Ordering::Relaxed),
                                completed_cost = completed_cost.load(Ordering::Relaxed),
                                elapsed_secs = pipeline_start.elapsed().as_secs(),
                                "Host discovery channel closed, transitioning to deep scan phase"
                            );

                            // ARP complete - recalculate concurrency without ARP FD reservation
                            // Those FDs (2 per subnet) are now available for deep scanning
                            if let Ok(new_concurrency) = utils.get_optimal_deep_scan_concurrency(
                                effective_batch_size,
                                0, // No more ARP channels open
                            ) && new_concurrency > deep_scan_concurrency {
                                tracing::info!(
                                    old = deep_scan_concurrency,
                                    new = new_concurrency,
                                    "Increasing deep scan concurrency"
                                );
                                deep_scan_concurrency = new_concurrency;
                            }
                        }
                    }
                }

                // Collect completed deep scans and spawn buffered hosts
                Some(result) = pending_scans.next(), if !pending_scans.is_empty() => {
                    if let Some(host) = result {
                        results.push(host);
                    }

                    // Spawn next buffered host if available
                    // Note: batches only counted for MAC hosts when buffered; non-MAC hosts
                    // have batches counted in deep_scan_host after responsiveness check
                    if let Some((ip, subnet, mac)) = pending_hosts.pop() {
                        let cancel = cancel.clone();
                        let gateway_ips = gateway_ips.clone();
                        let hosts_scanned = hosts_scanned.clone();
                        let last_activity = last_activity.clone();
                        let completed_cost = completed_cost.clone();
                        let total_cost = total_cost.clone();
                        let hosts_discovered = hosts_discovered.clone();
                        let scan_controller = scan_controller.clone();
                        let probe_raw_socket_ports = self.scan_settings.probe_raw_socket_ports;
                        let light_scan_ports = self.light_scan_ports.clone();
                        let network_subnets_ref = network_subnets.clone();
                        let early_host_handle = early_reported_hosts.remove(&ip);

                        pending_scans.push(Box::pin(async move {
                            let early_host_id = match early_host_handle {
                                Some(handle) => match handle.await {
                                    Ok(Ok(id)) => id,
                                    _ => Uuid::new_v4(),
                                },
                                None => Uuid::new_v4(),
                            };

                            let result = self
                                .deep_scan_host(DeepScanParams {
                                    ip,
                                    subnet: &subnet,
                                    mac,
                                    cancel,
                                    scan_rate_pps,
                                    port_scan_batch_size: effective_batch_size,
                                    gateway_ips: &gateway_ips,
                                    completed_cost: Some(&completed_cost),
                                    total_cost: Some(&total_cost),
                                    hosts_discovered: Some(&hosts_discovered),
                                    batches_per_host,
                                    scan_cost_cs,
                                    scan_controller,
                                    probe_raw_socket_ports,
                                    early_host_id,
                                    is_full_scan,
                                    light_scan_ports: &light_scan_ports,
                                    credential_mappings: &self.credential_mappings,
                                    known_subnets: network_subnets_ref,
                                }, ops, utils)
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
                    let total_cost_val = total_cost.load(Ordering::Relaxed);
                    let completed_cost_val = completed_cost.load(Ordering::Relaxed);
                    let hosts_discovered_val = hosts_discovered.load(Ordering::Relaxed);
                    let hosts_scanned_val = hosts_scanned.load(Ordering::Relaxed);
                    let arp_packets_sent_val = arp_packets_sent.load(Ordering::Relaxed);

                    // Calculate progress. Clamp to be monotonic: the blended ARP+deep-scan
                    // value can dip when ARP discovers more hosts (growing the deep-scan
                    // denominator), and a progress bar must never go backwards.
                    let progress = calculate_progress(
                        channel_closed,
                        has_pending,
                        grace_elapsed,
                        total_cost_val,
                        completed_cost_val,
                        hosts_discovered_val,
                        hosts_scanned_val,
                        arp_packets_sent_val,
                        arp_packets_total,
                    )
                    .max(last_progress_report);

                    // Remaining ARP send/retry work — dominates a sparse subnet's tail
                    // (rounds 2-3 re-probe every dead IP). Derived from real send rate so
                    // the ETA stops claiming "<1 min" while ARP still has minutes to go.
                    let arp_remaining_secs = if !channel_closed
                        && arp_packets_total > 0
                        && arp_packets_sent_val > 0
                    {
                        let arp_elapsed = pipeline_start.elapsed().as_secs_f64();
                        let rate = arp_packets_sent_val as f64 / arp_elapsed.max(0.001);
                        let remaining = arp_packets_total.saturating_sub(arp_packets_sent_val);
                        (remaining as f64 / rate.max(0.001)) as u32
                    } else {
                        0
                    };

                    // Update estimation atomics on the session
                    if let Ok(session) = ops.get_session().await {
                        session.hosts_discovered.store(hosts_discovered_val as u32, Ordering::Relaxed);

                        if channel_closed && hosts_scanned_val > 0 {
                            let started = deep_scan_started_at.get_or_insert(Instant::now());
                            let deep_scan_elapsed = started.elapsed();
                            // Host-based estimate: real per-host completion time for the
                            // responsive hosts (TCP + endpoints + SNMP + host creation).
                            let time_per_host = deep_scan_elapsed.as_secs_f64() / hosts_scanned_val as f64;
                            let remaining_hosts = hosts_discovered_val.saturating_sub(hosts_scanned_val);
                            let host_based = (remaining_hosts as f64 * time_per_host) as u32;
                            // Cost-based estimate also captures pending non-interfaced
                            // responsiveness work — dead IPs never increment
                            // hosts_discovered, so host-based alone collapses to "<1 min"
                            // while a large non-interfaced range is still draining. Take
                            // the larger so the ETA reflects whichever work dominates.
                            let cost_based = if completed_cost_val > 0 {
                                let time_per_cost_unit =
                                    deep_scan_elapsed.as_secs_f64() / completed_cost_val as f64;
                                let remaining_cost = total_cost_val.saturating_sub(completed_cost_val);
                                (remaining_cost as f64 * time_per_cost_unit) as u32
                            } else {
                                0
                            };
                            let remaining_secs = host_based.max(cost_based)
                                + LATE_ARRIVAL_GRACE_PERIOD.as_secs() as u32;
                            session.estimated_remaining_secs.store(remaining_secs, Ordering::Relaxed);
                        } else {
                            // ARP phase still active. The ETA must cover BOTH the remaining
                            // ARP send/retry work and any concurrent deep-scan cost, taking
                            // whichever dominates — otherwise the near-done deep-scan cost
                            // alone reports "<1 min" while the ARP tail still runs.
                            let deep_remaining = if completed_cost_val > 0 {
                                let started = deep_scan_started_at.get_or_insert(Instant::now());
                                let deep_scan_elapsed = started.elapsed();
                                let time_per_cost_unit = deep_scan_elapsed.as_secs_f64() / completed_cost_val as f64;
                                let remaining_cost = total_cost_val.saturating_sub(completed_cost_val);
                                (remaining_cost as f64 * time_per_cost_unit * 1.2) as u32
                            } else {
                                0
                            };
                            let remaining_secs = deep_remaining.max(arp_remaining_secs)
                                + LATE_ARRIVAL_GRACE_PERIOD.as_secs() as u32;
                            session.estimated_remaining_secs.store(remaining_secs, Ordering::Relaxed);
                        }
                    }

                    // Report progress if it changed OR if enough time has passed (heartbeat)
                    let time_since_last_report = last_progress_time.elapsed();
                    if progress != last_progress_report || time_since_last_report >= MAX_PROGRESS_REPORT_INTERVAL {
                        last_progress_report = progress;
                        last_progress_time = Instant::now();
                        let _ = ops.report_progress(progress.min(99)).await;
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
                tracing::info!("Discovery cancelled by user");
                return Err(Error::msg("Discovery session was cancelled"));
            }

            // Global timeout safety net
            if pipeline_start.elapsed() >= max_discovery_duration {
                let discovered = hosts_discovered.load(Ordering::Relaxed);
                let scanned = hosts_scanned.load(Ordering::Relaxed);
                // Hosts that were discovered but never deep-scanned (queued work + the
                // discovered/scanned gap), so the user sees what was left behind.
                let not_scanned = pending_scans
                    .len()
                    .saturating_add(pending_hosts.len())
                    .max(discovered.saturating_sub(scanned));
                tracing::error!(
                    elapsed_secs = pipeline_start.elapsed().as_secs(),
                    hosts_discovered = discovered,
                    hosts_scanned = scanned,
                    pending_scans = pending_scans.len(),
                    pending_hosts = pending_hosts.len(),
                    channel_closed,
                    "Discovery hit global timeout, forcing completion"
                );
                if not_scanned > 0 {
                    // Reuse the estimate already computed this tick to say how much
                    // work was left, without any new estimation plumbing.
                    let remaining = ops
                        .get_session()
                        .await
                        .ok()
                        .map(|s| s.estimated_remaining_secs.load(Ordering::Relaxed))
                        .filter(|&s| s != u32::MAX && s > 0);
                    let msg = match remaining {
                        Some(secs) => format!(
                            "Scan hit its time limit ({}h) — {} host(s) not scanned (~{} min of estimated work remaining). Raise Max Discovery Duration or rescan.",
                            max_discovery_duration.as_secs() / 3600,
                            not_scanned,
                            secs.div_ceil(60),
                        ),
                        None => format!(
                            "Scan hit its time limit ({}h) — {} host(s) not scanned. Raise Max Discovery Duration or rescan.",
                            max_discovery_duration.as_secs() / 3600,
                            not_scanned,
                        ),
                    };
                    if let Ok(session) = ops.get_session().await
                        && let Ok(mut warnings) = session.warnings.lock()
                    {
                        warnings.push(msg);
                    }
                }
                break;
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

        ops.report_progress(100).await?;

        // A credential pinned to an in-scope address that nothing answered at is skipped just as
        // silently as one pinned outside the scanned subnets — the deep scan only ever runs for
        // hosts that responded. Only knowable once the run is over, hence here rather than
        // alongside the pre-scan check.
        let answered: HashSet<IpAddr> = results.iter().map(|(ip, _, _)| *ip).collect();
        let unanswered = unanswered_credential_targets(
            &self.credential_mappings,
            &scanned_subnets,
            target_ips.as_ref(),
            &answered,
        );
        if !unanswered.is_empty()
            && let Ok(session_state) = ops.get_session().await
            && let Ok(mut issues) = session_state.credential_issues.lock()
        {
            issues.extend(unanswered);
        }

        let discovered = hosts_discovered.load(Ordering::Relaxed);
        tracing::info!(
            hosts_discovered = discovered,
            hosts_scanned = hosts_scanned.load(Ordering::Relaxed),
            results = results.len(),
            elapsed_secs = pipeline_start.elapsed().as_secs(),
            "Discovery pipeline complete"
        );

        Ok(results)
    }

    async fn deep_scan_host(
        &self,
        params: DeepScanParams<'_>,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
    ) -> Result<Option<(IpAddr, Host, DiscoveredHostData)>, Error> {
        let DeepScanParams {
            ip,
            subnet,
            mac,
            cancel,
            scan_rate_pps,
            port_scan_batch_size,
            gateway_ips,
            completed_cost,
            total_cost,
            hosts_discovered,
            batches_per_host,
            scan_cost_cs,
            scan_controller,
            probe_raw_socket_ports,
            early_host_id,
            is_full_scan,
            light_scan_ports,
            credential_mappings,
            known_subnets,
        } = params;

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        // Use fixed batch size, limited by scan controller if FD exhaustion has occurred
        let effective_batch_size = port_scan_batch_size.min(scan_controller.batch_size());

        // For non-interfaced hosts (no MAC from ARP), check responsiveness first.
        // This avoids full 65k port scans on hosts that aren't online.
        let mut responsiveness_ports: HashSet<u16> = HashSet::new();
        if mac.is_none() {
            let discovery_ports: Vec<u16> = Service::all_discovery_ports()
                .iter()
                .filter(|p| p.is_tcp())
                .map(|p| p.number())
                .collect();

            tracing::debug!(
                ip = %ip,
                ports = discovery_ports.len(),
                "Checking responsiveness (non-interfaced host)"
            );

            let responsive_ports = scan_tcp_ports(
                ip,
                cancel.clone(),
                effective_batch_size,
                scan_rate_pps,
                discovery_ports,
                scan_controller.clone(),
            )
            .await?;

            // The responsiveness check itself is accounted work (seeded into total_cost
            // up front for every non-interfaced IP); mark it complete now, on both the
            // responsive and unresponsive paths.
            if let Some(counter) = completed_cost {
                counter.fetch_add(RESPONSIVENESS_COST_CS, Ordering::Relaxed);
            }

            if responsive_ports.is_empty() {
                tracing::debug!(ip = %ip, "Host unresponsive, skipping deep scan");
                return Ok(None);
            }

            // Host is responsive - NOW we count it in hosts_discovered and total_cost
            // This ensures only responsive hosts contribute to progress calculation
            if let Some(discovered) = hosts_discovered {
                discovered.fetch_add(1, Ordering::Relaxed);
            }
            if let Some(total) = total_cost {
                // Integration cost, counted once per distinct integration for this IP
                // (see integration_cost_for_ip) so it matches the completed-cost accrual.
                let integration_cost_cs = integration_cost_for_ip(credential_mappings, ip);
                total.fetch_add(scan_cost_cs + integration_cost_cs, Ordering::Relaxed);
            }

            tracing::debug!(
                ip = %ip,
                open_ports = responsive_ports.len(),
                "Host responsive, proceeding with deep scan"
            );

            // Track discovered ports so we don't re-scan them
            responsiveness_ports.extend(responsive_ports.iter().map(|(p, _)| p.number()));
        }

        let remaining_tcp_ports: Vec<u16> = if is_full_scan {
            (1..=65535)
                .filter(|p| !responsiveness_ports.contains(p))
                .collect()
        } else {
            // Light scan: only discovery ports + credential custom ports
            light_scan_ports
                .iter()
                .copied()
                .filter(|p| !responsiveness_ports.contains(p))
                .collect()
        };

        tracing::debug!(
            ip = %ip,
            is_full_scan,
            responsiveness_ports = responsiveness_ports.len(),
            remaining_ports = remaining_tcp_ports.len(),
            effective_batch_size,
            "Starting deep scan"
        );

        // Scan in batches with rate limiting and graceful degradation
        let mut all_tcp_ports = Vec::new();
        for chunk in remaining_tcp_ports.chunks(effective_batch_size) {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            let open_ports = scan_tcp_ports(
                ip,
                cancel.clone(),
                effective_batch_size,
                scan_rate_pps,
                chunk.to_vec(),
                scan_controller.clone(),
            )
            .await?;
            all_tcp_ports.extend(open_ports);

            // Update cost-based progress: each batch contributes a fraction of scan_cost_cs
            if let Some(counter) = completed_cost {
                let cost_per_batch = if batches_per_host > 0 {
                    scan_cost_cs / batches_per_host
                } else {
                    0
                };
                counter.fetch_add(cost_per_batch, Ordering::Relaxed);
            }
        }

        let use_https_ports: HashMap<u16, bool> = all_tcp_ports
            .iter()
            .map(|(p, h)| (p.number(), *h))
            .collect();
        let mut open_ports: Vec<PortType> = all_tcp_ports.iter().map(|(p, _)| *p).collect();

        // Merge responsiveness check discovered ports (for non-interfaced hosts)
        for port_num in responsiveness_ports {
            let port = PortType::new_tcp(port_num);
            if !open_ports.contains(&port) {
                open_ports.push(port);
            }
        }
        open_ports.sort_by_key(|p| (p.number(), p.protocol()));
        open_ports.dedup();

        // Non-credentialed UDP scanning (DNS, NTP, DHCP, BACnet).
        // SNMP probing is now handled by SnmpIntegration.probe() below.
        let udp_ports = scan_udp_ports(
            ip,
            cancel.clone(),
            effective_batch_size,
            scan_rate_pps,
            subnet.base.cidr,
            gateway_ips.to_vec(),
            &[], // No SNMP credentials — SNMP probing handled by integration
        )
        .await?;
        open_ports.extend(udp_ports);

        // Read once here rather than at the endpoint scan below: integration probes make their
        // own TLS calls and need the same policy.
        let accept_invalid_certs = ops.config_store.get_accept_invalid_scan_certs().await?;

        // Integration probes — each checks connectivity and returns a ClientProbe for service matching
        use crate::daemon::discovery::integration::dispatch;
        let probe_results = dispatch::probe_integrations(
            ip,
            credential_mappings,
            &open_ports,
            false, // network scan: keep the probe-gate (cheap broad scan)
            &cancel,
            utils,
            accept_invalid_certs,
        )
        .await?;
        open_ports.extend(probe_results.additional_ports.iter());
        // Hand any IP-targeted credential that produced nothing to the session, which renders
        // them as one line at the end. `probe_integrations` has no session handle of its own.
        ops.record_credential_issues(&probe_results.credential_issues)
            .await;
        // Mark this host's integration cost as completed once its probes resolve. Uses
        // the SAME per-distinct-integration cost that total_cost accrued for the host,
        // so completed_cost converges to total_cost (the scan ETA/progress stay accurate
        // even when several SNMP credentials cover the host).
        if let Some(counter) = completed_cost {
            counter.fetch_add(
                integration_cost_for_ip(credential_mappings, ip),
                Ordering::Relaxed,
            );
        }
        let client_responses = &probe_results.client_responses;

        // Endpoint scanning
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
            effective_batch_size,
            probe_raw_socket_ports,
            accept_invalid_certs,
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

        // DNS hostname lookup (SNMP sysName fallback now handled by SnmpIntegration.execute())
        let hostname = self.get_hostname_for_ip(ip).await?;
        // MAC enrichment from SNMP ipAddrTable now handled by SnmpIntegration.execute()
        let ip_address = IPAddress::new(IPAddressBase {
            network_id: subnet.base.network_id,
            host_id: Uuid::nil(), // Placeholder - server will set correct host_id
            name: None,
            subnet_id: subnet.id,
            ip_address: ip,
            mac_address: mac,
            position: 0,
        });

        // Filter raw socket ports from service matching when probe_raw_socket_ports is off,
        // matching the same filtering applied in scan_endpoints() for endpoint probing.
        if !probe_raw_socket_ports {
            open_ports.retain(|p| !p.is_raw_socket());
        }

        if let Ok(Some(mut host_data)) = ops
            .build_host_from_scan(
                ServiceMatchBaselineParams {
                    subnet,
                    ip_address: &ip_address,
                    all_ports: &open_ports,
                    endpoint_responses: &endpoint_responses,
                    virtualization_metadata: &None,
                    virtualization_service_id: None,
                    client_responses,
                    // Directly scanned, not reported by a controller.
                    managed_device: &None,
                },
                hostname,
                self.host_naming_fallback,
            )
            .await
        {
            // Reuse the early-reported host ID so the server updates the existing record
            host_data.host.id = early_host_id;

            // Execute integrations whose probe succeeded and service matched
            let execute_params = dispatch::ExecuteParams {
                ip,
                cancel: &cancel,
                ops,
                utils,
                open_ports: &open_ports,
                endpoint_responses: &endpoint_responses,
                host_id: early_host_id,
                host_naming_fallback: self.host_naming_fallback,
                known_subnets: &known_subnets,
                scanning_subnet: Some(subnet),
                ip_address_id: Some(ip_address.id),
            };
            dispatch::execute_integrations(
                credential_mappings,
                &probe_results,
                &mut host_data,
                &execute_params,
            )
            .await
            .ok();

            // Extract final state from host_data
            let interfaces_complete = host_data.interfaces_complete;
            let interface_data_complete = host_data.interface_data_complete;
            let host = host_data.host;
            let ip_addresses = host_data.ip_addresses;
            let ports = host_data.ports;
            let services = host_data.services;
            let interfaces = host_data.interfaces;
            let subnets = host_data.subnets;

            let services_count = services.len();
            let if_entries_count = interfaces.len();
            let docker_services = services
                .iter()
                .filter(|s| s.base.virtualization_metadata.is_some())
                .count();
            if docker_services > 0 {
                tracing::info!(
                    ip = %ip,
                    total_services = services_count,
                    docker_container_services = docker_services,
                    ip_addresses = ip_addresses.len(),
                    "Creating host with container services from integration"
                );
            }

            match ops
                .create_host(
                    host,
                    ip_addresses,
                    ports,
                    services,
                    interfaces,
                    subnets,
                    interfaces_complete,
                    interface_data_complete,
                    &cancel,
                )
                .await
            {
                Ok(host_response) => {
                    tracing::info!(
                        ip = %ip,
                        services = services_count,
                        interfaces = if_entries_count,
                        "Host created"
                    );
                    let host_data = DiscoveredHostData {
                        docker_service_id: host_response
                            .services
                            .iter()
                            .find(|s| s.base.service_definition.id() == "Docker")
                            .map(|s| s.id),
                        ip_addresses: host_response.ip_addresses.clone(),
                    };
                    return Ok(Some((ip, host_response.to_host(), host_data)));
                }
                Err(e) => {
                    // Include the server error so create rejections are diagnosable
                    // from the daemon log alone (create_host does not retry on
                    // ApiErrorResponse, so the reason otherwise lives only server-side).
                    tracing::warn!(ip = %ip, error = %e, "Host creation failed");
                }
            }
        } else {
            tracing::debug!(ip = %ip, "Host processing returned None");
        }

        Ok(None)
    }
}

/// Number of addresses a scan will actually probe across `subnets`.
///
/// Without a target filter this is the full address space of each CIDR, derived
/// from the prefix so nothing is materialized. With one — a rescan, which
/// substitutes its /32 for the real subnet containing it — the prefix says
/// nothing about the work: counting it would seed the progress budget with a
/// whole subnet's worth of scanning for a single address, pinning the bar near
/// zero and reporting an ETA in minutes for a job that takes seconds.
pub(crate) fn count_scan_ips(subnets: &[Subnet], target_ips: Option<&HashSet<IpAddr>>) -> u64 {
    match target_ips {
        Some(targets) => subnets
            .iter()
            .map(|s| targets.iter().filter(|ip| s.base.cidr.contains(ip)).count() as u64)
            .sum(),
        None => subnets
            .iter()
            .map(|s| 1u64 << (32 - s.base.cidr.network_length() as u64))
            .sum(),
    }
}

/// Credentials pinned to an address that this scan will never visit.
///
/// Only `ip_overrides` are considered: a default credential is network-wide and has no target
/// to be unreachable. Loopback overrides are excluded because they are how a daemon-host
/// credential (a Docker or Podman socket) addresses the daemon's own machine — that address is
/// deliberately absent from the scannable subnets and reporting it would be a false alarm on
/// every scan.
///
/// Deliberately independent of `target_ips`. A targeted rescan narrows to one host on purpose,
/// and flagging every other host's credentials as "not tried" would make the warning noise on
/// the most common operation there is.
pub(crate) fn unreachable_credential_targets(
    mappings: &[CredentialMapping<CredentialQueryPayload>],
    subnets: &[Subnet],
) -> Vec<CredentialIssue> {
    mappings
        .iter()
        .flat_map(|m| m.ip_overrides.iter())
        .filter(|o| !o.ip.is_loopback())
        .filter(|o| !subnets.iter().any(|s| s.base.cidr.contains(&o.ip)))
        .map(|o| CredentialIssue {
            label: o.credential.discovery_label(),
            ip: o.ip,
            reason: CredentialIssueReason::TargetNotScanned,
        })
        .collect()
}

/// Credentials pinned to an in-scope address that no host answered at.
///
/// The complement of [`unreachable_credential_targets`]: that one catches an address the scan
/// never looked at, this one an address it looked at and found nobody home. Both end with the
/// credential untried and nothing said about it, but they have different fixes, so they stay
/// separate reasons.
///
/// `answered` is the set of addresses that produced a scanned host. Addresses narrowed out by a
/// targeted rescan are excluded for the same reason as in the pre-scan check: skipping them is
/// the point of a rescan, not a fault.
pub(crate) fn unanswered_credential_targets(
    mappings: &[CredentialMapping<CredentialQueryPayload>],
    subnets: &[Subnet],
    target_ips: Option<&HashSet<IpAddr>>,
    answered: &HashSet<IpAddr>,
) -> Vec<CredentialIssue> {
    mappings
        .iter()
        .flat_map(|m| m.ip_overrides.iter())
        .filter(|o| !o.ip.is_loopback())
        .filter(|o| subnets.iter().any(|s| s.base.cidr.contains(&o.ip)))
        .filter(|o| target_ips.is_none_or(|t| t.contains(&o.ip)))
        .filter(|o| !answered.contains(&o.ip))
        .map(|o| CredentialIssue {
            label: o.credential.discovery_label(),
            ip: o.ip,
            reason: CredentialIssueReason::TargetNotResponding,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::storage::traits::Storable;
    use crate::server::subnets::r#impl::base::SubnetBase;
    use crate::server::subnets::r#impl::types::SubnetType;
    use std::str::FromStr;

    fn subnet(cidr: &str) -> Subnet {
        Subnet::new(SubnetBase {
            cidr: cidr::IpCidr::from_str(cidr).unwrap(),
            network_id: uuid::Uuid::new_v4(),
            name: cidr.to_string(),
            description: None,
            subnet_type: SubnetType::Lan,
            virtualization_service_id: None,
            source: crate::server::shared::types::entities::EntitySource::System,
            tags: Vec::new(),
        })
    }

    fn ip(s: &str) -> IpAddr {
        IpAddr::from_str(s).unwrap()
    }

    #[test]
    fn unfiltered_count_is_the_whole_address_space() {
        let subnets = [subnet("10.0.5.0/24"), subnet("10.0.6.0/30")];
        assert_eq!(count_scan_ips(&subnets, None), 256 + 4);
    }

    #[test]
    fn a_rescan_costs_one_address_not_the_subnet_it_sits_in() {
        // The substituted subnet is the real /24, so an unfiltered count would
        // budget 256 addresses of work for a single-host rescan.
        let subnets = [subnet("10.0.5.0/24")];
        let targets = HashSet::from([ip("10.0.5.7")]);
        assert_eq!(count_scan_ips(&subnets, Some(&targets)), 1);
    }

    #[test]
    fn targets_outside_the_scanned_subnets_are_not_counted() {
        let subnets = [subnet("10.0.5.0/24")];
        let targets = HashSet::from([ip("10.0.5.7"), ip("192.168.1.9")]);
        assert_eq!(count_scan_ips(&subnets, Some(&targets)), 1);
    }

    #[test]
    fn multiple_targets_in_one_subnet_each_count() {
        // A host with several addresses on the same NIC mints one /32 per
        // address; all substitute to the same parent subnet.
        let subnets = [subnet("10.0.5.0/24")];
        let targets = HashSet::from([ip("10.0.5.7"), ip("10.0.5.8")]);
        assert_eq!(count_scan_ips(&subnets, Some(&targets)), 2);
    }

    fn mapping_targeting(addr: &str) -> CredentialMapping<CredentialQueryPayload> {
        use crate::server::credentials::r#impl::mapping::IpOverride;
        CredentialMapping {
            default_credential: None,
            ip_overrides: vec![IpOverride {
                ip: ip(addr),
                credential: CredentialQueryPayload::default(), // Snmp
                credential_id: Uuid::new_v4(),
            }],
        }
    }

    /// The reported failure mode: a credential assigned to a controller on a network the
    /// discovery does not cover is never tried, and said nothing about it.
    #[test]
    fn a_target_outside_every_scanned_subnet_is_reported() {
        let subnets = [subnet("10.0.5.0/24")];
        let mappings = [mapping_targeting("192.168.1.9")];

        let issues = unreachable_credential_targets(&mappings, &subnets);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].ip, ip("192.168.1.9"));
        assert_eq!(issues[0].reason, CredentialIssueReason::TargetNotScanned);
    }

    #[test]
    fn a_target_inside_a_scanned_subnet_is_not_reported() {
        let subnets = [subnet("10.0.5.0/24")];
        let mappings = [mapping_targeting("10.0.5.7")];
        assert!(unreachable_credential_targets(&mappings, &subnets).is_empty());
    }

    /// A Docker/Podman socket credential addresses the daemon's own machine over loopback,
    /// which is deliberately not a scannable subnet. Reporting it would fire on every scan.
    #[test]
    fn a_loopback_target_is_never_reported() {
        let subnets = [subnet("10.0.5.0/24")];
        let mappings = [mapping_targeting("127.0.0.1")];
        assert!(unreachable_credential_targets(&mappings, &subnets).is_empty());
    }

    /// The case a real scan exposed: the address sits inside a scanned subnet, so the pre-scan
    /// check passes it, but nothing answers there and the deep scan never runs — leaving the
    /// credential untried and, before this, entirely silent.
    #[test]
    fn an_in_scope_address_nobody_answered_at_is_reported() {
        let subnets = [subnet("192.168.4.0/22")];
        let mappings = [mapping_targeting("192.168.4.141")];
        let answered = HashSet::from([ip("192.168.4.196")]);

        assert!(
            unreachable_credential_targets(&mappings, &subnets).is_empty(),
            "the address is inside a scanned subnet, so it is not 'not scanned'"
        );

        let issues = unanswered_credential_targets(&mappings, &subnets, None, &answered);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].ip, ip("192.168.4.141"));
        assert_eq!(issues[0].reason, CredentialIssueReason::TargetNotResponding);
    }

    #[test]
    fn an_address_that_answered_is_not_reported() {
        let subnets = [subnet("192.168.4.0/22")];
        let mappings = [mapping_targeting("192.168.4.196")];
        let answered = HashSet::from([ip("192.168.4.196")]);
        assert!(unanswered_credential_targets(&mappings, &subnets, None, &answered).is_empty());
    }

    /// A rescan narrows to specific addresses on purpose. Reporting every other pinned
    /// credential as unanswered would make the warning noise on the most common operation.
    #[test]
    fn a_rescan_does_not_report_the_hosts_it_deliberately_skipped() {
        let subnets = [subnet("192.168.4.0/22")];
        let mappings = [mapping_targeting("192.168.4.141")];
        let targets = HashSet::from([ip("192.168.4.196")]);
        let answered = HashSet::from([ip("192.168.4.196")]);
        assert!(
            unanswered_credential_targets(&mappings, &subnets, Some(&targets), &answered)
                .is_empty()
        );
    }

    /// A network default has no target, so it can never be unreachable — only pinned
    /// `ip_overrides` can.
    #[test]
    fn a_network_default_is_not_reported() {
        let subnets = [subnet("10.0.5.0/24")];
        let mappings = [CredentialMapping {
            default_credential: Some(CredentialQueryPayload::default()),
            ip_overrides: Vec::new(),
        }];
        assert!(unreachable_credential_targets(&mappings, &subnets).is_empty());
    }
}
