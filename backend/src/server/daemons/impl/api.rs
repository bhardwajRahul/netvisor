use std::fmt::Display;

use crate::{
    daemon::discovery::types::base::{
        DiscoveryPhase, DiscoverySessionInfo, DiscoverySessionUpdate,
    },
    server::{
        daemons::r#impl::{
            base::{Daemon, DaemonBase, DaemonMode},
            version::{DaemonVersionStatus, DeprecationWarning},
        },
        discovery::r#impl::types::DiscoveryType,
    },
};
use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Daemon capabilities
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash, ToSchema)]
pub struct DaemonCapabilities {
    #[serde(default)]
    pub has_docker_socket: bool,
    #[serde(default)]
    #[schema(required)]
    pub interfaced_subnet_ids: Vec<Uuid>,
}

impl Display for DaemonCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DaemonCapabilities {{ has_docker_socket: {}, interfaced_subnet_ids: {:?} }}",
            self.has_docker_socket, self.interfaced_subnet_ids
        )
    }
}

/// Daemon registration request from daemon to server
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonRegistrationRequest {
    pub daemon_id: Uuid,
    pub network_id: Uuid,
    pub name: String,
    pub url: String,
    pub mode: DaemonMode,
    pub capabilities: DaemonCapabilities,
    /// User responsible for maintaining this daemon (from frontend install command)
    /// Optional for backwards compat with old daemons - defaults to nil UUID
    #[serde(default)]
    pub user_id: Uuid,
    /// Daemon software version (optional for backwards compat with old daemons)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Daemon registration response from server to daemon
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonRegistrationResponse {
    pub daemon: Daemon,
    pub host_id: Uuid,
    /// Server capabilities (returned if daemon sends version info)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_capabilities: Option<ServerCapabilities>,
}

/// Daemon discovery request from server to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonDiscoveryRequest {
    pub session_id: Uuid,
    pub discovery_type: DiscoveryType,
}

impl From<DiscoveryUpdatePayload> for DaemonDiscoveryRequest {
    fn from(payload: DiscoveryUpdatePayload) -> Self {
        Self {
            session_id: payload.session_id,
            discovery_type: payload.discovery_type,
        }
    }
}

/// Daemon discovery response (for immediate acknowledgment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonDiscoveryResponse {
    pub session_id: Uuid,
}

/// Progress update from daemon to server during discovery
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct DiscoveryUpdatePayload {
    pub session_id: Uuid,
    pub daemon_id: Uuid,
    pub network_id: Uuid,
    pub phase: DiscoveryPhase,
    pub discovery_type: DiscoveryType,
    pub progress: u8,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl DiscoveryUpdatePayload {
    pub fn new(
        session_id: Uuid,
        daemon_id: Uuid,
        network_id: Uuid,
        discovery_type: DiscoveryType,
    ) -> Self {
        Self {
            session_id,
            daemon_id,
            network_id,
            phase: DiscoveryPhase::Pending,
            progress: 0,
            discovery_type,
            error: None,
            started_at: None,
            finished_at: None,
        }
    }

    pub fn from_state_and_update(
        discovery_type: DiscoveryType,
        info: DiscoverySessionInfo,
        update: DiscoverySessionUpdate,
    ) -> Self {
        Self {
            session_id: info.session_id,
            discovery_type,
            network_id: info.network_id,
            daemon_id: info.daemon_id,
            phase: update.phase,
            progress: update.progress,
            error: update.error,
            started_at: info.started_at,
            finished_at: update.finished_at,
        }
    }
}

/// Daemon status payload sent when polling for work or in heartbeats.
/// Used by DaemonPoll mode to send status alongside work requests,
/// and by ServerPoll mode when processing daemon status.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonStatusPayload {
    pub url: String,
    pub name: String,
    pub mode: DaemonMode,
    /// Daemon software version (optional for backwards compat)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub version: Option<Version>,
}

/// Sent by daemon on startup to report version
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonStartupRequest {
    /// Daemon software version (semver format)
    #[schema(value_type = String)]
    pub daemon_version: Version,
}

/// Server capabilities returned on startup/registration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ServerCapabilities {
    /// Server software version
    #[schema(value_type = String)]
    pub server_version: Version,
    /// Minimum daemon version supported by this server
    #[schema(value_type = String)]
    pub minimum_daemon_version: Version,
    /// Deprecation warnings for the daemon
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deprecation_warnings: Vec<DeprecationWarning>,
}

/// Daemon response for UI including computed version status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonResponse {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    pub base: DaemonBase,
    /// Computed version status including health and warnings
    pub version_status: DaemonVersionStatus,
}
