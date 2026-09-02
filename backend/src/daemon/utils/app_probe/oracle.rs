//! Oracle Database detection with a TNS connect packet.
//!
//! The listener answers a `CONNECT` before any credentials: it either accepts, redirects to the
//! port a dedicated server is on, or refuses. The refusals are the common case against a probe like
//! this one and they identify the listener just as well — `REFUSE` carries an ORA error, and a
//! listener answering `ORA-12514` has parsed our connect string to tell us the service name is
//! unknown.
//!
//! What is checked is the TNS framing: a packet length that agrees with what arrived and a packet
//! type the listener actually sends.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Packet types a listener sends back.
const TYPE_ACCEPT: u8 = 0x02;
const TYPE_REFUSE: u8 = 0x04;
const TYPE_REDIRECT: u8 = 0x05;
const TYPE_RESEND: u8 = 0x0B;
/// Packet type we send.
const TYPE_CONNECT: u8 = 0x01;

/// Length (2), checksum (2), type, reserved, header checksum (2).
const TNS_HEADER_LEN: usize = 8;

/// A `CONNECT` carrying a minimal connect descriptor.
///
/// The service name is deliberately one nothing will be listening for: a listener that refuses it
/// answers just as usefully as one that accepts, and asking for a real service we do not know the
/// name of would be no more informative.
fn connect_packet() -> Vec<u8> {
    let descriptor = b"(DESCRIPTION=(CONNECT_DATA=(SERVICE_NAME=)(CID=(PROGRAM=)(HOST=)(USER=))))";

    // Connect data begins straight after the 26-byte CONNECT body that follows the header.
    let offset = (TNS_HEADER_LEN + 26) as u16;
    let mut body = Vec::new();
    body.extend_from_slice(&0x0136u16.to_be_bytes()); // version
    body.extend_from_slice(&0x012Cu16.to_be_bytes()); // minimum compatible version
    body.extend_from_slice(&0x0000u16.to_be_bytes()); // service options
    body.extend_from_slice(&0x0800u16.to_be_bytes()); // session data unit size
    body.extend_from_slice(&0x7FFFu16.to_be_bytes()); // maximum transmission data unit
    body.extend_from_slice(&0x4F98u16.to_be_bytes()); // protocol characteristics
    body.extend_from_slice(&0x0000u16.to_be_bytes()); // line turnaround
    body.extend_from_slice(&0x0001u16.to_be_bytes()); // value of 1 in hardware
    body.extend_from_slice(&(descriptor.len() as u16).to_be_bytes());
    body.extend_from_slice(&offset.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes()); // maximum receivable connect data
    body.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // flags and trailing fields

    let total = (TNS_HEADER_LEN + body.len() + descriptor.len()) as u16;
    let mut packet = Vec::with_capacity(total as usize);
    packet.extend_from_slice(&total.to_be_bytes());
    packet.extend_from_slice(&0u16.to_be_bytes()); // packet checksum, unused
    packet.push(TYPE_CONNECT);
    packet.push(0x00); // reserved
    packet.extend_from_slice(&0u16.to_be_bytes()); // header checksum, unused
    packet.extend_from_slice(&body);
    packet.extend_from_slice(descriptor);
    packet
}

pub struct OracleProbe;

#[async_trait]
impl AppProbe for OracleProbe {
    fn port(&self) -> PortType {
        PortType::new_tcp(1521)
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::OracleTns)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &connect_packet(), 1024).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is a TNS packet of a type a listener sends.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    let Some(header) = bytes.get(..TNS_HEADER_LEN) else {
        return AppProbeOutcome::NoAnswer;
    };
    let declared = u16::from_be_bytes([header[0], header[1]]) as usize;
    let packet_type = header[4];

    presence(
        declared >= TNS_HEADER_LEN
            && declared >= bytes.len()
            && matches!(
                packet_type,
                TYPE_ACCEPT | TYPE_REFUSE | TYPE_REDIRECT | TYPE_RESEND
            ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(packet_type: u8, body: &[u8]) -> Vec<u8> {
        let total = (TNS_HEADER_LEN + body.len()) as u16;
        let mut packet = total.to_be_bytes().to_vec();
        packet.extend_from_slice(&[0, 0]);
        packet.push(packet_type);
        packet.push(0x00);
        packet.extend_from_slice(&[0, 0]);
        packet.extend_from_slice(body);
        packet
    }

    #[test]
    fn an_accept_is_oracle() {
        assert_eq!(
            parse_reply(&reply(TYPE_ACCEPT, &[0x01, 0x36, 0x00, 0x00])),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// The common answer to a probe: the listener parsed our descriptor and refused the service
    /// name.
    #[test]
    fn a_refusal_is_oracle() {
        assert_eq!(
            parse_reply(&reply(TYPE_REFUSE, b"(ERROR=(CODE=12514))")),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn a_redirect_or_resend_is_oracle() {
        for packet_type in [TYPE_REDIRECT, TYPE_RESEND] {
            assert_eq!(
                parse_reply(&reply(packet_type, b"")),
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    #[test]
    fn silence_an_echo_or_another_protocol_is_not_oracle() {
        let mut trailing = reply(TYPE_ACCEPT, b"");
        trailing.extend_from_slice(b"unaccounted for");
        for bytes in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // Our own CONNECT reflected back is not a listener's reply.
            &reply(TYPE_CONNECT, b""),
            &trailing,
        ] {
            assert_eq!(parse_reply(bytes), AppProbeOutcome::NoAnswer);
        }
    }

    #[test]
    fn the_connect_packet_declares_its_length_and_carries_a_descriptor() {
        let packet = connect_packet();
        let declared = u16::from_be_bytes([packet[0], packet[1]]) as usize;
        assert_eq!(declared, packet.len());
        assert_eq!(packet[4], TYPE_CONNECT);
        assert!(
            packet.windows(12).any(|w| w == b"(DESCRIPTION"),
            "the connect descriptor follows the body"
        );
    }
}
