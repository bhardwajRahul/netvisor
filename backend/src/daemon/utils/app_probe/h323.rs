//! H.323 call-signalling detection with a Q.931 `SETUP`.
//!
//! **This one closes a hole rather than adding a capability.** 1720 is one of the six ports FortiOS
//! ships a session helper for, so a FortiGate answers it on behalf of every address it fronts — the
//! same mechanism that produced the phantom SIP servers this branch started from. A *light* scan
//! never touches it, because the port list is derived from what definitions claim and nothing
//! claimed 1720. A **full** scan sweeps 1..=65535, finds it open, and
//! `enumerated_host_has_evidence` reads an open port no probe covers as proof a host exists. One
//! phantom host per address, on a routed subnet, from a port nothing had a definition for.
//!
//! Defining it is the narrow fix. The general one is the setting in Part B, because 1720 is an
//! instance of an open-ended class: any TCP port a middlebox answers that no definition claims.
//!
//! The exchange is Q.931 over TPKT, the same framing RDP uses, so [`ironrdp_pdu`] supplies the
//! outer layer. A `SETUP` for a call nothing will accept draws `RELEASE COMPLETE` from a gateway
//! with no matching endpoint, `CALL PROCEEDING` or `ALERTING` from one that would take it. **All of
//! them count**: each is a Q.931 response, and only an H.323 stack sends one.
//!
//! Nothing completes a call. The `SETUP` names no destination and carries no user-user information
//! element, so a gateway that would otherwise route it has nothing to route; and the connection is
//! closed the moment the reply arrives, which releases the call reference.
//!
//! Omitting the user-user element is deliberate rather than a shortcut. Its contents would be an
//! H.225 `Setup-UUIE` in ASN.1 PER naming a called party, which is the difference between asking
//! whether an H.323 stack is there and asking it to place a call. A real gatekeeper logs
//! `Setup message ... without associated UUIE` and answers `RELEASE COMPLETE` — which is exactly
//! the reply this wants, so the identifying exchange happens without ever describing a call.

use anyhow::Error;
use async_trait::async_trait;
use ironrdp_core::WriteCursor;
use ironrdp_pdu::tpkt::TpktHeader;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// H.323 call signalling, "h323hostcall".
pub(crate) const H323_PORT: u16 = 1720;

/// Q.931's protocol discriminator, fixed at 8 for call-control messages.
const PROTOCOL_DISCRIMINATOR: u8 = 0x08;

/// Q.931 message types. The high bit of the call reference distinguishes the two directions, so
/// these are the same values in both.
const MSG_SETUP: u8 = 0x05;
const MSG_ALERTING: u8 = 0x01;
const MSG_CALL_PROCEEDING: u8 = 0x02;
const MSG_CONNECT: u8 = 0x07;
const MSG_RELEASE_COMPLETE: u8 = 0x5A;
const MSG_STATUS: u8 = 0x7D;

/// The call reference we choose. Q.931 sets the high bit on messages from the side that did *not*
/// allocate it, so a reply that adopted our call arrives with `0x80` set on the first octet.
const CALL_REFERENCE: u16 = 0x0B0B;
/// The flag the answering side sets on the call reference's first octet.
const CALL_REFERENCE_FROM_DESTINATION: u8 = 0x80;
/// Q.931's global call reference, which a stack uses when it rejects before adopting the call.
///
/// Accepting it is not a loosening for its own sake: a live GNU Gatekeeper answers this probe's
/// `SETUP` with `RELEASE COMPLETE` on call reference 0, and requiring the echo would have rejected
/// a real gatekeeper. Correlation is not what carries the check anyway — the message type is, and
/// this is a reply on a connection opened one round trip earlier.
const CALL_REFERENCE_GLOBAL: u16 = 0;

/// A `RELEASE COMPLETE` is short; a `CONNECT` with an H.225 payload can run to a few hundred octets.
const READ_LIMIT: usize = 1024;

/// TPKT header, then the Q.931 header: discriminator, call-reference length, call reference,
/// message type.
const TPKT_HEADER_LEN: usize = 4;
const Q931_HEADER_LEN: usize = 5;

/// A Q.931 `SETUP` inside a TPKT frame.
fn setup() -> Vec<u8> {
    let mut q931 = vec![PROTOCOL_DISCRIMINATOR, 0x02];
    q931.extend_from_slice(&CALL_REFERENCE.to_be_bytes());
    q931.push(MSG_SETUP);
    // Bearer capability, the one information element Q.931 requires in a SETUP: ITU-T standard
    // coding, unrestricted digital information, 64 kbit/s circuit mode.
    q931.extend_from_slice(&[0x04, 0x03, 0x88, 0x93, 0xA5]);
    // No user-user element. Its contents would be an H.225 SETUP-UUIE in ASN.1 PER, and building
    // one would mean naming a destination — which is the difference between asking whether an
    // H.323 stack is there and asking it to place a call.

    let mut frame = vec![0u8; TPKT_HEADER_LEN];
    let packet_length = u16::try_from(TPKT_HEADER_LEN + q931.len())
        .expect("a fixed SETUP is far shorter than a TPKT frame's limit");
    let mut cursor = WriteCursor::new(&mut frame);
    TpktHeader { packet_length }
        .write(&mut cursor)
        .expect("the header is four octets into a four-octet buffer");
    frame.extend_from_slice(&q931);
    frame
}

pub struct H323Probe;

#[async_trait]
impl AppProbe for H323Probe {
    fn port(&self) -> PortType {
        PortType::new_tcp(H323_PORT)
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::H323)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &setup(), READ_LIMIT).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is a Q.931 message answering our call.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    presence(message_type(bytes).is_some())
}

/// The Q.931 message type the peer answered with, when the reply answers our call reference.
fn message_type(bytes: &[u8]) -> Option<u8> {
    let header = bytes.get(..TPKT_HEADER_LEN + Q931_HEADER_LEN)?;
    // The TPKT length has to account for what arrived; the read is capped, so it may exceed it.
    let declared = u16::from_be_bytes([header[2], header[3]]) as usize;
    if header[0] != 3 || declared < bytes.len() || declared <= TPKT_HEADER_LEN {
        return None;
    }

    let q931 = &header[TPKT_HEADER_LEN..];
    if q931[0] != PROTOCOL_DISCRIMINATOR || q931[1] != 0x02 {
        return None;
    }
    // Either the call reference we chose, echoed with the direction flag, or the global reference a
    // stack uses to reject before adopting the call.
    let reference = u16::from_be_bytes([q931[2] & !CALL_REFERENCE_FROM_DESTINATION, q931[3]]);
    let answered_our_call =
        q931[2] & CALL_REFERENCE_FROM_DESTINATION != 0 && reference == CALL_REFERENCE;
    if !answered_our_call && reference != CALL_REFERENCE_GLOBAL {
        return None;
    }

    let message = q931[4];
    matches!(
        message,
        MSG_ALERTING | MSG_CALL_PROCEEDING | MSG_CONNECT | MSG_RELEASE_COMPLETE | MSG_STATUS
    )
    .then_some(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(message: u8) -> Vec<u8> {
        let mut q931 = vec![PROTOCOL_DISCRIMINATOR, 0x02];
        let reference = CALL_REFERENCE.to_be_bytes();
        q931.push(reference[0] | CALL_REFERENCE_FROM_DESTINATION);
        q931.push(reference[1]);
        q931.push(message);
        // Cause information element: normal call clearing.
        q931.extend_from_slice(&[0x08, 0x02, 0x80, 0x90]);

        let total = (TPKT_HEADER_LEN + q931.len()) as u16;
        let mut frame = vec![0x03, 0x00];
        frame.extend_from_slice(&total.to_be_bytes());
        frame.extend_from_slice(&q931);
        frame
    }

    /// The expected outcome: a gateway with nothing to route the call to.
    #[test]
    fn a_release_complete_is_h323() {
        assert_eq!(
            parse_reply(&response(MSG_RELEASE_COMPLETE)),
            AppProbeOutcome::Answered { identity: None }
        );
        assert_eq!(
            message_type(&response(MSG_RELEASE_COMPLETE)),
            Some(MSG_RELEASE_COMPLETE)
        );
    }

    /// What a live GNU Gatekeeper in routed mode actually sends this probe: a `RELEASE COMPLETE`
    /// on the *global* call reference rather than an echo of ours, because it rejects the `SETUP`
    /// before adopting the call. Requiring the echo rejected a real gatekeeper.
    #[test]
    fn a_rejection_on_the_global_call_reference_is_h323() {
        let mut q931 = vec![
            PROTOCOL_DISCRIMINATOR,
            0x02,
            0x00,
            0x00,
            MSG_RELEASE_COMPLETE,
        ];
        // The user-user element the gatekeeper attaches to say why.
        q931.extend_from_slice(&[
            0x7E, 0x00, 0x0E, 0x05, 0x25, 0x40, 0x06, 0x00, 0x08, 0x91, 0x4A, 0x00, 0x02, 0x58,
            0x14, 0x01, 0x00,
        ]);
        let total = (TPKT_HEADER_LEN + q931.len()) as u16;
        let mut frame = vec![0x03, 0x00];
        frame.extend_from_slice(&total.to_be_bytes());
        frame.extend_from_slice(&q931);

        assert_eq!(
            parse_reply(&frame),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// An endpoint that would take the call answers just as identifiably.
    #[test]
    fn a_call_proceeding_or_alerting_is_h323() {
        for message in [MSG_CALL_PROCEEDING, MSG_ALERTING, MSG_CONNECT, MSG_STATUS] {
            assert_eq!(
                parse_reply(&response(message)),
                AppProbeOutcome::Answered { identity: None },
                "{message:#04x}"
            );
        }
    }

    /// The case this definition exists for. A FortiGate session helper on 1720 completes the TCP
    /// handshake and sends nothing, which before this was an open port no definition claimed — and
    /// on a full scan of a routed subnet, that alone manufactured a host.
    #[test]
    fn a_listener_that_completes_the_handshake_and_says_nothing_is_not_h323() {
        assert_eq!(parse_reply(&[]), AppProbeOutcome::NoAnswer);
    }

    #[test]
    fn another_protocol_or_our_own_setup_is_not_h323() {
        let full = response(MSG_RELEASE_COMPLETE);
        for reply in [
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // Our own SETUP reflected back: TPKT-framed Q.931, but the call reference has no
            // direction flag, so it is not an answer to us.
            &setup(),
            // A reply to somebody else's call.
            &{
                let mut other = full.clone();
                other[6] = 0x80 | 0x77;
                other[7] = 0x77;
                other
            }[..],
            // TPKT framing around a message type Q.931 does not define.
            &response(0x33)[..],
            // A declared length shorter than what arrived.
            &{
                let mut short = full.clone();
                short[2..4].copy_from_slice(&5u16.to_be_bytes());
                short
            }[..],
            &full[..TPKT_HEADER_LEN + 2],
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }

    /// The SETUP names no destination, so there is nothing for a gateway to route.
    #[test]
    fn the_setup_is_tpkt_framed_and_names_no_destination() {
        let frame = setup();
        assert_eq!(frame[0], 3, "TPKT version 3");
        assert_eq!(
            u16::from_be_bytes([frame[2], frame[3]]) as usize,
            frame.len()
        );
        assert_eq!(frame[TPKT_HEADER_LEN], PROTOCOL_DISCRIMINATOR);
        assert_eq!(frame[TPKT_HEADER_LEN + 4], MSG_SETUP);
        // Information element 0x7E is user-user, which is where an H.225 SETUP-UUIE with a called
        // party would go. Nothing here carries one.
        assert!(
            !frame[TPKT_HEADER_LEN + Q931_HEADER_LEN..].contains(&0x7E),
            "no user-user information element"
        );
    }
}
