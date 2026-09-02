//! Redis detection with `PING`, including the refusals.
//!
//! RESP is a text protocol and `PING` is its cheapest command, but the interesting answers are the
//! errors. A Redis with `requirepass` set replies `-NOAUTH`, and one in protected mode replies
//! `-DENIED`; both parsed the command to say so, which is what identifies the server. Treating only
//! `+PONG` as Redis would miss every instance with a password on it.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Inline command syntax, which every Redis accepts on a fresh connection.
const PING: &[u8] = b"PING\r\n";

pub struct RedisProbe;

#[async_trait]
impl AppProbe for RedisProbe {
    fn port(&self) -> PortType {
        PortType::Redis
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Redis)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), PING, 256).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply to `PING` is RESP.
///
/// The three answers a `PING` can legitimately draw, and no more. Accepting any RESP error (`-`)
/// would be looser than needed and would match anything that happens to start with a hyphen; these
/// are the errors Redis itself defines for this situation.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    presence(
        bytes.starts_with(b"+PONG")
            || bytes.starts_with(b"-NOAUTH")
            || bytes.starts_with(b"-DENIED"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pong_or_a_refusal_is_redis() {
        for reply in [
            &b"+PONG\r\n"[..],
            &b"-NOAUTH Authentication required.\r\n"[..],
            &b"-DENIED Redis is running in protected mode\r\n"[..],
        ] {
            assert_eq!(
                parse_reply(reply),
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    #[test]
    fn silence_or_another_protocol_is_not_redis() {
        for reply in [
            &b""[..],
            &b"-ERR unknown command\r\n"[..],
            &b"HTTP/1.1 400 Bad Request\r\n"[..],
            &b"+OK\r\n"[..],
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer);
        }
    }
}
