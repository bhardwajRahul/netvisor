//! IPsec detection with an `IKE_SA_INIT`.
//!
//! IKE's opening exchange is unauthenticated by construction: both peers have to agree on a cipher
//! suite and exchange Diffie-Hellman values before either can prove who it is, so a responder
//! answers a well-formed `IKE_SA_INIT` from anyone. This is what `ike-scan` has done for twenty
//! years.
//!
//! **The rejections count.** A responder that dislikes our proposal answers `NO_PROPOSAL_CHOSEN`,
//! and one that dislikes our DH group answers `INVALID_KE_PAYLOAD`. Both are IKE responses and both
//! prove a responder is there, which is the claim the definition makes. Only the initiator SPI
//! matters: it comes back in every reply, and it is what separates an answer to us from a stray
//! packet.
//!
//! Two ports: 500 is the standard one, 4500 is used once NAT traversal is detected. On 4500 an IKE
//! message is prefixed with four zero bytes to distinguish it from ESP, so the same exchange is
//! framed differently there.

use anyhow::Error;
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{Duration, timeout};

use crate::daemon::utils::app_probe::{AppProbe, AppProbeOutcome, ProbeContext, presence};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// IKEv2.
const IKE_VERSION_2: u8 = 0x20;
/// Exchange type 34: `IKE_SA_INIT`.
const EXCHANGE_IKE_SA_INIT: u8 = 34;
/// Flags bit 3: this message is from the initiator.
const FLAG_INITIATOR: u8 = 0x08;
/// Initiator SPI (8) + responder SPI (8) + next payload + version + exchange + flags + message id
/// (4) + length (4).
const ISAKMP_HEADER_LEN: usize = 28;

/// Ours, echoed by any responder that answers.
const INITIATOR_SPI: [u8; 8] = [0x5C, 0xA1, 0x00, 0x9E, 0x1C, 0xE0, 0x00, 0x01];

/// How long to wait for a responder. Longer than a TCP connect because IKE responders are commonly
/// rate-limited and answer slowly under load.
const IKE_TIMEOUT: Duration = Duration::from_millis(2000);

/// Payload type 33: Security Association.
const PAYLOAD_SA: u8 = 33;
/// Payload type 0: none follows.
const PAYLOAD_NONE: u8 = 0;

pub struct IkeProbe;

#[async_trait]
impl AppProbe for IkeProbe {
    fn port(&self) -> PortType {
        PortType::new_udp(500)
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Ike)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(presence(
            exchange(ctx, 500, false).await || exchange(ctx, 4500, true).await,
        ))
    }
}

/// Send an `IKE_SA_INIT` to one port and report whether a responder answered.
async fn exchange(ctx: &ProbeContext, port: u16, nat_traversal: bool) -> bool {
    let Ok(socket) = UdpSocket::bind("0.0.0.0:0").await else {
        return false;
    };
    let target = SocketAddr::new(ctx.ip, port);

    let mut message = sa_init_request();
    if nat_traversal {
        // On 4500 an IKE message carries a four-zero-byte non-ESP marker.
        message.splice(0..0, [0u8; 4]);
    }

    if socket.send_to(&message, target).await.is_err() {
        return false;
    }

    let mut buf = [0u8; 1024];
    let Ok(Ok((read, _))) = timeout(IKE_TIMEOUT, socket.recv_from(&mut buf)).await else {
        return false;
    };

    let reply = if nat_traversal {
        &buf[4..read]
    } else {
        &buf[..read]
    };
    is_ike_response(reply, &INITIATOR_SPI)
}

/// An `IKE_SA_INIT` carrying a minimal SA proposal.
///
/// Deliberately not a complete, negotiable proposal: no KE or nonce payload follows, so a responder
/// will reject it. That is fine and is the point — the rejection is an IKE message and proves the
/// responder is there, without this having to carry a Diffie-Hellman implementation.
fn sa_init_request() -> Vec<u8> {
    // One transform: encryption AES-CBC.
    let transform = [
        PAYLOAD_NONE,
        0x00,
        0x00,
        0x0C, // last transform, length 12
        0x01,
        0x00,
        0x00,
        0x0C, // type ENCR, id AES-CBC
        0x80,
        0x0E,
        0x00,
        0x80, // attribute: key length 128
    ];

    let mut proposal = vec![
        PAYLOAD_NONE,
        0x00,
        0x00,
        0x00, // last proposal; length patched below
        0x01, // proposal number 1
        0x01, // protocol id: IKE
        0x00, // SPI size
        0x01, // one transform
    ];
    proposal.extend_from_slice(&transform);
    let proposal_len = proposal.len() as u16;
    proposal[2..4].copy_from_slice(&proposal_len.to_be_bytes());

    let mut sa_payload = vec![PAYLOAD_NONE, 0x00, 0x00, 0x00];
    sa_payload.extend_from_slice(&proposal);
    let sa_len = sa_payload.len() as u16;
    sa_payload[2..4].copy_from_slice(&sa_len.to_be_bytes());

    let mut message = Vec::with_capacity(ISAKMP_HEADER_LEN + sa_payload.len());
    message.extend_from_slice(&INITIATOR_SPI);
    message.extend_from_slice(&[0u8; 8]); // responder SPI: unknown until it answers
    message.push(PAYLOAD_SA);
    message.push(IKE_VERSION_2);
    message.push(EXCHANGE_IKE_SA_INIT);
    message.push(FLAG_INITIATOR);
    message.extend_from_slice(&0u32.to_be_bytes()); // message id
    message.extend_from_slice(&((ISAKMP_HEADER_LEN + sa_payload.len()) as u32).to_be_bytes());
    message.extend_from_slice(&sa_payload);
    message
}

/// Whether the bytes are an IKE message answering the SPI we sent.
///
/// The initiator flag has to be *clear*: a responder's reply carries our SPI with the flag unset,
/// whereas an echo of our own packet would carry it set. That is what stops a reflecting middlebox
/// from being read as a VPN gateway.
fn is_ike_response(bytes: &[u8], initiator_spi: &[u8; 8]) -> bool {
    let Some(header) = bytes.get(..ISAKMP_HEADER_LEN) else {
        return false;
    };
    let version_major = header[17] & 0xF0;

    &header[..8] == initiator_spi
        && version_major == IKE_VERSION_2
        && header[19] & FLAG_INITIATOR == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(spi: [u8; 8], flags: u8, exchange_type: u8) -> Vec<u8> {
        let mut message = spi.to_vec();
        message.extend_from_slice(&[0xAA; 8]); // responder SPI, now assigned
        message.push(PAYLOAD_NONE);
        message.push(IKE_VERSION_2);
        message.push(exchange_type);
        message.push(flags);
        message.extend_from_slice(&0u32.to_be_bytes());
        message.extend_from_slice(&(ISAKMP_HEADER_LEN as u32).to_be_bytes());
        message
    }

    #[test]
    fn a_reply_carrying_our_spi_is_ike() {
        assert!(is_ike_response(
            &response(INITIATOR_SPI, 0x20, EXCHANGE_IKE_SA_INIT),
            &INITIATOR_SPI
        ));
    }

    /// A responder rejecting our proposal answers with an informational exchange, and that is still
    /// a responder.
    #[test]
    fn a_rejection_is_ike() {
        assert!(is_ike_response(
            &response(INITIATOR_SPI, 0x20, 37),
            &INITIATOR_SPI
        ));
    }

    /// Our own packet reflected back carries the initiator flag set, and is not evidence of a peer.
    #[test]
    fn an_echo_of_our_own_request_is_not_a_response() {
        let echoed = sa_init_request();
        assert!(!is_ike_response(&echoed, &INITIATOR_SPI));
    }

    #[test]
    fn an_uncorrelated_or_short_message_is_not_ike() {
        assert!(!is_ike_response(
            &response([0xFF; 8], 0x20, EXCHANGE_IKE_SA_INIT),
            &INITIATOR_SPI
        ));
        assert!(!is_ike_response(b"", &INITIATOR_SPI));
        assert!(!is_ike_response(
            b"not an isakmp header at all",
            &INITIATOR_SPI
        ));
    }

    #[test]
    fn the_request_declares_its_length_and_proposes_ike() {
        let request = sa_init_request();
        let declared = u32::from_be_bytes(request[24..28].try_into().unwrap()) as usize;
        assert_eq!(
            declared,
            request.len(),
            "the length counts the whole message"
        );
        assert_eq!(request[16], PAYLOAD_SA, "an SA payload follows the header");
        assert_eq!(request[17], IKE_VERSION_2);
        assert_eq!(request[18], EXCHANGE_IKE_SA_INIT);
        assert_eq!(&request[..8], &INITIATOR_SPI);
    }
}
