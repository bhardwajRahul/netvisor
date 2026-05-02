//! Snapshotable / DiscoveryTracked traits + FkMaps.
//!
//! `Snapshotable` is the SCD2 (slowly-changing-dimension type 2) substrate:
//! every implementing row has `valid_from` / `valid_to` / `lineage_id`.
//! Live rows have `valid_to IS NULL` and `lineage_id IS NULL`. Closed
//! historical copies have a synthetic id, `lineage_id` pointing at the
//! original live row's id, and a `valid_to` timestamp.
//!
//! `DiscoveryTracked` extends `Snapshotable` with audit columns populated by
//! daemon discovery: `last_seen_at` (refreshed on every successful natural-key
//! match) plus FK columns to `discoveries(id)` for the discovery that first
//! saw the entity and the discovery that last touched it.
//!
//! `FkMaps` carries the per-entity-type live-id → closed-id mapping populated
//! parents-first during snapshot close-and-clone. Children read from it in
//! `Snapshotable::remap_fks_for_clone` to rewrite their within-tracked FK
//! columns to point at closed counterparts (rather than at live rows whose
//! data has since moved on).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::traits::Storable;

/// SCD2 row lifecycle accessors. Shared by entities (Host, Service, …) and
/// junction tables (subnet_vlans, dependency_members, entity_tags) that
/// participate in network snapshots or per-action close-and-clone.
pub trait Snapshotable: Storable {
    fn id_value(&self) -> Uuid;
    fn set_id_value(&mut self, id: Uuid);

    fn valid_from(&self) -> DateTime<Utc>;
    fn valid_to(&self) -> Option<DateTime<Utc>>;
    fn lineage_id(&self) -> Option<Uuid>;

    fn set_valid_from(&mut self, t: DateTime<Utc>);
    fn set_valid_to(&mut self, t: Option<DateTime<Utc>>);
    fn set_lineage_id(&mut self, id: Option<Uuid>);

    /// Build a closed historical copy of this row at `close_at`. The caller
    /// assigns a new id to the returned copy via `set_id_value` before INSERT.
    /// Used by both snapshot-driven cloning (SnapshotService) and per-action
    /// close-and-clone (TagService::update via SnapshotMutator blanket impl).
    fn make_closed_copy(&self, close_at: DateTime<Utc>) -> Self {
        let mut closed = self.clone();
        closed.set_lineage_id(Some(self.id_value()));
        closed.set_valid_to(Some(close_at));
        // valid_from preserved (the closed row covers [valid_from, close_at]).
        closed
    }

    /// Per-entity FK remapping during snapshot-driven cloning. Each impl
    /// rewrites any of its FK columns that point at other tracked entities
    /// using the supplied id maps (live_id → closed_id, populated parents-
    /// first by the SnapshotService orchestrator).
    ///
    /// Default: no FKs to remap. Used by hosts, subnets, vlans (top-level
    /// entities with no within-tracked FKs).
    fn remap_fks_for_clone(&mut self, _maps: &FkMaps) {}
}

/// Accessors for the discovery-driven audit columns.
///
/// `last_seen_at` advances on every successful natural-key match by daemon
/// discovery. `last_discovery_id` and `first_discovery_id` are populated
/// post-terminal by per-entity-service subscribers on the
/// `DiscoveryProcessed` event.
pub trait DiscoveryTracked: Snapshotable {
    fn last_seen_at(&self) -> DateTime<Utc>;
    fn last_discovery_id(&self) -> Option<Uuid>;
    fn first_discovery_id(&self) -> Option<Uuid>;

    fn set_last_seen_at(&mut self, t: DateTime<Utc>);
    fn set_last_discovery_id(&mut self, id: Option<Uuid>);
    fn set_first_discovery_id(&mut self, id: Option<Uuid>);
}

/// Per-entity-type live-id → closed-id mappings populated parents-first
/// during snapshot close-and-clone. Children consult these to rewrite their
/// FK columns so closed copies reference closed parents.
#[derive(Debug, Default, Clone)]
pub struct FkMaps {
    pub hosts: HashMap<Uuid, Uuid>,
    pub subnets: HashMap<Uuid, Uuid>,
    pub vlans: HashMap<Uuid, Uuid>,
    pub services: HashMap<Uuid, Uuid>,
    pub ip_addresses: HashMap<Uuid, Uuid>,
    pub ports: HashMap<Uuid, Uuid>,
    pub interfaces: HashMap<Uuid, Uuid>,
    pub dependencies: HashMap<Uuid, Uuid>,
}

impl FkMaps {
    /// Lookup helper for entity_tags. The row's `entity_type` is typed as
    /// `EntityDiscriminants` in the app (serialized to/from text in DB via
    /// `SqlValue::EntityDiscriminant`). Returns None for org-scoped variants
    /// (Daemon, User, DaemonApiKey, UserApiKey, etc.) — those rows aren't
    /// cloned at network snapshot.
    pub fn lookup_by_entity_type(
        &self,
        entity_type: EntityDiscriminants,
        live_id: Uuid,
    ) -> Option<Uuid> {
        match entity_type {
            EntityDiscriminants::Host => self.hosts.get(&live_id).copied(),
            EntityDiscriminants::Service => self.services.get(&live_id).copied(),
            EntityDiscriminants::Subnet => self.subnets.get(&live_id).copied(),
            EntityDiscriminants::Dependency => self.dependencies.get(&live_id).copied(),
            _ => None,
        }
    }
}
