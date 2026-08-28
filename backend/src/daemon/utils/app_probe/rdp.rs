//! Remote Desktop detection with an X.224 connection request.
//!
//! RDP rides on TPKT and X.224: the client sends a Connection Request TPDU and the server answers a
//! Connection Confirm, all before any TLS, CredSSP or credentials. A server that refuses our
//! requested security protocols answers a negotiation *failure* inside the same Connection Confirm,
//! which identifies it as readily as a success.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// TPKT version, which is 3 in every deployment of this stack.
const TPKT_VERSION: u8 = 0x03;
/// X.224 Connection Request.
const X224_CONNECTION_REQUEST: u8 = 0xE0;
/// X.224 Connection Confirm, the reply to the above.
const X224_CONNECTION_CONFIRM: u8 = 0xD0;
/// TPKT header (4) plus the X.224 length indicator byte.
const TPKT_HEADER_LEN: usize = 4;

/// A Connection Request carrying an RDP negotiation request.
fn connection_request() -> Vec<u8> {
    // RDP Negotiation Request: type, flags, length (LE), requested protocols (LE).
    // Requesting nothing but standard RDP security keeps this from depending on TLS support.
    let negotiation = [0x01u8, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00];

    // X.224 CR: length indicator, code, dst-ref, src-ref, class.
    let mut x224 = vec![
        (6 + negotiation.len()) as u8,
        X224_CONNECTION_REQUEST,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
    ];
    x224.extend_from_slice(&negotiation);

    let total = (TPKT_HEADER_LEN + x224.len()) as u16;
    let mut packet = vec![TPKT_VERSION, 0x00];
    packet.extend_from_slice(&total.to_be_bytes());
    packet.extend_from_slice(&x224);
    packet
}

pub struct RdpProbe;

#[async_trait]
impl AppProbe for RdpProbe {
    fn port(&self) -> PortType {
        PortType::Rdp
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Rdp)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &connection_request(), 256).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is a TPKT-framed X.224 Connection Confirm.
///
/// Both layers are checked. The TPKT version and its length field say this is the right framing;
/// the X.224 code at offset 5 says the server accepted the connection request rather than sending
/// something else that happened to be TPKT-shaped.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    let Some(header) = bytes.get(..6) else {
        return AppProbeOutcome::NoAnswer;
    };
    let declared = u16::from_be_bytes([header[2], header[3]]) as usize;

    presence(
        header[0] == TPKT_VERSION
            && declared >= bytes.len()
            && declared > TPKT_HEADER_LEN
            && header[5] == X224_CONNECTION_CONFIRM,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn confirm(tail: &[u8]) -> Vec<u8> {
        let mut x224 = vec![
            (6 + tail.len()) as u8,
            X224_CONNECTION_CONFIRM,
            0,
            0,
            0,
            0,
            0,
        ];
        x224.extend_from_slice(tail);
        let total = (TPKT_HEADER_LEN + x224.len()) as u16;
        let mut packet = vec![TPKT_VERSION, 0x00];
        packet.extend_from_slice(&total.to_be_bytes());
        packet.extend_from_slice(&x224);
        packet
    }

    #[test]
    fn a_connection_confirm_is_rdp() {
        // Negotiation response: type 2, granted TLS.
        assert_eq!(
            parse_reply(&confirm(&[0x02, 0x00, 0x08, 0x00, 0x01, 0x00, 0x00, 0x00])),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A server refusing our security protocols answers a negotiation failure, inside a Connection
    /// Confirm. Still RDP.
    #[test]
    fn a_negotiation_failure_is_rdp() {
        assert_eq!(
            parse_reply(&confirm(&[0x03, 0x00, 0x08, 0x00, 0x02, 0x00, 0x00, 0x00])),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_or_another_protocol_is_not_rdp() {
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &[TPKT_VERSION, 0x00, 0x00, 0x0B][..],
            // TPKT framing, but the X.224 code is a connection request rather than a confirm.
            &[
                TPKT_VERSION,
                0x00,
                0x00,
                0x0B,
                0x06,
                X224_CONNECTION_REQUEST,
                0,
            ][..],
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }

    #[test]
    fn the_connection_request_is_tpkt_framed() {
        let packet = connection_request();
        assert_eq!(packet[0], TPKT_VERSION);
        assert_eq!(
            u16::from_be_bytes([packet[2], packet[3]]) as usize,
            packet.len()
        );
        assert_eq!(packet[5], X224_CONNECTION_REQUEST);
    }
}
