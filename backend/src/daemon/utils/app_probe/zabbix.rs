//! Zabbix agent detection with a passive check.
//!
//! The agent answers passive checks from whatever address its `Server=` list allows, and
//! `agent.ping` is the cheapest of them. What identifies the agent is not the value it returns but
//! the framing around it: every response carries the `ZBXD` header, a protocol version byte and a
//! little-endian payload length.
//!
//! The body is deliberately not inspected: an agent that knows the item but declines it answers
//! `ZBX_NOTSUPPORTED` inside the same framing, and that is still an agent.
//!
//! **An agent whose `Server=` list does not include the scanner is undetectable, and that is a real
//! loss.** Verified against `zabbix/zabbix-agent`: a connection from an address outside the list is
//! accepted and then closed with nothing sent (`connection from "…" rejected, allowed hosts: …` in
//! its log), which is byte-for-byte what a firewall session helper does. There is no way to tell
//! them apart, so this reports `NoAnswer` for both. Before this probe the open port alone was
//! enough, so such an agent was detected — at the cost of every intercepted address being detected
//! too. Adding the daemon's address to the agent's `Server=` list restores it.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The header every framed Zabbix message carries.
const ZBXD: &[u8] = b"ZBXD";
/// Protocol flag 0x01 is the plain (uncompressed) framing.
const ZBXD_PLAIN: u8 = 0x01;
/// `ZBXD` + flags + 8-byte length.
const HEADER_LEN: usize = 13;

const AGENT_PING: &[u8] = b"agent.ping";

pub struct ZabbixAgentProbe;

#[async_trait]
impl AppProbe for ZabbixAgentProbe {
    fn port(&self) -> PortType {
        PortType::new_tcp(10050)
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::ZabbixAgent)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &framed(AGENT_PING), 256).await;
        Ok(parse_reply(&reply))
    }
}

/// Wrap a payload in the `ZBXD` framing an agent expects.
fn framed(payload: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(HEADER_LEN + payload.len());
    message.extend_from_slice(ZBXD);
    message.push(ZBXD_PLAIN);
    message.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    message.extend_from_slice(payload);
    message
}

/// Whether the reply carries the agent's framing.
///
/// The declared length is checked against what arrived, which is what makes this a framing match
/// rather than four bytes that happen to spell `ZBXD`.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    let Some(header) = bytes.get(..HEADER_LEN) else {
        return AppProbeOutcome::NoAnswer;
    };
    if !header.starts_with(ZBXD) {
        return AppProbeOutcome::NoAnswer;
    }

    let declared = u64::from_le_bytes(header[5..13].try_into().unwrap_or_default());
    presence(declared as usize == bytes.len() - HEADER_LEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_framed_reply_is_a_zabbix_agent() {
        let mut reply = framed(b"1");
        assert_eq!(
            parse_reply(&reply),
            AppProbeOutcome::Answered { identity: None }
        );
        reply = framed(b"ZBX_NOTSUPPORTED\0Unsupported item key.");
        assert_eq!(
            parse_reply(&reply),
            AppProbeOutcome::Answered { identity: None },
            "an agent refusing the item is still an agent"
        );
    }

    #[test]
    fn silence_or_a_length_that_disagrees_is_not_a_zabbix_agent() {
        let mut wrong_length = framed(b"1");
        wrong_length.extend_from_slice(b"trailing");
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"ZBXD"[..],
            &wrong_length,
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer);
        }
    }

    #[test]
    fn the_request_is_framed_with_its_own_length() {
        let request = framed(AGENT_PING);
        assert_eq!(&request[..4], ZBXD);
        assert_eq!(request[4], ZBXD_PLAIN);
        assert_eq!(
            u64::from_le_bytes(request[5..13].try_into().unwrap()) as usize,
            AGENT_PING.len()
        );
    }
}
