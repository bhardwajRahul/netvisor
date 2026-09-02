//! OpenVPN detection with a control-channel hard reset.
//!
//! A client opens an OpenVPN session by sending `P_CONTROL_HARD_RESET_CLIENT_V2`, and the server
//! answers `P_CONTROL_HARD_RESET_SERVER_V2` before any TLS handshake or certificate exchange. The
//! reply echoes our session id, which is what correlates it.
//!
//! **A stated limit.** A server configured with `tls-auth` or `tls-crypt` HMACs its control packets
//! and drops ours without a word, so those stay undetected. That is a real gap and it is worth
//! being clear about: `Answered` means definitely OpenVPN, `NoAnswer` means unknown. Unknown is
//! also what every OpenVPN server produced before this probe existed, because 1194/udp was never
//! reported open at all — nothing reports a UDP port open without a probe behind it — so the
//! definition could never match. This detects some of them rather than none.

use anyhow::Error;
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time::{Duration, timeout};

use crate::daemon::utils::app_probe::{AppProbe, AppProbeOutcome, ProbeContext, presence};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Opcode 7, in the upper five bits of the first byte.
const P_CONTROL_HARD_RESET_CLIENT_V2: u8 = 7 << 3;
/// Opcode 8, the server's answer to the above.
const P_CONTROL_HARD_RESET_SERVER_V2: u8 = 8 << 3;
/// Opcode 10, sent instead when the server wants a different reset.
const P_CONTROL_SOFT_RESET_V1: u8 = 3 << 3;

/// Ours, echoed by the server in its own reply as the "remote session id".
const SESSION_ID: [u8; 8] = [0x5C, 0xA1, 0x00, 0x0B, 0xE0, 0x00, 0x00, 0x01];

const OPENVPN_TIMEOUT: Duration = Duration::from_millis(2000);

pub struct OpenVpnProbe;

#[async_trait]
impl AppProbe for OpenVpnProbe {
    fn port(&self) -> PortType {
        PortType::OpenVPN
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::OpenVpn)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(socket) => socket,
            Err(_) => return Ok(AppProbeOutcome::NoAnswer),
        };
        let target = SocketAddr::new(ctx.ip, self.port().number());

        if socket.send_to(&hard_reset(), target).await.is_err() {
            return Ok(AppProbeOutcome::NoAnswer);
        }

        let mut buf = [0u8; 512];
        let Ok(Ok((read, _))) = timeout(OPENVPN_TIMEOUT, socket.recv_from(&mut buf)).await else {
            return Ok(AppProbeOutcome::NoAnswer);
        };

        Ok(presence(is_server_reset(&buf[..read], &SESSION_ID)))
    }
}

/// A `P_CONTROL_HARD_RESET_CLIENT_V2` with no packet-id history.
fn hard_reset() -> Vec<u8> {
    let mut packet = vec![P_CONTROL_HARD_RESET_CLIENT_V2];
    packet.extend_from_slice(&SESSION_ID);
    packet.push(0x00); // packet-id array length: none acknowledged yet
    packet.extend_from_slice(&0u32.to_be_bytes()); // our packet id
    packet
}

/// Whether the reply is a server reset echoing our session id.
///
/// The echo is at offset 18: opcode (1), the server's own session id (8), the ack array length (1),
/// one acked packet id (4), then the remote session id it is acknowledging — ours.
fn is_server_reset(bytes: &[u8], session_id: &[u8; 8]) -> bool {
    let opcode = match bytes.first() {
        Some(byte) => byte & 0xF8,
        None => return false,
    };
    if opcode != P_CONTROL_HARD_RESET_SERVER_V2 && opcode != P_CONTROL_SOFT_RESET_V1 {
        return false;
    }
    bytes.get(14..22).is_some_and(|echoed| echoed == session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server_reset(opcode: u8, echoed: [u8; 8]) -> Vec<u8> {
        let mut packet = vec![opcode];
        packet.extend_from_slice(&[0xAA; 8]); // the server's own session id
        packet.push(0x01); // one packet id acknowledged
        packet.extend_from_slice(&0u32.to_be_bytes());
        packet.extend_from_slice(&echoed);
        packet
    }

    #[test]
    fn a_server_hard_reset_echoing_our_session_is_openvpn() {
        assert!(is_server_reset(
            &server_reset(P_CONTROL_HARD_RESET_SERVER_V2, SESSION_ID),
            &SESSION_ID
        ));
    }

    #[test]
    fn a_soft_reset_is_openvpn() {
        assert!(is_server_reset(
            &server_reset(P_CONTROL_SOFT_RESET_V1, SESSION_ID),
            &SESSION_ID
        ));
    }

    /// Our own packet reflected back has the client opcode and no echo, so it proves nothing.
    #[test]
    fn an_echo_of_our_own_reset_is_not_a_response() {
        assert!(!is_server_reset(&hard_reset(), &SESSION_ID));
    }

    #[test]
    fn an_uncorrelated_reset_or_junk_is_not_openvpn() {
        assert!(!is_server_reset(
            &server_reset(P_CONTROL_HARD_RESET_SERVER_V2, [0xFF; 8]),
            &SESSION_ID
        ));
        assert!(!is_server_reset(b"", &SESSION_ID));
        assert!(!is_server_reset(b"SSH-2.0-OpenSSH_9.6\r\n", &SESSION_ID));
    }

    #[test]
    fn the_reset_carries_the_client_opcode_and_our_session_id() {
        let packet = hard_reset();
        assert_eq!(packet[0] & 0xF8, P_CONTROL_HARD_RESET_CLIENT_V2);
        assert_eq!(&packet[1..9], &SESSION_ID);
    }
}
