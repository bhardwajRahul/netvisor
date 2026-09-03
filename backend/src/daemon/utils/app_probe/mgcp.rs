//! MGCP detection with an `AUEP`, the command a call agent uses to ask whether an endpoint is there.
//!
//! A new capability rather than a conversion: 2427 and 2727 had no definition. MGCP is what ties an
//! analogue telephone adapter or a voice gateway to its call agent, so these ports mark the boundary
//! between a phone system and the PSTN.
//!
//! RFC 3435 defines `AUEP` (AuditEndPoint) as a query, and the reply is a plain-text response line:
//!
//! ```text
//! → AUEP 1 *@scanopy.invalid MGCP 1.0
//! ← 500 1 Endpoint unknown
//! ```
//!
//! **The rejection is the expected answer.** `500` means the gateway parsed the command and has no
//! such endpoint, which is precisely what an endpoint named `*@scanopy.invalid` should draw.
//! Nothing is created, connected or torn down — `AUEP` cannot change state, which is why it is the
//! command used here rather than any of the ones that can.
//!
//! Correlation is what makes the reply evidence rather than a coincidence: the transaction id we
//! send comes back as the second field, so an unrelated datagram is not read as our answer.
//!
//! Hand-rolled: nothing on crates.io implements MGCP at all, and the protocol is line-oriented text.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, udp_request_response,
};
use crate::server::ports::r#impl::base::PortType;

/// The gateway port and the call-agent port. Both speak the same protocol; which end is listening
/// says which role the host plays, and either is worth finding.
pub(crate) const MGCP_GATEWAY_PORT: u16 = 2427;
pub(crate) const MGCP_CALL_AGENT_PORT: u16 = 2727;

/// Echoed back as the second field of the response line.
const TRANSACTION_ID: &str = "1";

/// A response line is short; a gateway listing capabilities can run to a few hundred octets.
const READ_LIMIT: usize = 1024;

/// An `AUEP` for an endpoint that should not exist.
///
/// RFC 3435 §3.5 fixes the line as `verb transaction-id endpoint protocol-version`, and the
/// terminating blank line is what tells the gateway the command has no parameters.
fn audit_endpoint() -> Vec<u8> {
    format!("AUEP {TRANSACTION_ID} *@scanopy.invalid MGCP 1.0\r\n\r\n").into_bytes()
}

/// A probe for one of the two ports MGCP is spoken on.
pub struct MgcpProbe {
    port: PortType,
}

impl MgcpProbe {
    pub fn new(port: u16) -> Self {
        Self {
            port: PortType::new_udp(port),
        }
    }
}

#[async_trait]
impl AppProbe for MgcpProbe {
    fn port(&self) -> PortType {
        self.port
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = udp_request_response(ctx, self.port(), &audit_endpoint(), READ_LIMIT).await;
        Ok(parse_response(&reply))
    }
}

/// Whether the reply is an MGCP response to our transaction.
///
/// Any response code counts. `200` is an endpoint that exists, `500` one that does not, `518` an
/// unsupported package — every one of them means something parsed an MGCP command, which is the
/// claim being made. What is *not* accepted is a response carrying someone else's transaction id.
fn parse_response(bytes: &[u8]) -> AppProbeOutcome {
    presence(response_code(bytes).is_some())
}

/// The response code, when the line is a response to the transaction we sent.
fn response_code(bytes: &[u8]) -> Option<u16> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut fields = text.lines().next()?.split_whitespace();

    // RFC 3435 §3.5.2: a three-digit code, then the transaction id it answers.
    let code = fields.next()?;
    if code.len() != 3 || !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    (fields.next()? == TRANSACTION_ID).then_some(())?;
    code.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The expected outcome: a gateway that has no such endpoint, having parsed the command to say
    /// so.
    #[test]
    fn an_endpoint_unknown_response_is_mgcp() {
        assert_eq!(
            parse_response(b"500 1 Endpoint unknown\r\n"),
            AppProbeOutcome::Answered { identity: None }
        );
        assert_eq!(response_code(b"500 1 Endpoint unknown\r\n"), Some(500));
    }

    #[test]
    fn any_response_code_is_mgcp() {
        for line in [
            &b"200 1 OK\r\n"[..],
            &b"518 1 Unsupported package\r\n"[..],
            &b"400 1 Transient error\r\n"[..],
            // A response with parameter lines behind it.
            &b"200 1 OK\r\nZ: aaln/1@gw\r\nM: sendrecv\r\n"[..],
        ] {
            assert_eq!(
                parse_response(line),
                AppProbeOutcome::Answered { identity: None },
                "{}",
                String::from_utf8_lossy(line)
            );
        }
    }

    #[test]
    fn a_response_to_another_transaction_is_not_our_answer() {
        assert_eq!(
            parse_response(b"200 4271 OK\r\n"),
            AppProbeOutcome::NoAnswer
        );
    }

    #[test]
    fn silence_our_own_command_or_another_protocol_is_not_mgcp() {
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"SIP/2.0 200 OK\r\n"[..],
            // Our own AUEP reflected back is a command, not a response.
            &audit_endpoint(),
            // A code that is not three digits.
            &b"20 1 OK\r\n"[..],
            &b"2000 1 OK\r\n"[..],
            // A code-shaped first field with no transaction id behind it.
            &b"500\r\n"[..],
            &[0xFF, 0xFE, 0xFD][..],
        ] {
            assert_eq!(
                parse_response(reply),
                AppProbeOutcome::NoAnswer,
                "{}",
                String::from_utf8_lossy(reply)
            );
        }
    }

    /// `AUEP` audits; it cannot create or tear down a connection. That is why it is the command
    /// sent, and the endpoint named is one nothing should match.
    #[test]
    fn the_command_audits_an_endpoint_that_should_not_exist() {
        let command = audit_endpoint();
        let text = String::from_utf8(command).unwrap();
        assert!(
            text.starts_with("AUEP 1 *@scanopy.invalid MGCP 1.0\r\n"),
            "{text}"
        );
        assert!(
            text.ends_with("\r\n\r\n"),
            "the blank line ends the command"
        );
    }
}
