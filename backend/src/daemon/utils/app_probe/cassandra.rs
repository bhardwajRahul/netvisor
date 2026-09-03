//! Cassandra detection with a CQL `OPTIONS` frame.
//!
//! `OPTIONS` is the native protocol's capability query and is answered before `STARTUP`, so it needs
//! no keyspace, no credentials and no negotiated compression. A server speaking a different protocol
//! version answers `ERROR` rather than `SUPPORTED`, which identifies it just as well: it parsed the
//! frame to object to its version.
//!
//! [`cassandra_protocol`] builds and parses the envelope. `Envelope::from_buffer` checks the body
//! length against the bytes available, so a truncated or short-declared frame fails where the
//! header-offset check this replaced accepted it, and the version and direction come back as typed
//! values rather than as bits masked out of byte zero.
//!
//! The crate is the envelope codec, published separately from the `cdrs-tokio` driver that would
//! have insisted on completing `STARTUP`. The judgement — that an `ERROR` is a positive — stays here.

use anyhow::Error;
use async_trait::async_trait;
use cassandra_protocol::compression::Compression;
use cassandra_protocol::frame::{Direction, Envelope, Flags, Opcode, StreamId, Version};

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Echoed back in the response, so an unrelated frame is not read as our answer.
const STREAM_ID: StreamId = 0x2A;

/// A v4 `OPTIONS` with an empty body, on a stream we can recognise.
fn options_frame() -> Vec<u8> {
    Envelope::new(
        Version::V4,
        Direction::Request,
        Flags::empty(),
        Opcode::Options,
        STREAM_ID,
        Vec::new(),
        None,
        Vec::new(),
    )
    .encode_with(Compression::None)
    .expect("an OPTIONS frame with an empty body and no compression")
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
fn parse_frame(bytes: &[u8], stream_id: StreamId) -> AppProbeOutcome {
    let Ok(parsed) = Envelope::from_buffer(bytes, Compression::None) else {
        return AppProbeOutcome::NoAnswer;
    };
    let envelope = parsed.envelope;

    presence(
        envelope.direction == Direction::Response
            && envelope.stream_id == stream_id
            // `SUPPORTED` is the answer; `ERROR` means the server parsed the frame and objected to
            // its version, which identifies it just as well.
            && matches!(envelope.opcode, Opcode::Supported | Opcode::Error),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(version: Version, stream: StreamId, opcode: Opcode) -> Vec<u8> {
        Envelope::new(
            version,
            Direction::Response,
            Flags::empty(),
            opcode,
            stream,
            Vec::new(),
            None,
            Vec::new(),
        )
        .encode_with(Compression::None)
        .unwrap()
    }

    #[test]
    fn a_supported_frame_is_cassandra() {
        assert_eq!(
            parse_frame(
                &response(Version::V4, STREAM_ID, Opcode::Supported),
                STREAM_ID
            ),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A server on another protocol version objects to ours, having parsed the frame to do so.
    #[test]
    fn a_version_error_is_cassandra() {
        assert_eq!(
            parse_frame(&response(Version::V3, STREAM_ID, Opcode::Error), STREAM_ID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn a_frame_answering_another_stream_is_not_our_answer() {
        assert_eq!(
            parse_frame(&response(Version::V4, 0x11, Opcode::Supported), STREAM_ID),
            AppProbeOutcome::NoAnswer
        );
    }

    #[test]
    fn silence_a_request_or_another_protocol_is_not_cassandra() {
        let supported = response(Version::V4, STREAM_ID, Opcode::Supported);
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // Our own OPTIONS reflected back: a valid envelope, but a request.
            &options_frame(),
            // A response header whose body never arrived.
            &{
                let mut truncated = supported.clone();
                truncated[8] = 32;
                truncated
            }[..],
        ] {
            assert_eq!(
                parse_frame(reply, STREAM_ID),
                AppProbeOutcome::NoAnswer,
                "{reply:?}"
            );
        }
    }

    #[test]
    fn the_request_is_an_options_frame_on_our_stream() {
        let parsed = Envelope::from_buffer(&options_frame(), Compression::None).unwrap();
        assert_eq!(parsed.envelope.opcode, Opcode::Options);
        assert_eq!(parsed.envelope.direction, Direction::Request);
        assert_eq!(parsed.envelope.stream_id, STREAM_ID);
    }
}
