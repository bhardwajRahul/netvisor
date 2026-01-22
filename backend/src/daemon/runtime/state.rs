use std::sync::Arc;

use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use uuid::Uuid;

use crate::{
    daemon::{
        discovery::{buffer::EntityBuffer, service::base::DaemonDiscoveryService},
        shared::config::ConfigStore,
    },
    server::{
        daemons::r#impl::{
            api::{DaemonCapabilities, DiscoveryUpdatePayload},
            base::DaemonMode,
        },
        hosts::r#impl::{api::DiscoveryHostRequest, api::HostResponse},
        subnets::r#impl::base::Subnet,
    },
};

/// Lightweight daemon status for polling responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonStatus {
    pub url: String,
    pub name: String,
    pub mode: DaemonMode,
    /// Daemon software version (semver format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub version: Option<Version>,
    /// Daemon capabilities (docker socket, interfaced subnets)
    #[serde(default)]
    pub capabilities: DaemonCapabilities,
}

/// Buffered entities discovered during a discovery session.
/// Used to batch entity creation when server polls daemon (ServerPoll mode).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct BufferedEntities {
    /// Hosts with their interfaces, ports, and services
    pub hosts: Vec<DiscoveryHostRequest>,
    /// Discovered subnets
    pub subnets: Vec<Subnet>,
}

impl BufferedEntities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.hosts.is_empty() && self.subnets.is_empty()
    }
}

/// Response type for GET /api/discovery endpoint.
/// Returns current progress and any buffered entities since last poll.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiscoveryPollResponse {
    /// Current discovery session progress (if any active session)
    pub progress: Option<DiscoveryUpdatePayload>,
    /// Entities discovered since last poll
    pub entities: BufferedEntities,
}

/// Payload sent by server to daemon with created entity confirmations.
/// Maps pending (daemon-generated) IDs to actual server entities (after deduplication).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreatedEntitiesPayload {
    /// Subnets: (pending_id, actual_subnet) pairs
    pub subnets: Vec<(Uuid, Subnet)>,
    /// Hosts: (pending_id, actual_host_response) pairs - includes children (interfaces, ports, services)
    pub hosts: Vec<(Uuid, HostResponse)>,
}

/// Daemon state for handlers.
/// Delegates to ConfigStore for metadata, DaemonDiscoveryService for progress,
/// and EntityBuffer for buffered entities.
pub struct DaemonState {
    config: Arc<ConfigStore>,
    discovery_service: Arc<DaemonDiscoveryService>,
    entity_buffer: Arc<EntityBuffer>,
    /// Cached daemon URL (computed once on startup)
    daemon_url: String,
}

impl DaemonState {
    pub fn new(
        config: Arc<ConfigStore>,
        discovery_service: Arc<DaemonDiscoveryService>,
        entity_buffer: Arc<EntityBuffer>,
        daemon_url: String,
    ) -> Self {
        Self {
            config,
            discovery_service,
            entity_buffer,
            daemon_url,
        }
    }

    /// Get the entity buffer for pushing discovered entities.
    pub fn entity_buffer(&self) -> &Arc<EntityBuffer> {
        &self.entity_buffer
    }
}

impl DaemonState {
    /// Get lightweight daemon status (url, name, mode, version).
    pub async fn get_status(&self) -> DaemonStatus {
        let name = self.config.get_name().await.unwrap_or_default();
        let mode = self.config.get_mode().await.unwrap_or_default();
        let version = Version::parse(env!("CARGO_PKG_VERSION")).ok();
        let capabilities = self.config.get_capabilities().await.unwrap_or_default();

        DaemonStatus {
            url: self.daemon_url.clone(),
            name,
            mode,
            version,
            capabilities,
        }
    }

    /// Get current discovery session progress, if any.
    pub async fn get_progress(&self) -> Option<DiscoveryUpdatePayload> {
        // Get the current session from the discovery service
        let session = self.discovery_service.current_session.read().await;

        session.as_ref().map(|s| {
            let progress = s.last_progress.load(std::sync::atomic::Ordering::Relaxed);

            DiscoveryUpdatePayload {
                session_id: s.info.session_id,
                daemon_id: s.info.daemon_id,
                network_id: s.info.network_id,
                // Note: We report the last known progress percentage.
                // The actual phase might have changed since last report.
                // For polling, this is acceptable as the server will get
                // the terminal state in the next poll.
                phase: crate::daemon::discovery::types::base::DiscoveryPhase::Scanning,
                discovery_type: s.info.discovery_type.clone(),
                progress,
                error: None,
                started_at: s.info.started_at,
                finished_at: None,
            }
        })
    }

    /// Drain buffered entities since last call.
    /// Returns accumulated hosts/subnets and clears the buffer.
    pub async fn drain_entities(&self) -> BufferedEntities {
        self.entity_buffer.drain().await
    }
}
