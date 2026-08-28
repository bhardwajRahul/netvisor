//! LDAP detection with an anonymous simple bind.
//!
//! A bind is the first operation of every LDAP session, and a directory answers one whether or not
//! it permits anonymous access: refusing takes a `bindResponse` carrying result code 48
//! (`inappropriateAuthentication`) or 53 (`unwillingToPerform`), which is a parsed LDAP reply and
//! therefore proof of a directory. The result code is deliberately not inspected.
//!
//! BER by hand rather than a crate: the request is fourteen fixed bytes and the reply needs only its
//! outer tags read, which is less code than wiring a parser in and far less than a dependency.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Universal SEQUENCE, which wraps every `LDAPMessage`.
const TAG_SEQUENCE: u8 = 0x30;
/// Universal INTEGER, the message id.
const TAG_INTEGER: u8 = 0x02;
/// `[APPLICATION 1]` — `bindResponse`.
const TAG_BIND_RESPONSE: u8 = 0x61;

/// The message id we send and expect echoed.
const MESSAGE_ID: u8 = 0x01;

/// An anonymous simple bind: LDAP v3, empty DN, empty password.
const BIND_REQUEST: [u8; 14] = [
    TAG_SEQUENCE,
    0x0C, // length of everything following
    TAG_INTEGER,
    0x01,
    MESSAGE_ID,
    0x60, // [APPLICATION 0] bindRequest
    0x07, // length
    TAG_INTEGER,
    0x01,
    0x03, // version 3
    0x04,
    0x00, // name: empty OCTET STRING
    0x80,
    0x00, // [0] simple authentication, empty
];

pub struct LdapProbe;

#[async_trait]
impl AppProbe for LdapProbe {
    fn port(&self) -> PortType {
        PortType::Ldap
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Ldap)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &BIND_REQUEST, 512).await;
        Ok(parse_reply(&reply, MESSAGE_ID))
    }
}

/// Whether the reply is a `bindResponse` to our message.
///
/// Reads only the outer structure: SEQUENCE, then the message id, then the `bindResponse` tag. The
/// message id is checked so an unsolicited notice (which LDAP sends with id 0) is not read as an
/// answer to us.
fn parse_reply(bytes: &[u8], message_id: u8) -> AppProbeOutcome {
    // 0: SEQUENCE, 1: length, 2: INTEGER, 3: length 1, 4: the id, 5: the operation's tag.
    let Some(prefix) = bytes.get(..6) else {
        return AppProbeOutcome::NoAnswer;
    };
    presence(
        prefix[0] == TAG_SEQUENCE
            && prefix[2] == TAG_INTEGER
            && prefix[3] == 0x01
            && prefix[4] == message_id
            && prefix[5] == TAG_BIND_RESPONSE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bind_response(message_id: u8, result_code: u8) -> Vec<u8> {
        vec![
            TAG_SEQUENCE,
            0x0C,
            TAG_INTEGER,
            0x01,
            message_id,
            TAG_BIND_RESPONSE,
            0x07,
            0x0A, // ENUMERATED
            0x01,
            result_code,
            0x04,
            0x00, // matchedDN
            0x04,
            0x00, // diagnosticMessage
        ]
    }

    #[test]
    fn a_successful_bind_is_ldap() {
        assert_eq!(
            parse_reply(&bind_response(MESSAGE_ID, 0), MESSAGE_ID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A directory refusing anonymous access parsed our request to refuse it.
    #[test]
    fn a_refused_bind_is_ldap() {
        for result_code in [48, 53] {
            assert_eq!(
                parse_reply(&bind_response(MESSAGE_ID, result_code), MESSAGE_ID),
                AppProbeOutcome::Answered { identity: None }
            );
        }
    }

    /// An unsolicited notice carries message id 0 and is not an answer to our bind.
    #[test]
    fn an_unsolicited_notice_is_not_evidence() {
        assert_eq!(
            parse_reply(&bind_response(0x00, 0), MESSAGE_ID),
            AppProbeOutcome::NoAnswer
        );
    }

    #[test]
    fn silence_or_another_protocol_is_not_ldap() {
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &[TAG_SEQUENCE, 0x0C, TAG_INTEGER, 0x01, MESSAGE_ID][..],
            // A SEQUENCE whose operation is not a bindResponse.
            &[TAG_SEQUENCE, 0x0C, TAG_INTEGER, 0x01, MESSAGE_ID, 0x65][..],
        ] {
            assert_eq!(parse_reply(reply, MESSAGE_ID), AppProbeOutcome::NoAnswer);
        }
    }

    #[test]
    fn the_bind_request_is_a_well_formed_ldap_message() {
        assert_eq!(BIND_REQUEST[0], TAG_SEQUENCE);
        assert_eq!(
            BIND_REQUEST[1] as usize,
            BIND_REQUEST.len() - 2,
            "the outer length counts everything after it"
        );
        assert_eq!(BIND_REQUEST[9], 0x03, "LDAP version 3");
    }
}
