//! SNMP Session Management
//!
//! Functions for creating and managing SNMP sessions.

use anyhow::{Result, anyhow};
use snmp2::AsyncSession;
use std::net::IpAddr;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tokio::time::timeout;

use crate::server::credentials::r#impl::mapping::SnmpQueryCredential;
use crate::server::credentials::r#impl::types::{
    SnmpV3AuthProtocol, SnmpV3PrivProtocol, SnmpVersion,
};

impl From<SnmpV3AuthProtocol> for snmp2::v3::AuthProtocol {
    fn from(p: SnmpV3AuthProtocol) -> Self {
        match p {
            SnmpV3AuthProtocol::Sha1 => snmp2::v3::AuthProtocol::Sha1,
            SnmpV3AuthProtocol::Sha256 => snmp2::v3::AuthProtocol::Sha256,
        }
    }
}

impl From<SnmpV3PrivProtocol> for snmp2::v3::Cipher {
    fn from(p: SnmpV3PrivProtocol) -> Self {
        match p {
            SnmpV3PrivProtocol::Aes128 => snmp2::v3::Cipher::Aes128,
            SnmpV3PrivProtocol::Aes256 => snmp2::v3::Cipher::Aes256,
        }
    }
}

/// Default timeout for SNMP operations
pub const SNMP_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for SNMP session creation (UDP socket setup)
pub const SNMP_SESSION_TIMEOUT: Duration = Duration::from_secs(5);

/// Overall timeout for a single SNMP liveness probe (one credential, one port). Caps
/// the whole create-session + sysDescr GET so a non-responder — especially v3, whose
/// engine-discovery handshake otherwise waits the full 5s SNMP_SESSION_TIMEOUT — costs
/// ~2s instead of up to 7s. A responsive device answers in well under this on a LAN.
pub const SNMP_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// Default timeout for table walks (longer since they involve multiple requests).
///
/// Sized against real switches rather than a round number: a busy device's bridge FDB and
/// per-port VLAN membership are the two largest tables SNMP reads, and at 30s they were being
/// cut short often enough that operators saw "incomplete" warnings on every scan. Raising this
/// only costs time on devices that are genuinely slow — a healthy walk returns as soon as it is
/// done. Keep [`super::SnmpIntegration::timeout`] above `13 * SNMP_WALK_TIMEOUT`, since the
/// walks run sequentially and the outer cap would otherwise kill the last ones first.
pub const SNMP_WALK_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum number of varbinds to process in a single walk
pub const MAX_WALK_ENTRIES: usize = 10000;

/// A starting request id unique to this session.
///
/// Every session used to start at 0, so hosts scanned concurrently issued identical request ids in
/// lockstep — leaving the community string as the only thing telling one collection's responses
/// from another's. RFC 3416 asks for request ids that are hard to predict; a process-wide counter
/// gives each session its own range at no cost.
fn starting_request_id() -> i32 {
    static NEXT: AtomicI32 = AtomicI32::new(0);
    // Stride so two long-running sessions can't converge on the same id.
    NEXT.fetch_add(0x0001_0000, Ordering::Relaxed)
}

/// Create an SNMP session with the given credentials.
///
/// Returns a boxed session because `AsyncSession` contains ~131KB of stack-allocated
/// buffers (recv_buf + send_pdu). Without boxing, the async state machines that hold
/// a session overflow the tokio worker thread stacks in debug builds — the daemon
/// uses 4MB stacks, but `deep_scan_host`'s state machine (which includes SNMP query
/// sub-futures) is large enough in debug mode to overflow.
pub async fn create_session(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<Box<AsyncSession>> {
    let target = format!("{}:{}", ip, port);

    match credential.version {
        SnmpVersion::V1 | SnmpVersion::V2c => {
            let is_v1 = matches!(credential.version, SnmpVersion::V1);
            let community_secret = credential.community.resolve("community", "SNMP")?;
            let community = community_secret.expose_secret();
            tracing::debug!(
                ip = %ip,
                version = %credential.version,
                community_len = community.len(),
                "Creating SNMP community session"
            );

            // v1 and v2c differ only in the wire protocol version negotiated;
            // both authenticate with a community string.
            let create = async {
                if is_v1 {
                    AsyncSession::new_v1(&target, community.as_bytes(), starting_request_id()).await
                } else {
                    AsyncSession::new_v2c(&target, community.as_bytes(), starting_request_id())
                        .await
                }
            };

            match timeout(SNMP_SESSION_TIMEOUT, create).await {
                Ok(Ok(session)) => {
                    tracing::debug!(ip = %ip, "SNMP session created successfully");
                    Ok(Box::new(session))
                }
                Ok(Err(e)) => Err(anyhow!(
                    "Failed to create SNMP{} session to {}: {:?}",
                    if is_v1 { "v1" } else { "v2c" },
                    ip,
                    e
                )),
                Err(_) => Err(anyhow!(
                    "Timeout creating SNMP session to {} ({}s)",
                    ip,
                    SNMP_SESSION_TIMEOUT.as_secs()
                )),
            }
        }
        SnmpVersion::V3 => {
            let v3 = credential
                .v3
                .as_ref()
                .ok_or_else(|| anyhow!("SNMPv3 credential missing USM parameters"))?;
            let auth_pw = v3.auth_password.resolve("auth_password", "SNMPv3")?;
            let priv_pw = v3.priv_password.resolve("priv_password", "SNMPv3")?;

            // AuthPriv: both authentication and encryption. snmp2 0.5.0 only
            // transmits the default (empty) context, so context_name is ignored
            // on the wire here.
            let security = snmp2::v3::Security::new(
                v3.security_name.as_bytes(),
                auth_pw.expose_secret().as_bytes(),
            )
            .with_auth_protocol(v3.auth_protocol.into())
            .with_auth(snmp2::v3::Auth::AuthPriv {
                cipher: v3.priv_protocol.into(),
                privacy_password: priv_pw.expose_secret().as_bytes().to_vec(),
            });

            tracing::debug!(
                ip = %ip,
                security_name = %v3.security_name,
                "Creating SNMPv3 session"
            );

            let mut session = match timeout(
                SNMP_SESSION_TIMEOUT,
                AsyncSession::new_v3(&target, starting_request_id(), security),
            )
            .await
            {
                Ok(Ok(session)) => session,
                Ok(Err(e)) => {
                    return Err(anyhow!(
                        "Failed to create SNMPv3 session to {}: {:?}",
                        ip,
                        e
                    ));
                }
                Err(_) => {
                    return Err(anyhow!(
                        "Timeout creating SNMPv3 session to {} ({}s)",
                        ip,
                        SNMP_SESSION_TIMEOUT.as_secs()
                    ));
                }
            };

            // SNMPv3 requires an engine-discovery handshake before queries so the
            // session learns the authoritative engine ID / boots / time.
            match timeout(SNMP_SESSION_TIMEOUT, session.init()).await {
                Ok(Ok(())) => {
                    tracing::debug!(ip = %ip, "SNMPv3 session initialized");
                    Ok(Box::new(session))
                }
                Ok(Err(e)) => Err(anyhow!(
                    "Failed SNMPv3 engine discovery / authentication to {}: {:?}",
                    ip,
                    e
                )),
                Err(_) => Err(anyhow!("Timeout during SNMPv3 engine discovery to {}", ip)),
            }
        }
    }
}

/// Regressions for a session that has lost sync with its own traffic.
///
/// The daemon abandons SNMP requests constantly — a 5s cap per query and 30s per walk — and then
/// keeps using the same session for the next query. That makes request-id hygiene load-bearing
/// rather than theoretical: a timed-out request leaves its answer queued on the socket, so if the
/// next request reuses the id, that stale answer validates and every subsequent response is one
/// behind for the life of the session. Truncated columns then look like completed ones.
#[cfg(test)]
mod session_sync_tests {
    use super::*;
    use snmp2::{AsyncSession, Oid};
    use tokio::net::UdpSocket;

    const SYS_DESCR: &[u64] = &[1, 3, 6, 1, 2, 1, 1, 1, 0];

    /// A socket that receives requests and answers only when told to.
    async fn silent_agent() -> (UdpSocket, String) {
        let agent = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = agent.local_addr().unwrap().to_string();
        (agent, addr)
    }

    async fn abandon_one_request(session: &mut AsyncSession) {
        let oid = Oid::from(SYS_DESCR).unwrap();
        // Drop the future mid-`recv`, exactly as `query_or_default` does on its timeout.
        let _ = timeout(Duration::from_millis(150), session.get(&oid)).await;
    }

    /// A request that is abandoned must still consume its id. Reusing it is what lets the
    /// previous request's answer satisfy the next one.
    #[tokio::test]
    async fn an_abandoned_request_does_not_reuse_its_id() {
        let (agent, addr) = silent_agent().await;
        let mut session = AsyncSession::new_v2c(&addr, b"public", 0).await.unwrap();

        let mut ids = Vec::new();
        for _ in 0..2 {
            abandon_one_request(&mut session).await;
            let mut buf = [0u8; 2048];
            let (len, _) = agent.recv_from(&mut buf).await.unwrap();
            ids.push(snmp2::pdu::Pdu::from_bytes(&buf[..len]).unwrap().req_id);
        }

        assert_ne!(
            ids[0], ids[1],
            "the second request reused the abandoned request's id"
        );
    }

    /// The answer to an abandoned request is still sitting in the socket buffer. The next request
    /// must not read it: it belongs to a question nobody is waiting for.
    #[tokio::test]
    async fn a_late_answer_is_not_served_to_the_next_request() {
        let (agent, addr) = silent_agent().await;
        let mut session = AsyncSession::new_v2c(&addr, b"public", 0).await.unwrap();

        abandon_one_request(&mut session).await;

        // The agent answers after the caller gave up; the datagram queues on the session socket.
        let mut buf = [0u8; 2048];
        let (len, from) = agent.recv_from(&mut buf).await.unwrap();
        agent.send_to(&buf[..len], from).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // The next request goes unanswered, so it must stay pending. If the queued datagram were
        // consumed the call would resolve immediately instead — right or wrong, but not pending.
        let oid = Oid::from(SYS_DESCR).unwrap();
        let outcome = timeout(Duration::from_millis(300), session.get(&oid)).await;

        assert!(
            outcome.is_err(),
            "the stale datagram was served to a request that never got an answer"
        );
    }
}
