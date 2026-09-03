//! Beszel agent detection by the software name in its SSH identification string.
//!
//! 45876 was declared `NoVerifiableImplementation` — "no published exchange an unauthenticated peer
//! can complete to confirm it". Two things were wrong with that. The agent is published as
//! `henrygd/beszel-agent` with an arm64 image, so it *is* verifiable; and no exchange has to be
//! completed, because RFC 4253 §4.2 makes the server speak first. A real agent opens with:
//!
//! ```text
//! SSH-2.0-beszel_0.18.8
//! ```
//!
//! The `softwareversion` field is free text an implementation chooses for itself, and Beszel's names
//! the product. That is the difference between this probe and [`ssh`](super::ssh): the same banner,
//! read for identity rather than for the protocol.
//!
//! Nothing authenticates. The agent expects the hub's key and this has no key; the banner arrives
//! before any of that is relevant, and the connection is closed as soon as it does.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, read_greeting,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The agent's default port.
const BESZEL_AGENT_PORT: u16 = 45876;

/// The identification string every SSH server opens with.
const SSH_BANNER: &[u8] = b"SSH-";

/// What Beszel puts in the `softwareversion` field. Lowercase and underscore-separated, as
/// `gliderlabs/ssh` formats it.
const BESZEL_SOFTWARE: &[u8] = b"beszel";

pub struct BeszelAgentProbe;

#[async_trait]
impl AppProbe for BeszelAgentProbe {
    fn port(&self) -> PortType {
        PortType::new_tcp(BESZEL_AGENT_PORT)
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::BeszelAgent)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(parse_banner(&read_greeting(ctx, self.port(), 512).await))
    }
}

/// Whether the opening bytes are an SSH identification string naming Beszel.
///
/// Both halves matter. The `SSH-` prefix says this is the identification string rather than any
/// stream containing the word; the software name says it is Beszel rather than the OpenSSH that
/// might be listening on an unusual port.
fn parse_banner(bytes: &[u8]) -> AppProbeOutcome {
    presence(
        bytes.starts_with(SSH_BANNER)
            && bytes
                .windows(BESZEL_SOFTWARE.len())
                .any(|w| w.eq_ignore_ascii_case(BESZEL_SOFTWARE)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The banner a live `henrygd/beszel-agent` sends, copied from a run against one.
    #[test]
    fn a_beszel_banner_is_a_beszel_agent() {
        assert_eq!(
            parse_banner(b"SSH-2.0-beszel_0.18.8\r\n"),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// Another SSH server on the same port is not a Beszel agent — which is the whole reason the
    /// software name is checked rather than the protocol.
    #[test]
    fn another_ssh_server_on_the_port_is_not_a_beszel_agent() {
        for banner in [
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"SSH-2.0-dropbear_2022.83\r\n"[..],
            &b"SSH-1.99-Cisco-1.25\r\n"[..],
        ] {
            assert_eq!(
                parse_banner(banner),
                AppProbeOutcome::NoAnswer,
                "{}",
                String::from_utf8_lossy(banner)
            );
        }
    }

    #[test]
    fn silence_or_another_protocol_is_not_a_beszel_agent() {
        for banner in [
            &b""[..],
            &b"220 ProFTPD Server ready\r\n"[..],
            &b"\0\0\0\0"[..],
            // The name without the identification string: a page that merely mentions Beszel.
            &b"HTTP/1.1 200 OK\r\n\r\n<title>beszel</title>"[..],
        ] {
            assert_eq!(
                parse_banner(banner),
                AppProbeOutcome::NoAnswer,
                "{}",
                String::from_utf8_lossy(banner)
            );
        }
    }
}
