//! MySQL and MariaDB detection by the handshake packet the server sends first.
//!
//! The server opens the conversation with either an initial handshake or an error packet. The error
//! is worth accepting as readily as the handshake, because the most common one is `Host '…' is not
//! allowed to connect to this MySQL server` — a server refusing our address by name, which
//! identifies it beyond doubt. So does `Too many connections`.
//!
//! [`mysql_common`] supplies both halves: `PacketCodec` reads the 4-byte header, checks the sequence
//! id and reassembles a payload split across packets, and `HandshakePacket` / `ErrPacket` decode the
//! payload. The check this replaced compared two bytes at fixed offsets and a declared length, which
//! accepted any stream opening with a plausible header followed by `0x0A`.
//!
//! `mysql_common` is declared `default-features = false` and that is load bearing rather than tidy:
//! its default features pull `flate2/zlib`, the **C** zlib backend via `libz-sys`, which on the
//! static musl release build is a link failure. There is no `libz-sys` in this tree and this must
//! not be what introduces one.

use anyhow::Error;
use async_trait::async_trait;
use bytes::BytesMut;
use mysql_common::constants::CapabilityFlags;
use mysql_common::io::ParseBuf;
use mysql_common::packets::{ErrPacket, HandshakePacket};
use mysql_common::proto::MyDeserialize;
use mysql_common::proto::codec::PacketCodec;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, read_greeting,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// A handshake with a long plugin-auth section still fits comfortably.
const READ_LIMIT: usize = 1024;

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
            &read_greeting(ctx, self.port(), READ_LIMIT).await,
        ))
    }
}

/// What the server said first.
#[derive(Debug, PartialEq, Eq)]
enum Greeting {
    /// The initial handshake, carrying the server's version string.
    Handshake { server_version: String },
    /// An error packet, carrying the code that says why. A refusal is still a MySQL server.
    Refusal { code: u16 },
}

/// Whether the opening bytes are a MySQL packet.
fn parse_handshake(bytes: &[u8]) -> AppProbeOutcome {
    presence(decode_greeting(bytes).is_some())
}

fn decode_greeting(bytes: &[u8]) -> Option<Greeting> {
    // The codec owns the framing: the little-endian payload length, the sequence id (0, since the
    // server speaks first), and reassembly if the payload spans packets.
    let mut src = BytesMut::from(bytes);
    let mut payload = Vec::new();
    if !PacketCodec::default()
        .decode(&mut src, &mut payload)
        .ok()
        .unwrap_or(false)
    {
        return None;
    }

    if let Ok(handshake) = HandshakePacket::deserialize((), &mut ParseBuf(&payload)) {
        return Some(Greeting::Handshake {
            server_version: String::from_utf8_lossy(handshake.server_version_ref()).into_owned(),
        });
    }

    // No capabilities have been negotiated at this point, which is exactly the context an error
    // packet arriving before the handshake is sent in.
    match ErrPacket::deserialize(CapabilityFlags::empty(), &mut ParseBuf(&payload)) {
        Ok(ErrPacket::Error(error)) => Some(Greeting::Refusal {
            code: error.error_code(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mysql_common::constants::StatusFlags;
    use mysql_common::proto::MySerialize;

    fn packet(payload: &[u8]) -> Vec<u8> {
        let len = payload.len();
        let mut out = vec![len as u8, (len >> 8) as u8, (len >> 16) as u8, 0x00];
        out.extend_from_slice(payload);
        out
    }

    /// A handshake as a server sends it, built with the same crate that reads it — a hand-laid
    /// payload is exactly the thing this refactor removed, and getting it subtly wrong here would
    /// test the fixture rather than the probe.
    fn handshake_payload() -> Vec<u8> {
        let packet = HandshakePacket::new(
            0x0A,
            &b"8.0.36"[..],
            1,
            *b"12345678",
            Some(&b"9abcdefghijk"[..]),
            CapabilityFlags::CLIENT_PROTOCOL_41 | CapabilityFlags::CLIENT_PLUGIN_AUTH,
            0x21,
            StatusFlags::SERVER_STATUS_AUTOCOMMIT,
            Some(&b"mysql_native_password"[..]),
        );
        let mut out = Vec::new();
        packet.serialize(&mut out);
        out
    }

    #[test]
    fn a_handshake_packet_is_mysql_and_names_its_version() {
        let bytes = packet(&handshake_payload());
        assert_eq!(
            parse_handshake(&bytes),
            AppProbeOutcome::Answered { identity: None }
        );
        assert_eq!(
            decode_greeting(&bytes),
            Some(Greeting::Handshake {
                server_version: "8.0.36".to_owned()
            })
        );
    }

    /// The refusal that identifies a server as surely as the handshake does — and the case a client
    /// crate would have collapsed into an opaque connection error.
    #[test]
    fn a_host_not_allowed_error_is_mysql() {
        // 0xFF, error code 1130 (ER_HOST_NOT_PRIVILEGED), then the message.
        let mut payload = vec![0xFF];
        payload.extend_from_slice(&1130u16.to_le_bytes());
        payload.extend_from_slice(b"Host '10.1.2.3' is not allowed to connect");
        let bytes = packet(&payload);
        assert_eq!(
            parse_handshake(&bytes),
            AppProbeOutcome::Answered { identity: None }
        );
        assert_eq!(
            decode_greeting(&bytes),
            Some(Greeting::Refusal { code: 1130 })
        );
    }

    #[test]
    fn silence_or_another_protocol_is_not_mysql() {
        let full = packet(&handshake_payload());
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"\x0B\x00\x00\x00"[..],
            // A header whose sequence id is not 0, so it is not the first packet of an exchange.
            &{
                let mut wrong = full.clone();
                wrong[3] = 7;
                wrong
            }[..],
            // A well-formed header followed by a byte that is neither a handshake nor an error.
            &packet(b"\x99not mysql")[..],
            // Truncated mid-payload: the length check this replaced accepted it.
            &full[..full.len() - 12],
        ] {
            assert_eq!(
                parse_handshake(reply),
                AppProbeOutcome::NoAnswer,
                "{reply:?}"
            );
        }
    }
}
