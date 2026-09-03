//! SMB detection with an SMB2 NEGOTIATE.
//!
//! NEGOTIATE is the first message of an SMB2 conversation and precedes SESSION_SETUP, so it needs no
//! credentials. The reply carries the protocol's own magic — `0xFE S M B` for SMB2 and 3, `0xFF S M
//! B` for the SMB1 servers that still answer — and both are accepted, because the definition claims
//! a file server rather than a dialect.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// SMB2/3 protocol id.
const SMB2_MAGIC: [u8; 4] = [0xFE, b'S', b'M', b'B'];
/// SMB1 protocol id, still sent by servers that answer an SMB2 negotiate with a downgrade.
const SMB1_MAGIC: [u8; 4] = [0xFF, b'S', b'M', b'B'];
/// A NetBIOS session-service header: message type and a 3-byte length.
const NETBIOS_HEADER_LEN: usize = 4;

/// An SMB2 NEGOTIATE offering the two dialects every server understands.
fn negotiate_request() -> Vec<u8> {
    let mut header = vec![0u8; 64];
    header[..4].copy_from_slice(&SMB2_MAGIC);
    header[4] = 64; // StructureSize, fixed by the specification
    header[14] = 1; // CreditRequest
    // Command (offset 12) is 0 for NEGOTIATE, and everything else stays zero.

    let mut body = vec![0u8; 36];
    body[0] = 36; // StructureSize, fixed
    body[2] = 2; // DialectCount
    body[4] = 1; // SecurityMode: signing enabled
    // ClientGuid stays zero: servers do not require a real one to negotiate.

    let mut message = header;
    message.extend_from_slice(&body);
    message.extend_from_slice(&0x0202u16.to_le_bytes()); // SMB 2.0.2
    message.extend_from_slice(&0x0210u16.to_le_bytes()); // SMB 2.1

    let length = message.len();
    let mut packet = vec![
        0x00,
        (length >> 16) as u8,
        (length >> 8) as u8,
        length as u8,
    ];
    packet.extend_from_slice(&message);
    packet
}

pub struct SmbProbe;

#[async_trait]
impl AppProbe for SmbProbe {
    fn port(&self) -> PortType {
        PortType::Samba
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Smb)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &negotiate_request(), 1024).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply carries an SMB protocol id after its NetBIOS header.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    let Some(magic) = bytes.get(NETBIOS_HEADER_LEN..NETBIOS_HEADER_LEN + 4) else {
        return AppProbeOutcome::NoAnswer;
    };
    presence(magic == SMB2_MAGIC || magic == SMB1_MAGIC)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed(magic: [u8; 4]) -> Vec<u8> {
        let mut packet = vec![0x00, 0x00, 0x00, 0x40];
        packet.extend_from_slice(&magic);
        packet.extend_from_slice(&[0u8; 60]);
        packet
    }

    #[test]
    fn an_smb2_negotiate_response_is_smb() {
        assert_eq!(
            parse_reply(&framed(SMB2_MAGIC)),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A server that only speaks SMB1 answers with its own protocol id, and is still a file server.
    #[test]
    fn an_smb1_response_is_smb() {
        assert_eq!(
            parse_reply(&framed(SMB1_MAGIC)),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_or_another_protocol_is_not_smb() {
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // The magic has to sit after the NetBIOS header, not at the very start.
            &[0xFE, b'S', b'M', b'B', 0, 0, 0, 0][..],
            &[0x00, 0x00, 0x00, 0x40][..],
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }

    #[test]
    fn the_negotiate_request_declares_its_length_and_offers_dialects() {
        let packet = negotiate_request();
        let declared =
            usize::from(packet[1]) << 16 | usize::from(packet[2]) << 8 | usize::from(packet[3]);
        assert_eq!(declared, packet.len() - NETBIOS_HEADER_LEN);
        assert_eq!(&packet[4..8], &SMB2_MAGIC);
        assert_eq!(
            packet[NETBIOS_HEADER_LEN + 64 + 2],
            2,
            "two dialects offered"
        );
    }
}
