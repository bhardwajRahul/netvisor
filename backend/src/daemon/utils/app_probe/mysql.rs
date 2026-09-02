//! MySQL and MariaDB detection by the handshake packet the server sends first.
//!
//! The server opens the conversation: a 4-byte packet header, then either protocol version `10`
//! (the initial handshake) or `0xFF` (an error packet). The error is worth accepting as readily as
//! the handshake, because the most common one is `Host '…' is not allowed to connect to this MySQL
//! server` — a server refusing our address by name, which identifies it beyond doubt. So does
//! `Too many connections`.
//!
//! The packet header is what makes this a protocol match rather than a coincidence: the first three
//! bytes are a little-endian payload length that has to agree with how much followed, and the
//! fourth is a sequence number that is 0 on the first packet of an exchange.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, read_greeting,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The only protocol version any current server sends.
const PROTOCOL_VERSION_10: u8 = 0x0A;
/// First byte of an error packet.
const ERROR_PACKET: u8 = 0xFF;
/// Payload length (3) plus sequence id (1).
const HEADER_LEN: usize = 4;

pub struct MySqlProbe;

#[async_trait]
impl AppProbe for MySqlProbe {
    fn port(&self) -> PortType {
        PortType::MySql
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::MySql)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(parse_handshake(
            &read_greeting(ctx, self.port(), 1024).await,
        ))
    }
}

/// Whether the opening bytes are a MySQL packet.
fn parse_handshake(bytes: &[u8]) -> AppProbeOutcome {
    let Some(header) = bytes.get(..HEADER_LEN) else {
        return AppProbeOutcome::NoAnswer;
    };

    let payload_len = u32::from(header[0]) | u32::from(header[1]) << 8 | u32::from(header[2]) << 16;
    let sequence = header[3];

    // The server speaks first, so this is packet 0 of the exchange. A non-empty payload whose
    // declared length is not longer than what arrived: the read is capped, so a large handshake can
    // legitimately be truncated, but a length *shorter* than the bytes received is not this
    // protocol.
    let framed =
        sequence == 0 && payload_len > 0 && payload_len as usize >= bytes.len() - HEADER_LEN;

    let kind = bytes.get(HEADER_LEN);
    presence(framed && matches!(kind, Some(&PROTOCOL_VERSION_10) | Some(&ERROR_PACKET)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real 8.0 handshake: version 10, then a NUL-terminated server version string.
    #[test]
    fn a_handshake_packet_is_mysql() {
        let mut packet = vec![0x0B, 0x00, 0x00, 0x00, PROTOCOL_VERSION_10];
        packet.extend_from_slice(b"8.0.36\0\0\0\0");
        assert_eq!(
            parse_handshake(&packet),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// The refusal that identifies a server as surely as the handshake does.
    #[test]
    fn a_host_not_allowed_error_is_mysql() {
        let body = b"\xFF\x6A\x04Host '10.1.2.3' is not allowed to connect";
        let mut packet = vec![body.len() as u8, 0x00, 0x00, 0x00];
        packet.extend_from_slice(body);
        assert_eq!(
            parse_handshake(&packet),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_or_an_unframed_stream_is_not_mysql() {
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"\x00\x00\x00"[..],
            // Correct framing, wrong packet type.
            &[0x05, 0x00, 0x00, 0x00, 0x42, 0, 0, 0, 0][..],
            // Right first byte, but the sequence number says this is mid-exchange.
            &[0x05, 0x00, 0x00, 0x07, PROTOCOL_VERSION_10, 0, 0, 0, 0][..],
            // A declared length shorter than what arrived is not this framing.
            &[
                0x01,
                0x00,
                0x00,
                0x00,
                PROTOCOL_VERSION_10,
                0,
                0,
                0,
                0,
                0,
                0,
            ][..],
        ] {
            assert_eq!(
                parse_handshake(reply),
                AppProbeOutcome::NoAnswer,
                "{reply:?}"
            );
        }
    }
}
