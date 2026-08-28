//! SIP detection over TCP, by asking a question only a SIP stack can answer.
//!
//! This probe exists because of a specific report. A customer scanning remote VLANs through a
//! FortiGate 400F got a "SIP Server" host on 5060 on **every** VLAN, none with a MAC, none
//! corresponding to any device. FortiOS ships `config system session-helper` with a `sip` entry
//! enabled by default, and the helper completes the TCP handshake for any destination routed
//! through the firewall. Their packet capture on the destination VLAN showed zero packets on the
//! wire: nothing was ever there. The detection was `Pattern::Port(PortType::Sip)`, so a completed
//! handshake was the whole of the evidence, and a firewall answering on behalf of an empty address
//! satisfied it exactly as a PBX would.
//!
//! An `OPTIONS` request is the cheapest thing that separates the two. It is the SIP equivalent of a
//! ping — RFC 3261 §11 defines it as "query a server as to its capabilities", every stack
//! implements it, and it creates no dialog and no call. What comes back has to be a SIP status
//! line, and **any** final response counts, including the rejections:
//!
//! | Reply | Meaning |
//! |---|---|
//! | `SIP/2.0 200 OK` | A SIP stack, answering properly |
//! | `SIP/2.0 405`, `403`, `404`, `486`… | **Still a SIP stack** — it parsed the request and declined |
//! | Anything else, or silence | Not SIP |
//!
//! The middle row carries the weight. A hardened PBX answers `403 Forbidden` to an unknown peer and
//! a proxy with no route answers `404`; reading either as "not SIP" would lose the deployments most
//! worth documenting. What the status line proves is that something parsed SIP, which is the claim
//! the service definition makes.
//!
//! **This probe is not a complete answer to the report, and is not meant to be.** A SIP-aware ALG
//! may well answer an `OPTIONS` itself, since parsing SIP is its whole purpose. What removes the
//! false positive in general is that the definition now rests on a protocol exchange at all: see
//! `connect_only_definitions_are_declared` in `services/impl/tests.rs`, which is what stops the
//! next definition being written this way.

use anyhow::Error;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::daemon::utils::app_probe::{AppProbe, AppProbeOutcome, ProbeContext};
use crate::daemon::utils::scanner::SCAN_TIMEOUT;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The status line every SIP response opens with, whatever the code that follows.
const SIP_VERSION: &[u8] = b"SIP/2.0 ";

/// Enough for a status line and headers. A response body is of no interest here and is allowed to
/// be truncated.
const READ_BUF: usize = 2048;

pub struct SipProbe;

#[async_trait]
impl AppProbe for SipProbe {
    fn port(&self) -> PortType {
        PortType::Sip
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Sip)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let addr = std::net::SocketAddr::new(ctx.ip, self.port().number());

        let mut stream = match timeout(SCAN_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => {
                ctx.note_connect_error(&e);
                return Ok(AppProbeOutcome::NoAnswer);
            }
            Err(_) => return Ok(AppProbeOutcome::NoAnswer),
        };

        stream.write_all(options_request(ctx.ip).as_bytes()).await?;

        let mut buf = [0u8; READ_BUF];
        let read = match timeout(SCAN_TIMEOUT, stream.read(&mut buf)).await {
            Ok(Ok(read)) => read,
            // A listener that accepts and then says nothing is exactly the middlebox case.
            _ => return Ok(AppProbeOutcome::NoAnswer),
        };

        Ok(parse_response(&buf[..read]))
    }
}

/// An `OPTIONS` request addressed to the target.
///
/// Deliberately minimal, and deliberately not a real dialog participant: the branch and tag are
/// fixed rather than random because nothing correlates responses here — one request, one read, one
/// connection, then closed. `Max-Forwards: 0` keeps a proxy from relaying it onward, so the answer
/// comes from the host being probed rather than from whatever it fronts.
fn options_request(ip: std::net::IpAddr) -> String {
    let target = match ip {
        std::net::IpAddr::V4(v4) => v4.to_string(),
        std::net::IpAddr::V6(v6) => format!("[{v6}]"),
    };
    format!(
        "OPTIONS sip:{target} SIP/2.0\r\n\
         Via: SIP/2.0/TCP scanopy.invalid;branch=z9hG4bKscanopy\r\n\
         Max-Forwards: 0\r\n\
         To: <sip:{target}>\r\n\
         From: <sip:scanopy@scanopy.invalid>;tag=scanopy\r\n\
         Call-ID: scanopy-probe@scanopy.invalid\r\n\
         CSeq: 1 OPTIONS\r\n\
         Content-Length: 0\r\n\
         \r\n"
    )
}

/// Whether these bytes are a SIP response.
///
/// Split out from the socket work so the decision is testable without a listener, and so the
/// "accepted the connection and sent something that is not SIP" case can be asserted directly —
/// that is the shape a middlebox produces.
fn parse_response(bytes: &[u8]) -> AppProbeOutcome {
    // A status line is `SIP/2.0 <3-digit code> <reason>`. The version prefix alone would match a
    // request line's trailing `SIP/2.0` too, so the code is checked as well.
    let is_response = bytes.starts_with(SIP_VERSION)
        && bytes
            .get(SIP_VERSION.len()..SIP_VERSION.len() + 3)
            .is_some_and(|code| code.iter().all(u8::is_ascii_digit));

    if is_response {
        AppProbeOutcome::Answered { identity: None }
    } else {
        AppProbeOutcome::NoAnswer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_success_response_is_sip() {
        let reply = b"SIP/2.0 200 OK\r\nVia: SIP/2.0/TCP host\r\nContent-Length: 0\r\n\r\n";
        assert_eq!(
            parse_response(reply),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// The row that matters: a stack that parsed the request and refused it is still a stack. A
    /// hardened PBX answers unknown peers this way, and reading it as "not SIP" would lose exactly
    /// the deployments worth documenting.
    #[test]
    fn a_rejection_is_still_sip() {
        for reply in [
            &b"SIP/2.0 403 Forbidden\r\n\r\n"[..],
            &b"SIP/2.0 404 Not Found\r\n\r\n"[..],
            &b"SIP/2.0 405 Method Not Allowed\r\n\r\n"[..],
            &b"SIP/2.0 486 Busy Here\r\n\r\n"[..],
        ] {
            assert_eq!(
                parse_response(reply),
                AppProbeOutcome::Answered { identity: None },
                "{}",
                String::from_utf8_lossy(reply)
            );
        }
    }

    /// The middlebox. A FortiGate session helper completes the handshake and the read returns
    /// nothing, or something that is not a SIP status line. Either way this is not a SIP server,
    /// and before this probe existed both produced a "SIP Server" host.
    #[test]
    fn a_listener_that_does_not_speak_sip_is_not_sip() {
        for reply in [
            &b""[..],
            &b"\0\0\0\0"[..],
            &b"HTTP/1.1 200 OK\r\n\r\n"[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // A SIP *request* arriving unsolicited is not a response to ours.
            &b"OPTIONS sip:10.0.0.1 SIP/2.0\r\n\r\n"[..],
            // The version with no status code after it.
            &b"SIP/2.0 \r\n"[..],
            &b"SIP/2.0 OK\r\n"[..],
        ] {
            assert_eq!(
                parse_response(reply),
                AppProbeOutcome::NoAnswer,
                "{}",
                String::from_utf8_lossy(reply)
            );
        }
    }

    #[test]
    fn the_request_addresses_the_target_and_does_not_relay() {
        let request = options_request("10.1.2.3".parse().unwrap());
        assert!(request.starts_with("OPTIONS sip:10.1.2.3 SIP/2.0\r\n"));
        assert!(request.contains("Max-Forwards: 0\r\n"));
        assert!(request.ends_with("\r\n\r\n"));
    }

    #[test]
    fn an_ipv6_target_is_bracketed() {
        let request = options_request("2001:db8::1".parse().unwrap());
        assert!(request.starts_with("OPTIONS sip:[2001:db8::1] SIP/2.0\r\n"));
    }
}
