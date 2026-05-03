//! Topology subscribers.
//!
//! Two responsibilities:
//!
//! 1. **Live-update broadcaster** for the topology view. When discovery
//!    inserts/updates/deletes any topology-relevant entity (host, ip_address,
//!    service, subnet, dependency, port, binding, interface, vlan, tag),
//!    we broadcast the affected `network_id` on `live_update_tx`. Frontend
//!    SSE consumers refetch and re-render. The legacy
//!    `is_stale + removed_* + auto/manual rebuild` state machine is gone —
//!    the live view is now always fresh.
//!
//! 2. **Snapshot subscriber.** When a `Snapshot` row is `Created`, build a
//!    fresh topology row pinned to that snapshot via
//!    `snapshot_id = snapshot.id`. Loads the as-of-T entity set, runs
//!    `build_graph`, and INSERTs the topology row. Live-view options are
//!    cloned over.

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

        // Broadcast live-update pings for every affected network. The
        // SSE handler filters by user network_ids before forwarding.
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
            self.build_snapshot_topology(snapshot_id, network_id, taken_at)
                .await?;
        }

        Ok(())
    }

    fn debounce_window_ms(&self) -> u64 {
        200
    }
}

impl TopologyService {
    /// Insert a topology row pinned to `snapshot_id`. Loads the as-of-T
    /// entity set, runs `build_graph` from scratch, and clones `options`
    /// from the network's live-view topology row (if one exists).
    async fn build_snapshot_topology(
        &self,
        snapshot_id: Uuid,
        network_id: Uuid,
        taken_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), Error> {
        // Find the live-view row for this network to seed `options`.
        let topo_filter = StorageFilter::<Topology>::new_from_network_ids(&[network_id]);
        let topologies = self.get_all(topo_filter).await?;
        let live_options = topologies
            .into_iter()
            .find(|t| t.base.snapshot_id.is_none())
            .map(|t| t.base.options)
            .unwrap_or_default();

        let data = self.get_topology_data(network_id, Some(taken_at)).await?;

        let (nodes, edges) = self.build_graph(BuildGraphParams {
            options: &live_options,
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
            old_nodes: &[],
            old_edges: &[],
            old_view: None,
        });

        let topology = Topology {
            id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            base: TopologyBase {
                network_id,
                options: live_options,
                nodes,
                edges,
                snapshot_id: Some(snapshot_id),
            },
        };

        self.storage().create(&topology).await?;
        Ok(())
    }
}

inventory::submit!(SubscriberRegistration::new::<
    TopologyService,
    EntityOperation,
>());
