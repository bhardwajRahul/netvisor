//! FTP detection by the greeting RFC 959 requires before any command is accepted.
//!
//! A server announces itself with a `220` reply as soon as the control connection opens. Rejections
//! count too: `421 Service not available` is a server refusing this client, and it is still an FTP
//! server, which is the claim the definition makes.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, read_greeting,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

pub struct FtpProbe;

#[async_trait]
impl AppProbe for FtpProbe {
    fn port(&self) -> PortType {
        PortType::Ftp
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Ftp)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(parse_greeting(&read_greeting(ctx, self.port(), 512).await))
    }
}

/// Whether the greeting is an FTP reply.
///
/// A reply line is three digits followed by a space or a hyphen (the hyphen marking a multi-line
/// reply). Only the two codes a connection can legitimately open with are accepted: `220` ready and
/// `421` refusing. Accepting any 3-digit code would match SMTP and NNTP, which greet the same way.
fn parse_greeting(bytes: &[u8]) -> AppProbeOutcome {
    let opens_with =
        |code: &[u8]| bytes.starts_with(code) && matches!(bytes.get(3), Some(b' ') | Some(b'-'));
    presence(opens_with(b"220") || opens_with(b"421"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ready_or_refusing_greeting_is_ftp() {
        for greeting in [
            &b"220 ProFTPD Server (Debian) [::ffff:172.17.0.2]\r\n"[..],
            &b"220-FileZilla Server 1.7.3\r\n220 Please visit\r\n"[..],
            &b"421 Service not available, closing control connection\r\n"[..],
        ] {
            assert_eq!(
                parse_greeting(greeting),
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    #[test]
    fn silence_or_another_greeting_protocol_is_not_ftp() {
        for greeting in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // A 3-digit code we do not accept: SMTP and NNTP greet in the same shape.
            &b"200 news.example.com NNRP Service Ready\r\n"[..],
            &b"2200 not a reply code\r\n"[..],
            &b"220\r\n"[..],
        ] {
            assert_eq!(parse_greeting(greeting), AppProbeOutcome::NoAnswer);
        }
    }
}
