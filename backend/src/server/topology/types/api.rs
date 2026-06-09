use crate::server::{
    bindings::r#impl::base::Binding, dependencies::r#impl::base::Dependency,
    hosts::r#impl::base::Host, interfaces::r#impl::base::Interface,
    ip_addresses::r#impl::base::IPAddress, ports::r#impl::base::Port,
    services::r#impl::base::Service, subnets::r#impl::base::Subnet, tags::r#impl::base::Tag,
    vlans::r#impl::base::Vlan,
};

/// Bundle of entities that feed `build_graph` and the topology export pipeline.
///
/// Loaded by [`crate::server::topology::service::main::TopologyService::get_topology_data`]
/// for either the live view (`at = None`) or a point-in-time snapshot
/// (`at = Some(taken_at)`). Replaces the entity-blob columns previously
/// persisted on the topology row.
#[derive(Debug, Clone, Default)]
pub struct TopologyData {
    pub hosts: Vec<Host>,
    pub ip_addresses: Vec<IPAddress>,
    pub subnets: Vec<Subnet>,
    pub dependencies: Vec<Dependency>,
    pub ports: Vec<Port>,
    pub bindings: Vec<Binding>,
    pub interfaces: Vec<Interface>,
    pub services: Vec<Service>,
    pub vlans: Vec<Vlan>,
    pub tags: Vec<Tag>,
}
