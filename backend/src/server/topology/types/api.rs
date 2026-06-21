use crate::server::{
    bindings::r#impl::base::Binding, dependencies::r#impl::base::Dependency,
    hosts::r#impl::base::Host, interfaces::r#impl::base::Interface,
    ip_addresses::r#impl::base::IPAddress, ports::r#impl::base::Port,
    services::r#impl::base::Service, subnets::r#impl::base::Subnet, tags::r#impl::base::Tag,
    topology::types::views::TopologyView, vlans::r#impl::base::Vlan,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Bundle of entities that feed `build_graph` and the topology export pipeline.
///
/// Loaded by [`crate::server::topology::service::main::TopologyService::get_topology_data`]
/// for either the live view (`at = None`) or a point-in-time snapshot
/// (`at = Some(taken_at)`). Replaces the entity-blob columns previously
/// persisted on the topology row.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
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
    /// Views whose data is present in this entity set (L3/Workloads always;
    /// L2 Physical iff LLDP/CDP neighbors exist; Application iff app-flagged
    /// tags are used). The topology tab restricts a snapshot's view picker to
    /// these — you can't set up SNMP or create app tags on a historical
    /// snapshot — while the live view shows all views with setup prompts.
    #[serde(default)]
    pub available_views: Vec<TopologyView>,
}
