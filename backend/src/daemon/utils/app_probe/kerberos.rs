//! Kerberos KDC detection with an `AS-REQ` that is meant to be refused.
//!
//! A KDC answers an `AS-REQ` from anyone, because that is the message a client sends before it has
//! any ticket to authenticate with. Asking for a principal that does not exist draws
//! `KDC_ERR_C_PRINCIPAL_UNKNOWN`; asking for one that does draws `KDC_ERR_PREAUTH_REQUIRED`. Either
//! way what comes back is a `KRB-ERROR`, which is a Kerberos message and therefore proof of a KDC.
//!
//! **The refusal is the whole point.** Nothing here tries to obtain a ticket, and the request
//! deliberately names a principal nothing will match, so this cannot be mistaken for an
//! authentication attempt: no password is guessed, nothing is decrypted, and the only outcome
//! sought is an error.
//!
//! DER by hand rather than a crate. The request is fixed apart from the realm and principal names,
//! and the reply needs only its outermost application tag read, so a parser would be more code than
//! this and a dependency more still.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// `[APPLICATION 30]` — `KRB-ERROR`, which is what an unauthenticated `AS-REQ` earns.
const TAG_KRB_ERROR: u8 = 0x7E;
/// `[APPLICATION 11]` — `AS-REP`, in the unusual case that a principal needs no pre-authentication.
const TAG_AS_REP: u8 = 0x6B;
/// Kerberos over TCP prefixes every message with a four-byte length.
const LENGTH_PREFIX_LEN: usize = 4;

/// The realm and principal the request names. Neither is expected to exist; the point is to be
/// refused by something that speaks Kerberos.
const REALM: &[u8] = b"SCANOPY.INVALID";
const PRINCIPAL: &[u8] = b"scanopy-probe";

/// A DER length header for a body of `len` bytes.
fn der_len(len: usize) -> Vec<u8> {
    if len < 0x80 {
        vec![len as u8]
    } else if len < 0x100 {
        vec![0x81, len as u8]
    } else {
        vec![0x82, (len >> 8) as u8, len as u8]
    }
}

/// A DER element: tag, length, body.
fn der(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    out.extend_from_slice(&der_len(body.len()));
    out.extend_from_slice(body);
    out
}

/// A `GeneralString`, which is how Kerberos carries realm and name components.
fn general_string(value: &[u8]) -> Vec<u8> {
    der(0x1B, value)
}

/// `PrincipalName ::= SEQUENCE { name-type [0] Int32, name-string [1] SEQUENCE OF GeneralString }`
fn principal_name(name_type: u8, components: &[&[u8]]) -> Vec<u8> {
    let mut strings = Vec::new();
    for component in components {
        strings.extend_from_slice(&general_string(component));
    }
    let mut body = der(0xA0, &der(0x02, &[name_type]));
    body.extend_from_slice(&der(0xA1, &der(0x30, &strings)));
    der(0x30, &body)
}

/// An `AS-REQ` for a principal that is not expected to exist.
fn as_req() -> Vec<u8> {
    // KDC-REQ-BODY
    let mut body = Vec::new();
    // kdc-options [0]: a BIT STRING with no options set.
    body.extend_from_slice(&der(
        0xA0,
        &der(0x03, &[0x05, 0x00, 0x00, 0x00, 0x00, 0x10]),
    ));
    // cname [1]
    body.extend_from_slice(&der(0xA1, &principal_name(1, &[PRINCIPAL])));
    // realm [2]
    body.extend_from_slice(&der(0xA2, &general_string(REALM)));
    // sname [3]: krbtgt/REALM
    body.extend_from_slice(&der(0xA3, &principal_name(2, &[b"krbtgt", REALM])));
    // till [5]: a fixed absolute time, well past any clock skew check that matters here.
    body.extend_from_slice(&der(0xA5, &der(0x18, b"19700101000000Z")));
    // nonce [7]
    body.extend_from_slice(&der(0xA7, &der(0x02, &[0x5C, 0xA1, 0x0B, 0x05])));
    // etype [8]: AES256-CTS-HMAC-SHA1-96 (18), which every current KDC offers.
    body.extend_from_slice(&der(0xA8, &der(0x30, &der(0x02, &[0x12]))));
    let kdc_req_body = der(0x30, &body);

    // KDC-REQ
    let mut request = Vec::new();
    request.extend_from_slice(&der(0xA1, &der(0x02, &[0x05]))); // pvno 5
    request.extend_from_slice(&der(0xA2, &der(0x02, &[0x0A]))); // msg-type 10: AS-REQ
    request.extend_from_slice(&der(0xA4, &kdc_req_body));
    // [APPLICATION 10] AS-REQ
    let message = der(0x6A, &der(0x30, &request));

    let mut framed = (message.len() as u32).to_be_bytes().to_vec();
    framed.extend_from_slice(&message);
    framed
}

pub struct KerberosProbe;

#[async_trait]
impl AppProbe for KerberosProbe {
    fn port(&self) -> PortType {
        PortType::Kerberos
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Kerberos)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &as_req(), 2048).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is a Kerberos message.
///
/// Only the application tag after the length prefix is read. The declared length is checked against
/// what arrived, which is what makes `0x7E` a Kerberos tag here rather than a byte that happens to
/// be in the right place.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    let Some(prefix) = bytes.get(..LENGTH_PREFIX_LEN) else {
        return AppProbeOutcome::NoAnswer;
    };
    let declared = u32::from_be_bytes(prefix.try_into().unwrap_or_default()) as usize;
    let tag = bytes.get(LENGTH_PREFIX_LEN);

    // The read is capped, so a long reply may be truncated, but the length must account for at
    // least what arrived and be a plausible message size.
    presence(
        declared > 0
            && declared >= bytes.len() - LENGTH_PREFIX_LEN
            && matches!(tag, Some(&TAG_KRB_ERROR) | Some(&TAG_AS_REP)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed(tag: u8, body_len: usize) -> Vec<u8> {
        let body = vec![0xAA; body_len];
        let message = der(tag, &body);
        let mut out = (message.len() as u32).to_be_bytes().to_vec();
        out.extend_from_slice(&message);
        out
    }

    /// The expected outcome: a KDC refusing a principal it does not know.
    #[test]
    fn a_krb_error_is_a_kdc() {
        assert_eq!(
            parse_reply(&framed(TAG_KRB_ERROR, 40)),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A principal configured without pre-authentication answers with a ticket instead. Still a KDC.
    #[test]
    fn an_as_rep_is_a_kdc() {
        assert_eq!(
            parse_reply(&framed(TAG_AS_REP, 40)),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_an_echo_or_another_protocol_is_not_a_kdc() {
        let mut trailing = framed(TAG_KRB_ERROR, 4);
        trailing.extend_from_slice(&[0xFF; 40]);
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // Our own AS-REQ reflected back carries the request's application tag.
            &as_req(),
            &[0, 0, 0, 0][..],
            &trailing,
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer);
        }
    }

    #[test]
    fn the_request_is_length_prefixed_and_tagged_as_an_as_req() {
        let request = as_req();
        let declared = u32::from_be_bytes(request[0..4].try_into().unwrap()) as usize;
        assert_eq!(declared, request.len() - LENGTH_PREFIX_LEN);
        assert_eq!(request[LENGTH_PREFIX_LEN], 0x6A, "[APPLICATION 10] AS-REQ");
    }

    /// The principal named is one nothing is expected to match, so the exchange cannot be mistaken
    /// for an attempt to obtain a ticket.
    #[test]
    fn the_request_names_a_principal_that_should_not_exist() {
        let request = as_req();
        assert!(request.windows(PRINCIPAL.len()).any(|w| w == PRINCIPAL));
        assert!(request.windows(REALM.len()).any(|w| w == REALM));
    }

    #[test]
    fn der_lengths_use_the_shortest_form_that_fits() {
        assert_eq!(der_len(0x7F), vec![0x7F]);
        assert_eq!(der_len(0x80), vec![0x81, 0x80]);
        assert_eq!(der_len(0x1234), vec![0x82, 0x12, 0x34]);
    }
}
