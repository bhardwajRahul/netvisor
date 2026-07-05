use itertools::Itertools;
use petgraph::{Graph, graph::NodeIndex};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::server::{
    dependencies::r#impl::{base::Dependency, types::DependencyType},
    interfaces::r#impl::base::Neighbor,
    topology::{
        service::context::TopologyContext,
        types::{
            edges::{DiscoveryProtocol, Edge, EdgeHandle, EdgeType, EdgeViewConfig},
            nodes::Node,
        },
    },
};

pub struct EdgeBuilder;

impl EdgeBuilder {
    /// Create dependency edges (connecting services in a dependency's service chain)
    pub fn create_dependency_edges(ctx: &TopologyContext) -> Vec<Edge> {
        ctx.dependencies
            .iter()
            .flat_map(|dependency| {
                let binding_ids = dependency.binding_ids();
                match &dependency.base.dependency_type {
                    DependencyType::RequestPath => binding_ids
                        .windows(2)
                        .filter_map(|window| {
                            EdgeBuilder::edge_from_service_bindings(
                                ctx, window[0], window[1], dependency,
                            )
                        })
                        .collect::<Vec<Edge>>(),
                    DependencyType::HubAndSpoke => {
                        let mut binding_ids = binding_ids.clone();
                        binding_ids.reverse();
                        if let Some(hub_binding_id) = binding_ids.pop() {
                            return binding_ids
                                .iter()
                                .filter_map(|spoke_binding| {
                                    EdgeBuilder::edge_from_service_bindings(
                                        ctx,
                                        hub_binding_id,
                                        *spoke_binding,
                                        dependency,
                                    )
                                })
                                .collect::<Vec<Edge>>();
                        }
                        Vec::new()
                    }
                }
            })
            .collect()
    }

    /// Create edges to connect a host that virtualizes containers via Docker
    /// to the containerized services on Docker bridge subnets.
    pub fn create_containerized_service_edges(ctx: &TopologyContext) -> Vec<Edge> {
        let mut docker_service_to_containerized_service_ids: HashMap<Uuid, Vec<Uuid>> =
            HashMap::new();

        ctx.services.iter().for_each(|s| {
            // Any container-runtime manager (Docker, Podman, …), keyed off the
            // generic service_id() accessor rather than a specific variant.
            if let Some(runtime_service_id) =
                s.base.virtualization.as_ref().and_then(|v| v.service_id())
            {
                let entry = docker_service_to_containerized_service_ids
                    .entry(runtime_service_id)
                    .or_default();
                if !entry.contains(&s.id) {
                    entry.push(s.id);
                }
            }
        });

        ctx.services
            .iter()
            .filter(|s| {
                docker_service_to_containerized_service_ids
                    .keys()
                    .contains(&s.id)
            })
            .filter_map(|s| {
                let host = ctx.get_host_by_id(s.base.host_id)?;
                let origin_ip_address =
                    ctx.get_first_non_container_bridge_ip_address_for_host(host.id)?;
                Some((s, host, origin_ip_address))
            })
            .flat_map(|(s, host, origin_ip_address)| {
                let host_interfaces = ctx.get_ip_addresses_for_host(host.id);
                let container_subnets: Vec<Uuid> = host_interfaces
                    .iter()
                    .filter_map(|i| ctx.get_subnet_by_id(i.base.subnet_id))
                    .filter_map(|s| {
                        if s.base.subnet_type.is_container_bridge() {
                            return Some(s.id);
                        }
                        None
                    })
                    .collect();

                let container_subnet_ip_address_ids: Vec<Uuid> = host_interfaces
                    .iter()
                    .filter_map(|i| {
                        if container_subnets.contains(&i.base.subnet_id) {
                            return Some(i.id);
                        }
                        None
                    })
                    .collect();

                docker_service_to_containerized_service_ids
                    .get(&s.id)
                    .unwrap_or(&Vec::new())
                    .iter()
                    .filter_map(move |cs| {
                        let containerized = ctx.get_service_by_id(*cs)?;

                        let container_binding_ip_address_id = containerized
                            .base
                            .bindings
                            .iter()
                            .filter_map(|b| b.ip_address_id())
                            .find(|i| container_subnet_ip_address_ids.contains(i))?;

                        let is_multi_hop = ctx.edge_is_multi_hop(
                            &origin_ip_address.id,
                            &container_binding_ip_address_id,
                        );

                        Some(Edge {
                            id: Uuid::new_v4(),
                            source: origin_ip_address.id,
                            target: container_binding_ip_address_id,
                            edge_type: EdgeType::ContainerRuntime {
                                service_id: s.id,
                                host_id: host.id,
                            },
                            label: Some(format!("{} on {}", s.base.name, host.base.name)),
                            source_handle: EdgeHandle::Bottom,
                            target_handle: EdgeHandle::Top,
                            is_multi_hop,
                            view_config: EdgeViewConfig::default(),
                        })
                    })
                    .collect::<Vec<Edge>>()
            })
            .collect()
    }

    // Create edges to connect a host that virtualizes other hosts as VMs
    pub fn create_vm_host_edges(ctx: &TopologyContext) -> Vec<Edge> {
        // Proxmox service interface binding that is present for a given subnet.
        // There could be multiple host ip_addresses with a given subnet, we arbitrarily choose the first one so there's
        // one clustering hub rather than multiple hubs
        // (subnet_id, proxmox_service_id) : (ip_address_id)
        let mut subnet_to_promxox_host_ip_address_id: HashMap<(Uuid, Uuid), Uuid> = HashMap::new();

        // Hosts VMs managed by a given proxmox service
        let mut vm_host_id_to_proxmox_service: HashMap<Uuid, Uuid> = HashMap::new();

        ctx.hosts.iter().for_each(|h| {
            // Any host-level virtualization manager (Proxmox, vCenter, ESXi, …).
            // Keyed off the generic service_id() accessor, not a specific variant.
            if let Some(vm_manager_service_id) =
                h.base.virtualization.as_ref().and_then(|v| v.service_id())
            {
                // Create mapping between subnet and hypervisor interface(s) on that subnet
                if let Some(promxox_service) = ctx.get_service_by_id(vm_manager_service_id) {
                    promxox_service
                        .base
                        .bindings
                        .iter()
                        .filter_map(|b| b.ip_address_id())
                        .for_each(|i| {
                            if let Some(subnet) = ctx.get_subnet_from_ip_address_id(i)
                                && !subnet_to_promxox_host_ip_address_id
                                    .contains_key(&(subnet.id, promxox_service.id))
                            {
                                subnet_to_promxox_host_ip_address_id
                                    .entry((subnet.id, promxox_service.id))
                                    .insert_entry(i);
                            }
                        });
                }

                vm_host_id_to_proxmox_service.insert(h.id, vm_manager_service_id);
            }
        });

        // Creates edges between interface that proxmox service has on a given subnet with ip_addresses that the virtualized host has on the subnet
        ctx.hosts
            .iter()
            .flat_map(|h| {
                if let Some(proxmox_service_id) = vm_host_id_to_proxmox_service.get(&h.id) {
                    let host_interfaces = ctx.get_ip_addresses_for_host(h.id);
                    return host_interfaces
                        .into_iter()
                        .filter_map(|i| {
                            if let Some(proxmox_service_ip_address_id) =
                                subnet_to_promxox_host_ip_address_id
                                    .get(&(i.base.subnet_id, *proxmox_service_id))
                            {
                                let is_multi_hop =
                                    ctx.edge_is_multi_hop(proxmox_service_ip_address_id, &i.id);

                                return Some(Edge {
                                    id: Uuid::new_v4(),
                                    source: *proxmox_service_ip_address_id,
                                    target: i.id,
                                    edge_type: EdgeType::Hypervisor {
                                        hypervisor_service_id: *proxmox_service_id,
                                    },
                                    label: None,
                                    source_handle: EdgeHandle::Bottom,
                                    target_handle: EdgeHandle::Top,
                                    is_multi_hop,
                                    view_config: EdgeViewConfig::default(),
                                });
                            }
                            None
                        })
                        .collect();
                }
                Vec::new()
            })
            .collect()
    }

    /// Create host-level Hypervisor edges (hypervisor host → VM host).
    /// Unlike `create_vm_host_edges` which uses interface IDs, this uses host IDs
    /// as source/target for views where elements are hosts (e.g. Infrastructure).
    pub fn create_vm_host_edges_by_host(ctx: &TopologyContext) -> Vec<Edge> {
        ctx.hosts
            .iter()
            .filter_map(|h| {
                let vm_manager_service_id = h
                    .base
                    .virtualization
                    .as_ref()
                    .and_then(|v| v.service_id())?;
                let proxmox_service = ctx.get_service_by_id(vm_manager_service_id)?;
                Some(Edge {
                    id: Uuid::new_v4(),
                    source: proxmox_service.base.host_id,
                    target: h.id,
                    edge_type: EdgeType::Hypervisor {
                        hypervisor_service_id: vm_manager_service_id,
                    },
                    label: None,
                    source_handle: EdgeHandle::Bottom,
                    target_handle: EdgeHandle::Top,
                    is_multi_hop: false,
                    view_config: EdgeViewConfig::default(),
                })
            })
            .collect()
    }

    /// Create interface edges (connecting multiple ip_addresses on the same host)
    pub fn create_interface_edges(ctx: &TopologyContext) -> Vec<Edge> {
        ctx.hosts
            .iter()
            .flat_map(|host| {
                let host_interfaces = ctx.get_ip_addresses_for_host(host.id);

                // Use the first non-container-bridge interface as the origin
                // This ensures we don't try to create edges FROM a container-bridge interface
                if let Some(origin_ip_address) =
                    ctx.get_first_non_container_bridge_ip_address_for_host(host.id)
                {
                    host_interfaces
                        .iter()
                        .filter(|ip_address| ip_address.id != origin_ip_address.id)
                        .filter_map(|ip_address| {
                            let source_subnet =
                                ctx.get_subnet_by_id(origin_ip_address.base.subnet_id);
                            let target_subnet = ctx.get_subnet_by_id(ip_address.base.subnet_id);

                            if let Some(source_subnet) = source_subnet
                                && source_subnet.base.subnet_type.is_container_bridge()
                            {
                                return None;
                            }

                            if let Some(target_subnet) = target_subnet
                                && target_subnet.base.subnet_type.is_container_bridge()
                            {
                                return None;
                            }

                            let is_multi_hop =
                                ctx.edge_is_multi_hop(&origin_ip_address.id, &ip_address.id);

                            // Hide label if both ip_addresses are on the same subnet
                            let label =
                                if origin_ip_address.base.subnet_id == ip_address.base.subnet_id {
                                    None
                                } else {
                                    Some(host.base.name.to_string())
                                };

                            Some(Edge {
                                id: Uuid::new_v4(),
                                source: origin_ip_address.id,
                                target: ip_address.id,
                                edge_type: EdgeType::SameHost { host_id: host.id },
                                label,
                                source_handle: EdgeHandle::Bottom,
                                target_handle: EdgeHandle::Top,
                                is_multi_hop,
                                view_config: EdgeViewConfig::default(),
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            })
            .collect()
    }

    /// Create physical link edges from LLDP/CDP neighbor discovery
    /// Only creates edges when both endpoints have associated ip_addresses (nodes)
    pub fn create_physical_link_edges(ctx: &TopologyContext) -> Vec<Edge> {
        // Track processed pairs to avoid duplicate edges (A→B and B→A)
        let mut processed_pairs: HashSet<(Uuid, Uuid)> = HashSet::new();

        ctx.get_interfaces_with_neighbor()
            .into_iter()
            .filter_map(|source_entry| {
                // Get the target Interface ID from resolved neighbor
                let target_interface_id = match &source_entry.base.neighbor {
                    Some(Neighbor::Interface(id)) => *id,
                    _ => return None, // Already filtered by get_interfaces_with_neighbor
                };

                // Skip if we've already processed this pair (in either direction)
                let pair_key = if source_entry.id < target_interface_id {
                    (source_entry.id, target_interface_id)
                } else {
                    (target_interface_id, source_entry.id)
                };

                if processed_pairs.contains(&pair_key) {
                    return None;
                }
                processed_pairs.insert(pair_key);

                // Resolve interface IDs with single-interface host fallback
                let source_ip_address_id = ctx.resolve_ip_address_for_interface(source_entry)?;
                let target_entry = ctx.get_interface_by_id(target_interface_id)?;
                let target_ip_address_id = ctx.resolve_ip_address_for_interface(target_entry)?;

                let is_multi_hop =
                    ctx.edge_is_multi_hop(&source_ip_address_id, &target_ip_address_id);

                // Build label from port descriptions: "Gi0/1 ↔ Gi0/2"
                let label = Some(format!(
                    "{} ↔ {}",
                    source_entry.display_name(),
                    target_entry.display_name()
                ));

                Some(Edge {
                    id: Uuid::new_v4(),
                    source: source_ip_address_id,
                    target: target_ip_address_id,
                    edge_type: EdgeType::PhysicalLink {
                        source_entity_id: source_entry.id,
                        target_entity_id: target_entry.id,
                        protocol: DiscoveryProtocol::LLDP, // TODO: Support CDP when implemented
                    },
                    label,
                    source_handle: EdgeHandle::Bottom,
                    target_handle: EdgeHandle::Top,
                    is_multi_hop,
                    view_config: EdgeViewConfig::default(),
                })
            })
            .collect()
    }

    /// Add edges to a petgraph Graph
    pub fn add_edges_to_graph(
        graph: &mut Graph<Node, Edge>,
        node_indices: &HashMap<Uuid, NodeIndex>,
        edges: Vec<Edge>,
    ) {
        for edge in edges {
            if let (Some(&src_idx), Some(&tgt_idx)) = (
                node_indices.get(&edge.source),
                node_indices.get(&edge.target),
            ) {
                graph.add_edge(src_idx, tgt_idx, edge);
            }
        }
    }

    pub fn edge_from_service_bindings(
        ctx: &TopologyContext,
        source_id: Uuid,
        target_id: Uuid,
        dependency: &Dependency,
    ) -> Option<Edge> {
        let source_ip_address = ctx.services.iter().find_map(|s| {
            if let Some(source_binding) = s.get_binding(source_id) {
                return Some(source_binding.ip_address_id());
            }
            None
        });

        let target_ip_address = ctx.services.iter().find_map(|s| {
            if let Some(target_binding) = s.get_binding(target_id) {
                return Some(target_binding.ip_address_id());
            }
            None
        });

        let (Some(Some(source_ip_address)), Some(Some(target_ip_address))) =
            (source_ip_address, target_ip_address)
        else {
            return None;
        };

        let is_multi_hop = ctx.edge_is_multi_hop(&source_ip_address, &target_ip_address);

        // If edge is intra-subnet, don't label - gets too messy
        let label = if ctx
            .get_subnet_from_ip_address_id(source_ip_address)
            .map(|s| s.id)
            == ctx
                .get_subnet_from_ip_address_id(target_ip_address)
                .map(|s| s.id)
        {
            None
        } else {
            Some(dependency.base.name.to_string())
        };

        Some(Edge {
            id: Uuid::new_v4(),
            source: source_ip_address,
            target: target_ip_address,
            edge_type: match dependency.base.dependency_type {
                DependencyType::HubAndSpoke => EdgeType::HubAndSpoke {
                    source_id,
                    target_id,
                    dependency_id: dependency.id,
                },
                DependencyType::RequestPath => EdgeType::RequestPath {
                    source_id,
                    target_id,
                    dependency_id: dependency.id,
                },
            },
            label,
            source_handle: EdgeHandle::Bottom,
            target_handle: EdgeHandle::Top,
            is_multi_hop,
            view_config: EdgeViewConfig::default(),
        })
    }
}
