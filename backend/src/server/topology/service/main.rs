use std::{collections::HashMap, sync::Arc};

use anyhow::Error;
use async_trait::async_trait;
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
    networks::{r#impl::Network, service::NetworkService},
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
            base::{SetEntitiesParams, Topology, TopologyOptions},
            edges::{Edge, EdgeHandle},
            grouping::{ElementRule, GroupingConfig, IdentifiedRule},
            nodes::Node,
            views::{TopologyView, TopologyViewSupport},
        },
    },
    vlans::{r#impl::base::Vlan, service::VlanService},
};

pub struct TopologyService {
    storage: Arc<GenericPostgresStorage<Topology>>,
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
    pub(crate) network_service: Arc<NetworkService>,
    event_bus: Arc<EventBus>,
    pub staleness_tx: broadcast::Sender<Topology>,
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

    /// Create entity
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

        let (hosts, ip_addresses, subnets, dependencies, ports, bindings, interfaces) =
            self.get_entity_data(topology.base.network_id).await?;

        let services = self.get_service_data(topology.base.network_id).await?;

        // Fetch tag definitions for all tags used by entities and element rules
        let entity_tags = self
            .get_entity_tags(
                &hosts,
                &services,
                &subnets,
                &topology.base.options.request.element_rules,
            )
            .await?;

        // Fetch VLANs for the network
        let vlans = self.get_vlans(topology.base.network_id).await?;

        let params = BuildGraphParams {
            hosts: &hosts,
            ip_addresses: &ip_addresses,
            services: &services,
            subnets: &subnets,
            dependencies: &dependencies,
            ports: &ports,
            bindings: &bindings,
            interfaces: &interfaces,
            entity_tags: &entity_tags,
            vlans: &vlans,
            old_edges: &[],
            old_nodes: &[],
            options: &topology.base.options,
            old_view: None,
        };

        let (nodes, edges) = self.build_graph(params);

        topology.set_entities(SetEntitiesParams {
            hosts,
            ip_addresses,
            services,
            subnets,
            dependencies,
            interfaces,
            entity_tags,
            vlans,
            ports,
            bindings,
        });

        topology.set_graph(nodes, edges);
        topology.clear_stale();

        let created = self.storage().create(&topology).await?;

        if let Some(scope) = EntityScope::from_ids(
            created.id(),
            created.clone().into(),
            self.get_network_id(&created),
            self.get_organization_id(&created),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Created, authentication).with_flags(
                        EntityEventFlags {
                            clear_stale: true,
                            ..Default::default()
                        },
                    ),
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
    /// Returns true if changes to this tag should mark topologies stale.
    /// Fires for either: application tags, or tags referenced by any ByTag
    /// element rule in any topology in the given org.
    pub async fn tag_affects_any_topology(&self, tag_id: Uuid, org_id: Uuid) -> bool {
        let is_app = self
            .tag_service
            .get_by_id(&tag_id)
            .await
            .ok()
            .flatten()
            .map(|t| t.base.is_application)
            .unwrap_or(false);
        if is_app {
            return true;
        }
        let Ok(networks) = self
            .network_service
            .get_all(StorableFilter::<Network>::new_from_org_id(&org_id))
            .await
        else {
            return false;
        };
        let network_ids: Vec<Uuid> = networks.iter().map(|n| n.id).collect();
        if network_ids.is_empty() {
            return false;
        }
        let Ok(topologies) = self
            .get_all(StorableFilter::<Topology>::new_from_network_ids(
                &network_ids,
            ))
            .await
        else {
            return false;
        };
        topologies.iter().any(|t| {
            t.base.options.request.element_rules.iter().any(|r| {
                matches!(&r.rule, ElementRule::ByTag { tag_ids, .. } if tag_ids.contains(&tag_id))
            })
        })
    }

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
        let (staleness_tx, _) = broadcast::channel(100);
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
            staleness_tx,
        }
    }

    pub fn subscribe_staleness_changes(&self) -> broadcast::Receiver<Topology> {
        self.staleness_tx.subscribe()
    }

    pub async fn get_entity_data(
        &self,
        network_id: Uuid,
    ) -> Result<
        (
            Vec<Host>,
            Vec<IPAddress>,
            Vec<Subnet>,
            Vec<Dependency>,
            Vec<Port>,
            Vec<Binding>,
            Vec<Interface>,
        ),
        Error,
    > {
        // Fetch all data - each service needs its own properly typed filter
        let hosts = self
            .host_service
            .get_all(StorableFilter::<Host>::new_from_network_ids(&[network_id]).hidden_is(false))
            .await?;

        let ip_addresses = self
            .ip_address_service
            .get_all(StorableFilter::<IPAddress>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;
        let subnets = self
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;
        let dependencies = self
            .dependency_service
            .get_all(StorableFilter::<Dependency>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;

        let ports = self
            .port_service
            .get_all(StorableFilter::<Port>::new_from_network_ids(&[network_id]))
            .await?;
        let bindings = self
            .binding_service
            .get_all(StorableFilter::<Binding>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;

        let interfaces = self
            .interface_service
            .get_all(StorableFilter::<Interface>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;

        Ok((
            hosts,
            ip_addresses,
            subnets,
            dependencies,
            ports,
            bindings,
            interfaces,
        ))
    }

    pub async fn get_service_data(&self, network_id: Uuid) -> Result<Vec<Service>, Error> {
        self.service_service
            .get_all(StorableFilter::<Service>::new_from_network_ids(&[
                network_id,
            ]))
            .await
    }

    /// Fetch tag definitions for all tags used by hosts, services, and subnets.
    pub async fn get_entity_tags(
        &self,
        hosts: &[Host],
        services: &[Service],
        subnets: &[Subnet],
        element_rules: &[IdentifiedRule<ElementRule>],
    ) -> Result<Vec<Tag>, Error> {
        // Collect all unique tag IDs from entities
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

        // Include tags referenced by ByTag element rules so their metadata
        // is available in the response even if no entities currently have them
        for rule in element_rules {
            if let ElementRule::ByTag {
                tag_ids: rule_tag_ids,
                ..
            } = &rule.rule
            {
                tag_ids.extend(rule_tag_ids);
            }
        }

        // Deduplicate
        tag_ids.sort();
        tag_ids.dedup();

        if tag_ids.is_empty() {
            return Ok(vec![]);
        }

        // Fetch the tag definitions
        let tags = self
            .tag_service
            .get_all(StorableFilter::<Tag>::new_from_entity_ids(&tag_ids))
            .await?;

        Ok(tags)
    }

    /// Fetch all VLANs for a network.
    pub async fn get_vlans(&self, network_id: Uuid) -> Result<Vec<Vlan>, Error> {
        let filter = StorableFilter::<Vlan>::new_from_uuid_column("network_id", &network_id);
        self.vlan_service.storage().get_all(filter).await
    }

    /// Compute per-view data-support flags for a network's topology by
    /// querying raw entity tables — independent of whatever the topology
    /// was last rebuilt under. Used by the share handlers to decide which
    /// views to expose; previously this was read from the persisted
    /// `topology.base.edges` / `entity_tags` snapshot, which flips based
    /// on the most recently rendered view.
    pub async fn get_view_support(&self, network_id: Uuid) -> Result<TopologyViewSupport, Error> {
        // L2 physical support: any interface in this network has an LLDP/CDP
        // neighbor pointing at another interface. The `neighbor` field on
        // Interface is raw discovery data — unchanged by topology rebuilds.
        let interfaces = self
            .interface_service
            .get_all(StorableFilter::<Interface>::new_from_network_ids(&[
                network_id,
            ]))
            .await?;
        let l2_physical = interfaces
            .iter()
            .any(|i| matches!(i.base.neighbor, Some(Neighbor::Interface(_))));

        // Application support: the topology's organization has at least one
        // application-flagged tag defined. We deliberately do NOT require the
        // tag to be applied to an entity in this specific network — the main
        // app always exposes the Application view (rendering services as
        // "Ungrouped" when no app tags are assigned yet), and the share must
        // match that behavior. If the org has no application tags at all,
        // Application grouping is meaningless and we gate it out.
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

    /// Rebuild a topology: fetch entities from DB, compute nodes/edges, persist.
    /// Used by the rebuild handler and demo data seeder.
    pub async fn rebuild(
        &self,
        topology: &mut Topology,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let (hosts, ip_addresses, subnets, dependencies, ports, bindings, interfaces) =
            self.get_entity_data(topology.base.network_id).await?;

        let services = self.get_service_data(topology.base.network_id).await?;

        let entity_tags = self
            .get_entity_tags(
                &hosts,
                &services,
                &subnets,
                &topology.base.options.request.element_rules,
            )
            .await?;

        let vlans = self.get_vlans(topology.base.network_id).await?;

        let (nodes, edges) = self.build_graph(BuildGraphParams {
            options: &topology.base.options,
            hosts: &hosts,
            ip_addresses: &ip_addresses,
            subnets: &subnets,
            services: &services,
            dependencies: &dependencies,
            ports: &ports,
            bindings: &bindings,
            interfaces: &interfaces,
            entity_tags: &entity_tags,
            vlans: &vlans,
            old_nodes: &[],
            old_edges: &[],
            old_view: None,
        });

        topology.set_entities(SetEntitiesParams {
            hosts,
            ip_addresses,
            services,
            subnets,
            dependencies,
            ports,
            bindings,
            interfaces,
            entity_tags,
            vlans,
        });

        topology.set_graph(nodes, edges);
        topology.clear_stale();

        self.update(topology, authentication).await?;

        Ok(())
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
