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
//! The message is built and read with [`picky_krb`] rather than by hand. This replaced 217 lines of
//! hand-written DER, and the parse is strictly stronger than what it replaced: the old code read the
//! outermost application tag and trusted the byte, where this decodes the whole `KRB-ERROR` down to
//! its `error-code`. A byte in the right place is cheap for something that is not a KDC to produce;
//! a well-formed `KRB-ERROR` is not.
//!
//! What the crate does *not* decide is which replies count. That judgement — that a refusal is the
//! positive result — stays here, which is why a session crate would have been the wrong shape: its
//! job is to turn `KDC_ERR_PREAUTH_REQUIRED` into an error, and here it is the answer.

use anyhow::Error;
use async_trait::async_trait;
use picky_asn1::bit_string::BitString;
use picky_asn1::date::Date;
use picky_asn1::restricted_string::Ia5String;
use picky_asn1::wrapper::{
    Asn1SequenceOf, BitStringAsn1, ExplicitContextTag0, ExplicitContextTag1, ExplicitContextTag2,
    ExplicitContextTag3, ExplicitContextTag4, ExplicitContextTag5, ExplicitContextTag7,
    ExplicitContextTag8, GeneralStringAsn1, IntegerAsn1, Optional,
};
use picky_krb::constants::types::{AS_REQ_MSG_TYPE, NT_PRINCIPAL, NT_SRV_INST};
use picky_krb::data_types::{KerberosTime, PrincipalName};
use picky_krb::messages::{AsRep, AsReq, KdcReq, KdcReqBody, KrbError};

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Kerberos over TCP prefixes every message with a four-byte length.
const LENGTH_PREFIX_LEN: usize = 4;

/// The realm and principal the request names. Neither is expected to exist; the point is to be
/// refused by something that speaks Kerberos.
const REALM: &str = "SCANOPY.INVALID";
const PRINCIPAL: &str = "scanopy-probe";

/// AES256-CTS-HMAC-SHA1-96, which every current KDC offers. One entry rather than the eight a real
/// client lists, because nothing here intends to encrypt anything.
const ETYPE_AES256_CTS_HMAC_SHA1_96: u8 = 18;

/// Enough for a `KRB-ERROR`, which is the reply this expects and runs to a few hundred bytes.
const READ_LIMIT: usize = 16_384;

/// A Kerberos name built from its components, e.g. `krbtgt/SCANOPY.INVALID`.
fn principal_name(name_type: u8, components: &[&str]) -> PrincipalName {
    PrincipalName {
        name_type: ExplicitContextTag0::from(IntegerAsn1(vec![name_type])),
        name_string: ExplicitContextTag1::from(Asn1SequenceOf::from(
            components
                .iter()
                .map(|c| {
                    GeneralStringAsn1::from(
                        Ia5String::from_string((*c).to_owned())
                            .expect("the realm and principal are ASCII literals"),
                    )
                })
                .collect::<Vec<_>>(),
        )),
    }
}

/// An `AS-REQ` for a principal that is not expected to exist, length-prefixed for TCP.
fn as_req() -> Vec<u8> {
    let request = AsReq::from(KdcReq {
        pvno: ExplicitContextTag1::from(IntegerAsn1(vec![5])),
        msg_type: ExplicitContextTag2::from(IntegerAsn1(vec![AS_REQ_MSG_TYPE])),
        // No pre-authentication data. Supplying it is what an authentication attempt looks like;
        // omitting it is what draws the refusal this is after.
        padata: Optional::from(None),
        req_body: ExplicitContextTag4::from(KdcReqBody {
            // KDCOptions with no options set beyond `renewable-ok`, as a client's first request has.
            kdc_options: ExplicitContextTag0::from(BitStringAsn1::from(BitString::with_bytes(
                vec![0, 0, 0, 16],
            ))),
            cname: Optional::from(Some(ExplicitContextTag1::from(principal_name(
                NT_PRINCIPAL,
                &[PRINCIPAL],
            )))),
            realm: ExplicitContextTag2::from(GeneralStringAsn1::from(
                Ia5String::from_string(REALM.to_owned()).expect("an ASCII literal"),
            )),
            sname: Optional::from(Some(ExplicitContextTag3::from(principal_name(
                NT_SRV_INST,
                &["krbtgt", REALM],
            )))),
            from: Optional::from(None),
            // A fixed absolute time in the future. Fixed rather than derived from the clock so the
            // request is byte-identical every run; a KDC that dislikes it still answers with a
            // `KRB-ERROR`, which is the outcome sought either way.
            till: ExplicitContextTag5::from(KerberosTime::from(
                Date::new(2037, 1, 1, 0, 0, 0).expect("a valid date"),
            )),
            rtime: Optional::from(None),
            nonce: ExplicitContextTag7::from(IntegerAsn1(vec![0x5C, 0xA1, 0x0B, 0x05])),
            etype: ExplicitContextTag8::from(Asn1SequenceOf::from(vec![IntegerAsn1(vec![
                ETYPE_AES256_CTS_HMAC_SHA1_96,
            ])])),
            addresses: Optional::from(None),
            enc_authorization_data: Optional::from(None),
            additional_tickets: Optional::from(None),
        }),
    });

    let message = picky_asn1_der::to_vec(&request).expect("the request is built from literals");
    let mut framed = (message.len() as u32).to_be_bytes().to_vec();
    framed.extend_from_slice(&message);
    framed
}

/// What a KDC sent back.
#[derive(Debug, PartialEq, Eq)]
enum KerberosReply {
    /// A `KRB-ERROR`, carrying the code that says why. `KDC_ERR_C_PRINCIPAL_UNKNOWN` (6) is the
    /// expected one; `KDC_ERR_PREAUTH_REQUIRED` (25) arrives if the principal happens to exist.
    Error { code: u32 },
    /// An `AS-REP`. Only reachable for a principal configured without pre-authentication, which the
    /// named principal is not expected to be.
    Ticket,
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
        let reply = request_response(ctx, self.port(), &as_req(), READ_LIMIT).await;
        Ok(parse_reply(&reply))
    }
}

fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    presence(decode(bytes).is_some())
}

/// Decode a length-prefixed Kerberos reply.
///
/// The declared length must account for what arrived — a reply that claims to be shorter than the
/// bytes on the wire is not a framed Kerberos message however its body decodes.
fn decode(bytes: &[u8]) -> Option<KerberosReply> {
    let prefix = bytes.get(..LENGTH_PREFIX_LEN)?;
    let declared = u32::from_be_bytes(prefix.try_into().ok()?) as usize;
    let body = bytes.get(LENGTH_PREFIX_LEN..)?;
    if declared == 0 || declared < body.len() {
        return None;
    }

    if let Ok(error) = picky_asn1_der::from_bytes::<KrbError>(body) {
        return Some(KerberosReply::Error {
            code: *error.0.error_code,
        });
    }
    picky_asn1_der::from_bytes::<AsRep>(body)
        .ok()
        .map(|_| KerberosReply::Ticket)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `KRB-ERROR` as a KDC would frame it, built with the same crate that reads it.
    fn krb_error(code: u32) -> Vec<u8> {
        use picky_asn1::wrapper::ExplicitContextTag6;
        use picky_asn1::wrapper::{ExplicitContextTag9, ExplicitContextTag10};
        use picky_krb::messages::KrbErrorInner;

        let inner = KrbErrorInner {
            pvno: ExplicitContextTag0::from(IntegerAsn1(vec![5])),
            msg_type: ExplicitContextTag1::from(IntegerAsn1(vec![30])),
            ctime: Optional::from(None),
            cusec: Optional::from(None),
            stime: ExplicitContextTag4::from(KerberosTime::from(
                Date::new(2026, 1, 1, 0, 0, 0).unwrap(),
            )),
            susec: ExplicitContextTag5::from(IntegerAsn1(vec![0])),
            error_code: ExplicitContextTag6::from(code),
            crealm: Optional::from(None),
            cname: Optional::from(None),
            realm: ExplicitContextTag9::from(GeneralStringAsn1::from(
                Ia5String::from_string("EXAMPLE.COM".to_owned()).unwrap(),
            )),
            sname: ExplicitContextTag10::from(principal_name(
                NT_SRV_INST,
                &["krbtgt", "EXAMPLE.COM"],
            )),
            e_text: Optional::from(None),
            e_data: Optional::from(None),
        };
        let message = picky_asn1_der::to_vec(&KrbError::from(inner)).unwrap();
        let mut framed = (message.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(&message);
        framed
    }

    /// The expected outcome: a KDC refusing a principal it does not know.
    #[test]
    fn a_krb_error_is_a_kdc_and_says_why() {
        const KDC_ERR_C_PRINCIPAL_UNKNOWN: u32 = 6;
        assert_eq!(
            decode(&krb_error(KDC_ERR_C_PRINCIPAL_UNKNOWN)),
            Some(KerberosReply::Error {
                code: KDC_ERR_C_PRINCIPAL_UNKNOWN
            })
        );
        assert_eq!(
            parse_reply(&krb_error(KDC_ERR_C_PRINCIPAL_UNKNOWN)),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A KDC that wants pre-authentication is as much a KDC as one that does not know the name.
    #[test]
    fn a_preauth_required_error_is_also_a_kdc() {
        const KDC_ERR_PREAUTH_REQUIRED: u32 = 25;
        assert_eq!(
            parse_reply(&krb_error(KDC_ERR_PREAUTH_REQUIRED)),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_an_echo_or_another_protocol_is_not_a_kdc() {
        let mut short_declaration = krb_error(6);
        short_declaration[0..4].copy_from_slice(&4u32.to_be_bytes());
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // Our own AS-REQ reflected back is a Kerberos message, but not a *reply*: it carries
            // application tag 10, and neither `KRB-ERROR` nor `AS-REP` decodes from it.
            &as_req(),
            &[0, 0, 0, 0][..],
            // A well-formed body whose frame claims fewer bytes than arrived.
            &short_declaration,
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer);
        }
    }

    /// A truncated reply is not accepted on the strength of its first bytes — which is exactly what
    /// the hand-written parser this replaced used to do.
    #[test]
    fn a_truncated_krb_error_does_not_decode() {
        let full = krb_error(6);
        let cut = &full[..full.len() - 8];
        assert_eq!(parse_reply(cut), AppProbeOutcome::NoAnswer);
    }

    #[test]
    fn the_request_is_length_prefixed_and_decodes_as_an_as_req() {
        let request = as_req();
        let declared = u32::from_be_bytes(request[0..4].try_into().unwrap()) as usize;
        assert_eq!(declared, request.len() - LENGTH_PREFIX_LEN);
        picky_asn1_der::from_bytes::<AsReq>(&request[LENGTH_PREFIX_LEN..])
            .expect("the request we send is a well-formed AS-REQ");
    }

    /// The principal named is one nothing is expected to match, so the exchange cannot be mistaken
    /// for an attempt to obtain a ticket — and no pre-authentication data is offered.
    #[test]
    fn the_request_names_a_principal_that_should_not_exist_and_offers_no_credential() {
        let request = as_req();
        let decoded = picky_asn1_der::from_bytes::<AsReq>(&request[LENGTH_PREFIX_LEN..]).unwrap();
        assert!(
            decoded.0.padata.0.is_none(),
            "no pre-authentication data is sent"
        );
        let body = &decoded.0.req_body.0;
        assert_eq!(body.realm.0.to_string(), REALM);
        let cname = body.cname.0.as_ref().expect("a client name is sent");
        assert_eq!(cname.0.name_string.0.0[0].to_string(), PRINCIPAL);
    }
}
