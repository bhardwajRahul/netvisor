//! Redis detection with `PING`, including the refusals.
//!
//! RESP is a text protocol and `PING` is its cheapest command, but the interesting answers are the
//! errors. A Redis with `requirepass` set replies `-NOAUTH`, and one in protected mode replies
//! `-DENIED`; both parsed the command to say so, which is what identifies the server. Treating only
//! `+PONG` as Redis would miss every instance with a password on it.
//!
//! [`redis_protocol`] decodes the reply into a `Frame`, so the CRLF framing is checked rather than
//! assumed — `starts_with(b"+PONG")` matched an unterminated fragment, and a frame does not.
//!
//! The crate is the codec, not the `redis` client crate, and the difference is the whole probe: a
//! client returns `-NOAUTH` as `Err`, where here it is the positive result. Which errors count stays
//! a decision made below.

use anyhow::Error;
use async_trait::async_trait;
use redis_protocol::resp2::decode::decode;
use redis_protocol::resp2::types::OwnedFrame;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Inline command syntax, which every Redis accepts on a fresh connection.
const PING: &[u8] = b"PING\r\n";

/// The errors Redis itself defines for an unauthenticated `PING`. Accepting *any* RESP error would
/// be looser than needed: `-ERR unknown command` means something spoke RESP but is not Redis.
const EXPECTED_ERRORS: [&str; 2] = ["NOAUTH", "DENIED"];

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

/// Whether the reply to `PING` is a RESP frame Redis would have sent.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    let Ok(Some((frame, _consumed))) = decode(bytes) else {
        return AppProbeOutcome::NoAnswer;
    };

    presence(match &frame {
        OwnedFrame::SimpleString(s) => s == b"PONG",
        OwnedFrame::Error(e) => EXPECTED_ERRORS.iter().any(|prefix| e.starts_with(prefix)),
        _ => false,
    })
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
                AppProbeOutcome::Answered { identity: None },
                "{}",
                String::from_utf8_lossy(reply)
            );
        }
    }

    #[test]
    fn silence_or_another_protocol_is_not_redis() {
        for reply in [
            &b""[..],
            // RESP, but not an error Redis sends for this command.
            &b"-ERR unknown command\r\n"[..],
            &b"HTTP/1.1 400 Bad Request\r\n"[..],
            &b"+OK\r\n"[..],
            // The bytes of a PONG without the frame terminator. The prefix comparison this
            // replaced accepted it.
            &b"+PONG"[..],
        ] {
            assert_eq!(
                parse_reply(reply),
                AppProbeOutcome::NoAnswer,
                "{}",
                String::from_utf8_lossy(reply)
            );
        }
    }
}
