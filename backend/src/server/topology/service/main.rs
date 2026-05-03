use std::{collections::HashMap, sync::Arc};

use anyhow::Error;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use petgraph::{Graph, graph::NodeIndex, visit::EdgeRef};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::{r#impl::base::Binding, service::BindingService},
    dependencies::{r#impl::base::Dependency, service::DependencyService},
    hosts::{r#impl::base::Host, service::HostService},
    interfaces::{
        r#impl::base::{Interface, Neighbor},
        service::InterfaceService,
    },
    ip_addresses::{r#impl::base::IPAddress, service::IPAddressService},
    networks::service::NetworkService,
    ports::{r#impl::base::Port, service::PortService},
    services::{r#impl::base::Service, service::ServiceService},
    shared::{
        events::{bus::EventBus, types::EntityOperation},
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, Storable, Storage},
        },
    },
    subnets::{r#impl::base::Subnet, service::SubnetService},
    tags::{entity_tags::EntityTagService, r#impl::base::Tag, service::TagService},
    topology::{
        service::{context::TopologyContext, edge_builder::EdgeBuilder},
        types::{
            api::TopologyData,
            base::{Topology, TopologyOptions},
            edges::{Edge, EdgeHandle},
            grouping::GroupingConfig,
            nodes::Node,
            views::{TopologyView, TopologyViewSupport},
        },
    },
    vlans::{r#impl::base::Vlan, service::VlanService},
};

pub struct TopologyService {
    storage: Arc<GenericPostgresStorage<Topology>>,
    pub(crate) host_service: Arc<HostService>,
    pub(crate) ip_address_service: Arc<IPAddressService>,
    pub(crate) subnet_service: Arc<SubnetService>,
    pub(crate) dependency_service: Arc<DependencyService>,
    pub(crate) service_service: Arc<ServiceService>,
    pub(crate) port_service: Arc<PortService>,
    pub(crate) binding_service: Arc<BindingService>,
    pub(crate) interface_service: Arc<InterfaceService>,
    pub(crate) tag_service: Arc<TagService>,
    pub(crate) vlan_service: Arc<VlanService>,
    pub(crate) network_service: Arc<NetworkService>,
    event_bus: Arc<EventBus>,
    /// Broadcast channel emitting `network_id`s whose live entity set has
    /// just changed. Frontend SSE consumers refetch the live topology row +
    /// entity data on receipt. Replaces the legacy staleness state machine.
    pub live_update_tx: broadcast::Sender<Uuid>,
}

impl EventBusService<Topology> for TopologyService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Topology) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Topology) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Topology> for TopologyService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Topology>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }

    /// Create a topology row.
    ///
    /// Live-view rows: caller passes `snapshot_id = None`. Snapshot rows:
    /// caller passes `snapshot_id = Some(snapshot.id)` and is responsible for
    /// supplying the entity data (via the `Snapshot` subscriber path) — the
    /// service no longer fetches+caches entities on the topology row.
    async fn create(
        &self,
        entity: Topology,
        authentication: AuthenticatedEntity,
    ) -> Result<Topology, anyhow::Error> {
        let mut topology = if entity.id() == Uuid::nil() {
            Topology::new(entity.get_base())
        } else {
            entity
        };

        // For live-view rows we still seed nodes/edges from the current
        // entity state so the first render after network creation has a
        // graph to display. Snapshot rows are inserted directly by the
        // subscriber with already-built nodes/edges and don't pass through
        // here as `id == nil`.
        if topology.base.snapshot_id.is_none()
            && topology.base.nodes.is_empty()
            && topology.base.edges.is_empty()
        {
            let data = self
                .get_topology_data(topology.base.network_id, None)
                .await?;
            let (nodes, edges) = self.build_graph(BuildGraphParams {
                hosts: &data.hosts,
                ip_addresses: &data.ip_addresses,
                services: &data.services,
                subnets: &data.subnets,
                dependencies: &data.dependencies,
                ports: &data.ports,
                bindings: &data.bindings,
                interfaces: &data.interfaces,
                entity_tags: &data.tags,
                vlans: &data.vlans,
                old_edges: &[],
                old_nodes: &[],
                options: &topology.base.options,
                old_view: None,
            });
            topology.set_graph(nodes, edges);
        }

        let created = self.storage().create(&topology).await?;

        if let Some(scope) = EntityScope::from_ids(
            created.id(),
            created.clone().into(),
            self.get_network_id(&created),
            self.get_organization_id(&created),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Created, authentication)
                        .with_flags(EntityEventFlags::default()),
                )
                .await?;
        }

        Ok(created)
    }
}

pub struct BuildGraphParams<'a> {
    pub options: &'a TopologyOptions,
    pub hosts: &'a [Host],
    pub ip_addresses: &'a [IPAddress],
    pub subnets: &'a [Subnet],
    pub services: &'a [Service],
    pub dependencies: &'a [Dependency],
    pub ports: &'a [Port],
    pub bindings: &'a [Binding],
    pub interfaces: &'a [Interface],
    pub entity_tags: &'a [Tag],
    pub vlans: &'a [Vlan],
    pub old_nodes: &'a [Node],
    pub old_edges: &'a [Edge],
    pub old_view: Option<TopologyView>,
}

impl TopologyService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host_service: Arc<HostService>,
        ip_address_service: Arc<IPAddressService>,
        subnet_service: Arc<SubnetService>,
        dependency_service: Arc<DependencyService>,
        service_service: Arc<ServiceService>,
        port_service: Arc<PortService>,
        binding_service: Arc<BindingService>,
        interface_service: Arc<InterfaceService>,
        tag_service: Arc<TagService>,
        vlan_service: Arc<VlanService>,
        network_service: Arc<NetworkService>,
        storage: Arc<GenericPostgresStorage<Topology>>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        let (live_update_tx, _) = broadcast::channel(100);
        Self {
            host_service,
            ip_address_service,
            subnet_service,
            dependency_service,
            service_service,
            storage,
            port_service,
            binding_service,
            interface_service,
            tag_service,
            vlan_service,
            network_service,
            event_bus,
            live_update_tx,
        }
    }

    /// Subscribe to live-topology update pings. Each emitted `Uuid` is a
    /// network whose live entity set just changed; consumers refetch the
    /// live topology row + entity data.
    pub fn subscribe_live_topology_updates(&self) -> broadcast::Receiver<Uuid> {
        self.live_update_tx.subscribe()
    }

    /// Unified entity-set loader for both live and as-of-T topology reads.
    ///
    /// `at = None` reads live entity rows (`valid_to IS NULL`).
    /// `at = Some(t)` reads as-of-T rows (`valid_from <= t AND (valid_to IS NULL OR valid_to > t)`).
    ///
    /// Tag lookup uses the `id_or_lineage_in` filter when `at` is set, since
    /// tag IDs may have been close-and-cloned at any prior snapshot.
    pub async fn get_topology_data(
        &self,
        network_id: Uuid,
        at: Option<DateTime<Utc>>,
    ) -> Result<TopologyData, Error> {
        let hosts = self
            .host_service
            .get_all(apply_at(
                StorableFilter::<Host>::new_from_network_ids(&[network_id]).hidden_is(false),
                at,
            ))
            .await?;
        let ip_addresses = self
            .ip_address_service
            .get_all(apply_at(
                StorableFilter::<IPAddress>::new_from_network_ids(&[network_id]),
                at,
            ))
            .await?;
        let subnets = self
            .subnet_service
            .get_all(apply_at(
                StorableFilter::<Subnet>::new_from_network_ids(&[network_id]),
                at,
            ))
            .await?;
        let dependencies = self
            .dependency_service
            .get_all(apply_at(
                StorableFilter::<Dependency>::new_from_network_ids(&[network_id]),
                at,
            ))
            .await?;
        let ports = self
            .port_service
            .get_all(apply_at(
                StorableFilter::<Port>::new_from_network_ids(&[network_id]),
                at,
            ))
            .await?;
        let bindings = self
            .binding_service
            .get_all(apply_at(
                StorableFilter::<Binding>::new_from_network_ids(&[network_id]),
                at,
            ))
            .await?;
        let interfaces = self
            .interface_service
            .get_all(apply_at(
                StorableFilter::<Interface>::new_from_network_ids(&[network_id]),
                at,
            ))
            .await?;
        let services = self
            .service_service
            .get_all(apply_at(
                StorableFilter::<Service>::new_from_network_ids(&[network_id]),
                at,
            ))
            .await?;
        let vlans = self
            .vlan_service
            .get_all(apply_at(
                StorableFilter::<Vlan>::new_from_uuid_column("network_id", &network_id),
                at,
            ))
            .await?;
        let tags = self
            .get_entity_tags(&hosts, &services, &subnets, at)
            .await?;

        Ok(TopologyData {
            hosts,
            ip_addresses,
            subnets,
            dependencies,
            ports,
            bindings,
            interfaces,
            services,
            vlans,
            tags,
        })
    }

    /// Fetch tag definitions for all tags used by hosts, services, and subnets.
    ///
    /// When `at = Some(t)`, the SCD2 `as_of(t)` filter combined with the
    /// `id_or_lineage_in(...)` OR-join resolves tags whose stable IDs may
    /// have been close-and-cloned during snapshot creation.
    pub async fn get_entity_tags(
        &self,
        hosts: &[Host],
        services: &[Service],
        subnets: &[Subnet],
        at: Option<DateTime<Utc>>,
    ) -> Result<Vec<Tag>, Error> {
        let mut tag_ids: Vec<Uuid> = Vec::new();
        for host in hosts {
            tag_ids.extend(&host.base.tags);
        }
        for service in services {
            tag_ids.extend(&service.base.tags);
        }
        for subnet in subnets {
            tag_ids.extend(&subnet.base.tags);
        }

        tag_ids.sort();
        tag_ids.dedup();

        if tag_ids.is_empty() {
            return Ok(vec![]);
        }

        let filter = match at {
            None => StorableFilter::<Tag>::new_from_entity_ids(&tag_ids).live(),
            Some(t) => StorableFilter::<Tag>::new_unfiltered()
                .id_or_lineage_in(&tag_ids)
                .as_of(t),
        };
        let tags = self.tag_service.get_all(filter).await?;

        Ok(tags)
    }

    /// Compute per-view data-support flags for a network's topology by
    /// querying raw entity tables — independent of whatever the topology
    /// was last rebuilt under.
    pub async fn get_view_support(&self, network_id: Uuid) -> Result<TopologyViewSupport, Error> {
        let interfaces = self
            .interface_service
            .get_all(StorableFilter::<Interface>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;
        let l2_physical = interfaces
            .iter()
            .any(|i| matches!(i.base.neighbor, Some(Neighbor::Interface(_))));

        let application = match self.network_service.get_by_id(&network_id).await? {
            Some(network) => self
                .tag_service
                .get_all(StorableFilter::<Tag>::new_from_org_id(
                    &network.base.organization_id,
                ))
                .await?
                .iter()
                .any(|t| t.base.is_application),
            None => false,
        };

        Ok(TopologyViewSupport {
            l2_physical,
            application,
        })
    }

    pub fn build_graph(&self, params: BuildGraphParams) -> (Vec<Node>, Vec<Edge>) {
        let BuildGraphParams {
            hosts,
            ip_addresses,
            subnets,
            services,
            dependencies,
            ports,
            bindings,
            interfaces,
            entity_tags,
            vlans,
            old_edges,
            old_nodes,
            options,
            old_view,
        } = params;

        // Create context to avoid parameter passing
        let ctx = TopologyContext::new(
            hosts,
            ip_addresses,
            subnets,
            services,
            dependencies,
            ports,
            bindings,
            interfaces,
            entity_tags,
            vlans,
            options,
        );

        // Build grouping config from request options
        let grouping = GroupingConfig::from_request_options(&options.request);

        // Select builder by view and build nodes + edges
        let builder = super::view::builder_for_view(options.request.view);
        let (all_nodes, mut all_edges) = builder.build(&ctx, &grouping);

        // Set per-view edge configuration
        let view = options.request.view;
        for edge in &mut all_edges {
            edge.view_config = view.edge_view_config((&edge.edge_type).into());
        }

        let final_edges = all_edges;

        // Build graph
        let mut graph: Graph<Node, Edge> = Graph::new();
        let node_indices: HashMap<Uuid, NodeIndex> = all_nodes
            .into_iter()
            .map(|node| {
                let node_id = node.id;
                let node_idx = graph.add_node(node);
                (node_id, node_idx)
            })
            .collect();

        // Add edges to graph
        EdgeBuilder::add_edges_to_graph(&mut graph, &node_indices, final_edges);

        // Skip handle preservation when view has changed — old handles are not meaningful
        let view_unchanged = match old_view {
            Some(old_v) => old_v == view,
            None => true,
        };

        if view_unchanged {
            // Build previous graph to compare and determine if user edits should be persisted
            // If nodes have changed edges, assume they have moved and user edits are no longer applicable
            let mut old_graph: Graph<Node, Edge> = Graph::new();
            let old_node_indices: HashMap<Uuid, NodeIndex> = old_nodes
                .iter()
                .map(|node| {
                    let node_id = node.id;
                    let node_idx = old_graph.add_node(node.clone());
                    (node_id, node_idx)
                })
                .collect();

            EdgeBuilder::add_edges_to_graph(&mut old_graph, &old_node_indices, old_edges.to_vec());

            // Create a map of old edges by their source/target for quick lookup
            let mut old_edges_map: HashMap<(Uuid, Uuid), &Edge> = HashMap::new();
            for edge_ref in old_graph.edge_references() {
                let edge = edge_ref.weight();
                old_edges_map.insert((edge.source, edge.target), edge);
            }

            // Preserve handles for nodes with unchanged edge count
            let mut edges_to_update: Vec<(petgraph::prelude::EdgeIndex, EdgeHandle, EdgeHandle)> =
                Vec::new();

            for node in graph.node_weights() {
                if let Some(old_idx) = old_node_indices.get(&node.id)
                    && let Some(new_idx) = node_indices.get(&node.id)
                {
                    let old_edge_count = old_graph.edges(*old_idx).count();
                    let new_edge_count = graph.edges(*new_idx).count();

                    if old_edge_count == new_edge_count {
                        for edge_ref in graph.edges(*new_idx) {
                            let new_edge = edge_ref.weight();
                            if let Some(old_edge) =
                                old_edges_map.get(&(new_edge.source, new_edge.target))
                            {
                                edges_to_update.push((
                                    edge_ref.id(),
                                    old_edge.source_handle,
                                    old_edge.target_handle,
                                ));
                            }
                        }
                    }
                }
            }

            // Now apply the updates
            for (edge_idx, source_handle, target_handle) in edges_to_update {
                if let Some(edge) = graph.edge_weight_mut(edge_idx) {
                    edge.source_handle = source_handle;
                    edge.target_handle = target_handle;
                }
            }
        }

        (
            graph.node_weights().cloned().collect(),
            graph.edge_weights().cloned().collect(),
        )
    }
}

/// Switch a filter between live and as-of-T modes based on `at`.
fn apply_at<T: Storable>(f: StorableFilter<T>, at: Option<DateTime<Utc>>) -> StorableFilter<T> {
    match at {
        None => f.live(),
        Some(t) => f.as_of(t),
    }
}
