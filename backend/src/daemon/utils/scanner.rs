use crate::daemon::discovery::types::base::DiscoveryCriticalError;
use crate::server::services::r#impl::base::Service;
use crate::server::services::r#impl::endpoints::{Endpoint, EndpointResponse};
use anyhow::anyhow;
use anyhow::{Error, Result};
use cidr::IpCidr;
use dhcproto::Encodable;
use dhcproto::v4::{self, Decodable, Encoder, Message, MessageType};
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use hickory_resolver::Resolver;
use hickory_resolver::config::{NameServerConfig, ResolverConfig};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::xfer::Protocol;
use rand::{Rng, SeedableRng};
use rsntp::AsyncSntpClient;
use snmp2::{AsyncSession, Oid};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::{net::TcpStream, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::server::ports::r#impl::base::{PortType, TransportProtocol};

pub const SCAN_TIMEOUT: Duration = Duration::from_millis(800);
pub const PORT_SCAN_BATCH_MAX: usize = 400;
pub const PORT_SCAN_BATCH_HIGH: usize = 200;
pub const PORT_SCAN_BATCH_MED: usize = 128;
pub const PORT_SCAN_BATCH_LOW: usize = 64;

/// Generic batch scanner that maintains constant parallelism with rate limiting
/// This is the core RustScan pattern extracted into a reusable function
///
/// # Arguments
/// * `items` - Items to scan
/// * `batch_size` - Number of concurrent operations to maintain
/// * `scan_rate_pps` - Maximum probes per second (0 = unlimited)
/// * `cancel` - Cancellation token
/// * `scan_fn` - Async function that scans an item and returns Option<Result>
///
/// # Returns
/// Vector of successfully scanned results
async fn batch_scan<T, O, F, Fut>(
    items: Vec<T>,
    batch_size: usize,
    scan_rate_pps: u32,
    cancel: CancellationToken,
    scan_fn: F,
) -> Vec<O>
where
    T: Send + 'static,
    O: Send + 'static,
    F: Fn(T) -> Fut,
    Fut: std::future::Future<Output = Option<O>> + Send + 'static,
{
    let mut results = Vec::new();
    let mut item_iter = items.into_iter();

    // Calculate stagger delay from rate limit
    let stagger_delay = if scan_rate_pps > 0 {
        Duration::from_micros(1_000_000 / scan_rate_pps as u64)
    } else {
        Duration::ZERO
    };

    let mut futures: FuturesUnordered<Pin<Box<dyn Future<Output = Option<O>> + Send>>> =
        FuturesUnordered::new();

    // Initial seeding with staggered starts
    for _ in 0..batch_size {
        if cancel.is_cancelled() {
            break;
        }

        if let Some(item) = item_iter.next() {
            futures.push(Box::pin(scan_fn(item)));
            // Stagger connection starts to avoid SYN burst
            if !stagger_delay.is_zero() {
                tokio::time::sleep(stagger_delay).await;
            }
        } else {
            break;
        }
    }

    while let Some(result) = futures.next().await {
        if cancel.is_cancelled() {
            break;
        }

        if let Some(output) = result {
            results.push(output);
        }

        while futures.len() < batch_size && !cancel.is_cancelled() {
            if let Some(item) = item_iter.next() {
                futures.push(Box::pin(scan_fn(item)));
                // Stagger connection starts to avoid SYN burst
                if !stagger_delay.is_zero() {
                    tokio::time::sleep(stagger_delay).await;
                }
            } else {
                break;
            }
        }
    }

    results
}

/// Check if ARP scanning is available on this platform.
///
/// # Arguments
/// * `use_npcap` - (Windows only) Check for Npcap availability instead of SendARP
pub fn can_arp_scan(use_npcap: bool) -> bool {
    let available = crate::daemon::utils::arp::is_available(use_npcap);

    if available {
        tracing::info!("ARP scanning capability confirmed. Fast host discovery enabled.");
    } else {
        tracing::warn!(
            "ARP scanning not available. Will fall back to TCP port scanning for host discovery. \
             For MACVLAN deployments, ensure: (1) container has NET_RAW and NET_ADMIN capabilities, \
             (2) network interface is properly configured with a MAC address."
        );
    }

    available
}

/// Result of probing a host's connection capacity
#[derive(Debug, Clone, Copy)]
pub struct HostCapacityProbe {
    /// Recommended batch size for this host
    pub recommended_batch_size: usize,
    /// Number of probe ports that responded
    pub responsive_ports: usize,
}

impl Default for HostCapacityProbe {
    fn default() -> Self {
        Self {
            recommended_batch_size: PORT_SCAN_BATCH_LOW, // Conservative default
            responsive_ports: 0,
        }
    }
}

/// Probe a host with a small set of common ports to estimate its connection capacity.
/// This helps determine an appropriate batch size before the full port scan.
///
/// # Arguments
/// * `ip` - The IP address to probe
/// * `timeout_ms` - Timeout per connection attempt in milliseconds
///
/// # Returns
/// HostCapacityProbe with recommended batch size based on response pattern
pub async fn probe_host_capacity(ip: IpAddr, timeout_ms: u64) -> HostCapacityProbe {
    use std::time::Instant;

    // Common TCP ports to probe - mix of services found on different device types
    // Using PortType enum for named constants
    let probe_ports = [                                                                                                                                                
        PortType::Ssh,      // 22                                                                                                                                      
        PortType::Telnet,   // 23                                                                                                                                      
        PortType::Http,     // 80                                                                                                                                      
        PortType::Https,    // 443                                                                                                                                     
        PortType::Samba,    // 445 - Windows/NAS                                                                                                                       
        PortType::Http8080, // 8080 - alternate web                                                                                                                    
        PortType::Rdp,      // 3389                                                                                                                                    
    ]; 

    let probe_timeout = Duration::from_millis(timeout_ms);
    let mut response_times: Vec<Duration> = Vec::new();
    let mut responsive_count = 0;

    // Send all probes concurrently
    let start = Instant::now();
    let mut futures: FuturesUnordered<Pin<Box<dyn Future<Output = Option<Duration>> + Send>>> =
        FuturesUnordered::new();

    for port_type in &probe_ports {
        let socket = SocketAddr::new(ip, port_type.number());
        let probe_start = start;

        futures.push(Box::pin(async move {
            match timeout(probe_timeout, TcpStream::connect(socket)).await {
                Ok(Ok(_stream)) => Some(probe_start.elapsed()),
                Ok(Err(_)) => None, // Connection refused - port closed but host responsive
                Err(_) => None,     // Timeout - port filtered or host slow
            }
        }));
    }

    while let Some(result) = futures.next().await {
        if let Some(duration) = result {
            response_times.push(duration);
            responsive_count += 1;
        }
    }

    // Check response time consistency
    let avg_response_time = if !response_times.is_empty() {
        response_times.iter().sum::<Duration>() / response_times.len() as u32
    } else {
        Duration::from_millis(500)
    };

    // Determine host capacity based on probe results
    let recommended_batch_size = if responsive_count >= 3 && avg_response_time < Duration::from_millis(100) {                                                          
        // Multiple services + very fast = robust server                                                                                                               
        PORT_SCAN_BATCH_MAX                                                                                                                                            
    } else if responsive_count >= 2 && avg_response_time < Duration::from_millis(200) {                                                                                
        // Good responsiveness                                                                                                                                         
        PORT_SCAN_BATCH_HIGH                                                                                                                                           
    } else if responsive_count >= 1 && avg_response_time < Duration::from_millis(500) {                                                                                
        // Some response, moderate speed                                                                                                                               
        PORT_SCAN_BATCH_MED                                                                                                                                            
    } else {                                                                                                                                                           
        // Slow, filtered, or no response - likely constrained device                                                                                                  
        PORT_SCAN_BATCH_LOW                                                                                                                                            
    }; 

    tracing::debug!(
        ip = %ip,
        responsive_ports = responsive_count,
        recommended_batch_size = recommended_batch_size,
        "Host capacity probe complete"
    );

    HostCapacityProbe {
        recommended_batch_size,
        responsive_ports: responsive_count,
    }
}

pub async fn scan_ports_and_endpoints(
    ip: IpAddr,
    cancel: CancellationToken,
    port_scan_batch_size: usize,
    scan_rate_pps: u32,
    cidr: IpCidr,
    gateway_ips: Vec<IpAddr>,
    tcp_ports_to_check: Vec<u16>,
) -> Result<(Vec<PortType>, Vec<EndpointResponse>), Error> {
    if cancel.is_cancelled() {
        return Err(anyhow!("Operation cancelled"));
    }

    let mut open_ports = Vec::new();
    let mut endpoint_responses = Vec::new();

    // Scan TCP ports with batching and rate limiting
    let tcp_ports = scan_tcp_ports(
        ip,
        cancel.clone(),
        port_scan_batch_size,
        scan_rate_pps,
        tcp_ports_to_check,
    )
    .await?;

    let use_https_ports: HashMap<u16, bool> =
        tcp_ports.iter().map(|(p, h)| (p.number(), *h)).collect();
    let tcp_ports: Vec<PortType> = tcp_ports.iter().map(|(p, _)| *p).collect();

    open_ports.extend(tcp_ports.clone());

    if cancel.is_cancelled() {
        return Err(anyhow!("Operation cancelled"));
    }

    // Scan UDP ports with batching and rate limiting
    let udp_ports = scan_udp_ports(
        ip,
        cancel.clone(),
        port_scan_batch_size,
        scan_rate_pps,
        cidr,
        gateway_ips,
    )
    .await?;
    open_ports.extend(udp_ports);

    if cancel.is_cancelled() {
        return Err(anyhow!("Operation cancelled"));
    }

    // Scan endpoints - check on ALL open TCP ports, not just filtered ones
    let mut ports_to_check = tcp_ports.clone();

    // Also add endpoint-only ports that we didn't scan during port scanning
    let endpoint_only_ports = Service::endpoint_only_ports();
    ports_to_check.extend(endpoint_only_ports);
    ports_to_check.sort_by_key(|p| (p.number(), p.protocol()));
    ports_to_check.dedup();

    let endpoints = scan_endpoints(
        ip,
        cancel.clone(),
        Some(ports_to_check),
        Some(use_https_ports),
        port_scan_batch_size,
    )
    .await?;
    endpoint_responses.extend(endpoints);

    // Add any ports that had endpoint responses but weren't in open_ports
    // This handles cases where we got HTTP response but port scan didn't detect it
    for endpoint_response in &endpoint_responses {
        let port = endpoint_response.endpoint.port_type;
        if !open_ports.contains(&port) {
            tracing::debug!(
                "Adding port {} to open ports based on successful endpoint response",
                port
            );
            open_ports.push(port);
        }
    }

    // Deduplicate ports (sort first for consistent deduplication)
    open_ports.sort_by_key(|p| (p.number(), p.protocol()));
    open_ports.dedup();

    tracing::info!(
        ip = %ip,
        open_ports = %open_ports.len(),
        endpoint_responses = %endpoint_responses.len(),
        "Host scan complete"
    );

    Ok((open_ports, endpoint_responses))
}

pub async fn scan_tcp_ports(
    ip: IpAddr,
    cancel: CancellationToken,
    batch_size: usize,
    scan_rate_pps: u32,
    tcp_ports_to_check: Vec<u16>,
) -> Result<Vec<(PortType, bool)>, Error> {
    let ports: Vec<PortType> = tcp_ports_to_check
        .iter()
        .map(|p| PortType::new_tcp(*p))
        .collect();

    let open_ports = batch_scan(
        ports.clone(),
        batch_size,
        scan_rate_pps,
        cancel,
        move |port| async move {
            let socket = SocketAddr::new(ip, port.number());

            // Try connection with timeout, retry once on timeout for slow hosts
            let mut attempts = 0;
            let max_attempts = 2;

            loop {
                attempts += 1;
                let start = std::time::Instant::now();

                match timeout(SCAN_TIMEOUT, TcpStream::connect(socket)).await {
                    Ok(Ok(stream)) => {
                        let connect_time = start.elapsed();

                        // Try to peek at the connection to detect immediate disconnects
                        let mut buf = [0u8; 1];
                        let peek_result =
                            timeout(Duration::from_millis(50), stream.peek(&mut buf)).await;

                        let use_https = match peek_result {
                            Ok(Ok(0)) => {
                                // Port open - HTTPS (immediate close)"
                                true
                            }
                            Ok(Ok(_)) => {
                                // Port open - got bytes
                                false
                            }
                            Ok(Err(_)) => {
                                // Port open - peek error
                                false
                            }
                            Err(_) => {
                                // Port open - no immediate response
                                false
                            }
                        };

                        tracing::debug!(
                            "Found open TCP port {}:{} (took {:?})",
                            ip,
                            port,
                            connect_time
                        );

                        drop(stream);
                        return Some((
                            PortType::new_tcp(port.number()),
                            use_https || port.is_https(),
                        ));
                    }
                    Ok(Err(e)) => {
                        if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                            tracing::error!(
                                "Critical error scanning {}:{}: {}",
                                socket.ip(),
                                port,
                                e
                            );
                        }
                        return None;
                    }
                    Err(_) => {
                        let elapsed = start.elapsed();

                        if attempts < max_attempts {
                            tracing::trace!(
                                "Port {}:{} timeout attempt {}/{} (took {:?}), retrying...",
                                ip,
                                port,
                                attempts,
                                max_attempts,
                                elapsed
                            );
                            // Small delay before retry
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        } else {
                            tracing::trace!(
                                "Port {}:{} timeout after {} attempts",
                                ip,
                                port,
                                attempts
                            );
                            return None;
                        }
                    }
                }
            }
        },
    )
    .await;

    tracing::debug!(
        ip = %ip,
        ports_scanned = %ports.len(),
        responses = %open_ports.len(),
        "TCP ports scanned"
    );

    Ok(open_ports)
}

pub async fn scan_udp_ports(
    ip: IpAddr,
    cancel: CancellationToken,
    batch_size: usize,
    scan_rate_pps: u32,
    cidr: IpCidr,
    gateway_ips: Vec<IpAddr>,
) -> Result<Vec<PortType>, Error> {
    let discovery_ports = Service::all_discovery_ports();
    let ports: Vec<u16> = discovery_ports
        .iter()
        .filter(|p| p.protocol() == TransportProtocol::Udp)
        .map(|p| p.number())
        .collect();

    // UDP is slower and less reliable, cap at 10 concurrent
    let udp_batch_size = std::cmp::min(batch_size, 10);

    let is_gateway = gateway_ips.contains(&ip);

    // UDP rate limiting is less critical but still useful
    let open_ports = batch_scan(
        ports.clone(),
        udp_batch_size,
        scan_rate_pps,
        cancel,
        |port| async move {
            let result = match port {
                53 => test_dns_service(ip).await,
                123 => test_ntp_service(ip).await,
                161 => test_snmp_service(ip).await,
                67 => {
                    if is_gateway {
                        test_dhcp_service(ip, &cidr).await
                    } else {
                        Ok(None)
                    }
                }
                _ => Ok(None),
            };

            match result {
                Ok(Some(detected_port)) => {
                    tracing::trace!("Found open UDP port {}:{}", ip, detected_port);
                    Some(PortType::new_udp(detected_port))
                }
                Ok(None) => None,
                Err(e) => {
                    if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                        tracing::error!("Critical error scanning UDP {}:{}: {}", ip, port, e);
                    }
                    None
                }
            }
        },
    )
    .await;

    tracing::debug!(
        ip = %ip,
        ports_scanned = %ports.len(),
        responses = %open_ports.len(),
        "UDP ports scanned"
    );

    Ok(open_ports)
}

pub async fn scan_endpoints(
    ip: IpAddr,
    cancel: CancellationToken,
    filter_ports: Option<Vec<PortType>>,
    use_https_ports: Option<HashMap<u16, bool>>,
    batch_size: usize,
) -> Result<Vec<EndpointResponse>, Error> {
    use std::collections::HashMap;

    let client = reqwest::Client::builder()
        .timeout(SCAN_TIMEOUT)
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| anyhow!("Could not build client {}", e))?;

    let all_endpoints: Vec<Endpoint> = Service::all_discovery_endpoints()
        .into_iter()
        .filter_map(|e| {
            if let Some(filter_ports) = &filter_ports {
                if filter_ports.contains(&e.port_type) {
                    return Some(e);
                }
                None
            } else {
                Some(e)
            }
        })
        .collect();

    // Group endpoints by (port, path) to avoid duplicate requests
    let mut unique_endpoints: HashMap<(u16, String), Endpoint> = HashMap::new();
    for endpoint in all_endpoints {
        let key = (endpoint.port_type.number(), endpoint.path.clone());
        unique_endpoints.entry(key).or_insert(endpoint);
    }

    let endpoints: Vec<Endpoint> = unique_endpoints.into_values().collect();
    let total_endpoints = endpoints.len();

    let endpoint_batch_size = std::cmp::min(batch_size / 2, 50);

    let use_https_ports_is_none = use_https_ports.is_none();
    let https_ports = use_https_ports.unwrap_or_default();

    // Endpoint scanning uses HTTP client with connection pooling, rate limiting less critical
    let responses = batch_scan(endpoints, endpoint_batch_size, 0, cancel, move |endpoint| {
        let client = client.clone();
        let https_ports = https_ports.clone();
        async move {
            let endpoint_with_ip = endpoint.use_ip(ip);

            // Common HTTPS ports
            let use_https = https_ports
                .get(&endpoint.port_type.number())
                .unwrap_or(&false);
            let url = format!(
                "{}:{}{}",
                ip,
                endpoint_with_ip.port_type.number(),
                endpoint_with_ip.path
            );
            let http_url = format!("http://{}", url);
            let https_url = format!("https://{}", url);

            // Decide which of HTTP or HTTPS to try first
            let urls = if use_https_ports_is_none {
                // No info = try both
                vec![http_url, https_url]
            } else if *use_https {
                vec![https_url, http_url]
            } else {
                vec![http_url, https_url]
            };

            for url in urls {
                tracing::trace!("Trying endpoint: {}", url);

                match client.get(&url).send().await {
                    Ok(response) => {
                        let status = response.status().as_u16();

                        let headers = response
                            .headers()
                            .iter()
                            .filter_map(|(name, value)| {
                                // Convert HeaderValue to string
                                value.to_str().ok().map(|v| {
                                    (
                                        name.as_str().to_lowercase(), // Normalize to lowercase
                                        v.to_string(),
                                    )
                                })
                            })
                            .collect();

                        match response.text().await {
                            Ok(body) => {
                                tracing::debug!(
                                    "Endpoint {} returned {} (length: {})",
                                    url,
                                    status,
                                    body.len()
                                );
                                return Some(EndpointResponse {
                                    endpoint: endpoint_with_ip,
                                    headers,
                                    body,
                                    status,
                                });
                            }
                            Err(e) => {
                                tracing::trace!("Failed to read response from {}: {}", url, e);
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::trace!("Endpoint {} failed: {}", url, e);
                        if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                            tracing::error!("Critical error scanning endpoint {}: {}", url, e);
                        }
                        continue;
                    }
                }
            }

            None
        }
    })
    .await;

    tracing::debug!(
        ip = %ip,
        endpoints_scanned = %total_endpoints,
        responses = %responses.len(),
        "Endpoint scan complete"
    );

    Ok(responses)
}

pub async fn test_dns_service(ip: IpAddr) -> Result<Option<u16>, Error> {
    let mut config = ResolverConfig::new();
    let name_server = NameServerConfig::new(SocketAddr::new(ip, 53), Protocol::Udp);
    config.add_name_server(name_server);

    let resolver =
        Resolver::builder_with_config(config, TokioConnectionProvider::default()).build();

    match timeout(
        Duration::from_millis(2000),
        resolver.lookup_ip("google.com"),
    )
    .await
    {
        Ok(Ok(_)) => Ok(Some(53)),
        _ => Ok(None),
    }
}

pub async fn test_ntp_service(ip: IpAddr) -> Result<Option<u16>, Error> {
    let client = AsyncSntpClient::new();
    let server_addr = format!("{}:123", ip);

    match timeout(
        Duration::from_millis(2000),
        client.synchronize(&server_addr),
    )
    .await
    {
        Ok(Ok(result)) => {
            // Validate that we got a meaningful time response
            if let Ok(datetime) = result.datetime().unix_timestamp() {
                if datetime > Duration::from_secs(0) {
                    Ok(Some(123))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
        Ok(Err(_)) => Ok(None),
        Err(_) => Ok(None),
    }
}

pub async fn test_snmp_service(ip: IpAddr) -> Result<Option<u16>, Error> {
    let target = format!("{}:161", ip);
    let community = b"public";

    match AsyncSession::new_v2c(&target, community, 0).await {
        Ok(mut session) => {
            let sys_descr_oid = Oid::from(&[1, 3, 6, 1, 2, 1, 1, 1, 0])
                .map_err(|e| anyhow!("Invalid Oid: {:?}", e))?;

            match timeout(Duration::from_millis(2000), session.get(&sys_descr_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some(_varbind) = response.varbinds.next() {
                        Ok(Some(161))
                    } else {
                        Ok(None)
                    }
                }
                Ok(Err(_)) => Ok(None),
                Err(_) => Ok(None),
            }
        }
        Err(_) => Ok(None),
    }
}

/// Test if a host is running a DHCP server on port 67
pub async fn test_dhcp_service(ip: IpAddr, subnet_cidr: &IpCidr) -> Result<Option<u16>, Error> {
    let socket = match UdpSocket::bind("0.0.0.0:68").await {
        Ok(s) => s,
        Err(_) => {
            // If port 68 is busy (another DHCP client), try random port
            match UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => s,
                Err(_) => {
                    return Ok(None);
                }
            }
        }
    };

    if socket.set_broadcast(true).is_err() {
        return Ok(None);
    }

    // Calculate broadcast address for this subnet
    let broadcast_addr = match subnet_cidr {
        IpCidr::V4(cidr) => {
            let broadcast_ip = cidr.last_address();
            SocketAddr::new(IpAddr::V4(broadcast_ip), 67)
        }
        IpCidr::V6(_) => {
            return Ok(None);
        }
    };

    // Create a more complete DHCP DISCOVER message
    let mut rng = rand::rngs::StdRng::from_os_rng();
    let mac_addr: [u8; 6] = rng.random();
    let transaction_id = rng.random::<u32>();

    let mut msg = Message::default();
    msg.set_opcode(v4::Opcode::BootRequest)
        .set_htype(v4::HType::Eth)
        .set_xid(transaction_id)
        .set_flags(v4::Flags::default().set_broadcast())
        .set_chaddr(&mac_addr);

    // Add required and common DHCP options
    msg.opts_mut()
        .insert(v4::DhcpOption::MessageType(MessageType::Discover));

    // Add parameter request list (commonly requested by clients)
    msg.opts_mut()
        .insert(v4::DhcpOption::ParameterRequestList(vec![
            v4::OptionCode::SubnetMask,
            v4::OptionCode::Router,
            v4::OptionCode::DomainNameServer,
            v4::OptionCode::DomainName,
        ]));

    // Encode DHCP DISCOVER packet
    let mut buf = Vec::new();
    let mut encoder = Encoder::new(&mut buf);
    msg.encode(&mut encoder)?;

    if socket.send_to(&buf, broadcast_addr).await.is_ok()
        && let Some(port) = wait_for_dhcp_responses(&socket, ip, transaction_id, 3).await?
    {
        return Ok(Some(port));
    }

    // Fall back to unicast
    let unicast_addr = SocketAddr::new(ip, 67);

    if socket.send_to(&buf, unicast_addr).await.is_ok()
        && let Some(port) = wait_for_dhcp_responses(&socket, ip, transaction_id, 3).await?
    {
        return Ok(Some(port));
    }

    Ok(None)
}

/// Helper function to wait for and validate DHCP responses (checks multiple times)
async fn wait_for_dhcp_responses(
    socket: &UdpSocket,
    expected_ip: IpAddr,
    expected_xid: u32,
    max_attempts: usize,
) -> Result<Option<u16>, Error> {
    let mut response_buf = [0u8; 1500];

    for _ in 1..=max_attempts {
        match timeout(
            Duration::from_millis(2000), // Longer timeout per attempt
            socket.recv_from(&mut response_buf),
        )
        .await
        {
            Ok(Ok((len, from))) => {
                if len == 0 {
                    continue;
                }

                let response_ip = from.ip();

                // Check if response came from the IP we're testing
                if response_ip != expected_ip {
                    continue; // Keep trying - might get another response
                }

                // Parse and validate DHCP message
                match Message::decode(&mut dhcproto::Decoder::new(&response_buf[..len])) {
                    Ok(response_msg) => {
                        // Verify transaction ID matches
                        if response_msg.xid() != expected_xid {
                            continue;
                        }

                        // Check for valid DHCP response type
                        let message_type = response_msg.opts().iter().find_map(|(_, opt)| {
                            if let v4::DhcpOption::MessageType(msg_type) = opt {
                                Some(msg_type)
                            } else {
                                None
                            }
                        });

                        let is_valid = matches!(
                            message_type,
                            Some(&MessageType::Offer) | Some(&MessageType::Ack)
                        );

                        if is_valid {
                            return Ok(Some(67));
                        } else {
                            continue;
                        }
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
            Ok(Err(_)) => {
                break; // Socket error, no point continuing
            }
            Err(_) => {
                // Timeout - continue to next attempt
            }
        }
    }

    Ok(None)
}
