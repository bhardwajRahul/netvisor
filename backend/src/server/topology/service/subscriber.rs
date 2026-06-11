//! Topology subscribers.
//!
//! Two responsibilities, both implemented via the unified `rebuild_topology`
//! helper:
//!
//! 1. **Live-view rebuild + SSE broadcast.** When discovery
//!    inserts/updates/deletes any topology-relevant entity (host, ip_address,
//!    service, subnet, dependency, port, binding, interface, vlan, tag), we
//!    rebuild the network's live-view topology row in place (so the row's
//!    `nodes`/`edges` reflect current entity state) and then broadcast the
//!    affected `network_id` on `live_update_tx` so frontend SSE consumers
//!    refetch and render the fresh graph. The legacy
//!    `is_stale + removed_* + auto/manual rebuild` state machine is gone.
//!
//! 2. **Snapshot subscriber.** When a `Snapshot` row is `Created`, INSERT a
//!    new topology row pinned to that snapshot at `taken_at`, with
//!    `snapshot_id = snapshot.id`. Options are cloned from the live-view
//!    row.
//!
//! Both paths flow through `rebuild_topology(network_id, snapshot)` — the
//! snapshot path passes `Some((id, taken_at))` and INSERTs a new row, the
//! live path passes `None` and UPDATEs the existing live-view row.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::server::{
    shared::{
        entities::{Entity, EntityDiscriminants},
        events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, Subscriber},
            types::{EntityOperation, EntityOperationDiscriminants},
        },
        services::traits::CrudService,
        storage::{filter::StorableFilter as StorageFilter, traits::Storage},
    },
    topology::{
        service::main::{BuildGraphParams, TopologyService},
        types::base::{Topology, TopologyBase},
    },
};
use anyhow::Error;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
impl Subscriber<EntityOperation> for TopologyService {
    fn filter(&self) -> EntityEventFilter {
        let all_ops = None;
        // Snapshot is `Created`-only — no Updated/Deleted handling needed.
        let snapshot_ops = Some(vec![EntityOperationDiscriminants::Created]);
        EntityEventFilter::by_entity(HashMap::from([
            (EntityDiscriminants::Host, all_ops.clone()),
            (EntityDiscriminants::IPAddress, all_ops.clone()),
            (EntityDiscriminants::Service, all_ops.clone()),
            (EntityDiscriminants::Subnet, all_ops.clone()),
            (EntityDiscriminants::Dependency, all_ops.clone()),
            (EntityDiscriminants::Port, all_ops.clone()),
            (EntityDiscriminants::Binding, all_ops.clone()),
            (EntityDiscriminants::Interface, all_ops.clone()),
            (EntityDiscriminants::Vlan, all_ops.clone()),
            (EntityDiscriminants::Tag, all_ops),
            (EntityDiscriminants::Snapshot, snapshot_ops),
        ]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        let mut affected_networks: HashSet<Uuid> = HashSet::new();
        let mut snapshot_creations: Vec<(Uuid, Uuid, chrono::DateTime<chrono::Utc>)> = Vec::new();

        for event in events {
            // Handle Snapshot::Created — build a topology row anchored to it.
            if let Entity::Snapshot(snap) = event.scope.entity_type()
                && event.operation == EntityOperation::Created
            {
                snapshot_creations.push((snap.id, snap.base.network_id, snap.base.taken_at));
                continue;
            }

            // For org-scoped events (e.g., Tag changes), fan out to every
            // network in the org so live consumers refetch.
            let scope_network_id = event.scope.network_id();
            let scope_org_id = event.scope.organization_id();

            if let Some(network_id) = scope_network_id {
                affected_networks.insert(network_id);
            } else if let Some(org_id) = scope_org_id {
                let nets = self
                    .network_service
                    .get_all(
                        StorageFilter::<crate::server::networks::r#impl::Network>::new_from_org_id(
                            &org_id,
                        ),
                    )
                    .await?;
                for n in nets {
                    affected_networks.insert(n.id);
                }
            }
        }

        // Rebuild the live-view topology row for every affected network so
        // its nodes/edges reflect current entity state BEFORE we ping the
        // SSE consumers. Done first so the subsequent refetch reads the
        // updated row.
        for &network_id in &affected_networks {
            self.rebuild_topology(network_id, None).await?;
        }

        // Broadcast live-update pings. The SSE handler filters by user
        // network_ids before forwarding.
        for network_id in &affected_networks {
            let _ = self.live_update_tx.send(*network_id).inspect_err(|e| {
                tracing::debug!(
                    network_id = %network_id,
                    "Live-update broadcast skipped (no receivers): {e}"
                )
            });
        }

        // Build snapshot-pinned topology rows. We do this here rather than
        // in the snapshots handler so the topology lifecycle stays inside
        // its own service.
        for (snapshot_id, network_id, taken_at) in snapshot_creations {
            self.rebuild_topology(network_id, Some((snapshot_id, taken_at)))
                .await?;
        }

        Ok(())
    }

    fn debounce_window_ms(&self) -> u64 {
        200
    }
}

impl TopologyService {
    /// Materialize a topology row for `network_id`.
    ///
    /// - `snapshot: None` → UPDATE the live-view row in place so its
    ///   `nodes`/`edges` reflect current entity state. The existing `nodes` /
    ///   `edges` are passed as `old_*` to `build_graph` so its layout-
    ///   preservation logic fires (nodes don't randomly jump on every
    ///   discovery). Returns an error if no live-view row exists for the
    ///   network — that's a real bug since live rows are created at network
    ///   creation time.
    /// - `snapshot: Some((id, taken_at))` → INSERT a new topology row pinned
    ///   to the snapshot. Loads the as-of-T entity set, runs `build_graph`
    ///   from scratch (no `old_*` carry-over), clones `options` from the
    ///   live-view row.
    async fn rebuild_topology(
        &self,
        network_id: Uuid,
        snapshot: Option<(Uuid, chrono::DateTime<chrono::Utc>)>,
    ) -> Result<(), Error> {
        // Load the live-view row once — needed in both paths: snapshot path
        // uses its `options` to seed the new pinned row; live path UPDATEs
        // it in place and reads its existing nodes/edges for layout
        // preservation.
        let topo_filter = StorageFilter::<Topology>::new_from_network_ids(&[network_id]);
        let live_row = self
            .get_all(topo_filter)
            .await?
            .into_iter()
            .find(|t| t.base.snapshot_id.is_none());

        let at = snapshot.map(|(_, taken_at)| taken_at);
        let options = live_row
            .as_ref()
            .map(|t| t.base.options.clone())
            .unwrap_or_default();

        let data = self.get_topology_data(network_id, at).await?;

        // Snapshot builds preserve the existing "from scratch" behavior
        // (`old_* = &[]`). Live-view rebuilds pass the existing nodes/edges
        // so `build_graph` can keep stable node positions across discovery
        // events.
        let (old_nodes, old_edges): (&[_], &[_]) = match (snapshot.is_some(), live_row.as_ref()) {
            (false, Some(live)) => (live.base.nodes.as_slice(), live.base.edges.as_slice()),
            _ => (&[], &[]),
        };

        let (nodes, edges) = self.build_graph(BuildGraphParams {
            options: &options,
            hosts: &data.hosts,
            ip_addresses: &data.ip_addresses,
            subnets: &data.subnets,
            services: &data.services,
            dependencies: &data.dependencies,
            ports: &data.ports,
            bindings: &data.bindings,
            interfaces: &data.interfaces,
            entity_tags: &data.tags,
            vlans: &data.vlans,
            old_nodes,
            old_edges,
            old_view: None,
        });

        match snapshot {
            Some((snapshot_id, _)) => {
                let topology = Topology {
                    id: Uuid::new_v4(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    base: TopologyBase {
                        network_id,
                        options,
                        nodes,
                        edges,
                        snapshot_id: Some(snapshot_id),
                    },
                };
                self.storage().create(&topology).await?;
            }
            None => {
                let mut live = live_row.ok_or_else(|| {
                    anyhow::anyhow!(
                        "No live-view topology row for network {} — should have been created at network creation",
                        network_id,
                    )
                })?;
                live.base.nodes = nodes;
                live.base.edges = edges;
                self.storage().update(&mut live).await?;
            }
        }

        Ok(())
    }
}

inventory::submit!(SubscriberRegistration::new::<
    TopologyService,
    EntityOperation,
>());
