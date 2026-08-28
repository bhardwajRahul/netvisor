//! RTSP detection with the `OPTIONS` request RFC 2326 §10.1 requires every server to answer.
//!
//! `OPTIONS` is the protocol's own capability query: it addresses no stream, starts no session and
//! needs no credentials, so it works against a camera whose media requires authentication. As with
//! SIP, a rejection still identifies the protocol — a server answering `401 Unauthorized` parsed an
//! RTSP request to do so, which is exactly the claim being made.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The status line every RTSP response opens with.
const RTSP_VERSION: &[u8] = b"RTSP/1.0 ";

/// `*` is the request-URI meaning "the server rather than a stream", so this needs no path.
const OPTIONS_REQUEST: &[u8] = b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\n\r\n";

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
        let reply = request_response(ctx, self.port(), OPTIONS_REQUEST, 1024).await;
        Ok(parse_response(&reply))
    }
}

/// Whether these bytes are an RTSP status line.
///
/// The three-digit code is checked as well as the version, so a request line ending in `RTSP/1.0`
/// is not mistaken for a response to ours.
fn parse_response(bytes: &[u8]) -> AppProbeOutcome {
    presence(
        bytes.starts_with(RTSP_VERSION)
            && bytes
                .get(RTSP_VERSION.len()..RTSP_VERSION.len() + 3)
                .is_some_and(|code| code.iter().all(u8::is_ascii_digit)),
    )
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
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    #[test]
    fn silence_or_http_is_not_rtsp() {
        for reply in [
            &b""[..],
            &b"HTTP/1.1 200 OK\r\n\r\n"[..],
            &b"SIP/2.0 200 OK\r\n\r\n"[..],
            &b"OPTIONS * RTSP/1.0\r\n\r\n"[..],
            &b"RTSP/1.0 OK\r\n"[..],
        ] {
            assert_eq!(parse_response(reply), AppProbeOutcome::NoAnswer);
        }
    }
}
