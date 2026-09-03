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
//!
//! [`kafka_protocol`] owns the wire format on both sides. Encoding a `RequestHeader` and decoding a
//! `ResponseHeader` at the right *header version* for the API version is the part that was fiddly
//! and easy to get subtly wrong; the crate derives it from the message type. The check this replaced
//! read the correlation id at a fixed offset, which happens to be right for header v0 alone.
//!
//! The judgement stays here: a broker answering `UNSUPPORTED_VERSION` is a broker. That is the
//! positive result, and it is the one a client crate would have raised as an error.

use anyhow::Error;
use async_trait::async_trait;
use bytes::BytesMut;
use kafka_protocol::messages::{ApiKey, ApiVersionsRequest, RequestHeader, ResponseHeader};
use kafka_protocol::protocol::{Decodable, Encodable, HeaderVersion};

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Version 0, whose request body is empty and whose response header is just the correlation id.
/// Every broker still answers it, which is the point of asking at the floor of the range.
const API_VERSION: i16 = 0;
/// Echoed back by the broker.
const CORRELATION_ID: i32 = 0x5CA1_0BA1u32 as i32;

/// A v0 `ApiVersions` request with a null client id.
fn api_versions_request() -> Vec<u8> {
    let header = RequestHeader::default()
        .with_request_api_key(ApiKey::ApiVersions as i16)
        .with_request_api_version(API_VERSION)
        .with_correlation_id(CORRELATION_ID)
        // A nullable string; null is what a client with no configured id sends.
        .with_client_id(None);

    let mut body = BytesMut::new();
    let header_version = ApiVersionsRequest::header_version(API_VERSION);
    header
        .encode(&mut body, header_version)
        .expect("the header is built from constants");
    ApiVersionsRequest::default()
        .encode(&mut body, API_VERSION)
        .expect("a v0 ApiVersions body is empty");

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
    presence(echoed_correlation_id(bytes) == Some(correlation_id))
}

fn echoed_correlation_id(bytes: &[u8]) -> Option<i32> {
    let size = i32::from_be_bytes(bytes.get(..4)?.try_into().ok()?);
    let body = bytes.get(4..)?;
    // The read is capped, so the declared size may exceed what arrived — but a size that does not
    // account for the bytes received is not this framing.
    if size < 4 || (size as usize) < body.len() {
        return None;
    }

    let mut cursor = BytesMut::from(body);
    let header_version = kafka_protocol::messages::ApiVersionsResponse::header_version(API_VERSION);
    ResponseHeader::decode(&mut cursor, header_version)
        .ok()
        .map(|header| header.correlation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(correlation_id: i32, body: &[u8]) -> Vec<u8> {
        let mut encoded = BytesMut::new();
        let header_version =
            kafka_protocol::messages::ApiVersionsResponse::header_version(API_VERSION);
        ResponseHeader::default()
            .with_correlation_id(correlation_id)
            .encode(&mut encoded, header_version)
            .unwrap();
        encoded.extend_from_slice(body);

        let mut reply = (encoded.len() as i32).to_be_bytes().to_vec();
        reply.extend_from_slice(&encoded);
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
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &[0, 0, 0, 4][..],
            // A size that claims less than what arrived.
            &[0, 0, 0, 4, 0x5C, 0xA1, 0x0B, 0xA1, 0xFF, 0xFF][..],
        ] {
            assert_eq!(
                parse_reply(reply, CORRELATION_ID),
                AppProbeOutcome::NoAnswer,
                "{reply:?}"
            );
        }
    }

    /// The request has to be one a broker will answer, which means the header decodes back to the
    /// API and correlation id we meant to send.
    #[test]
    fn the_request_is_size_prefixed_and_asks_for_api_versions() {
        let request = api_versions_request();
        let size = i32::from_be_bytes(request[0..4].try_into().unwrap());
        assert_eq!(size as usize, request.len() - 4);

        let mut body = BytesMut::from(&request[4..]);
        let header_version = ApiVersionsRequest::header_version(API_VERSION);
        let header = RequestHeader::decode(&mut body, header_version).unwrap();
        assert_eq!(header.request_api_key, ApiKey::ApiVersions as i16);
        assert_eq!(header.correlation_id, CORRELATION_ID);
        assert_eq!(header.client_id, None, "no client id is sent");
    }
}
