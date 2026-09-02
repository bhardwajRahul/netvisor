//! Telnet detection by option negotiation.
//!
//! RFC 854's command escape is `IAC` (255), and a Telnet server opens by negotiating options —
//! `IAC DO`, `IAC WILL` — before any login prompt. That byte is what identifies the protocol: a
//! plain text banner could come from anything, but 255 as the first byte of a stream that is
//! otherwise ASCII is Telnet's own framing.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, read_greeting,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Interpret As Command.
const IAC: u8 = 255;
/// The three command bytes that can follow `IAC` in an opening negotiation.
const WILL: u8 = 251;
const DO: u8 = 253;
const DONT: u8 = 254;
const WONT: u8 = 252;

pub struct TelnetProbe;

#[async_trait]
impl AppProbe for TelnetProbe {
    fn port(&self) -> PortType {
        PortType::Telnet
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Telnet)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(parse_negotiation(
            &read_greeting(ctx, self.port(), 256).await,
        ))
    }
}

/// Whether the opening bytes are Telnet option negotiation.
///
/// `IAC` followed by one of the four negotiation commands, rather than `IAC` alone: a lone 255 is a
/// plausible first byte of binary junk, and the command that must follow it is what makes this a
/// protocol match instead of a coincidence.
fn parse_negotiation(bytes: &[u8]) -> AppProbeOutcome {
    presence(
        bytes.first() == Some(&IAC) && matches!(bytes.get(1), Some(&WILL | &WONT | &DO | &DONT)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_negotiation_is_telnet() {
        for greeting in [
            // IAC DO TERMINAL-TYPE, IAC WILL ECHO
            &[IAC, DO, 24, IAC, WILL, 1][..],
            &[IAC, WILL, 3, IAC, DO, 31][..],
            &[IAC, WONT, 1][..],
        ] {
            assert_eq!(
                parse_negotiation(greeting),
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    #[test]
    fn silence_a_text_banner_or_a_lone_iac_is_not_telnet() {
        for greeting in [
            &[][..],
            b"login: ",
            b"SSH-2.0-OpenSSH_9.6\r\n",
            // A lone 255 followed by something that is not a negotiation command.
            &[IAC, 0x00, 0x01][..],
            &[IAC][..],
        ] {
            assert_eq!(parse_negotiation(greeting), AppProbeOutcome::NoAnswer);
        }
    }
}
