//! PostgreSQL detection with an `SSLRequest`, the one message a server answers before authenticating.
//!
//! Eight bytes: a length and the magic number 80877103. The server replies with a single byte —
//! `S` if it will negotiate TLS, `N` if it will not — and it does so before any startup packet,
//! any username and any password. Nothing else in the protocol is reachable unauthenticated, and
//! nothing else is this cheap.
//!
//! The request comes from [`postgres_protocol`] rather than a hand-written constant, so the magic
//! number is the library's business. The `S`/`N` replies stay a byte comparison because that is
//! genuinely all they are — they are not framed messages and no parser applies. The third reply, an
//! `ErrorResponse` from a server too old to know `SSLRequest`, *is* a framed message, and it is now
//! decoded as one instead of being accepted for starting with `E`.
//!
//! `postgres-protocol` rather than sqlx, which is already in the tree: sqlx implements its own wire
//! layer inside a connection type that authenticates. This crate is the codec on its own.

use anyhow::Error;
use async_trait::async_trait;
use bytes::BytesMut;
use postgres_protocol::message::backend::Message;
use postgres_protocol::message::frontend;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The server answers `S` or `N` in one byte; an `ErrorResponse` runs to a few hundred.
const READ_LIMIT: usize = 512;

fn ssl_request() -> Vec<u8> {
    let mut buf = BytesMut::new();
    frontend::ssl_request(&mut buf);
    buf.to_vec()
}

pub struct PostgresProbe;

#[async_trait]
impl AppProbe for PostgresProbe {
    fn port(&self) -> PortType {
        PortType::PostgreSQL
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::PostgreSql)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &ssl_request(), READ_LIMIT).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is a PostgreSQL `SSLRequest` response.
///
/// Exactly one byte is expected for the negotiation answer, which is what keeps this from matching
/// any stream that happens to open with an `N`. A server predating `SSLRequest` answers with an
/// `ErrorResponse` instead, and that is still PostgreSQL — but it has to decode as one.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    presence(matches!(bytes, [b'S'] | [b'N']) || is_error_response(bytes))
}

fn is_error_response(bytes: &[u8]) -> bool {
    let mut buf = BytesMut::from(bytes);
    matches!(
        Message::parse(&mut buf),
        Ok(Some(Message::ErrorResponse(_)))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_byte_negotiation_answer_is_postgres() {
        assert_eq!(
            parse_reply(b"S"),
            AppProbeOutcome::Answered { identity: None }
        );
        assert_eq!(
            parse_reply(b"N"),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A server predating `SSLRequest` answers with an ErrorResponse, which is still PostgreSQL.
    #[test]
    fn an_error_response_is_postgres() {
        // Tag, length (self-inclusive), one `S`everity field, then the terminating NUL.
        let mut packet = vec![b'E'];
        let body = b"SFATAL\0\0";
        packet.extend_from_slice(&((body.len() + 4) as u32).to_be_bytes());
        packet.extend_from_slice(body);
        assert_eq!(
            parse_reply(&packet),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_or_a_longer_reply_is_not_postgres() {
        for reply in [
            &b""[..],
            // A stream that merely opens with N is not a one-byte negotiation answer.
            &b"NOT POSTGRES"[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"\x00\x00\x00\x08"[..],
            // Opens with the ErrorResponse tag, but the length does not frame a message. The
            // `starts_with(b"E")` check this replaced accepted it.
            &b"ERROR: not postgres"[..],
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }

    #[test]
    fn the_request_is_the_ssl_negotiation_packet() {
        // Length 8 then request code 80877103, per the protocol. Asserted against the wire format
        // rather than against a copy of the constant, since the constant is now the library's.
        let request = ssl_request();
        assert_eq!(u32::from_be_bytes(request[0..4].try_into().unwrap()), 8);
        assert_eq!(
            u32::from_be_bytes(request[4..8].try_into().unwrap()),
            80_877_103
        );
    }
}
