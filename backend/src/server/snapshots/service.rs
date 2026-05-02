use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use crate::server::shared::storage::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    snapshot::{FkMaps, Snapshotable},
    traits::Storable,
};

/// Network snapshots: close-and-clone the live row set for a network at a
/// single timestamp inside one transaction.
pub struct SnapshotService {
    pool: Arc<PgPool>,
}

impl SnapshotService {
    pub fn new(pool: Arc<PgPool>) -> Arc<Self> {
        Arc::new(Self { pool })
    }

    /// Synchronous orchestration of one network snapshot at `taken_at`.
    ///
    /// Caller (the manual-snapshot API handler or future scheduled-snapshot
    /// machinery) is responsible for `DiscoveryService::try_acquire_network_for_snapshot`
    /// before calling, and `release_network_for_snapshot` after — regardless
    /// of result.
    ///
    /// All twelve network-scoped Snapshotable entity types are processed
    /// parents-first so child rows can remap their FK columns to the closed
    /// parent ids via [`FkMaps`]. The whole sequence runs in a single
    /// `sqlx::Transaction`; if any step fails, nothing is committed.
    pub async fn run_close_and_clone(
        &self,
        network_id: Uuid,
        taken_at: DateTime<Utc>,
    ) -> Result<()> {
        use crate::server::bindings::r#impl::base::Binding;
        use crate::server::dependencies::dependency_members::DependencyMemberRecord;
        use crate::server::dependencies::r#impl::base::Dependency;
        use crate::server::hosts::r#impl::base::Host;
        use crate::server::interfaces::r#impl::base::Interface;
        use crate::server::ip_addresses::r#impl::base::IPAddress;
        use crate::server::ports::r#impl::base::Port;
        use crate::server::services::r#impl::base::Service;
        use crate::server::subnets::r#impl::base::Subnet;
        use crate::server::tags::entity_tags::EntityTag;
        use crate::server::vlans::r#impl::base::Vlan;
        use crate::server::vlans::r#impl::subnet_vlans::SubnetVlanRecord;

        let mut tx = self.pool.begin().await?;
        let mut maps = FkMaps::default();

        // Top-level network-scoped entities (no within-tracked FKs to remap).
        let subnet_map = close_and_clone_for::<Subnet>(
            &mut tx,
            network_filter::<Subnet>(network_id),
            taken_at,
            &maps,
        )
        .await?;
        maps.subnets = subnet_map;

        let vlan_map = close_and_clone_for::<Vlan>(
            &mut tx,
            network_filter::<Vlan>(network_id),
            taken_at,
            &maps,
        )
        .await?;
        maps.vlans = vlan_map;

        let host_map = close_and_clone_for::<Host>(
            &mut tx,
            network_filter::<Host>(network_id),
            taken_at,
            &maps,
        )
        .await?;
        maps.hosts = host_map;

        // Children of hosts/subnets/vlans. Remap their FK columns using the
        // parent maps populated above.
        let ip_map = close_and_clone_for::<IPAddress>(
            &mut tx,
            network_filter::<IPAddress>(network_id),
            taken_at,
            &maps,
        )
        .await?;
        maps.ip_addresses = ip_map;

        // Ports filter through host_id (Port has no network_id column).
        let host_ids: Vec<Uuid> = maps.hosts.keys().copied().collect();
        let port_map = close_and_clone_for::<Port>(
            &mut tx,
            StorableFilter::<Port>::new_from_uuids_column("host_id", &host_ids).live(),
            taken_at,
            &maps,
        )
        .await?;
        maps.ports = port_map;

        let service_map = close_and_clone_for::<Service>(
            &mut tx,
            network_filter::<Service>(network_id),
            taken_at,
            &maps,
        )
        .await?;
        maps.services = service_map;

        // Interfaces filter through host_id (Interface has no network_id).
        let interface_map = close_and_clone_for::<Interface>(
            &mut tx,
            StorableFilter::<Interface>::new_from_uuids_column("host_id", &host_ids).live(),
            taken_at,
            &maps,
        )
        .await?;
        maps.interfaces = interface_map;

        // Bindings filter through service_id (Binding has no network_id).
        // BINDINGS must come before DEPENDENCY_MEMBERS so dep_member's
        // optional binding_id can be remapped.
        let service_ids: Vec<Uuid> = maps.services.keys().copied().collect();
        let binding_map = close_and_clone_for::<Binding>(
            &mut tx,
            StorableFilter::<Binding>::new_from_uuids_column("service_id", &service_ids).live(),
            taken_at,
            &maps,
        )
        .await?;
        maps.bindings = binding_map;

        // SubnetVlan junction filters through subnet_id.
        let subnet_ids: Vec<Uuid> = maps.subnets.keys().copied().collect();
        let _ = close_and_clone_for::<SubnetVlanRecord>(
            &mut tx,
            StorableFilter::<SubnetVlanRecord>::new_from_uuids_column("subnet_id", &subnet_ids)
                .live(),
            taken_at,
            &maps,
        )
        .await?;

        let dependency_map = close_and_clone_for::<Dependency>(
            &mut tx,
            network_filter::<Dependency>(network_id),
            taken_at,
            &maps,
        )
        .await?;
        maps.dependencies = dependency_map;

        // DependencyMembers filter through dependency_id.
        let dependency_ids: Vec<Uuid> = maps.dependencies.keys().copied().collect();
        let _ = close_and_clone_for::<DependencyMemberRecord>(
            &mut tx,
            StorableFilter::<DependencyMemberRecord>::new_from_uuids_column(
                "dependency_id",
                &dependency_ids,
            )
            .live(),
            taken_at,
            &maps,
        )
        .await?;

        // EntityTags: filter to network-scoped entity types only. Org-scoped
        // variants (Daemon, User, DaemonApiKey, UserApiKey, etc.) are not
        // cloned at network snapshot. The set of network-scoped entity ids
        // is the union of host/service/subnet/dependency live ids that
        // were just cloned (via maps).
        let entity_ids: Vec<Uuid> = maps
            .hosts
            .keys()
            .chain(maps.services.keys())
            .chain(maps.subnets.keys())
            .chain(maps.dependencies.keys())
            .copied()
            .collect();
        let _ = close_and_clone_for::<EntityTag>(
            &mut tx,
            StorableFilter::<EntityTag>::new_from_uuids_column("entity_id", &entity_ids).live(),
            taken_at,
            &maps,
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

/// Standard "live rows on this network" filter. Used by the entity types
/// that carry a `network_id` column directly (subnets, vlans, hosts,
/// ip_addresses, services, dependencies). Junction tables that lack
/// `network_id` are filtered through their parents above.
fn network_filter<T: Storable>(network_id: Uuid) -> StorableFilter<T> {
    StorableFilter::<T>::new_from_network_ids(&[network_id]).live()
}

/// Generic close-and-clone for one Snapshotable entity type within a shared
/// transaction. Returns the per-type live-id → closed-id map for downstream
/// children to consult via `FkMaps`.
async fn close_and_clone_for<T>(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    filter: StorableFilter<T>,
    taken_at: DateTime<Utc>,
    parent_maps: &FkMaps,
) -> Result<HashMap<Uuid, Uuid>>
where
    T: Snapshotable + std::fmt::Display,
{
    let live = GenericPostgresStorage::<T>::get_all_in_tx(filter, tx).await?;

    if live.is_empty() {
        return Ok(HashMap::new());
    }

    let mut closed: Vec<T> = Vec::with_capacity(live.len());
    let mut mapping: HashMap<Uuid, Uuid> = HashMap::with_capacity(live.len());

    for original in &live {
        let mut copy = original.make_closed_copy(taken_at);
        let new_id = Uuid::new_v4();
        copy.set_id_value(new_id);
        copy.remap_fks_for_clone(parent_maps);
        mapping.insert(original.id_value(), new_id);
        closed.push(copy);
    }

    GenericPostgresStorage::<T>::create_many_in_tx(&closed, tx).await?;

    let advanced_live: Vec<T> = live
        .into_iter()
        .map(|mut e| {
            e.set_valid_from(taken_at);
            e
        })
        .collect();
    GenericPostgresStorage::<T>::update_many_in_tx(&advanced_live, tx).await?;

    Ok(mapping)
}
