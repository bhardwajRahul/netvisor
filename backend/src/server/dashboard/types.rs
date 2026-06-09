use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::daemons::r#impl::api::DaemonResponse;
use crate::server::discovery::r#impl::base::Discovery;

/// Per-network summary of entity counts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkSummary {
    pub id: Uuid,
    pub name: String,
    pub host_count: u64,
    pub service_count: u64,
    pub subnet_count: u64,
    pub daemon_count: u64,
}

/// Plan usage limits and current counts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlanUsage {
    pub host_limit: Option<u64>,
    pub host_count: u64,
    pub network_limit: Option<u64>,
    pub network_count: u64,
    pub seat_limit: Option<u64>,
    pub seat_count: u64,
    /// Snapshot retention window in days. `0` means snapshots are not
    /// available on this plan; the UI uses this to disable the "Take
    /// snapshot" button and surface the upgrade hook.
    pub snapshot_retention_days: u32,
}

/// Dashboard summary response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardSummary {
    pub networks: Vec<NetworkSummary>,
    pub daemons: Vec<DaemonResponse>,
    pub recent_discoveries: Vec<Discovery>,
    pub plan_usage: PlanUsage,
}
