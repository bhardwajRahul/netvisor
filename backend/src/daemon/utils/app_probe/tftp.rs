//! TFTP detection by asking for a file that will not exist.
//!
//! A new capability rather than a conversion: 69 had no definition at all. It is worth having
//! because TFTP is how switches, phones and PXE clients load their firmware and configuration, so
//! finding one is finding the thing that provisions a network.
//!
//! RFC 1350 gives a two-octet opcode and no handshake to complete. A Read Request for a name nothing
//! will match draws `ERROR` opcode 5 with error code 1, `File not found`. **The error is the
//! positive result** — the server parsed the request to refuse it — and asking for a name that
//! cannot exist means nothing is ever transferred.
//!
//! Hand-rolled: the TFTP crates on crates.io are dead (11 and 172 recent downloads), and what would
//! be adopted is a pair of two-octet constants frozen by RFC 1350 since 1992.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, udp_request_response,
};
use crate::server::ports::r#impl::base::PortType;

/// RFC 1350 opcodes, all two octets, big-endian.
const OPCODE_READ_REQUEST: u16 = 1;
const OPCODE_DATA: u16 = 3;
const OPCODE_ERROR: u16 = 5;

/// The name asked for. Nothing is expected to match it, which is the point: the request cannot
/// transfer anything and the only outcome sought is the refusal.
const MISSING_FILE: &[u8] = b"scanopy-probe.invalid";

/// The only transfer mode every server implements.
const MODE_OCTET: &[u8] = b"octet";

/// An `ERROR` packet is a few dozen octets; a `DATA` block is at most 516.
const READ_LIMIT: usize = 1024;

/// A Read Request for a file that should not exist.
fn read_request() -> Vec<u8> {
    let mut out = OPCODE_READ_REQUEST.to_be_bytes().to_vec();
    out.extend_from_slice(MISSING_FILE);
    out.push(0);
    out.extend_from_slice(MODE_OCTET);
    out.push(0);
    out
}

pub struct TftpProbe;

#[async_trait]
impl AppProbe for TftpProbe {
    fn port(&self) -> PortType {
        PortType::new_udp(69)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = udp_request_response(ctx, self.port(), &read_request(), READ_LIMIT).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is a TFTP packet.
///
/// `ERROR` is what is expected. `DATA` is accepted too, in the unlikely event a server has a file by
/// that name: both are answers only a TFTP server sends, and neither is anything a silent listener
/// produces. A `DATA` block is not acknowledged, so no transfer begins.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    let Some(head) = bytes.get(..4) else {
        return AppProbeOutcome::NoAnswer;
    };
    let opcode = u16::from_be_bytes([head[0], head[1]]);

    presence(match opcode {
        // Error code then a NUL-terminated message. Code 0 is "not defined"; anything above 8 is
        // outside the range RFC 1350 defines and is not this protocol.
        OPCODE_ERROR => u16::from_be_bytes([head[2], head[3]]) <= 8 && bytes.last() == Some(&0),
        // Block numbers start at 1; a block 0 is not a TFTP data packet.
        OPCODE_DATA => u16::from_be_bytes([head[2], head[3]]) >= 1,
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_packet(code: u16, message: &[u8]) -> Vec<u8> {
        let mut out = OPCODE_ERROR.to_be_bytes().to_vec();
        out.extend_from_slice(&code.to_be_bytes());
        out.extend_from_slice(message);
        out.push(0);
        out
    }

    /// The expected outcome: a server refusing a file it does not have.
    #[test]
    fn a_file_not_found_error_is_tftp() {
        assert_eq!(
            parse_reply(&error_packet(1, b"File not found")),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A server that refuses on access grounds rather than absence still parsed the request.
    #[test]
    fn an_access_violation_is_also_tftp() {
        assert_eq!(
            parse_reply(&error_packet(2, b"Access violation")),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn a_data_block_is_tftp() {
        let mut data = OPCODE_DATA.to_be_bytes().to_vec();
        data.extend_from_slice(&1u16.to_be_bytes());
        data.extend_from_slice(b"contents");
        assert_eq!(
            parse_reply(&data),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_our_own_request_or_another_protocol_is_not_tftp() {
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &[0, 5][..],
            // Our own request reflected back carries opcode 1, which is not a reply.
            &read_request(),
            // An error code outside the range the RFC defines.
            &error_packet(99, b"nope")[..],
            // An unterminated error message.
            &{
                let mut unterminated = error_packet(1, b"File not found");
                unterminated.pop();
                unterminated
            }[..],
            // Data block 0, which TFTP never sends.
            &[0, 3, 0, 0, b'x'][..],
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }

    /// The request names a file nothing should have, so it cannot transfer anything.
    #[test]
    fn the_request_asks_for_a_file_that_should_not_exist() {
        let request = read_request();
        assert_eq!(
            u16::from_be_bytes([request[0], request[1]]),
            OPCODE_READ_REQUEST
        );
        // Opcode, NUL-terminated filename, NUL-terminated mode, and nothing else.
        let fields: Vec<&[u8]> = request[2..].split(|b| *b == 0).collect();
        assert_eq!(fields[0], MISSING_FILE);
        assert_eq!(fields[1], MODE_OCTET);
        assert_eq!(fields[2], b"", "the packet ends after the mode");
    }
}
