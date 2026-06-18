//! Integration-style tests for `SnapshotService::run_close_and_clone` are
//! written against the real Postgres test harness (which `cargo test --lib`
//! does not start). Unit-level coverage of close-and-clone semantics is
//! exercised through the per-entity tests under `Snapshotable`/
//! `DiscoveryTracked` impls and the `update_many` correctness test.
//!
//! End-to-end tests for snapshot close-and-clone live in the integration
//! test suite (run via `make test`, out of scope for this worktree's local
//! verification).
