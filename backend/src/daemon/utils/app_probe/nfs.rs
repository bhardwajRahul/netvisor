//! NFS detection with an ONC RPC NULL call.
//!
//! Procedure 0 of every RPC program is NULL: it takes no arguments, returns nothing, and exists so a
//! client can check that a program is reachable. Calling it against the NFS program needs no mount,
//! no export name and no credentials beyond `AUTH_NONE`.
//!
//! A server that rejects the call answers `MSG_DENIED` rather than staying silent, and that is still
//! an RPC server on the NFS port. What is checked is that the reply parses as an RPC *reply* and
//! carries our transaction id — not the accept status.
//!
//! [`onc_rpc`] owns the framing. Record marking is the fiddly part of RPC over TCP — a 31-bit length
//! with a "last fragment" bit above it, and a message that may span several fragments — and it is
//! now the library's rather than three lines of bit-twiddling here. `RpcMessage::try_from` also
//! rejects a buffer that does not hold exactly one complete message, where the check this replaced
//! read the xid at a fixed offset and ignored the record mark entirely.
//!
//! Codec only: nothing here mounts an export or issues a real procedure call.

use anyhow::Error;
use async_trait::async_trait;
use onc_rpc::auth::AuthFlavor;
use onc_rpc::{CallBody, MessageType, RpcMessage};

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The NFS program number.
const PROGRAM_NFS: u32 = 100_003;
/// Procedure 0 of any program.
const PROCEDURE_NULL: u32 = 0;
/// Echoed by the server, which is what correlates the reply to our call.
const XID: u32 = 0x5CA1_0F5Au32;

/// A NULL call to the NFS program, wrapped in RPC's record-marking framing for TCP.
fn null_call(version: u32) -> Vec<u8> {
    let body: CallBody<&[u8], &[u8]> = CallBody::new(
        PROGRAM_NFS,
        version,
        PROCEDURE_NULL,
        // No credential and no verifier. NULL is reachable without either, and offering one would
        // make this an authentication attempt rather than a reachability check.
        AuthFlavor::AuthNone(None),
        AuthFlavor::AuthNone(None),
        &[][..],
    );
    RpcMessage::new(XID, MessageType::Call(body))
        .serialise()
        .expect("serialising a fixed call into a Vec cannot fail")
}

pub struct NfsProbe;

#[async_trait]
impl AppProbe for NfsProbe {
    fn port(&self) -> PortType {
        PortType::Nfs
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Nfs)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        // NFSv3 first, then v4: a v4-only server answers the v3 call with a program-mismatch
        // reply, which is itself proof, but asking for the version it speaks avoids relying on
        // that.
        for version in [3u32, 4] {
            let reply = request_response(ctx, self.port(), &null_call(version), 256).await;
            if parse_reply(&reply, XID) == (AppProbeOutcome::Answered { identity: None }) {
                return Ok(AppProbeOutcome::Answered { identity: None });
            }
        }
        Ok(AppProbeOutcome::NoAnswer)
    }
}

/// Whether the reply is an RPC reply carrying our transaction id.
fn parse_reply(bytes: &[u8], xid: u32) -> AppProbeOutcome {
    let Ok(message) = RpcMessage::try_from(bytes) else {
        return AppProbeOutcome::NoAnswer;
    };
    // `reply_body` is `None` for a call, so an echo of our own bytes does not count.
    presence(message.xid() == xid && message.reply_body().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use onc_rpc::{AcceptedReply, AcceptedStatus, ReplyBody};

    fn accepted_reply(xid: u32) -> Vec<u8> {
        let reply: AcceptedReply<&[u8], &[u8]> =
            AcceptedReply::new(AuthFlavor::AuthNone(None), AcceptedStatus::Success(&[][..]));
        RpcMessage::new(xid, MessageType::Reply(ReplyBody::Accepted(reply)))
            .serialise()
            .unwrap()
    }

    #[test]
    fn an_accepted_reply_is_nfs() {
        assert_eq!(
            parse_reply(&accepted_reply(XID), XID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A server refusing the call is still an RPC server on the NFS port — and this is the case a
    /// client crate would have turned into an error rather than a detection.
    #[test]
    fn a_denied_reply_is_nfs() {
        use onc_rpc::{AuthError, RejectedReply};
        let denied = RpcMessage::<&[u8], &[u8]>::new(
            XID,
            MessageType::Reply(ReplyBody::Denied(RejectedReply::AuthError(
                AuthError::BadCredentials,
            ))),
        )
        .serialise()
        .unwrap();
        assert_eq!(
            parse_reply(&denied, XID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn an_uncorrelated_reply_or_an_echoed_call_is_not_evidence() {
        assert_eq!(
            parse_reply(&accepted_reply(0xDEAD_BEEF), XID),
            AppProbeOutcome::NoAnswer
        );
        // Our own call reflected back carries the right xid but is a call, not a reply.
        assert_eq!(parse_reply(&null_call(3), XID), AppProbeOutcome::NoAnswer);
    }

    #[test]
    fn silence_or_another_protocol_is_not_nfs() {
        let reply = accepted_reply(XID);
        for bytes in [
            &b""[..],
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &[0u8; 8][..],
            // Truncated inside the record. The offset read this replaced accepted it.
            &reply[..reply.len() - 4],
        ] {
            assert_eq!(
                parse_reply(bytes, XID),
                AppProbeOutcome::NoAnswer,
                "{bytes:?}"
            );
        }
    }

    #[test]
    fn the_call_is_record_marked_and_addresses_the_nfs_program() {
        let call = null_call(3);
        let mark = u32::from_be_bytes(call[0..4].try_into().unwrap());
        assert_eq!(mark & 0x8000_0000, 0x8000_0000, "last fragment");
        assert_eq!((mark & 0x7FFF_FFFF) as usize, call.len() - 4);

        let parsed = RpcMessage::try_from(&call[..]).expect("we send a valid message");
        assert_eq!(parsed.xid(), XID);
        let body = parsed.call_body().expect("we send a call");
        assert_eq!(body.program(), PROGRAM_NFS);
        assert_eq!(body.procedure(), PROCEDURE_NULL);
    }
}
