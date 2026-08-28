//! Kafka detection with an `ApiVersions` request.
//!
//! `ApiVersions` is the request a client sends before anything else, precisely so it can discover
//! what the broker supports before committing to a version — which means a broker answers it
//! without authentication even when SASL is required. A broker that dislikes the version we asked
//! for answers with an `UNSUPPORTED_VERSION` error code, and that is a parsed response too.
//!
//! Correlation is what makes the reply evidence: the correlation id we send comes back in the
//! response header, so a stream that merely opens with a plausible length is not mistaken for an
//! answer.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// `ApiVersions`.
const API_KEY_API_VERSIONS: i16 = 18;
/// Version 0, whose request body is empty and whose response header is just the correlation id.
const API_VERSION: i16 = 0;
/// Echoed back by the broker.
const CORRELATION_ID: i32 = 0x5CA1_0BA1u32 as i32;

/// A v0 `ApiVersions` request with a null client id.
fn api_versions_request() -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&API_KEY_API_VERSIONS.to_be_bytes());
    body.extend_from_slice(&API_VERSION.to_be_bytes());
    body.extend_from_slice(&CORRELATION_ID.to_be_bytes());
    // client_id is a nullable string; -1 is null, which every broker accepts here.
    body.extend_from_slice(&(-1i16).to_be_bytes());

    let mut request = (body.len() as i32).to_be_bytes().to_vec();
    request.extend_from_slice(&body);
    request
}

pub struct KafkaProbe;

#[async_trait]
impl AppProbe for KafkaProbe {
    fn port(&self) -> PortType {
        PortType::Kafka
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Kafka)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &api_versions_request(), 1024).await;
        Ok(parse_reply(&reply, CORRELATION_ID))
    }
}

/// Whether the reply is a Kafka response carrying our correlation id.
///
/// The declared size is checked as well, because a four-byte correlation id on its own is a weak
/// coincidence guard: the framing plus the echo together are what identify a broker.
fn parse_reply(bytes: &[u8], correlation_id: i32) -> AppProbeOutcome {
    let Some(header) = bytes.get(..8) else {
        return AppProbeOutcome::NoAnswer;
    };
    let size = i32::from_be_bytes(header[0..4].try_into().unwrap_or_default());
    let echoed = i32::from_be_bytes(header[4..8].try_into().unwrap_or_default());

    // The size counts everything after itself, and the read is capped, so it may exceed what
    // arrived but must account for at least the correlation id.
    presence(size >= 4 && size as usize >= bytes.len() - 4 && echoed == correlation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(correlation_id: i32, body: &[u8]) -> Vec<u8> {
        let size = (4 + body.len()) as i32;
        let mut reply = size.to_be_bytes().to_vec();
        reply.extend_from_slice(&correlation_id.to_be_bytes());
        reply.extend_from_slice(body);
        reply
    }

    #[test]
    fn a_correlated_response_is_kafka() {
        // error_code 0, then the api key array.
        assert_eq!(
            parse_reply(
                &response(CORRELATION_ID, &[0, 0, 0, 0, 0, 1]),
                CORRELATION_ID
            ),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A broker rejecting the version still parsed the request to do so.
    #[test]
    fn an_unsupported_version_error_is_kafka() {
        assert_eq!(
            parse_reply(&response(CORRELATION_ID, &[0, 35]), CORRELATION_ID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn an_uncorrelated_reply_is_not_evidence() {
        assert_eq!(
            parse_reply(&response(999, &[0, 0]), CORRELATION_ID),
            AppProbeOutcome::NoAnswer
        );
    }

    #[test]
    fn silence_or_another_protocol_is_not_kafka() {
        for reply in [&b""[..], &b"SSH-2.0-OpenSSH_9.6\r\n"[..], &[0, 0, 0, 4][..]] {
            assert_eq!(
                parse_reply(reply, CORRELATION_ID),
                AppProbeOutcome::NoAnswer
            );
        }
    }

    #[test]
    fn the_request_is_size_prefixed_and_asks_for_api_versions() {
        let request = api_versions_request();
        let size = i32::from_be_bytes(request[0..4].try_into().unwrap());
        assert_eq!(size as usize, request.len() - 4);
        assert_eq!(
            i16::from_be_bytes(request[4..6].try_into().unwrap()),
            API_KEY_API_VERSIONS
        );
    }
}
