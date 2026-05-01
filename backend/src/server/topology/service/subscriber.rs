//! Topology subscriber for entity events whose lifecycle invalidates a
//! topology snapshot (Host, IPAddress, Service, Subnet, Dependency, Port,
//! Binding, Interface, Vlan, Tag, Topology).
//!
//! Marks per-network topology snapshots stale, applies removed-entity lists,
//! and refreshes the cached entity bundles when underlying state changes.

use std::collections::HashMap;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        entities::{Entity, EntityDiscriminants},
        events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, Subscriber},
            types::{EntityOperation, EntityOperationDiscriminants},
        },
        services::traits::CrudService,
        storage::filter::StorableFilter as StorageFilter,
    },
    topology::{service::main::TopologyService, types::base::Topology},
};
use anyhow::Error;
use async_trait::async_trait;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Default)]
struct TopologyChanges {
    updated_hosts: bool,
    updated_ip_addresses: bool,
    updated_services: bool,
    updated_subnets: bool,
    updated_dependencies: bool,
    updated_ports: bool,
    updated_bindings: bool,
    updated_if_entries: bool,
    updated_vlans: bool,
    removed_hosts: HashSet<Uuid>,
    removed_ip_addresses: HashSet<Uuid>,
    removed_services: HashSet<Uuid>,
    removed_subnets: HashSet<Uuid>,
    removed_dependencies: HashSet<Uuid>,
    removed_ports: HashSet<Uuid>,
    removed_bindings: HashSet<Uuid>,
    removed_interfaces: HashSet<Uuid>,
    removed_vlans: HashSet<Uuid>,
    should_mark_stale: bool,
    clear_stale: bool,
}

#[async_trait]
impl Subscriber<EntityOperation> for TopologyService {
    fn filter(&self) -> EntityEventFilter {
        // All-ops on entities whose lifecycle invalidates a topology view.
        // Topology itself only matters on Created/Updated (Deleted handled
        // elsewhere via the cascading delete).
        let topology_ops = Some(vec![
            EntityOperationDiscriminants::Created,
            EntityOperationDiscriminants::Updated,
        ]);
        let all_ops = None;
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
            (EntityDiscriminants::Topology, topology_ops),
        ]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        // Collect all affected network IDs
        let mut network_ids = std::collections::HashSet::new();

        // Group events by network_id -> topology changes
        let mut topology_updates: HashMap<Uuid, TopologyChanges> = HashMap::new();

        // Track org-level staleness triggers (e.g., tag is_application changes)
        let mut stale_org_ids: HashSet<Uuid> = HashSet::new();

        for event in events {
            let entity_type = event.scope.entity_type().clone();
            let entity_id = event.scope.entity_id();
            let scope_network_id = event.scope.network_id();
            let scope_org_id = event.scope.organization_id();

            // Handle org-level entities without network_id (e.g., Tags)
            if scope_network_id.is_none() {
                if let Some(org_id) = scope_org_id
                    && event.flags.trigger_stale
                {
                    // For Tag events, the trait fires true for every Tag change.
                    // Narrow to only tags that actually affect a topology.
                    let should_mark = match &entity_type {
                        Entity::Tag(tag) => self.tag_affects_any_topology(tag.id, org_id).await,
                        _ => true,
                    };
                    if should_mark {
                        stale_org_ids.insert(org_id);
                    }
                }
                continue;
            }

            if let Some(network_id) = scope_network_id {
                let trigger_stale = event.flags.trigger_stale;
                let clear_stale = event.flags.clear_stale;

                // Topology updates from changes to options should be applied immediately and not processed alongside
                // other changes, otherwise another call to topology_service.update will be made which will trigger
                // an infinite loop
                if let Entity::Topology(boxed_topology) = entity_type.clone()
                    && event.operation == EntityOperation::Updated
                {
                    let topology = *boxed_topology;
                    let _ = self.staleness_tx.send(topology).inspect_err(|e| {
                        tracing::debug!("Staleness notification skipped (no receivers): {}", e)
                    });
                    continue;
                }

                network_ids.insert(network_id);

                let changes = topology_updates.entry(network_id).or_default();

                // Track removed entities
                if event.operation == EntityOperation::Deleted {
                    match &entity_type {
                        Entity::Host(_) => changes.removed_hosts.insert(entity_id),
                        Entity::IPAddress(_) => changes.removed_ip_addresses.insert(entity_id),
                        Entity::Service(_) => changes.removed_services.insert(entity_id),
                        Entity::Subnet(_) => changes.removed_subnets.insert(entity_id),
                        Entity::Dependency(_) => changes.removed_dependencies.insert(entity_id),
                        Entity::Port(_) => changes.removed_ports.insert(entity_id),
                        Entity::Binding(_) => changes.removed_bindings.insert(entity_id),
                        Entity::Interface(_) => changes.removed_interfaces.insert(entity_id),
                        Entity::Vlan(_) => changes.removed_vlans.insert(entity_id),
                        _ => false,
                    };
                }

                if trigger_stale {
                    changes.should_mark_stale = true;
                } else if clear_stale {
                    changes.clear_stale = true;
                } else {
                    match &entity_type {
                        Entity::Host(_) => changes.updated_hosts = true,
                        Entity::IPAddress(_) => changes.updated_ip_addresses = true,
                        Entity::Service(_) => changes.updated_services = true,
                        Entity::Subnet(_) => changes.updated_subnets = true,
                        Entity::Dependency(_) => changes.updated_dependencies = true,
                        Entity::Port(_) => changes.updated_ports = true,
                        Entity::Binding(_) => changes.updated_bindings = true,
                        Entity::Interface(_) => changes.updated_if_entries = true,
                        Entity::Vlan(_) => changes.updated_vlans = true,
                        _ => (),
                    };
                }
            }
        }

        // Mark all topologies in affected orgs as stale (for org-level entities like tags)
        for org_id in &stale_org_ids {
            let network_filter =
                StorageFilter::<crate::server::networks::r#impl::Network>::new_from_org_id(org_id);
            let networks = self.network_service.get_all(network_filter).await?;
            for network in &networks {
                let topo_filter = StorageFilter::<Topology>::new_from_network_ids(&[network.id]);
                let topologies = self.get_all(topo_filter).await?;
                for mut topology in topologies {
                    if !topology.base.is_stale {
                        topology.base.is_stale = true;
                        let updated = self
                            .update(&mut topology, AuthenticatedEntity::System)
                            .await?;
                        let _ = self.staleness_tx.send(updated).inspect_err(|e| {
                            tracing::debug!("Staleness notification skipped (no receivers): {}", e)
                        });
                    }
                }
            }
        }

        // Apply changes to all topologies in affected networks
        for network_id in network_ids {
            let network_filter = StorageFilter::<Topology>::new_from_network_ids(&[network_id]);
            let topologies = self.get_all(network_filter).await?;

            let (hosts, ip_addresses, subnets, dependencies, ports, bindings, interfaces) =
                self.get_entity_data(network_id).await?;

            if let Some(changes) = topology_updates.get(&network_id) {
                for mut topology in topologies {
                    let services = self.get_service_data(network_id).await?;

                    for host_id in &changes.removed_hosts {
                        if !topology.base.removed_hosts.contains(host_id) {
                            topology.base.removed_hosts.push(*host_id);
                        }
                    }
                    for ip_address_id in &changes.removed_ip_addresses {
                        if !topology.base.removed_ip_addresses.contains(ip_address_id) {
                            topology.base.removed_ip_addresses.push(*ip_address_id);
                        }
                    }
                    for service_id in &changes.removed_services {
                        if !topology.base.removed_services.contains(service_id) {
                            topology.base.removed_services.push(*service_id);
                        }
                    }
                    for subnet_id in &changes.removed_subnets {
                        if !topology.base.removed_subnets.contains(subnet_id) {
                            topology.base.removed_subnets.push(*subnet_id);
                        }
                    }
                    for dependency_id in &changes.removed_dependencies {
                        if !topology.base.removed_dependencies.contains(dependency_id) {
                            topology.base.removed_dependencies.push(*dependency_id);
                        }
                    }
                    for port_id in &changes.removed_ports {
                        if !topology.base.removed_ports.contains(port_id) {
                            topology.base.removed_ports.push(*port_id);
                        }
                    }
                    for binding_id in &changes.removed_bindings {
                        if !topology.base.removed_bindings.contains(binding_id) {
                            topology.base.removed_bindings.push(*binding_id);
                        }
                    }
                    for interface_id in &changes.removed_interfaces {
                        if !topology.base.removed_interfaces.contains(interface_id) {
                            topology.base.removed_interfaces.push(*interface_id);
                        }
                    }

                    if changes.should_mark_stale && !changes.clear_stale {
                        topology.base.is_stale = true;
                    }

                    if changes.clear_stale {
                        topology.base.is_stale = false;
                    }

                    if changes.updated_hosts && changes.removed_hosts.is_empty() {
                        topology.base.hosts = hosts.clone()
                    }

                    if changes.updated_ip_addresses && changes.removed_ip_addresses.is_empty() {
                        topology.base.ip_addresses = ip_addresses.clone()
                    }

                    if changes.updated_services && changes.removed_services.is_empty() {
                        topology.base.services = services
                    }

                    if changes.updated_subnets && changes.removed_subnets.is_empty() {
                        topology.base.subnets = subnets.clone()
                    }

                    if changes.updated_dependencies && changes.removed_dependencies.is_empty() {
                        topology.base.dependencies = dependencies.clone();
                    }

                    if changes.updated_ports && changes.removed_ports.is_empty() {
                        topology.base.ports = ports.clone();
                    }

                    if changes.updated_bindings && changes.removed_bindings.is_empty() {
                        topology.base.bindings = bindings.clone();
                    }

                    if changes.updated_if_entries && changes.removed_interfaces.is_empty() {
                        topology.base.interfaces = interfaces.clone();
                    }

                    let updated = self
                        .update(&mut topology, AuthenticatedEntity::System)
                        .await?;

                    let _ = self.staleness_tx.send(updated).inspect_err(|e| {
                        tracing::debug!("Staleness notification skipped (no receivers): {}", e)
                    });
                }
            }
        }

        Ok(())
    }

    fn debounce_window_ms(&self) -> u64 {
        200
    }
}
inventory::submit!(SubscriberRegistration::new::<
    TopologyService,
    EntityOperation,
>());
