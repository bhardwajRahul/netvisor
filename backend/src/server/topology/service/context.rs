use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::Binding,
    dependencies::r#impl::base::Dependency,
    hosts::r#impl::{base::Host, virtualization::HostVirtualization},
    interfaces::r#impl::base::Interface,
    ip_addresses::r#impl::base::IPAddress,
    ports::r#impl::base::Port,
    services::r#impl::{base::Service, virtualization::ServiceVirtualization},
    subnets::r#impl::base::Subnet,
    tags::r#impl::base::Tag,
    topology::types::{
        base::TopologyOptions,
        edges::Edge,
        nodes::{ElementEntityType, Node, NodeType},
        views::TopologyView,
    },
    vlans::r#impl::base::Vlan,
};

/// Central context for topology building operations
/// Provides topology-specific business logic and data access
pub struct TopologyContext<'a> {
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
    pub options: &'a TopologyOptions,
    /// View this context is building — drives grouping rule selection.
    pub view: TopologyView,
}

impl<'a> TopologyContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        hosts: &'a [Host],
        ip_addresses: &'a [IPAddress],
        subnets: &'a [Subnet],
        services: &'a [Service],
        dependencies: &'a [Dependency],
        ports: &'a [Port],
        bindings: &'a [Binding],
        interfaces: &'a [Interface],
        entity_tags: &'a [Tag],
        vlans: &'a [Vlan],
        options: &'a TopologyOptions,
        view: TopologyView,
    ) -> Self {
        Self {
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
            view,
        }
    }

    // ============================================================================
    // Data Access Methods
    // ============================================================================

    pub fn get_subnet_by_id(&self, subnet_id: Uuid) -> Option<&'a Subnet> {
        self.subnets.iter().find(|s| s.id == subnet_id)
    }

    pub fn get_host_by_id(&self, host_id: Uuid) -> Option<&'a Host> {
        self.hosts.iter().find(|h| h.id == host_id)
    }

    pub fn get_service_by_id(&self, service_id: Uuid) -> Option<&'a Service> {
        self.services.iter().find(|s| s.id == service_id)
    }

    pub fn get_ip_address_by_id(&self, ip_address_id: Option<Uuid>) -> Option<&'a IPAddress> {
        let id = ip_address_id?;
        self.ip_addresses.iter().find(|i| i.id == id)
    }

    pub fn get_ip_addresses_for_host(&self, host_id: Uuid) -> Vec<&'a IPAddress> {
        self.ip_addresses
            .iter()
            .filter(|i| i.base.host_id == host_id)
            .collect()
    }

    pub fn get_services_for_host(&self, host_id: Uuid) -> Vec<&'a Service> {
        self.services
            .iter()
            .filter(|s| s.base.host_id == host_id)
            .collect()
    }

    /// Get the first non-docker-bridge interface for a host
    pub fn get_first_non_container_bridge_ip_address_for_host(
        &self,
        host_id: Uuid,
    ) -> Option<&'a IPAddress> {
        self.ip_addresses.iter().find(|ip_address| {
            if ip_address.base.host_id != host_id {
                return false;
            }
            if let Some(subnet) = self.get_subnet_by_id(ip_address.base.subnet_id) {
                return !subnet.base.subnet_type.is_container_bridge()
                    && !subnet.base.subnet_type.is_loopback();
            }
            false
        })
    }

    pub fn get_services_bound_to_ip_address(&self, ip_address_id: Uuid) -> Vec<&'a Service> {
        self.services
            .iter()
            .filter(|s| {
                s.to_bound_ip_address_ids()
                    .iter()
                    .any(|s| s.map(|id| id == ip_address_id).unwrap_or(false))
            })
            .collect()
    }

    pub fn get_subnet_from_ip_address_id(&self, ip_address_id: Uuid) -> Option<&'a Subnet> {
        let ip_address = self.ip_addresses.iter().find(|i| i.id == ip_address_id)?;
        self.get_subnet_by_id(ip_address.base.subnet_id)
    }

    pub fn get_host_from_ip_address_id(&self, ip_address_id: Uuid) -> Option<&'a Host> {
        let ip_address = self.ip_addresses.iter().find(|i| i.id == ip_address_id)?;
        self.hosts.iter().find(|h| h.id == ip_address.base.host_id)
    }

    // ============================================================================
    // Interface (SNMP Interface Table) Methods
    // ============================================================================

    pub fn get_interface_by_id(&self, id: Uuid) -> Option<&'a Interface> {
        self.interfaces.iter().find(|e| e.id == id)
    }

    /// Resolve an IP address ID from an Interface, with single-IP-address host fallback.
    /// Returns Some(ip_address_id) if:
    /// - The interface has ip_address_id set, OR
    /// - The interface's host has exactly one IP address
    pub fn resolve_ip_address_for_interface(&self, interface: &Interface) -> Option<Uuid> {
        // Direct resolution
        if let Some(ip_address_id) = interface.base.ip_address_id {
            return Some(ip_address_id);
        }

        // Single-interface host fallback
        let host_interfaces = self.get_ip_addresses_for_host(interface.base.host_id);
        if host_interfaces.len() == 1 {
            return Some(host_interfaces[0].id);
        }

        None
    }

    pub fn get_interfaces_for_host(&self, host_id: Uuid) -> Vec<&'a Interface> {
        self.interfaces
            .iter()
            .filter(|e| e.base.host_id == host_id)
            .collect()
    }

    /// Get all interfaces that have a resolved neighbor (full resolution only)
    pub fn get_interfaces_with_neighbor(&self) -> Vec<&'a Interface> {
        use crate::server::interfaces::r#impl::base::Neighbor;
        self.interfaces
            .iter()
            .filter(|e| matches!(e.base.neighbor, Some(Neighbor::Interface(_))))
            .collect()
    }

    /// Interfaces whose LLDP/CDP neighbour resolved to a host but not to a specific port.
    /// The adjacency is known, the remote port is not — see `EdgeType::NeighborLink`.
    pub fn get_interfaces_with_host_neighbor(&self) -> Vec<&'a Interface> {
        use crate::server::interfaces::r#impl::base::Neighbor;
        self.interfaces
            .iter()
            .filter(|e| matches!(e.base.neighbor, Some(Neighbor::Host(_))))
            .collect()
    }

    // ============================================================================
    // Virtualization Relationship Methods
    // ============================================================================

    pub fn get_host_is_virtualized_by(&self, host_id: &Uuid) -> Option<&Service> {
        if let Some(host) = self.get_host_by_id(*host_id)
            && let Some(HostVirtualization::Proxmox(proxmox_virtualization)) =
                &host.base.virtualization
        {
            return self
                .services
                .iter()
                .find(|s| s.id == proxmox_virtualization.service_id);
        }
        None
    }

    pub fn get_service_is_containerized_by(&self, service_id: &Uuid) -> Option<&Service> {
        if let Some(service) = self.get_service_by_id(*service_id)
            && let Some(ServiceVirtualization::Docker(docker_virtualization)) =
                &service.base.virtualization
        {
            return self
                .services
                .iter()
                .find(|s| s.id == docker_virtualization.service_id);
        }
        None
    }

    pub fn get_node_subnet(&self, node_id: Uuid, nodes: &[Node]) -> Option<Uuid> {
        nodes
            .iter()
            .find(|n| n.id == node_id)
            .and_then(|node| match &node.node_type {
                NodeType::Element {
                    element: ElementEntityType::IPAddress { subnet_id, .. },
                    ..
                } => Some(*subnet_id),
                NodeType::Container { .. } => Some(node.id),
                _ => None,
            })
    }

    // ============================================================================
    // VLAN Methods
    // ============================================================================

    /// Look up a VLAN by its entity UUID
    pub fn get_vlan_by_id(&self, vlan_id: Uuid) -> Option<&'a Vlan> {
        self.vlans.iter().find(|v| v.id == vlan_id)
    }

    // ============================================================================
    // Tag Methods
    // ============================================================================

    pub fn get_host_ids_with_tags(&self, tag_ids: &[Uuid]) -> HashSet<Uuid> {
        self.hosts
            .iter()
            .filter(|h| h.base.tags.iter().any(|t| tag_ids.contains(t)))
            .map(|h| h.id)
            .collect()
    }

    // ============================================================================
    // Edge Classification Methods
    // ============================================================================

    pub fn edge_is_intra_subnet(&self, edge: &Edge) -> bool {
        if let (Some(source_subnet), Some(target_subnet)) = (
            self.get_subnet_from_ip_address_id(edge.source),
            self.get_subnet_from_ip_address_id(edge.target),
        ) {
            return source_subnet.id == target_subnet.id;
        }
        false
    }

    pub fn edge_is_multi_hop(
        &self,
        source_ip_address_id: &Uuid,
        target_ip_address_id: &Uuid,
    ) -> bool {
        if let (Some(source_subnet), Some(target_subnet)) = (
            self.get_subnet_from_ip_address_id(*source_ip_address_id),
            self.get_subnet_from_ip_address_id(*target_ip_address_id),
        ) {
            let vertical_order_difference = source_subnet.base.subnet_type.vertical_order()
                as isize
                - target_subnet.base.subnet_type.vertical_order() as isize;

            return vertical_order_difference.abs() > 1;
        }
        false
    }

    // ============================================================================
    // Node Existence Checks
    // ============================================================================

    pub fn interface_will_have_node(&self, _ip_address_id: &Uuid) -> bool {
        true
    }

    pub fn service_will_have_node(&self, service_id: &Uuid) -> bool {
        self.get_service_by_id(*service_id)
            .map(|s| !s.base.bindings.is_empty())
            .unwrap_or(true)
    }

    /// Build a map of binding_id → service_id from all services.
    /// Used by builders that need to resolve dependency bindings to service IDs.
    pub fn build_binding_to_service_map(&self) -> HashMap<Uuid, Uuid> {
        self.services
            .iter()
            .flat_map(|s| s.base.bindings.iter().map(move |b| (b.id, s.id)))
            .collect()
    }
}
