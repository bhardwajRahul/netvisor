//! PostgreSQL detection with an `SSLRequest`, the one message a server answers before authenticating.
//!
//! Eight bytes: a length and the magic number 80877103. The server replies with a single byte —
//! `S` if it will negotiate TLS, `N` if it will not — and it does so before any startup packet,
//! any username and any password. Nothing else in the protocol is reachable unauthenticated, and
//! nothing else is this cheap.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// `SSLRequest`: length 8, then the request code 80877103 (`0x04D2162F`), both big-endian.
const SSL_REQUEST: [u8; 8] = [0x00, 0x00, 0x00, 0x08, 0x04, 0xD2, 0x16, 0x2F];

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
        let reply = request_response(ctx, self.port(), &SSL_REQUEST, 8).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is a PostgreSQL `SSLRequest` response.
///
/// Exactly one byte is expected. `E` is included because a server too old to know `SSLRequest`
/// answers with an ErrorResponse instead, and that is still PostgreSQL. The length check is what
/// keeps this from matching any stream that happens to open with an `N`.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    presence(matches!(bytes, [b'S'] | [b'N'] | [b'E', ..]))
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
        assert_eq!(
            parse_reply(b"E\0\0\0\x16SFATAL"),
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
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer);
        }
    }
}
