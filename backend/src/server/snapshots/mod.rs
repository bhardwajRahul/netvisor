//! Network snapshots: close-and-clone the live SCD2 set for a network at a
//! single timestamp T.
//!
//! There is no `Snapshot` entity / DB table in this worktree. A snapshot is
//! just a `taken_at` timestamp; closed entity rows have `valid_to = T` and a
//! synthetic id with `lineage_id` pointing at the live row's id, so the
//! "snapshot" is fully derivable from the SCD2 substrate. Future worktrees
//! may layer `network_snapshot_settings`, `snapshot_annotations` (for named
//! / pinned moments), and a retention deletion job on top.
//!
//! Coordination with discovery is in `DiscoveryService`:
//! `try_acquire_network_for_snapshot` / `release_network_for_snapshot` and
//! the `AwaitingSnapshot` discovery phase. The manual-snapshot API handler
//! (built in the UI worktree) wraps acquire → run → release around
//! `SnapshotService::run_close_and_clone`.

pub mod service;

#[cfg(test)]
mod service_test;
