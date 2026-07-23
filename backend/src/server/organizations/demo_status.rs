//! Status of an organization's background demo-data population task.
//!
//! `populate_demo_data` runs off the request thread (a `tokio::spawn`) and
//! returns `202` immediately, so the frontend needs a pollable signal for
//! completion and failure. The status lives in an in-memory registry on
//! [`OrganizationService`](super::service::OrganizationService) keyed by
//! organization id — populate is per-org and single-flight, and the task is
//! idempotent (it resets the org first), so an in-memory map is sufficient and
//! survives across the poll without a DB round-trip.

use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// Lifecycle of a demo-populate task. `Running` is set synchronously in the
/// POST handler (before the `202`), then flipped to a terminal variant by the
/// spawned task. `Failed` carries the error string so the UI can show why.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DemoPopulateStatus {
    Running {
        started_at: DateTime<Utc>,
    },
    Complete {
        finished_at: DateTime<Utc>,
    },
    Failed {
        error: String,
        finished_at: DateTime<Utc>,
    },
}
