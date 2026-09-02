//! DNS-over-TCP detection.
//!
//! The UDP side of DNS is covered by [`DnsProbe`](crate::daemon::utils::app_probe::udp::DnsProbe),
//! which resolves a real name through a library client. TCP/53 needs its own probe because a
//! resolver reachable only over TCP is a real deployment — some are configured that way
//! deliberately, and a large answer forces TCP regardless.
//!
//! The query asks for the root zone's NS records, which every resolver and every authoritative
//! server can answer from its own hints without recursion. What is checked is the reply's framing
//! and its transaction id, not the records: a server answering `REFUSED` has still parsed a DNS
//! query, which is the claim being made.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Echoed by the server, correlating the reply to our query.
const TRANSACTION_ID: u16 = 0x5CA1;
/// The response bit in the flags word.
const FLAG_RESPONSE: u16 = 0x8000;
/// DNS over TCP prefixes every message with its own length.
const LENGTH_PREFIX_LEN: usize = 2;

/// A query for `.` NS, length-prefixed for TCP.
fn root_ns_query() -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(&TRANSACTION_ID.to_be_bytes());
    message.extend_from_slice(&0x0000u16.to_be_bytes()); // flags: standard query, no recursion
    message.extend_from_slice(&1u16.to_be_bytes()); // one question
    message.extend_from_slice(&0u16.to_be_bytes()); // answers
    message.extend_from_slice(&0u16.to_be_bytes()); // authority
    message.extend_from_slice(&0u16.to_be_bytes()); // additional
    message.push(0x00); // QNAME: the root, a single empty label
    message.extend_from_slice(&2u16.to_be_bytes()); // QTYPE NS
    message.extend_from_slice(&1u16.to_be_bytes()); // QCLASS IN

    let mut framed = (message.len() as u16).to_be_bytes().to_vec();
    framed.extend_from_slice(&message);
    framed
}

pub struct DnsTcpProbe;

#[async_trait]
impl AppProbe for DnsTcpProbe {
    fn port(&self) -> PortType {
        PortType::DnsTcp
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::DnsTcp)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &root_ns_query(), 1024).await;
        Ok(parse_reply(&reply, TRANSACTION_ID))
    }
}

/// Whether the reply is a DNS response to our query.
fn parse_reply(bytes: &[u8], transaction_id: u16) -> AppProbeOutcome {
    let Some(header) = bytes.get(LENGTH_PREFIX_LEN..LENGTH_PREFIX_LEN + 4) else {
        return AppProbeOutcome::NoAnswer;
    };
    let echoed = u16::from_be_bytes([header[0], header[1]]);
    let flags = u16::from_be_bytes([header[2], header[3]]);

    presence(echoed == transaction_id && flags & FLAG_RESPONSE != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(transaction_id: u16, flags: u16) -> Vec<u8> {
        let mut message = transaction_id.to_be_bytes().to_vec();
        message.extend_from_slice(&flags.to_be_bytes());
        message.extend_from_slice(&[0u8; 8]);
        let mut framed = (message.len() as u16).to_be_bytes().to_vec();
        framed.extend_from_slice(&message);
        framed
    }

    #[test]
    fn a_correlated_response_is_dns() {
        assert_eq!(
            parse_reply(&response(TRANSACTION_ID, 0x8400), TRANSACTION_ID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A server refusing the query parsed it to refuse it.
    #[test]
    fn a_refused_response_is_dns() {
        assert_eq!(
            parse_reply(&response(TRANSACTION_ID, 0x8005), TRANSACTION_ID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn an_uncorrelated_reply_or_an_echoed_query_is_not_evidence() {
        assert_eq!(
            parse_reply(&response(0x1234, 0x8400), TRANSACTION_ID),
            AppProbeOutcome::NoAnswer
        );
        // The response bit clear: this is a query, which is what an echo of our own bytes gives.
        assert_eq!(
            parse_reply(&response(TRANSACTION_ID, 0x0000), TRANSACTION_ID),
            AppProbeOutcome::NoAnswer
        );
    }

    #[test]
    fn silence_or_another_protocol_is_not_dns() {
        for bytes in [&b""[..], &b"SSH-2.0-OpenSSH_9.6\r\n"[..], &[0, 12][..]] {
            assert_eq!(
                parse_reply(bytes, TRANSACTION_ID),
                AppProbeOutcome::NoAnswer
            );
        }
    }

    #[test]
    fn the_query_is_length_prefixed_and_asks_for_the_root() {
        let query = root_ns_query();
        let declared = u16::from_be_bytes([query[0], query[1]]) as usize;
        assert_eq!(declared, query.len() - LENGTH_PREFIX_LEN);
        assert_eq!(u16::from_be_bytes([query[2], query[3]]), TRANSACTION_ID);
        // QNAME is a single zero byte: the root.
        assert_eq!(query[LENGTH_PREFIX_LEN + 12], 0x00);
    }
}
