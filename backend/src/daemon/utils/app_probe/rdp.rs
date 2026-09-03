//! Remote Desktop detection with an X.224 connection request.
//!
//! RDP rides on TPKT and X.224: the client sends a Connection Request TPDU and the server answers a
//! Connection Confirm, all before any TLS, CredSSP or credentials. A server that refuses our
//! requested security protocols answers a negotiation *failure* inside the same Connection Confirm,
//! which identifies it as readily as a success.
//!
//! The three layers — TPKT framing, the X.224 TPDU header, the RDP negotiation payload — were hand-
//! stacked here. [`ironrdp_pdu`] models all three, so `X224<ConnectionConfirm>` decodes the reply in
//! one step and checks every layer while doing it: the TPKT length, the TPDU code, the length
//! indicator against the fixed header size, and the negotiation message type. The old check read
//! two bytes at fixed offsets and compared a declared length.
//!
//! Decoding also yields something the byte check could not: the reply comes back as a typed
//! `ConnectionConfirm`, naming *which* security protocols the server accepts or which failure code
//! it returned. Nothing reads that today — the service matches on the probe having answered — so it
//! is asserted in the tests rather than exposed as an unused accessor.
//!
//! The judgement stays here rather than in the crate: **a `Failure` counts as a positive.** That is
//! the whole point of the probe and exactly what a session crate would have collapsed into an error.

use anyhow::Error;
use async_trait::async_trait;
use ironrdp_core::{decode, encode_vec};
use ironrdp_pdu::nego::{ConnectionConfirm, ConnectionRequest, RequestFlags, SecurityProtocol};
use ironrdp_pdu::x224::X224;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// A Connection Request carrying an RDP negotiation request.
///
/// Requesting nothing but standard RDP security keeps this from depending on TLS support, and no
/// cookie or routing token is sent: both carry a username, and this probe has none and wants none.
fn connection_request() -> Vec<u8> {
    let request = X224(ConnectionRequest {
        nego_data: None,
        flags: RequestFlags::empty(),
        // Standard RDP security, which is the empty bitmask — there is no PROTOCOL_RDP flag to
        // set. Asking for nothing enhanced keeps this from depending on TLS support.
        protocol: SecurityProtocol::empty(),
    });
    encode_vec(&request).expect("the request is built from constants and has a fixed size")
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
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    presence(confirm(bytes).is_some())
}

/// The Connection Confirm a server sent, if it sent one.
fn confirm(bytes: &[u8]) -> Option<ConnectionConfirm> {
    decode::<X224<ConnectionConfirm>>(bytes).ok().map(|x| x.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironrdp_pdu::nego::{FailureCode, ResponseFlags};

    fn encoded(confirm: ConnectionConfirm) -> Vec<u8> {
        encode_vec(&X224(confirm)).unwrap()
    }

    #[test]
    fn a_connection_confirm_is_rdp_and_names_what_it_will_accept() {
        let reply = encoded(ConnectionConfirm::Response {
            flags: ResponseFlags::empty(),
            protocol: SecurityProtocol::SSL | SecurityProtocol::HYBRID,
        });
        assert_eq!(
            parse_reply(&reply),
            AppProbeOutcome::Answered { identity: None }
        );

        let ConnectionConfirm::Response { protocol, .. } = confirm(&reply).unwrap() else {
            panic!("the server accepted the negotiation");
        };
        assert!(protocol.contains(SecurityProtocol::HYBRID), "NLA offered");
    }

    /// A server refusing our security protocols answers a negotiation failure, inside a Connection
    /// Confirm. Still RDP — and this is the case a client crate would have handed back as an error.
    #[test]
    fn a_negotiation_failure_is_rdp() {
        let reply = encoded(ConnectionConfirm::Failure {
            code: FailureCode::SSL_REQUIRED_BY_SERVER,
        });
        assert_eq!(
            parse_reply(&reply),
            AppProbeOutcome::Answered { identity: None }
        );
        assert!(
            matches!(confirm(&reply), Some(ConnectionConfirm::Failure { .. })),
            "a refusal decodes as a failure, not as a protocol list"
        );
    }

    #[test]
    fn silence_or_another_protocol_is_not_rdp() {
        let request = connection_request();
        let confirm_bytes = encoded(ConnectionConfirm::Failure {
            code: FailureCode::SSL_REQUIRED_BY_SERVER,
        });
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // TPKT version and a length, and nothing after it.
            &[0x03, 0x00, 0x00, 0x0B][..],
            // Our own request reflected back: TPKT-framed X.224, but a request, not a confirm.
            &request,
            // A truncated confirm, which the offset-based check this replaced accepted.
            &confirm_bytes[..confirm_bytes.len() - 4],
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }

    #[test]
    fn the_connection_request_is_tpkt_framed_and_carries_no_username() {
        let packet = connection_request();
        assert_eq!(packet[0], 0x03, "TPKT version 3");
        assert_eq!(
            u16::from_be_bytes([packet[2], packet[3]]) as usize,
            packet.len(),
            "the TPKT length covers the whole packet"
        );
        let decoded = decode::<X224<ConnectionRequest>>(&packet).expect("we send a valid request");
        assert!(
            decoded.0.nego_data.is_none(),
            "no cookie or routing token, both of which carry a username"
        );
    }
}
