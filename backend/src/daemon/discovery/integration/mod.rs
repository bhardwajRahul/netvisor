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
pub mod podman;
pub mod snmp;

use std::any::Any;
use std::net::IpAddr;
use std::time::Duration;

use anyhow::Error;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
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
    ) -> Result<(), Error>;
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

/// Failed probe — credential rejected or service not responding.
pub struct ProbeFailure {
    /// Diagnostic message for logging.
    pub message: String,
}

impl std::fmt::Display for ProbeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
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
    pub created_subnets: &'a [Subnet],
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
) -> Result<(), Error>
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
        Err(_) => Err(anyhow::anyhow!(
            "Integration timed out after {:?}",
            timeout_duration
        )),
    }
}
