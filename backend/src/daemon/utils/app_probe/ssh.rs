//! SSH detection by its identification string.
//!
//! RFC 4253 §4.2 requires the server to send `SSH-protocolversion-softwareversion` before anything
//! else happens, so this needs no request at all. That also makes it one of the cleanest separations
//! available between a real service and a listener that merely completes a handshake: a middlebox
//! answering on behalf of an empty address has nothing to send.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, read_greeting,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The identification string every SSH server opens with.
const SSH_BANNER: &[u8] = b"SSH-";

pub struct SshProbe;

#[async_trait]
impl AppProbe for SshProbe {
    fn port(&self) -> PortType {
        PortType::Ssh
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Ssh)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(parse_banner(&read_greeting(ctx, self.port(), 512).await))
    }
}

/// Whether the opening bytes are an SSH identification string.
///
/// The version that follows is not checked: `SSH-1.99` and `SSH-2.0` are both real, and a server
/// free-texting its software name after them is expected. The prefix is what identifies the
/// protocol.
fn parse_banner(bytes: &[u8]) -> AppProbeOutcome {
    presence(bytes.starts_with(SSH_BANNER))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identification_string_is_ssh() {
        for banner in [
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"SSH-1.99-Cisco-1.25\r\n"[..],
            &b"SSH-2.0-dropbear_2022.83\r\n"[..],
        ] {
            assert_eq!(
                parse_banner(banner),
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    #[test]
    fn silence_or_another_protocol_is_not_ssh() {
        for banner in [
            &b""[..],
            &b"220 ProFTPD Server ready\r\n"[..],
            &b"HTTP/1.1 400 Bad Request\r\n"[..],
            &b"\0\0\0\0"[..],
        ] {
            assert_eq!(parse_banner(banner), AppProbeOutcome::NoAnswer);
        }
    }
}
