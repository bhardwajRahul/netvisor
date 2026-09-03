//! Check_MK agent detection by the section header it opens with.
//!
//! The agent needs no request: on connect it writes its whole inventory and closes, and the first
//! section is always `<<<check_mk>>>`. That is both the protocol's framing and its name, which makes
//! it about as unambiguous as a greeting gets.
//!
//! An agent configured with `only_from` refuses our address by closing the connection without
//! writing, which reads as `NoAnswer`. That is the correct outcome: we cannot tell it from a
//! middlebox completing a handshake, and inventing a service either way is what this change exists
//! to stop.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, read_greeting,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The first section header of every agent dump.
const CHECK_MK_SECTION: &[u8] = b"<<<check_mk>>>";

pub struct CheckMkAgentProbe;

#[async_trait]
impl AppProbe for CheckMkAgentProbe {
    fn port(&self) -> PortType {
        PortType::new_tcp(6556)
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::CheckMkAgent)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(parse_dump(&read_greeting(ctx, self.port(), 512).await))
    }
}

/// Whether the opening bytes are an agent dump.
///
/// A substring rather than a prefix: an encrypted agent prefixes the stream with its own marker,
/// and some builds emit a leading newline. The section header itself is what identifies the agent.
fn parse_dump(bytes: &[u8]) -> AppProbeOutcome {
    presence(
        bytes
            .windows(CHECK_MK_SECTION.len())
            .any(|w| w == CHECK_MK_SECTION),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_section_header_is_a_check_mk_agent() {
        for dump in [
            &b"<<<check_mk>>>\nVersion: 2.2.0p8\nAgentOS: linux\n"[..],
            &b"\n<<<check_mk>>>\nVersion: 2.1.0\n"[..],
        ] {
            assert_eq!(
                parse_dump(dump),
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    #[test]
    fn silence_or_another_protocol_is_not_a_check_mk_agent() {
        for dump in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"<<<other_section>>>\n"[..],
            &b"HTTP/1.1 200 OK\r\n"[..],
        ] {
            assert_eq!(parse_dump(dump), AppProbeOutcome::NoAnswer);
        }
    }
}
