//! Docker Swarm detection by the certificate its raft port presents.
//!
//! 2377 speaks mutual TLS, and the definition used to carry a `NoDistinguishingHandshake` rationale
//! saying an unauthenticated peer gets nothing that names Swarm. That was wrong twice over: the
//! server sends its `Certificate` before demanding ours, and **that certificate names the swarm**.
//! Docker issues every node one carrying its role and cluster, so the subject reads roughly:
//!
//! ```text
//! CN=<node id>, OU=swarm-manager, O=<swarm cluster id>
//! ```
//!
//! `OU` is the part that identifies the service rather than merely the transport. Matching on it
//! makes this a real detection and not "something answered TLS on 2377" — a distinction that
//! matters because plenty of things speak TLS on plenty of ports.
//!
//! Nothing is authenticated and no session is established; see
//! [`crate::daemon::utils::app_probe::tls`] for why the certificate is readable at all.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::tls::capture_certificate;
use crate::daemon::utils::app_probe::{AppProbe, AppProbeOutcome, ProbeContext, presence};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;

/// The organizational units Docker issues to swarm nodes. A worker's certificate is as much proof
/// of a swarm as a manager's — the raft port only listens on managers, but matching both keeps the
/// check about what the certificate says rather than about which port it arrived on.
const SWARM_ROLES: [&str; 2] = ["swarm-manager", "swarm-worker"];

pub struct DockerSwarmProbe;

#[async_trait]
impl AppProbe for DockerSwarmProbe {
    fn port(&self) -> PortType {
        PortType::new_tcp(2377)
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::DockerSwarm)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        let handshake = capture_certificate(ctx, self.port()).await;
        Ok(presence(
            handshake.subject().as_deref().is_some_and(names_a_swarm),
        ))
    }
}

/// Whether a certificate subject is one Docker issued to a swarm node.
fn names_a_swarm(subject: &str) -> bool {
    SWARM_ROLES.iter().any(|role| subject.contains(role))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manager_or_worker_certificate_names_a_swarm() {
        assert!(names_a_swarm(
            "CN=n1abc2def3ghi4jkl5mno6pqr,OU=swarm-manager,O=xyz789swarmcluster"
        ));
        assert!(names_a_swarm("CN=abc,OU=swarm-worker,O=def"));
    }

    /// The point of matching the subject rather than the handshake: TLS on 2377 is not Swarm.
    #[test]
    fn another_tls_service_on_the_same_port_is_not_a_swarm() {
        for subject in [
            "CN=example.com,O=Example Ltd",
            "CN=localhost",
            "CN=etcd,OU=etcd-peer,O=cluster",
            "",
        ] {
            assert!(!names_a_swarm(subject), "{subject}");
        }
    }
}
