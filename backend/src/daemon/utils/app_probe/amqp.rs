//! AMQP detection by the protocol header exchange.
//!
//! A client opens an AMQP connection by sending the literal `AMQP` followed by four version bytes.
//! A broker that speaks that version replies with a `connection.start` method frame; one that does
//! not replies with its *own* protocol header and closes. Both answers identify a broker, and the
//! second is the more useful of the two because it names the version the broker does speak.
//!
//! Nothing here authenticates: `connection.start` is what the broker sends *before* asking for
//! credentials, so this works against a broker with no anonymous access.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The literal every AMQP conversation opens with, in both directions.
const AMQP_MAGIC: &[u8] = b"AMQP";

/// `AMQP` + protocol id 0 + version 0-9-1, which is what RabbitMQ and most brokers speak.
const PROTOCOL_HEADER: [u8; 8] = [b'A', b'M', b'Q', b'P', 0, 0, 9, 1];

/// Frame type 1 is a method frame, which is what `connection.start` arrives as.
const FRAME_METHOD: u8 = 0x01;
/// `connection.start` is class 10, method 10.
const CLASS_CONNECTION: u16 = 10;
const METHOD_START: u16 = 10;

pub struct AmqpProbe;

#[async_trait]
impl AppProbe for AmqpProbe {
    fn port(&self) -> PortType {
        PortType::AMQP
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Amqp)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &PROTOCOL_HEADER, 512).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is either half of the AMQP header exchange.
///
/// A version rejection is the broker echoing `AMQP` with the version it wants. A `connection.start`
/// is a method frame whose class and method are both 10; checking those rather than just the frame
/// type is what stops any stream beginning with `0x01` from matching.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    if bytes.starts_with(AMQP_MAGIC) {
        return presence(true);
    }

    // Frame header: type (1), channel (2), payload length (4), then the payload.
    let is_connection_start = bytes.first() == Some(&FRAME_METHOD)
        && bytes.get(7..11).is_some_and(|payload| {
            let class = u16::from_be_bytes([payload[0], payload[1]]);
            let method = u16::from_be_bytes([payload[2], payload[3]]);
            class == CLASS_CONNECTION && method == METHOD_START
        });

    presence(is_connection_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A broker speaking a different version answers with its own header and closes.
    #[test]
    fn a_version_rejection_is_amqp() {
        assert_eq!(
            parse_reply(&[b'A', b'M', b'Q', b'P', 0, 0, 9, 1]),
            AppProbeOutcome::Answered { identity: None }
        );
        assert_eq!(
            parse_reply(&[b'A', b'M', b'Q', b'P', 0, 1, 0, 0]),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// The normal path: a `connection.start` method frame, sent before any credentials are asked
    /// for.
    #[test]
    fn a_connection_start_frame_is_amqp() {
        let mut frame = vec![FRAME_METHOD, 0x00, 0x00];
        frame.extend_from_slice(&[0x00, 0x00, 0x01, 0x00]); // payload length
        frame.extend_from_slice(&[0x00, 0x0A, 0x00, 0x0A]); // class 10, method 10
        frame.extend_from_slice(&[0x00, 0x09]); // version-major, version-minor
        assert_eq!(
            parse_reply(&frame),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_or_another_protocol_is_not_amqp() {
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"HTTP/1.1 400 Bad Request\r\n"[..],
            // A method frame for some other class/method.
            &[FRAME_METHOD, 0, 0, 0, 0, 0, 4, 0x00, 0x14, 0x00, 0x0A][..],
            &[FRAME_METHOD, 0, 0][..],
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }
}
