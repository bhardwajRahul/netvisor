//! ZeroMQ detection by the ZMTP greeting, which is sent in cleartext before any encryption.
//!
//! Salt's 4505 and 4506 were declared `NoDistinguishingHandshake` on the grounds that CurveZMQ
//! encrypts everything an unauthenticated peer could see. That conflated *the payload is encrypted*
//! with *there is no distinguishing exchange*. [RFC 23](https://rfc.zeromq.org/spec/23/) opens every
//! connection with a 64-octet greeting, sent before any mechanism runs, and says of the security
//! mechanism it names: "this is not considered sensitive information".
//!
//! ```text
//! signature  10 octets   0xFF, an 8-octet length, 0x7F
//! version     2 octets   major, minor
//! mechanism  20 octets   uppercase ASCII, NUL-padded: NULL, PLAIN, CURVE
//! as-server   1 octet
//! filler     31 octets
//! ```
//!
//! A middlebox that completes a TCP handshake cannot produce that. The signature alone is a weak
//! guard — two bytes at fixed offsets — so the version and the mechanism name are checked too, and
//! the mechanism is where the real content is: `CURVE` on Salt's ports says the master is running
//! with encryption on, and neither `NULL` nor `CURVE` is a byte pattern a silent listener emits.
//!
//! **Hand-rolled, and this is the one place in this directory where that is the right answer.**
//! `zmq` binds libzmq, which is disqualified on the static musl build. `zeromq` (zmq.rs) is pure
//! Rust but is a *socket* library: its `connect()` runs the whole handshake, which against CurveZMQ
//! always fails, collapsing "spoke ZMTP and then refused us" into one opaque `ZmqError` — the exact
//! shape that destroys a refusal-as-positive probe. The greeting is 64 fixed octets and yields more
//! read by hand than either crate would give.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_exact,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The greeting is exactly this long, in both directions.
const GREETING_LEN: usize = 64;
/// First octet of the signature.
const SIGNATURE_HEAD: u8 = 0xFF;
/// Tenth octet of the signature.
const SIGNATURE_TAIL: u8 = 0x7F;
/// Where the 20-octet mechanism name starts.
const MECHANISM_OFFSET: usize = 12;
const MECHANISM_LEN: usize = 20;

/// ZMTP 3.0, which every libzmq since 4.0 speaks — and so every Salt master.
const VERSION_MAJOR: u8 = 3;
const VERSION_MINOR: u8 = 1;

/// The greeting this sends. `NULL` because nothing here intends to authenticate; a peer configured
/// for `CURVE` still sends its own greeting first and refuses afterwards, which is the answer.
fn greeting() -> Vec<u8> {
    let mut out = Vec::with_capacity(GREETING_LEN);
    out.push(SIGNATURE_HEAD);
    // libzmq puts `routing_id_size + 1` here as a big-endian u64; with no routing id that is 1.
    out.extend_from_slice(&1u64.to_be_bytes());
    out.push(SIGNATURE_TAIL);
    out.push(VERSION_MAJOR);
    out.push(VERSION_MINOR);

    let mut mechanism = [0u8; MECHANISM_LEN];
    mechanism[..4].copy_from_slice(b"NULL");
    out.extend_from_slice(&mechanism);

    out.push(0x00); // as-server: we are the client
    out.extend_from_slice(&[0u8; 31]); // filler
    debug_assert_eq!(out.len(), GREETING_LEN);
    out
}

/// A probe for one of the ports a ZeroMQ service listens on.
pub struct ZmtpProbe {
    port: PortType,
}

impl ZmtpProbe {
    pub fn new(port: u16) -> Self {
        Self {
            port: PortType::new_tcp(port),
        }
    }
}

#[async_trait]
impl AppProbe for ZmtpProbe {
    fn port(&self) -> PortType {
        self.port
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Zmtp)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        // `request_exact` rather than a single read: a peer may send the signature, wait for ours,
        // and only then send the rest. Salt's 4506 does exactly that where its 4505 does not, so a
        // one-read probe recognised half a real Salt master.
        let reply = request_exact(ctx, self.port(), &greeting(), GREETING_LEN).await;
        Ok(parse_greeting(&reply))
    }
}

/// Whether the reply is a ZMTP greeting.
fn parse_greeting(bytes: &[u8]) -> AppProbeOutcome {
    presence(mechanism(bytes).is_some())
}

/// The security mechanism the peer named, if what it sent was a greeting at all.
///
/// The whole 64 octets are required. A peer that sent fewer did not send a greeting, and accepting a
/// prefix would put this back where the connect-only detection was: matching on too little.
fn mechanism(bytes: &[u8]) -> Option<String> {
    if bytes.len() < GREETING_LEN
        || bytes[0] != SIGNATURE_HEAD
        || bytes[9] != SIGNATURE_TAIL
        // ZMTP 1 and 2 predate the mechanism field entirely; nothing in circulation speaks them.
        || bytes[10] < VERSION_MAJOR
    {
        return None;
    }

    let field = &bytes[MECHANISM_OFFSET..MECHANISM_OFFSET + MECHANISM_LEN];
    let name = field.split(|b| *b == 0).next().unwrap_or_default();
    // RFC 23 defines the field as uppercase ASCII, NUL-padded to 20 octets. Requiring that is what
    // separates a greeting from 64 arbitrary bytes whose 1st and 10th happen to match.
    if name.is_empty()
        || !name
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'-')
        || field[name.len()..].iter().any(|b| *b != 0)
    {
        return None;
    }
    String::from_utf8(name.to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer_greeting(mechanism_name: &[u8], major: u8) -> Vec<u8> {
        let mut out = greeting();
        out[10] = major;
        let field = &mut out[MECHANISM_OFFSET..MECHANISM_OFFSET + MECHANISM_LEN];
        field.fill(0);
        field[..mechanism_name.len()].copy_from_slice(mechanism_name);
        out[MECHANISM_OFFSET + MECHANISM_LEN] = 0x01; // as-server
        out
    }

    #[test]
    fn a_greeting_names_the_security_mechanism() {
        assert_eq!(
            mechanism(&peer_greeting(b"NULL", 3)).as_deref(),
            Some("NULL")
        );
        assert_eq!(
            mechanism(&peer_greeting(b"CURVE", 3)).as_deref(),
            Some("CURVE")
        );
        assert_eq!(
            parse_greeting(&peer_greeting(b"PLAIN", 3)),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// The case the exception claimed was impossible: a master running CurveZMQ still says so in
    /// cleartext before refusing us.
    #[test]
    fn an_encrypted_peer_still_greets_before_it_refuses() {
        assert_eq!(
            parse_greeting(&peer_greeting(b"CURVE", 3)),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_a_truncated_greeting_or_another_protocol_is_not_zmtp() {
        let full = peer_greeting(b"NULL", 3);
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // The signature bytes and nothing else: the shape a naive check would accept.
            &full[..10],
            &full[..GREETING_LEN - 1],
            // 64 bytes whose 1st and 10th octets match but which carry no mechanism name.
            &{
                let mut bogus = vec![0u8; GREETING_LEN];
                bogus[0] = SIGNATURE_HEAD;
                bogus[9] = SIGNATURE_TAIL;
                bogus[10] = 3;
                bogus
            }[..],
            // A mechanism field that is not the uppercase ASCII the RFC defines.
            &peer_greeting(b"null", 3),
            // ZMTP 2, which has no mechanism field at all.
            &peer_greeting(b"NULL", 2),
        ] {
            assert_eq!(
                parse_greeting(reply),
                AppProbeOutcome::NoAnswer,
                "{reply:?}"
            );
        }
    }

    #[test]
    fn the_greeting_we_send_is_a_well_formed_one() {
        let sent = greeting();
        assert_eq!(sent.len(), GREETING_LEN);
        assert_eq!(mechanism(&sent).as_deref(), Some("NULL"));
        assert_eq!(
            sent[MECHANISM_OFFSET + MECHANISM_LEN],
            0x00,
            "we connect as a client, not as a server"
        );
    }
}
