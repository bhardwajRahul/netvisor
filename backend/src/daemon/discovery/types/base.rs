use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use strum_macros::EnumDiscriminants;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::discovery::r#impl::types::DiscoveryType;

#[derive(
    Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Hash, ToSchema, EnumDiscriminants,
)]
#[strum_discriminants(derive(Hash, EnumIter, strum::Display, Serialize, Deserialize))]
pub enum DiscoveryPhase {
    /// Blocked: a network snapshot is in progress on this network. The session
    /// can't enter the normal Queued/Pending decision until
    /// `release_network_for_snapshot` clears the block. Daemons polling for
    /// work do NOT see AwaitingSnapshot sessions.
    AwaitingSnapshot,
    Queued,   // Waiting in daemon queue behind another session
    Pending,  // Front of queue, eligible for dispatch. Clock ticking.
    Starting, // get_pending_work() picked it up, dispatching to daemon
    Started,  // Daemon acknowledged, actively running
    Scanning,
    Complete,
    Failed,
    Cancelled,
}

impl DiscoveryPhase {
    pub fn log_level(&self) -> crate::server::shared::events::types::EventLogLevel {
        use crate::server::shared::events::types::EventLogLevel;
        match self {
            DiscoveryPhase::Failed => EventLogLevel::Warn,
            _ => EventLogLevel::Info,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            DiscoveryPhase::Complete | DiscoveryPhase::Cancelled | DiscoveryPhase::Failed
        )
    }

    /// Whether this phase is subject to stall cleanup.
    ///
    /// - `Queued`: No — waiting in queue, no dispatch attempted, no clock running
    /// - `Pending`: Yes — promoted to front of queue, dispatch expected within a poll cycle.
    ///   If still here after 5 min, daemon is unreachable.
    /// - `Starting`/`Started`/`Scanning`: Yes — dispatched, should be progressing
    /// - Terminal states: No — already done
    pub fn can_be_cleaned_up(&self) -> bool {
        matches!(
            self,
            DiscoveryPhase::Pending
                | DiscoveryPhase::Starting
                | DiscoveryPhase::Started
                | DiscoveryPhase::Scanning
        )
    }
}

#[derive(Debug, Clone)]
pub struct DiscoverySessionInfo {
    pub session_id: Uuid,
    pub network_id: Uuid,
    pub daemon_id: Uuid,
    pub started_at: Option<DateTime<Utc>>,
    pub discovery_type: DiscoveryType,
    pub discovery_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct DiscoverySessionUpdate {
    pub phase: DiscoveryPhase,
    /// Percentage complete (0-100)
    pub progress: u8,
    pub error: Option<String>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl DiscoverySessionUpdate {
    pub fn scanning(progress: u8) -> Self {
        Self {
            phase: DiscoveryPhase::Scanning,
            progress: progress.min(100),
            error: None,
            finished_at: None,
        }
    }
}

impl std::fmt::Display for DiscoveryPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryPhase::AwaitingSnapshot => {
                write!(f, "Waiting for network snapshot to complete")
            }
            DiscoveryPhase::Queued => write!(f, "Waiting in queue behind another session"),
            DiscoveryPhase::Pending => {
                write!(f, "Session created, waiting for daemon availability")
            }
            DiscoveryPhase::Starting => write!(f, "Sending session to daemon"),
            DiscoveryPhase::Started => write!(f, "Session started in daemon"),
            DiscoveryPhase::Scanning => write!(f, "Scanning for active hosts"),
            DiscoveryPhase::Complete => write!(f, "Discovery complete"),
            DiscoveryPhase::Cancelled => write!(f, "Discovery cancelled"),
            DiscoveryPhase::Failed => write!(f, "Discovery failed"),
        }
    }
}

pub enum DiscoveryCriticalError {
    ResourceExhaustion,
}

impl DiscoveryCriticalError {
    pub fn is_critical_error(error_str: String) -> bool {
        Self::from_error_string(error_str).is_some()
    }

    pub fn from_error_string(error_str: String) -> Option<Self> {
        let lower_error = error_str.to_lowercase();

        if lower_error.contains("too many open files")
            || lower_error.contains("file descriptor limit")
            || lower_error.contains("cannot allocate memory")
            || lower_error.contains("out of memory")
            || lower_error.contains("os error 24")
            || lower_error.contains("emfile")
        {
            return Some(DiscoveryCriticalError::ResourceExhaustion);
        }

        None
    }
}

impl Display for DiscoveryCriticalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryCriticalError::ResourceExhaustion => {
                write!(
                    f,
                    "Resource exhaustion during scan: too many open files - CONCURRENT_SCANS is likely too high for this system. Check readme for troubleshooting."
                )
            }
        }
    }
}
