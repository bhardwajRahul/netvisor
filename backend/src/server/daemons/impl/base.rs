use std::fmt::Display;

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use semver::Version;
use serde::{Deserialize, Serialize};
use strum::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    daemons::r#impl::api::DaemonCapabilities, shared::entities::ChangeTriggersTopologyStaleness,
};

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct DaemonBase {
    pub host_id: Uuid,
    pub network_id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub url: String,
    /// Timestamp of last successful contact with daemon.
    /// NULL for provisioned ServerPoll daemons that haven't been contacted yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(read_only)]
    pub last_seen: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub capabilities: DaemonCapabilities,
    pub mode: DaemonMode,
    pub name: String,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Daemon software version (semver format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub version: Option<Version>,
    /// User responsible for maintaining this daemon
    pub user_id: Uuid,
    /// Foreign key to API key used for ServerPoll authentication.
    /// NULL for DaemonPoll daemons or those not yet linked to a key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<Uuid>,
    /// Whether the daemon is unreachable (for ServerPoll circuit breaker).
    /// Set to true after repeated polling failures, reset via retry-connection endpoint.
    #[serde(default)]
    pub is_unreachable: bool,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct Daemon {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: DaemonBase,
}

impl Daemon {
    pub fn suppress_logs(&self, other: &Self) -> bool {
        self.base.capabilities == other.base.capabilities
            && self.base.mode == other.base.mode
            && self.base.url == other.base.url
            && self.base.network_id == other.base.network_id
            && self.base.host_id == other.base.host_id
    }
}

impl Display for Daemon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.base.url, self.id)
    }
}

/// Daemon operating mode that determines the communication pattern.
///
/// - **DaemonPoll** (formerly "Pull"): Daemon makes outbound connections to the server.
///   The daemon registers itself and polls for work. Best for daemons behind NAT/firewall.
///
/// - **ServerPoll** (formerly "Push"): Server makes connections to the daemon.
///   Server polls daemon for status and discovery results. Best for DMZ deployments
///   where daemon cannot make outbound connections.
#[derive(
    Debug,
    Display,
    Copy,
    Clone,
    Serialize,
    Deserialize,
    Default,
    PartialEq,
    Eq,
    ValueEnum,
    Hash,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DaemonMode {
    /// Server polls daemon (daemon cannot make outbound connections)
    #[serde(alias = "push", alias = "Push")]
    ServerPoll,
    /// Daemon polls server (default, firewall-friendly)
    #[default]
    #[serde(alias = "pull", alias = "Pull")]
    DaemonPoll,
}

impl ChangeTriggersTopologyStaleness<Daemon> for Daemon {
    fn triggers_staleness(&self, _other: Option<Daemon>) -> bool {
        false
    }
}
