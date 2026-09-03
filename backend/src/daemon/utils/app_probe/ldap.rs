//! LDAP detection with an anonymous simple bind.
//!
//! A bind is the first operation of every LDAP session, and a directory answers one whether or not
//! it permits anonymous access: refusing takes a `bindResponse` carrying result code 48
//! (`inappropriateAuthentication`) or 53 (`unwillingToPerform`), which is a parsed LDAP reply and
//! therefore proof of a directory. The result code is deliberately not inspected.
//!
//! [`rasn_ldap`] supplies the types and `rasn` the BER codec, so the request is an `LdapMessage`
//! rather than fourteen hand-counted bytes and the reply is decoded rather than read at offsets.
//! The old check looked at bytes 0, 2, 3, 4 and 5, which is correct only while every length fits in
//! the short form — true for the reply a small directory sends and not true in general. A real
//! decode has no such ceiling.
//!
//! `rasn-ldap` rather than `ldap3`: the latter owns the socket and returns a refused bind as an
//! error, where here the refusal is the positive result. Which result codes count stays decided
//! below — and the answer is all of them, because any of them means a directory parsed the request.

use anyhow::Error;
use async_trait::async_trait;
use rasn_ldap::{
    AuthenticationChoice, BindRequest, LdapMessage, LdapString, MessageId, ProtocolOp, ResultCode,
};

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The message id we send and expect echoed. Not 0, which is reserved for the unsolicited notice a
/// server sends on its own initiative — that is not an answer to us.
const MESSAGE_ID: MessageId = 1;

/// The version this speaks. A directory that does not support it must answer `protocolError`, which
/// is still an answer.
const LDAP_V3: u8 = 3;

/// An anonymous simple bind: LDAP v3, empty DN, empty password.
fn bind_request() -> Vec<u8> {
    let message = LdapMessage::new(
        MESSAGE_ID,
        ProtocolOp::BindRequest(BindRequest::new(
            LDAP_V3,
            LdapString(String::new()),
            AuthenticationChoice::Simple(Vec::new().into()),
        )),
    );
    rasn::ber::encode(&message).expect("the request is built from literals")
}

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
        let reply = request_response(ctx, self.port(), &bind_request(), 512).await;
        Ok(parse_reply(&reply, MESSAGE_ID))
    }
}

/// Whether the reply is a `bindResponse` to our message.
fn parse_reply(bytes: &[u8], message_id: MessageId) -> AppProbeOutcome {
    presence(bind_result(bytes, message_id).is_some())
}

/// The result code a directory answered our bind with, if it answered one.
fn bind_result(bytes: &[u8], message_id: MessageId) -> Option<ResultCode> {
    let message = rasn::ber::decode::<LdapMessage>(bytes).ok()?;
    if message.message_id != message_id {
        return None;
    }
    match message.protocol_op {
        ProtocolOp::BindResponse(response) => Some(response.result_code),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rasn_ldap::BindResponse;

    fn bind_response(message_id: MessageId, result_code: ResultCode) -> Vec<u8> {
        let message = LdapMessage::new(
            message_id,
            ProtocolOp::BindResponse(BindResponse::new(
                result_code,
                LdapString(String::new()),
                LdapString(String::new()),
                None,
                None,
            )),
        );
        rasn::ber::encode(&message).unwrap()
    }

    #[test]
    fn a_successful_bind_is_ldap() {
        assert_eq!(
            parse_reply(&bind_response(MESSAGE_ID, ResultCode::Success), MESSAGE_ID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A directory refusing anonymous access parsed our request to refuse it. This is the case a
    /// client crate would have handed back as an error rather than as a detection.
    #[test]
    fn a_refused_bind_is_ldap() {
        for result_code in [
            ResultCode::InappropriateAuthentication,
            ResultCode::UnwillingToPerform,
        ] {
            assert_eq!(
                parse_reply(&bind_response(MESSAGE_ID, result_code), MESSAGE_ID),
                AppProbeOutcome::Answered { identity: None },
                "{result_code:?}"
            );
            assert_eq!(
                bind_result(&bind_response(MESSAGE_ID, result_code), MESSAGE_ID),
                Some(result_code)
            );
        }
    }

    /// An unsolicited notice carries message id 0 and is not an answer to our bind.
    #[test]
    fn an_unsolicited_notice_is_not_evidence() {
        assert_eq!(
            parse_reply(&bind_response(0, ResultCode::Success), MESSAGE_ID),
            AppProbeOutcome::NoAnswer
        );
    }

    #[test]
    fn silence_a_request_or_another_protocol_is_not_ldap() {
        let response = bind_response(MESSAGE_ID, ResultCode::Success);
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // Our own bind request reflected back: a valid LdapMessage, but not a bindResponse.
            &bind_request(),
            // A BER prefix with nothing behind it.
            &[0x30, 0x0C, 0x02, 0x01, 0x01][..],
            // Truncated mid-message, which the offset check this replaced accepted.
            &response[..response.len() - 3],
        ] {
            assert_eq!(
                parse_reply(reply, MESSAGE_ID),
                AppProbeOutcome::NoAnswer,
                "{reply:?}"
            );
        }
    }

    /// The bind carries no name and no password, so it cannot be mistaken for a credential attempt.
    #[test]
    fn the_bind_request_is_anonymous() {
        let decoded = rasn::ber::decode::<LdapMessage>(&bind_request()).expect("we send valid BER");
        assert_eq!(decoded.message_id, MESSAGE_ID);
        let ProtocolOp::BindRequest(request) = decoded.protocol_op else {
            panic!("we send a bind request");
        };
        assert_eq!(request.version, LDAP_V3);
        assert!(request.name.0.is_empty(), "no bind DN");
        assert_eq!(
            request.authentication,
            AuthenticationChoice::Simple(Vec::new().into()),
            "an empty simple password"
        );
    }
}
