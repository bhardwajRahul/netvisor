//! Bacula Director detection by the CRAM-MD5 challenge it issues before any credential.
//!
//! 9101 was declared `NoDistinguishingHandshake` on the grounds that the Director authenticates
//! before saying anything that names Bacula. That is not what happens. The Director answers a
//! `Hello` with the *challenge*, in plaintext, and the challenge line names the algorithm:
//!
//! ```text
//! → Hello *UserAgent* calling
//! ← auth cram-md5 <9349290.1788400000@bacula-dir> ssl=0
//! ```
//!
//! **The challenge is a challenge, not a credential.** Nothing here answers it: no digest is
//! computed, no shared secret is guessed, and the connection is closed as soon as the line arrives.
//! What it proves is that something on 9101 implements Bacula's authentication handshake, which a
//! middlebox completing a TCP handshake does not.
//!
//! Every message is framed with a 4-octet big-endian **signed** length. Negative values are Bacula's
//! signals (`BNET_EOD` and friends) rather than lengths, which is why the length is read as `i32`
//! and a negative one is not treated as an enormous message.
//!
//! Hand-rolled: there is no Bacula codec on crates.io at any level of quality, and the framing is
//! one signed integer.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// What a console sends to open a session. `*UserAgent*` is Bacula's own name for the console
/// client, so this is the greeting the Director expects rather than an invented one.
const HELLO: &[u8] = b"Hello *UserAgent* calling\n";

/// The Director's answer. `cram-md5` is the only algorithm Bacula's handshake names here.
const CHALLENGE_PREFIX: &[u8] = b"auth cram-md5 ";

/// A challenge line runs to well under this; the cap only bounds a peer that talks at length.
const READ_LIMIT: usize = 512;

/// Length prefix (4) plus the shortest thing that could be a challenge.
const LENGTH_PREFIX_LEN: usize = 4;

fn hello() -> Vec<u8> {
    let mut out = (HELLO.len() as i32).to_be_bytes().to_vec();
    out.extend_from_slice(HELLO);
    out
}

pub struct BaculaProbe;

#[async_trait]
impl AppProbe for BaculaProbe {
    fn port(&self) -> PortType {
        PortType::new_tcp(9101)
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Bacula)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let reply = request_response(ctx, self.port(), &hello(), READ_LIMIT).await;
        Ok(parse_reply(&reply))
    }
}

/// Whether the reply is a Bacula CRAM-MD5 challenge.
fn parse_reply(bytes: &[u8]) -> AppProbeOutcome {
    presence(challenge(bytes).is_some())
}

/// The challenge line, unframed, if the peer sent one.
fn challenge(bytes: &[u8]) -> Option<&[u8]> {
    let prefix = bytes.get(..LENGTH_PREFIX_LEN)?;
    // Signed: Bacula's negative values are signals, not lengths.
    let declared = i32::from_be_bytes(prefix.try_into().ok()?);
    let body = bytes.get(LENGTH_PREFIX_LEN..)?;
    if declared <= 0 || declared as usize != body.len() {
        return None;
    }
    body.starts_with(CHALLENGE_PREFIX).then_some(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed(line: &[u8]) -> Vec<u8> {
        let mut out = (line.len() as i32).to_be_bytes().to_vec();
        out.extend_from_slice(line);
        out
    }

    #[test]
    fn a_cram_md5_challenge_is_a_director() {
        let reply = framed(b"auth cram-md5 <9349290.1788400000@bacula-dir> ssl=0\n");
        assert_eq!(
            parse_reply(&reply),
            AppProbeOutcome::Answered { identity: None }
        );
        assert!(challenge(&reply).unwrap().starts_with(CHALLENGE_PREFIX));
    }

    #[test]
    fn silence_a_signal_or_another_protocol_is_not_bacula() {
        let real = framed(b"auth cram-md5 <1.2@dir> ssl=0\n");
        for reply in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            // A BNET signal: a negative length, which is not a message at all.
            &(-1i32).to_be_bytes()[..],
            // Framed, but not the challenge.
            &framed(b"1000 OK auth\n")[..],
            // The challenge text with no framing in front of it.
            CHALLENGE_PREFIX,
            // A length that disagrees with what arrived.
            &{
                let mut wrong = real.clone();
                wrong[..4].copy_from_slice(&999i32.to_be_bytes());
                wrong
            }[..],
        ] {
            assert_eq!(parse_reply(reply), AppProbeOutcome::NoAnswer, "{reply:?}");
        }
    }

    /// The greeting is a console's `Hello`, and it carries nothing that could authenticate.
    #[test]
    fn the_greeting_is_framed_and_offers_no_credential() {
        let sent = hello();
        assert_eq!(
            i32::from_be_bytes(sent[..4].try_into().unwrap()) as usize,
            sent.len() - LENGTH_PREFIX_LEN
        );
        assert_eq!(&sent[LENGTH_PREFIX_LEN..], HELLO);
    }
}
