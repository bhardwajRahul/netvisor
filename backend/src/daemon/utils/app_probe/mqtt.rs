//! MQTT detection by CONNECT / CONNACK.
//!
//! A broker answers a CONNECT before authenticating: if credentials are required and absent, the
//! CONNACK carries return code `0x05` (not authorized) rather than silence. Both are CONNACKs and
//! both identify a broker, so the return code is deliberately not checked — a broker that refuses
//! us is still a broker, which is the claim the definition makes.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// CONNACK, in the upper nibble of the fixed header's first byte.
const CONNACK: u8 = 0x20;
/// The remaining-length byte of a CONNACK, which is always 2.
const CONNACK_REMAINING_LEN: u8 = 0x02;

/// A CONNECT for protocol level 4 (MQTT 3.1.1), clean session, no credentials, no will.
///
/// 3.1.1 rather than 5.0 because a 5.0 broker answers a 3.1.1 CONNECT with a CONNACK carrying
/// "unacceptable protocol version" where a 3.1.1 broker would reject a 5.0 one outright. Either
/// way a CONNACK comes back, which is all this needs.
fn connect_packet() -> Vec<u8> {
    let client_id = b"scanopy";
    let mut variable = Vec::new();
    variable.extend_from_slice(&[0x00, 0x04]); // protocol name length
    variable.extend_from_slice(b"MQTT");
    variable.push(0x04); // protocol level 4
    variable.push(0x02); // connect flags: clean session
    variable.extend_from_slice(&[0x00, 0x3C]); // keep-alive 60s
    variable.extend_from_slice(&(client_id.len() as u16).to_be_bytes());
    variable.extend_from_slice(client_id);

    let mut packet = vec![0x10]; // CONNECT
    packet.push(variable.len() as u8); // remaining length, one byte for this size
    packet.extend_from_slice(&variable);
    packet
}

pub struct MqttProbe;

#[async_trait]
impl AppProbe for MqttProbe {
    fn port(&self) -> PortType {
        PortType::Mqtt
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Mqtt)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &connect_packet(), 64).await;
        Ok(parse_connack(&reply))
    }
}

/// Whether the reply is a CONNACK.
///
/// Four bytes: packet type, remaining length 2, acknowledge flags, return code. The remaining
/// length is checked because it is fixed by the specification, which is what makes `0x20` a packet
/// type here rather than an arbitrary first byte.
fn parse_connack(bytes: &[u8]) -> AppProbeOutcome {
    presence(matches!(bytes, [CONNACK, CONNACK_REMAINING_LEN, _, _, ..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_connack_is_mqtt() {
        // Accepted.
        assert_eq!(
            parse_connack(&[0x20, 0x02, 0x00, 0x00]),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A broker requiring credentials refuses with a return code and is still a broker.
    #[test]
    fn a_refusing_connack_is_mqtt() {
        for code in [0x01, 0x04, 0x05] {
            assert_eq!(
                parse_connack(&[0x20, 0x02, 0x00, code]),
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    #[test]
    fn silence_or_another_protocol_is_not_mqtt() {
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // Right first byte, wrong remaining length: not a CONNACK.
            &[0x20, 0x0A, 0x00, 0x00][..],
            &[0x20, 0x02][..],
            &[0x30, 0x02, 0x00, 0x00][..],
        ] {
            assert_eq!(parse_connack(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }

    #[test]
    fn the_connect_packet_is_well_formed() {
        let packet = connect_packet();
        assert_eq!(packet[0], 0x10, "CONNECT packet type");
        assert_eq!(
            packet[1] as usize,
            packet.len() - 2,
            "remaining length counts everything after the header"
        );
        assert_eq!(&packet[4..8], b"MQTT");
    }
}
