//! NFS detection with an ONC RPC NULL call.
//!
//! Procedure 0 of every RPC program is NULL: it takes no arguments, returns nothing, and exists so a
//! client can check that a program is reachable. Calling it against the NFS program needs no mount,
//! no export name and no credentials beyond `AUTH_NULL`.
//!
//! A server that rejects the call answers `MSG_DENIED` rather than staying silent, and that is still
//! an RPC server on the NFS port. What is checked is the reply's framing and its transaction id, not
//! the accept status.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, ProbeContext, presence, request_response,
};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The NFS program number.
const PROGRAM_NFS: u32 = 100_003;
/// Procedure 0 of any program.
const PROCEDURE_NULL: u32 = 0;
/// Message type 1: a reply.
const MSG_TYPE_REPLY: u32 = 1;
/// Echoed by the server, which is what correlates the reply to our call.
const XID: u32 = 0x5CA1_0F5Au32;

/// Record-marking header (4) plus xid (4) and message type (4).
const RECORD_MARK_LEN: usize = 4;

/// A NULL call to the NFS program, wrapped in RPC's record-marking framing for TCP.
fn null_call(version: u32) -> Vec<u8> {
    let mut call = Vec::new();
    call.extend_from_slice(&XID.to_be_bytes());
    call.extend_from_slice(&0u32.to_be_bytes()); // msg_type: CALL
    call.extend_from_slice(&2u32.to_be_bytes()); // rpcvers
    call.extend_from_slice(&PROGRAM_NFS.to_be_bytes());
    call.extend_from_slice(&version.to_be_bytes());
    call.extend_from_slice(&PROCEDURE_NULL.to_be_bytes());
    call.extend_from_slice(&0u32.to_be_bytes()); // cred flavour AUTH_NULL
    call.extend_from_slice(&0u32.to_be_bytes()); // cred length
    call.extend_from_slice(&0u32.to_be_bytes()); // verf flavour AUTH_NULL
    call.extend_from_slice(&0u32.to_be_bytes()); // verf length

    // Record marking: high bit set means "last fragment", low 31 bits are the length.
    let mark = 0x8000_0000u32 | call.len() as u32;
    let mut framed = mark.to_be_bytes().to_vec();
    framed.extend_from_slice(&call);
    framed
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
    let Some(body) = bytes.get(RECORD_MARK_LEN..RECORD_MARK_LEN + 8) else {
        return AppProbeOutcome::NoAnswer;
    };
    let echoed = u32::from_be_bytes(body[0..4].try_into().unwrap_or_default());
    let msg_type = u32::from_be_bytes(body[4..8].try_into().unwrap_or_default());

    presence(echoed == xid && msg_type == MSG_TYPE_REPLY)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(xid: u32, tail: &[u8]) -> Vec<u8> {
        let mut body = xid.to_be_bytes().to_vec();
        body.extend_from_slice(&MSG_TYPE_REPLY.to_be_bytes());
        body.extend_from_slice(tail);
        let mark = 0x8000_0000u32 | body.len() as u32;
        let mut framed = mark.to_be_bytes().to_vec();
        framed.extend_from_slice(&body);
        framed
    }

    #[test]
    fn an_accepted_reply_is_nfs() {
        // reply_stat MSG_ACCEPTED, AUTH_NULL verifier, accept_stat SUCCESS.
        assert_eq!(
            parse_reply(&reply(XID, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), XID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    /// A server refusing the call is still an RPC server on the NFS port.
    #[test]
    fn a_denied_reply_is_nfs() {
        assert_eq!(
            parse_reply(&reply(XID, &[0, 0, 0, 1, 0, 0, 0, 1]), XID),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn an_uncorrelated_reply_or_an_echoed_call_is_not_evidence() {
        assert_eq!(
            parse_reply(&reply(0xDEAD_BEEF, &[]), XID),
            AppProbeOutcome::NoAnswer
        );
        // msg_type 0 is a CALL, not a reply — an echo of our own bytes would look like this.
        let mut echoed_call = null_call(3);
        echoed_call.truncate(RECORD_MARK_LEN + 8);
        assert_eq!(parse_reply(&echoed_call, XID), AppProbeOutcome::NoAnswer);
    }

    #[test]
    fn silence_or_another_protocol_is_not_nfs() {
        for bytes in [&b""[..], &b"SSH-2.0-OpenSSH_9.6\r\n"[..], &[0u8; 8][..]] {
            assert_eq!(parse_reply(bytes, XID), AppProbeOutcome::NoAnswer);
        }
    }

    #[test]
    fn the_call_is_record_marked_and_addresses_the_nfs_program() {
        let call = null_call(3);
        let mark = u32::from_be_bytes(call[0..4].try_into().unwrap());
        assert_eq!(mark & 0x8000_0000, 0x8000_0000, "last fragment");
        assert_eq!((mark & 0x7FFF_FFFF) as usize, call.len() - RECORD_MARK_LEN);
        assert_eq!(
            u32::from_be_bytes(call[16..20].try_into().unwrap()),
            PROGRAM_NFS
        );
    }
}
