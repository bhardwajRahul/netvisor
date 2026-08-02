//! Discovery integration trait system.
//!
//! All discovery integrations follow the same flow:
//! 1. `probe()` — check if the integration's service responds with the given credential
//! 2. Service matching — probe result feeds into `Pattern::ClientResponse` matching
//! 3. `execute()` — scan/query the service, enrich HostData or create entities
//!
//! The pipeline dispatches integrations generically based on credential mappings
//! and service matches — no integration-specific code in the orchestrator.

pub mod container;
pub mod dispatch;
pub mod docker;
pub mod failure;
pub mod podman;
pub mod snmp;
pub mod unifi;

use std::any::Any;
use std::net::IpAddr;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    daemon::discovery::service::warnings::AttemptOutcome,
    daemon::utils::base::PlatformDaemonUtils,
    server::{
        credentials::r#impl::mapping::{
            CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
        },
        discovery::r#impl::types::HostNamingFallback,
        ports::r#impl::base::PortType,
        services::r#impl::{base::Service, endpoints::EndpointResponse, patterns::ClientProbe},
        subnets::r#impl::base::Subnet,
    },
};

use super::service::ops::{DiscoveryOps, HostData};
pub use failure::{IntegrationFailure, ProbeFailure};

// ============================================================================
// Trait
// ============================================================================

#[async_trait]
pub trait DiscoveryIntegration: Send + Sync {
    /// Which credential type this integration handles.
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants;

    /// Estimated execution time per invocation, in seconds.
    /// Used for cost-based progress estimation.
    fn estimated_seconds(&self) -> u32;

    /// Maximum execution time before the caller cancels.
    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// TCP ports that must be detected open before `probe()` is attempted.
    /// Returns empty to always attempt (e.g., SNMP does its own UDP probing).
    fn probe_gate_ports(&self, _credential: &CredentialQueryPayload) -> Vec<PortType> {
        vec![]
    }

    /// Probe the target host: check if this integration's service responds
    /// with the given credential.
    ///
    /// Success: `ClientProbe` feeds into service matching, `handle` is passed to `execute()`.
    /// Failure: credential rejected or service not responding, with diagnostic message.
    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure>;

    /// Execute the integration's scanning/discovery logic.
    ///
    /// Receives mutable `HostData` — enrich the scanned host via builder methods,
    /// or create separate entities via `ctx.ops` (e.g., Proxmox VMs).
    ///
    /// Only called when `probe()` succeeded AND the associated service was matched.
    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
    ) -> Result<(), IntegrationFailure>;
}

// ============================================================================
// Client-library error classification
// ============================================================================

/// One impl per client library, so each integration classifies its own errors once rather than
/// every call site guessing. The foreign→domain direction keeps the vocabulary ours: a library
/// that adds an error variant fails to compile here rather than silently landing in a catch-all.
impl From<&bollard::errors::Error> for AttemptOutcome {
    fn from(error: &bollard::errors::Error) -> Self {
        use bollard::errors::Error as E;
        match error {
            // The daemon answered and refused us. On a TLS-protected socket that is the
            // certificate; on a plain one it is the socket's permissions. Either way the
            // operator's fix is the credential, not the network.
            E::DockerResponseServerError { status_code, .. }
                if *status_code == 401 || *status_code == 403 =>
            {
                Self::Rejected
            }
            // Anything else it answered means the service exists and is talking to us.
            E::DockerResponseServerError { .. }
            | E::JsonDataError { .. }
            | E::JsonSerdeError { .. }
            | E::APIVersionParseError { .. }
            | E::DockerStreamError { .. } => Self::NotThisService,
            E::CertPathError { .. }
            | E::CertMultipleKeys { .. }
            | E::CertParseError { .. }
            | E::NoNativeCertsError { .. }
            | E::LoadNativeCertsErrors { .. } => Self::TlsFailed,
            E::RequestTimeoutError => Self::TimedOut,
            // A malformed URL or missing socket path is our configuration, not their host.
            E::URLParseError { .. }
            | E::InvalidURIError { .. }
            | E::InvalidURIPartsError { .. }
            | E::UnsupportedURISchemeError { .. }
            | E::SocketNotFoundError(_)
            | E::NoHomePathError => Self::Malformed,
            _ => Self::Unreachable,
        }
    }
}

impl From<&reqwest::Error> for AttemptOutcome {
    fn from(error: &reqwest::Error) -> Self {
        if error.is_timeout() {
            return Self::TimedOut;
        }
        // reqwest folds TLS failures into `is_connect`, so the message is the only way to tell a
        // refused connection from a certificate the client would not accept — and they send an
        // operator to completely different places.
        if error.is_connect() {
            let text = error.to_string().to_lowercase();
            if text.contains("certificate") || text.contains("tls") || text.contains("ssl") {
                return Self::TlsFailed;
            }
            return Self::Unreachable;
        }
        if error.is_status() {
            return match error.status().map(|s| s.as_u16()) {
                Some(401) | Some(403) => Self::Rejected,
                _ => Self::NotThisService,
            };
        }
        if error.is_decode() {
            return Self::NotThisService;
        }
        Self::Unreachable
    }
}

impl From<&snmp2::Error> for AttemptOutcome {
    fn from(error: &snmp2::Error) -> Self {
        use snmp2::Error as E;
        match error {
            // v3 said no: the USM user, auth password or privacy password is wrong.
            E::AuthFailure(_) | E::Crypto(_) => Self::Rejected,
            // A v2c agent that does not know the community simply does not answer, so a community
            // mismatch here means we read a datagram meant for a different session — not that the
            // community is wrong.
            E::CommunityMismatch | E::RequestIdMismatch | E::AuthUpdated => Self::TimedOut,
            // We are talking to something, and it is not answering the way SNMP does.
            E::AsnParse
            | E::AsnInvalidLen
            | E::AsnWrongType
            | E::AsnUnsupportedType
            | E::AsnEof
            | E::AsnIntOverflow
            | E::UnsupportedVersion
            | E::ValueOutOfRange
            | E::BufferOverflow
            | E::Mib(_) => Self::NotThisService,
            E::Send | E::Receive => Self::Unreachable,
        }
    }
}

// ============================================================================
// Probe types
// ============================================================================

pub struct ProbeContext<'a> {
    pub ip: IpAddr,
    pub credential: &'a CredentialQueryPayload,
    pub credential_id: Option<Uuid>,
    pub cancel: &'a CancellationToken,
    pub utils: &'a PlatformDaemonUtils,
    /// Whether to accept self-signed / otherwise invalid TLS certificates, from the daemon's
    /// `accept_invalid_scan_certs` config. Mirrors [`IntegrationContext::accept_invalid_certs`]:
    /// integrations that authenticate over HTTPS make their *first* call here, so the probe needs
    /// the same policy the execute phase gets. Appliance controllers (UniFi and friends) ship
    /// self-signed certs by default, so without this the probe fails before execute is reached.
    pub accept_invalid_certs: bool,
}

/// Successful probe — service responds with this credential.
pub struct ProbeSuccess {
    /// What was detected. Feeds into `client_responses` for `Pattern::ClientResponse` matching.
    pub client_probe: ClientProbe,
    /// Ports the probe was detected on.
    pub ports: Vec<PortType>,
    /// Opaque keep-alive state passed to `execute()`.
    /// E.g., connected Docker client, working SNMP credential + port.
    pub handle: Option<Box<dyn Any + Send + Sync>>,
}

// ============================================================================
// Execution context
// ============================================================================

pub struct IntegrationContext<'a> {
    pub ip: IpAddr,
    pub credential: &'a CredentialQueryPayload,
    pub credential_id: Option<Uuid>,
    pub cancel: &'a CancellationToken,
    pub ops: &'a DiscoveryOps,
    pub utils: &'a PlatformDaemonUtils,
    /// Opaque state from `probe()`. Integration downcasts to its expected type.
    pub probe_handle: Option<&'a (dyn Any + Send + Sync)>,
    pub matched_services: &'a [Service],
    pub open_ports: &'a [PortType],
    pub endpoint_responses: &'a [EndpointResponse],
    pub host_id: Uuid,
    pub host_naming_fallback: HostNamingFallback,
    /// Subnets an integration may place a discovered address in — the network's whole address
    /// space during the network phase, and the just-created ones during the daemon-host phase.
    ///
    /// Deliberately *not* the scan's subnet list. An integration learns about addresses the
    /// sweep never visits: a UniFi controller reports every switch it manages, most of them on
    /// subnets a rescan of the controller does not touch. Host identity is IP-based, so a device
    /// that cannot be placed in a subnet is dropped rather than deduplicated — which made a
    /// controller rescan silently enrich nothing.
    pub known_subnets: &'a [Subnet],
    pub accept_invalid_certs: bool,
    /// The subnet currently being scanned (needed by SNMP for remote subnet discovery).
    pub scanning_subnet: Option<&'a Subnet>,
}

// ============================================================================
// Registry
// ============================================================================

/// Maps credential types to their discovery integration.
/// Exhaustive match — every credential type has an integration.
pub struct IntegrationRegistry;

impl IntegrationRegistry {
    /// Resolve a credential type to its integration. Returns `None` for the
    /// forward-compat `Unknown` variant — a newer server may send a credential type
    /// this daemon doesn't recognize (deserialized via `#[serde(other)]`); callers
    /// skip it rather than failing the whole discovery request.
    pub fn get(d: CredentialQueryPayloadDiscriminants) -> Option<Box<dyn DiscoveryIntegration>> {
        Some(match d {
            CredentialQueryPayloadDiscriminants::Snmp => Box::new(snmp::SnmpIntegration),
            CredentialQueryPayloadDiscriminants::DockerProxy => Box::new(docker::DockerIntegration),
            CredentialQueryPayloadDiscriminants::DockerSocket => {
                Box::new(docker::DockerSocketIntegration)
            }
            CredentialQueryPayloadDiscriminants::PodmanProxy => Box::new(podman::PodmanIntegration),
            CredentialQueryPayloadDiscriminants::PodmanSocket => {
                Box::new(podman::PodmanSocketIntegration)
            }
            CredentialQueryPayloadDiscriminants::UnifiController => {
                Box::new(unifi::UnifiIntegration)
            }
            CredentialQueryPayloadDiscriminants::Unknown => return None,
        })
    }
}

// ============================================================================
// Progress reporting wrapper
// ============================================================================

/// Wraps `execute()` with periodic progress re-reporting to prevent the server's
/// 5-minute stall detector from killing the session.
///
/// Before calling this, the pipeline sets `session.set_progress_range(start, end)`
/// to the integration's share of overall progress. The integration calls
/// `ctx.ops.report_progress(percent)` (0-100 within its scope) which maps to
/// the correct overall percentage.
///
/// The `progress_fn` re-reports the current progress as a heartbeat every 30 seconds
/// if the integration hasn't reported recently.
pub async fn execute_with_progress_reporting<F, Fut>(
    integration: &dyn DiscoveryIntegration,
    ctx: &IntegrationContext<'_>,
    host_data: &mut HostData,
    progress_fn: F,
) -> Result<(), IntegrationFailure>
where
    F: Fn() -> Fut + Send,
    Fut: std::future::Future<Output = ()> + Send,
{
    let timeout_duration = integration.timeout();

    let result = tokio::time::timeout(timeout_duration, async {
        let execute_fut = integration.execute(ctx, host_data);
        tokio::pin!(execute_fut);
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.tick().await; // consume immediate first tick
        loop {
            tokio::select! {
                result = &mut execute_fut => return result,
                _ = interval.tick() => {
                    progress_fn().await;
                }
            }
        }
    })
    .await;

    match result {
        Ok(inner) => inner,
        // The outer cap firing is a timeout in its own right, not a generic collection failure —
        // the integration was still working when we stopped waiting for it.
        Err(_) => Err(IntegrationFailure::with_outcome(
            AttemptOutcome::TimedOut,
            format!("Integration timed out after {timeout_duration:?}"),
        )),
    }
}

#[cfg(test)]
mod classification_tests {
    use super::*;

    /// The customer-facing point of the whole mechanism: a wrong password and an unreachable
    /// host must not read the same. Docker's daemon answers 401/403 when it refuses us, and
    /// that used to arrive as "probe failed after 3 attempts" — indistinguishable from nothing
    /// listening on the port.
    #[test]
    fn a_refused_docker_socket_is_a_credential_problem_not_a_network_one() {
        let refused = bollard::errors::Error::DockerResponseServerError {
            status_code: 401,
            message: "unauthorized".to_string(),
        };
        assert_eq!(AttemptOutcome::from(&refused), AttemptOutcome::Rejected);

        let other = bollard::errors::Error::DockerResponseServerError {
            status_code: 500,
            message: "boom".to_string(),
        };
        assert_eq!(AttemptOutcome::from(&other), AttemptOutcome::NotThisService);
    }

    /// A certificate the client will not accept is fixed by a trust setting, not by re-typing a
    /// password — so it cannot share a line with a rejection.
    #[test]
    fn a_certificate_problem_is_reported_as_tls_not_as_a_rejection() {
        let error = bollard::errors::Error::CertMultipleKeys {
            count: 2,
            path: std::path::PathBuf::from("/tmp/key.pem"),
        };
        assert_eq!(AttemptOutcome::from(&error), AttemptOutcome::TlsFailed);
    }

    /// SNMPv3 authenticates during engine discovery, so a bad password is a genuine refusal.
    /// This is the case that told Motala their switch was unreachable when the password was
    /// simply wrong.
    #[test]
    fn snmp_auth_failure_is_a_rejection() {
        let error = snmp2::Error::AuthFailure(snmp2::v3::AuthErrorKind::NotAuthenticated);
        assert_eq!(AttemptOutcome::from(&error), AttemptOutcome::Rejected);
    }

    /// Reading someone else's datagram says nothing about the credential. Classifying it as a
    /// rejection would blame the operator's configuration for a transport race.
    #[test]
    fn a_desynced_session_is_not_a_rejection() {
        assert_eq!(
            AttemptOutcome::from(&snmp2::Error::RequestIdMismatch),
            AttemptOutcome::TimedOut
        );
        assert_eq!(
            AttemptOutcome::from(&snmp2::Error::Receive),
            AttemptOutcome::Unreachable
        );
    }

    /// A malformed response means something is listening and it is not SNMP — the operator's
    /// fix is the port, not the community.
    #[test]
    fn a_non_snmp_answer_points_at_the_port() {
        assert_eq!(
            AttemptOutcome::from(&snmp2::Error::AsnParse),
            AttemptOutcome::NotThisService
        );
    }

    /// `ProbeFailure` has no public fields and no `Default`, so this is the only way to build
    /// one. The constructors are the enforcement — a new integration cannot add a failure path
    /// without picking an outcome.
    #[test]
    fn constructors_carry_their_outcome() {
        assert_eq!(
            ProbeFailure::cancelled().outcome(),
            AttemptOutcome::Cancelled
        );
        assert_eq!(
            ProbeFailure::malformed("bad").outcome(),
            AttemptOutcome::Malformed
        );
        let with_context = ProbeFailure::rejected("refused").with_context("after 3 attempts");
        assert_eq!(with_context.outcome(), AttemptOutcome::Rejected);
        assert_eq!(with_context.message(), "after 3 attempts: refused");
    }

    /// An `anyhow` error from an integration degrades to the generic collection failure rather
    /// than forcing every `?` to be rewritten — but the outer timeout is more specific and says
    /// so, since the integration was still working when we stopped waiting.
    #[test]
    fn an_integration_failure_defaults_to_collection_failed() {
        let from_anyhow: IntegrationFailure = anyhow::Error::msg("something broke").into();
        assert_eq!(from_anyhow.outcome(), AttemptOutcome::CollectionFailed);

        assert_eq!(
            IntegrationFailure::with_outcome(AttemptOutcome::TimedOut, "slow").outcome(),
            AttemptOutcome::TimedOut
        );
    }
}
