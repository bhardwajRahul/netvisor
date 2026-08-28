//! Cassandra detection with a CQL `OPTIONS` frame.
//!
//! `OPTIONS` is the native protocol's capability query and is answered before `STARTUP`, so it needs
//! no keyspace, no credentials and no negotiated compression. A server speaking a different protocol
//! version answers `ERROR` rather than `SUPPORTED`, which identifies it just as well: it parsed the
//! frame to object to its version.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Set on the version byte of every response, clear on every request.
const RESPONSE_BIT: u8 = 0x80;
/// Native protocol versions in circulation: v3 through v5.
const SUPPORTED_VERSIONS: std::ops::RangeInclusive<u8> = 3..=5;

const OPCODE_ERROR: u8 = 0x00;
const OPCODE_OPTIONS: u8 = 0x05;
const OPCODE_SUPPORTED: u8 = 0x06;

/// Version, flags, stream id (2), opcode, body length (4).
const FRAME_HEADER_LEN: usize = 9;
/// Echoed back in the response, so an unrelated frame is not read as our answer.
const STREAM_ID: i16 = 0x2A;

/// A v4 `OPTIONS` with an empty body.
fn options_frame() -> Vec<u8> {
    let mut frame = vec![0x04, 0x00];
    frame.extend_from_slice(&STREAM_ID.to_be_bytes());
    frame.push(OPCODE_OPTIONS);
    frame.extend_from_slice(&0u32.to_be_bytes());
    frame
}

pub struct CassandraProbe;

#[async_trait]
impl AppProbe for CassandraProbe {
    fn port(&self) -> PortType {
        PortType::Cassandra
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Cassandra)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &options_frame(), 4096).await;
        Ok(parse_frame(&reply, STREAM_ID))
    }
}

/// Whether the reply is a CQL response frame answering our stream.
fn parse_frame(bytes: &[u8], stream_id: i16) -> AppProbeOutcome {
    let Some(header) = bytes.get(..FRAME_HEADER_LEN) else {
        return AppProbeOutcome::NoAnswer;
    };

    let is_response = header[0] & RESPONSE_BIT != 0;
    let version = header[0] & !RESPONSE_BIT;
    let stream = i16::from_be_bytes([header[2], header[3]]);
    let opcode = header[4];

    presence(
        is_response
            && SUPPORTED_VERSIONS.contains(&version)
            && stream == stream_id
            && matches!(opcode, OPCODE_SUPPORTED | OPCODE_ERROR),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(version: u8, stream: i16, opcode: u8) -> Vec<u8> {
        let mut frame = vec![version | RESPONSE_BIT, 0x00];
        frame.extend_from_slice(&stream.to_be_bytes());
        frame.push(opcode);
        frame.extend_from_slice(&0u32.to_be_bytes());
        frame
    }

    #[test]
    fn a_supported_frame_is_cassandra() {
        assert_eq!(
            parse_frame(&response(4, STREAM_ID, OPCODE_SUPPORTED), STREAM_ID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A server on another protocol version objects to ours, having parsed the frame to do so.
    #[test]
    fn a_version_error_is_cassandra() {
        assert_eq!(
            parse_frame(&response(3, STREAM_ID, OPCODE_ERROR), STREAM_ID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn an_uncorrelated_or_request_frame_is_not_evidence() {
        // Our stream id absent: not an answer to us.
        assert_eq!(
            parse_frame(&response(4, 0x01, OPCODE_SUPPORTED), STREAM_ID),
            AppProbeOutcome::NoAnswer
        );
        // Response bit clear: this is a request, not a reply.
        let mut request = response(4, STREAM_ID, OPCODE_SUPPORTED);
        request[0] &= !RESPONSE_BIT;
        assert_eq!(parse_frame(&request, STREAM_ID), AppProbeOutcome::NoAnswer);
    }

    #[test]
    fn silence_or_another_protocol_is_not_cassandra() {
        for reply in [&b""[..], &b"SSH-2.0-OpenSSH_9.6\r\n"[..], &b"\x84\x00"[..]] {
            assert_eq!(parse_frame(reply, STREAM_ID), AppProbeOutcome::NoAnswer);
        }
    }

    #[test]
    fn the_options_frame_is_a_request_on_our_stream() {
        let frame = options_frame();
        assert_eq!(
            frame[0] & RESPONSE_BIT,
            0,
            "requests clear the response bit"
        );
        assert_eq!(i16::from_be_bytes([frame[2], frame[3]]), STREAM_ID);
        assert_eq!(frame[4], OPCODE_OPTIONS);
        assert_eq!(frame.len(), FRAME_HEADER_LEN);
    }
}
