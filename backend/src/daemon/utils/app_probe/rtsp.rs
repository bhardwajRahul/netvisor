//! RTSP detection with the `OPTIONS` request RFC 2326 §10.1 requires every server to answer.
//!
//! `OPTIONS` is the protocol's own capability query: it addresses no stream, starts no session and
//! needs no credentials, so it works against a camera whose media requires authentication. As with
//! SIP, a rejection still identifies the protocol — a server answering `401 Unauthorized` parsed an
//! RTSP request to do so, which is exactly the claim being made.
//!
//! [`rtsp_types`] builds the request and parses the reply. The parse is what earns the dependency:
//! the check this replaced compared a nine-byte prefix and three digits, which any line beginning
//! `RTSP/1.0 401` satisfies, header block or not. `Message::parse` requires a complete status line,
//! a well-formed header block and a body consistent with `Content-Length`.
//!
//! Types and parser only — the crate owns no socket and makes no judgement about which status codes
//! count. That decision, that **any** status code counts, stays below.

use anyhow::Error;
use async_trait::async_trait;
use rtsp_types::{Empty, Message, Method, Request, StatusCode, Version, headers};

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// `OPTIONS` with no request-URI, which serialises as `*` — the request-URI meaning "the server
/// rather than a stream", so this needs no path and addresses no camera's media.
fn options_request() -> Vec<u8> {
    let request: Request<Empty> = Request::builder(Method::Options, Version::V1_0)
        .header(headers::CSEQ, "1")
        .empty();

    let mut out = Vec::new();
    request
        .write(&mut out)
        .expect("writing a fixed request into a Vec cannot fail");
    out
}

pub struct RtspProbe;

#[async_trait]
impl AppProbe for RtspProbe {
    fn port(&self) -> PortType {
        PortType::Rtsp
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Rtsp)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &options_request(), 1024).await;
        Ok(parse_response(&reply))
    }
}

/// Whether these bytes are an RTSP response.
///
/// Any status code counts. A camera that needs credentials answers `401`, one that does not
/// implement `OPTIONS` answers `501`, and both parsed an RTSP request to say so — which is the whole
/// claim. A *request* arriving unsolicited is not an answer to ours and does not count.
fn parse_response(bytes: &[u8]) -> AppProbeOutcome {
    presence(status_code(bytes).is_some())
}

/// The status code an RTSP server answered with, if it answered at all.
fn status_code(bytes: &[u8]) -> Option<StatusCode> {
    match Message::<Vec<u8>>::parse(bytes) {
        Ok((Message::Response(response), _consumed)) => Some(response.status()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_status_line_is_rtsp() {
        for reply in [
            &b"RTSP/1.0 200 OK\r\nCSeq: 1\r\nPublic: DESCRIBE, SETUP\r\n\r\n"[..],
            // A camera that needs credentials for media still answers OPTIONS as RTSP.
            &b"RTSP/1.0 401 Unauthorized\r\nCSeq: 1\r\n\r\n"[..],
            &b"RTSP/1.0 501 Not Implemented\r\nCSeq: 1\r\n\r\n"[..],
        ] {
            assert_eq!(
                parse_response(reply),
                AppProbeOutcome::Answered { identity: None },
                "{}",
                String::from_utf8_lossy(reply)
            );
        }
    }

    #[test]
    fn the_status_code_comes_back_typed() {
        assert_eq!(
            status_code(b"RTSP/1.0 401 Unauthorized\r\nCSeq: 1\r\n\r\n"),
            Some(StatusCode::Unauthorized)
        );
    }

    #[test]
    fn silence_or_http_is_not_rtsp() {
        for reply in [
            &b""[..],
            &b"HTTP/1.1 200 OK\r\n\r\n"[..],
            &b"SIP/2.0 200 OK\r\n\r\n"[..],
            // A request, not a response to ours.
            &b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\n"[..],
            &b"RTSP/1.0 OK\r\n"[..],
            // A status line with no header block behind it. The prefix comparison this replaced
            // accepted it.
            &b"RTSP/1.0 200 OK"[..],
        ] {
            assert_eq!(
                parse_response(reply),
                AppProbeOutcome::NoAnswer,
                "{}",
                String::from_utf8_lossy(reply)
            );
        }
    }

    /// `*` is the request-URI meaning "the server itself", so nothing about a stream is requested.
    #[test]
    fn the_request_addresses_the_server_rather_than_a_stream() {
        let request = options_request();
        assert!(
            request.starts_with(b"OPTIONS * RTSP/1.0\r\n"),
            "{}",
            String::from_utf8_lossy(&request)
        );
        let (parsed, _) = Message::<Vec<u8>>::parse(&request).expect("we send a valid request");
        let Message::Request(parsed) = parsed else {
            panic!("we send a request");
        };
        assert_eq!(parsed.method(), Method::Options);
        assert_eq!(parsed.request_uri(), None, "no stream is addressed");
    }
}
