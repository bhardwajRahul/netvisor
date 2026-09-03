//! The four non-credentialed UDP probes, migrated off `scan_udp_ports`'s hardcoded
//! `match port { 53 => …, 123 => …, 67 => …, 47808 => …, _ => Ok(None) }`.
//!
//! **The probe functions below are the pre-migration bodies, moved verbatim.** None of these four
//! had a test before the move and only BACnet's positive path is reachable in one (the others fix
//! privileged ports through their library clients), so the guarantee that behaviour did not change
//! is that nothing about them changed — the `AppProbe` impls are thin adapters and the dispatch
//! match was deleted rather than rewritten.
//!
//! None of the four returns a [`DeviceIdentity`]: DNS and NTP are library calls that never see a
//! raw response, DHCP's OFFER carries no device identity, and BACnet's I-Am does carry a vendor ID
//! and device instance but parsing it is not this change. The shape is now in place for that to be
//! a parser and nothing else.

use anyhow::Error;
use async_trait::async_trait;
use cidr::IpCidr;
use dhcproto::Encodable;
use dhcproto::v4::{self, Decodable, Encoder, Message, MessageType};
use hickory_resolver::Resolver;
use hickory_resolver::config::{NameServerConfig, ResolverConfig};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use rand::{Rng, SeedableRng};
use rsntp::AsyncSntpClient;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;

use crate::daemon::utils::app_probe::{AppProbe, AppProbeOutcome, ProbeContext};
use crate::server::ports::r#impl::base::PortType;

/// The four migrated probes report presence only, so a detected port is the whole outcome.
fn presence(detected: Option<u16>) -> AppProbeOutcome {
    match detected {
        Some(_) => AppProbeOutcome::Answered { identity: None },
        None => AppProbeOutcome::NoAnswer,
    }
}

pub struct DnsProbe;

#[async_trait]
impl AppProbe for DnsProbe {
    fn port(&self) -> PortType {
        PortType::DnsUdp
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(presence(test_dns_service(ctx.ip).await?))
    }
}

pub struct NtpProbe;

#[async_trait]
impl AppProbe for NtpProbe {
    fn port(&self) -> PortType {
        PortType::Ntp
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(presence(test_ntp_service(ctx.ip).await?))
    }
}

pub struct DhcpProbe;

#[async_trait]
impl AppProbe for DhcpProbe {
    fn port(&self) -> PortType {
        PortType::Dhcp
    }

    /// Only the gateway is asked. The probe broadcasts a DISCOVER and any DHCP server on the
    /// segment may answer, so running it against every address would attribute one server's OFFER
    /// to whichever host happened to be probed. Declared here rather than buried in a dispatch arm
    /// as `if is_gateway`, which is where this rule used to live.
    fn applies(&self, ctx: &ProbeContext) -> bool {
        ctx.is_gateway
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(presence(test_dhcp_service(ctx.ip, &ctx.subnet_cidr).await?))
    }
}

pub struct BacnetProbe;

#[async_trait]
impl AppProbe for BacnetProbe {
    fn port(&self) -> PortType {
        PortType::BACnet
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(presence(test_bacnet_service(ctx.ip).await?))
    }
}

pub async fn test_dns_service(ip: IpAddr) -> Result<Option<u16>, Error> {
    let mut config = ResolverConfig::default();
    config.add_name_server(NameServerConfig::udp(ip));

    let resolver =
        Resolver::builder_with_config(config, TokioRuntimeProvider::default()).build()?;

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

/// Test if a host is running a BACnet service on UDP port 47808
pub async fn test_bacnet_service(ip: IpAddr) -> Result<Option<u16>, Error> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    let target = SocketAddr::new(ip, 47808);

    // BACnet Who-Is probe packet
    // BVLC header + NPDU + Who-Is APDU
    let bacnet_probe: [u8; 12] = [
        0x81, // BVLC type indicator
        0x0a, // Original-Unicast-NPDU
        0x00, 0x0c, // Length: 12 bytes (big-endian)
        0x01, // NPDU version 1
        0x04, // NPDU control: expecting reply, no DNET/DLEN/DADR
        0x00, // Hop count (unused for unicast)
        0x00, // Reserved
        0x10, // APDU type: Unconfirmed service request
        0x08, // Service choice: Who-Is
        0x00, // No device instance range (optional field)
        0x00, // Padding to reach 12 bytes
    ];

    if socket.send_to(&bacnet_probe, target).await.is_err() {
        return Ok(None);
    }

    let mut response_buf = [0u8; 512];
    match timeout(
        Duration::from_millis(2000),
        socket.recv_from(&mut response_buf),
    )
    .await
    {
        Ok(Ok((len, from))) => {
            // Verify response is from the target IP
            if from.ip() != ip {
                return Ok(None);
            }

            // Check for valid BACnet response:
            // - At least 4 bytes (minimum BVLC header)
            // - First byte is 0x81 (BVLC type indicator)
            if len >= 4 && response_buf[0] == 0x81 {
                tracing::debug!("BACnet service detected on {}:47808", ip);
                return Ok(Some(47808));
            }

            Ok(None)
        }
        Ok(Err(_)) => Ok(None),
        Err(_) => Ok(None), // Timeout
    }
}

/// Characterization of the four non-credentialed UDP probes, written before they moved onto the
/// application-probe stage.
///
/// These pin behaviour rather than implementation: they move with the probe functions and must
/// keep passing across the migration unchanged, which is the whole point of writing them first.
///
/// **What cannot be pinned here, and why.** Only BACnet's positive path is reachable in a unit
/// test. `test_dns_service` fixes port 53 through hickory's resolver config, `test_ntp_service`
/// fixes 123 through sntpc, and `test_dhcp_service` binds 68 and targets 67 — all privileged, so
/// no unprivileged fake responder can stand in. For those three the negative path is pinned here
/// and the migration's safety comes from moving the function bodies verbatim, so a probe body
/// cannot drift while its coverage is thin.
#[cfg(test)]
mod probe_characterization {
    use super::*;
    use std::net::Ipv4Addr;
    use std::sync::LazyLock;
    use tokio::sync::Mutex;

    /// BACnet's port is fixed at 47808, so the fake responders below cannot run concurrently.
    static BACNET_PORT: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    /// RFC 5737 TEST-NET-1. Reserved for documentation and guaranteed never to answer, which is
    /// what makes the negative-path assertions deterministic rather than dependent on whatever
    /// the developer's machine happens to be running on loopback.
    const UNREACHABLE: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));

    /// A one-shot responder on 47808 that replies with `reply` to the first datagram it gets,
    /// handing back what it received so the request itself can be asserted on.
    async fn bacnet_responder(reply: Vec<u8>) -> tokio::task::JoinHandle<Vec<u8>> {
        let socket = UdpSocket::bind("127.0.0.1:47808")
            .await
            .expect("bind fake BACnet responder");

        tokio::spawn(async move {
            let mut buf = [0u8; 512];
            let (len, from) = socket.recv_from(&mut buf).await.expect("receive Who-Is");
            if !reply.is_empty() {
                socket.send_to(&reply, from).await.expect("send reply");
            }
            buf[..len].to_vec()
        })
    }

    fn localhost() -> IpAddr {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    }

    /// A minimal Original-Unicast-NPDU carrying an I-Am, which is what a real device answers
    /// Who-Is with.
    fn i_am_reply() -> Vec<u8> {
        vec![
            0x81, 0x0a, 0x00, 0x0c, 0x01, 0x00, 0x10, 0x00, 0xc4, 0x02, 0x00, 0x01,
        ]
    }

    #[tokio::test]
    async fn bacnet_detects_a_device_answering_with_a_bvlc_frame() {
        let _guard = BACNET_PORT.lock().await;
        let responder = bacnet_responder(i_am_reply()).await;

        let detected = test_bacnet_service(localhost()).await.expect("no error");

        assert_eq!(detected, Some(47808));
        responder.await.expect("responder finished");
    }

    /// The request has to be a well-formed Who-Is or no real device answers it. Asserted
    /// structurally — the BVLC type, the length field agreeing with the datagram, and the APDU
    /// service choice — rather than against the literal byte array, which would restate the
    /// constant and break on any harmless reflow.
    #[tokio::test]
    async fn bacnet_sends_a_well_formed_who_is() {
        let _guard = BACNET_PORT.lock().await;
        let responder = bacnet_responder(i_am_reply()).await;

        test_bacnet_service(localhost()).await.expect("no error");
        let request = responder.await.expect("responder finished");

        assert_eq!(request[0], 0x81, "BVLC type indicator");
        assert_eq!(request[1], 0x0a, "Original-Unicast-NPDU");
        assert_eq!(
            u16::from_be_bytes([request[2], request[3]]) as usize,
            request.len(),
            "BVLC length field must agree with the datagram length"
        );
        assert_eq!(request[8], 0x10, "unconfirmed service request");
        assert_eq!(request[9], 0x08, "service choice Who-Is");
    }

    #[tokio::test]
    async fn bacnet_rejects_a_reply_that_is_not_bvlc() {
        let _guard = BACNET_PORT.lock().await;
        let responder = bacnet_responder(vec![0x00, 0x0a, 0x00, 0x0c]).await;

        let detected = test_bacnet_service(localhost()).await.expect("no error");

        assert_eq!(detected, None, "first byte is not the BVLC type indicator");
        responder.await.expect("responder finished");
    }

    #[tokio::test]
    async fn bacnet_rejects_a_reply_too_short_to_be_a_bvlc_header() {
        let _guard = BACNET_PORT.lock().await;
        let responder = bacnet_responder(vec![0x81, 0x0a]).await;

        let detected = test_bacnet_service(localhost()).await.expect("no error");

        assert_eq!(detected, None, "a BVLC header is at least 4 bytes");
        responder.await.expect("responder finished");
    }

    #[tokio::test]
    async fn bacnet_reports_nothing_when_no_device_answers() {
        let detected = test_bacnet_service(UNREACHABLE).await.expect("no error");

        assert_eq!(detected, None);
    }

    /// DHCPv6 is a different protocol on different ports, and the probe declines it outright
    /// rather than sending a v4 DISCOVER somewhere meaningless.
    #[tokio::test]
    async fn dhcp_declines_ipv6_subnets() {
        let cidr: IpCidr = "2001:db8::/64".parse().expect("valid v6 cidr");

        let detected = test_dhcp_service(localhost(), &cidr)
            .await
            .expect("no error");

        assert_eq!(detected, None);
    }

    /// The contract every caller depends on: an address that does not answer is `Ok(None)`, not
    /// `Err`. `scan_udp_ports` treats an `Err` as a critical-error candidate, so a probe that
    /// started erroring on ordinary silence would flood the session with warnings.
    #[tokio::test]
    async fn dns_reports_nothing_rather_than_erroring_when_unreachable() {
        let detected = test_dns_service(UNREACHABLE).await.expect("no error");

        assert_eq!(detected, None);
    }

    #[tokio::test]
    async fn ntp_reports_nothing_rather_than_erroring_when_unreachable() {
        let detected = test_ntp_service(UNREACHABLE).await.expect("no error");

        assert_eq!(detected, None);
    }
}
