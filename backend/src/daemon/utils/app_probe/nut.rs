//! Network UPS Tools detection with `VER`.
//!
//! `upsd`'s protocol is line-oriented text and `VER` is the one command that needs no UPS name and
//! no login: the server answers with its own identification string. A server that requires
//! authentication for everything else still answers this, because there is nothing to protect in it.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The project's own name, which `upsd` puts in its version string.
const NUT_IDENTIFICATION: &[u8] = b"Network UPS Tools";

const VER: &[u8] = b"VER\n";

pub struct NutProbe;

#[async_trait]
impl AppProbe for NutProbe {
    fn port(&self) -> PortType {
        PortType::new_tcp(3493)
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Nut)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), VER, 256).await;
        Ok(parse_version(&reply))
    }
}

/// Whether the reply to `VER` is `upsd`'s identification string.
///
/// A substring rather than a prefix: the line's exact shape varies by build, and some distributions
/// prefix it. What does not vary is the project name inside it.
fn parse_version(bytes: &[u8]) -> AppProbeOutcome {
    presence(
        bytes
            .windows(NUT_IDENTIFICATION.len())
            .any(|w| w == NUT_IDENTIFICATION),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_string_is_nut() {
        for reply in [
            &b"Network UPS Tools upsd 2.8.0 - http://www.networkupstools.org/\n"[..],
            &b"Network UPS Tools upsd 2.7.4\n"[..],
        ] {
            assert_eq!(
                parse_version(reply),
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    #[test]
    fn silence_or_an_error_from_another_protocol_is_not_nut() {
        for reply in [
            &b""[..],
            &b"ERR UNKNOWN-COMMAND\n"[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"HTTP/1.1 400 Bad Request\r\n"[..],
        ] {
            assert_eq!(parse_version(reply), AppProbeOutcome::NoAnswer);
        }
    }
}
