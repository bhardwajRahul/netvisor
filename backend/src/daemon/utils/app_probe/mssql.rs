//! Microsoft SQL Server detection with a TDS `PRELOGIN`.
//!
//! `PRELOGIN` is the first message of every TDS conversation and precedes both TLS negotiation and
//! the login packet, so it is answered without credentials and without a certificate. The reply is
//! a TDS packet of type `0x04`, and its declared length is what separates a real response from a
//! stream that happens to open with that byte.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Packet type: a tabular response, which is what `PRELOGIN` is answered with.
const TYPE_TABULAR_RESPONSE: u8 = 0x04;
/// Packet type: `PRELOGIN`.
const TYPE_PRELOGIN: u8 = 0x12;
/// Status flag: end of message.
const STATUS_EOM: u8 = 0x01;
/// Type, status, length (2), SPID (2), packet id, window.
const TDS_HEADER_LEN: usize = 8;

/// A `PRELOGIN` carrying only the VERSION option.
///
/// The option table is a list of (token, offset, length) triples terminated by `0xFF`, with the
/// data following. One option is enough: the server answers the whole handshake regardless of how
/// much we asked about.
fn prelogin_packet() -> Vec<u8> {
    // Token table: VERSION at offset 6 (just past the table), 6 bytes long, then the terminator.
    let mut payload = vec![0x00, 0x00, 0x06, 0x00, 0x06, 0xFF];
    // VERSION data: four version bytes and a two-byte build number, all zero — the server does not
    // care what a client claims here.
    payload.extend_from_slice(&[0x00; 6]);

    let total = (TDS_HEADER_LEN + payload.len()) as u16;
    let mut packet = vec![TYPE_PRELOGIN, STATUS_EOM];
    packet.extend_from_slice(&total.to_be_bytes());
    packet.extend_from_slice(&[0x00, 0x00]); // SPID
    packet.push(0x00); // packet id
    packet.push(0x00); // window
    packet.extend_from_slice(&payload);
    packet
}

pub struct MsSqlProbe;

#[async_trait]
impl AppProbe for MsSqlProbe {
    fn port(&self) -> PortType {
        PortType::MsSql
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::MsSql)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &prelogin_packet(), 512).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is a TDS response packet.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    let Some(header) = bytes.get(..TDS_HEADER_LEN) else {
        return AppProbeOutcome::NoAnswer;
    };
    let declared = u16::from_be_bytes([header[2], header[3]]) as usize;

    // The length counts the header, so it can never be smaller than one, and it has to account for
    // everything that arrived.
    presence(
        header[0] == TYPE_TABULAR_RESPONSE && declared >= TDS_HEADER_LEN && declared >= bytes.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(payload: &[u8]) -> Vec<u8> {
        let total = (TDS_HEADER_LEN + payload.len()) as u16;
        let mut packet = vec![TYPE_TABULAR_RESPONSE, STATUS_EOM];
        packet.extend_from_slice(&total.to_be_bytes());
        packet.extend_from_slice(&[0, 0, 0, 0]);
        packet.extend_from_slice(payload);
        packet
    }

    #[test]
    fn a_prelogin_response_is_mssql() {
        assert_eq!(
            parse_reply(&response(&[
                0x00, 0x00, 0x1F, 0x00, 0x06, 0xFF, 16, 0, 0, 0, 0, 0
            ])),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_or_a_disagreeing_length_is_not_mssql() {
        let mut trailing = response(&[0x00; 6]);
        trailing.extend_from_slice(b"unaccounted for");
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // Right type byte, length shorter than the header itself.
            &[TYPE_TABULAR_RESPONSE, 0x01, 0x00, 0x02, 0, 0, 0, 0][..],
            &trailing,
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }

    #[test]
    fn the_prelogin_packet_declares_its_own_length() {
        let packet = prelogin_packet();
        assert_eq!(packet[0], TYPE_PRELOGIN);
        assert_eq!(
            u16::from_be_bytes([packet[2], packet[3]]) as usize,
            packet.len(),
            "the length field counts the header as well as the payload"
        );
        assert_eq!(packet[TDS_HEADER_LEN + 5], 0xFF, "option table terminator");
    }
}
