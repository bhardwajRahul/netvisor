use crate::server::bindings::r#impl::base::Binding;
use crate::server::groups::r#impl::base::Group;
use crate::server::hosts::r#impl::base::Host;
use crate::server::if_entries::r#impl::base::IfEntry;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::ports::r#impl::base::Port;
use crate::server::services::r#impl::base::Service;
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::tags::r#impl::base::Tag;
use crate::server::topology::types::edges::{Edge, EdgeHandle, EdgeTypeDiscriminants};
use crate::server::topology::types::layout::{Ixy, Uxy};
use crate::server::topology::types::nodes::Node;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, hash::Hash};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

pub struct SetEntitiesParams {
    pub hosts: Vec<Host>,
    pub services: Vec<Service>,
    pub subnets: Vec<Subnet>,
    pub groups: Vec<Group>,
    pub ports: Vec<Port>,
    pub bindings: Vec<Binding>,
    pub interfaces: Vec<Interface>,
    pub if_entries: Vec<IfEntry>,
    pub entity_tags: Vec<Tag>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct Topology {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: TopologyBase,
}

impl Topology {
    pub fn lock(&mut self, locked_by: Uuid) {
        self.base.is_locked = true;
        self.base.locked_at = Some(Utc::now());
        self.base.locked_by = Some(locked_by)
    }

    pub fn unlock(&mut self) {
        self.base.is_locked = false;
        self.base.locked_at = None;
        self.base.locked_by = None;
    }

    pub fn clear_stale(&mut self) {
        self.base.removed_groups = vec![];
        self.base.removed_hosts = vec![];
        self.base.removed_interfaces = vec![];
        self.base.removed_services = vec![];
        self.base.removed_subnets = vec![];
        self.base.removed_bindings = vec![];
        self.base.removed_ports = vec![];
        self.base.removed_if_entries = vec![];
        self.base.is_stale = false;
        self.base.last_refreshed = Utc::now()
    }

    pub fn set_entities(&mut self, params: SetEntitiesParams) {
        self.base.hosts = params.hosts;
        self.base.services = params.services;
        self.base.subnets = params.subnets;
        self.base.groups = params.groups;
        self.base.ports = params.ports;
        self.base.bindings = params.bindings;
        self.base.interfaces = params.interfaces;
        self.base.if_entries = params.if_entries;
        self.base.entity_tags = params.entity_tags;
    }

    pub fn set_graph(&mut self, nodes: Vec<Node>, edges: Vec<Edge>) {
        self.base.nodes = nodes;
        self.base.edges = edges;
    }
}

#[derive(
    Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema,
)]
pub struct TopologyBase {
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    pub options: TopologyOptions,
    pub network_id: Uuid,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    pub parent_id: Option<Uuid>,

    // Graph
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,

    // Entities
    pub hosts: Vec<Host>,
    pub interfaces: Vec<Interface>,
    pub ports: Vec<Port>,
    pub bindings: Vec<Binding>,
    pub subnets: Vec<Subnet>,
    pub services: Vec<Service>,
    pub groups: Vec<Group>,
    pub if_entries: Vec<IfEntry>,

    // Tag definitions for filtering
    pub entity_tags: Vec<Tag>,

    // Build state
    pub is_stale: bool,
    pub last_refreshed: DateTime<Utc>,
    pub is_locked: bool,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<Uuid>,

    pub removed_hosts: Vec<Uuid>,
    pub removed_interfaces: Vec<Uuid>,
    pub removed_subnets: Vec<Uuid>,
    pub removed_services: Vec<Uuid>,
    pub removed_groups: Vec<Uuid>,
    pub removed_ports: Vec<Uuid>,
    pub removed_bindings: Vec<Uuid>,
    pub removed_if_entries: Vec<Uuid>,
}

impl TopologyBase {
    pub fn new(name: String, network_id: Uuid) -> Self {
        Self {
            name,
            network_id,
            options: TopologyOptions::default(),
            nodes: vec![],
            edges: vec![],
            hosts: vec![],
            ports: vec![],
            interfaces: vec![],
            subnets: vec![],
            bindings: vec![],
            services: vec![],
            groups: vec![],
            if_entries: vec![],
            is_stale: true,
            last_refreshed: Utc::now(),
            is_locked: false,
            locked_at: None,
            locked_by: None,
            removed_hosts: vec![],
            removed_interfaces: vec![],
            removed_subnets: vec![],
            removed_services: vec![],
            removed_groups: vec![],
            removed_bindings: vec![],
            removed_ports: vec![],
            removed_if_entries: vec![],
            parent_id: None,
            tags: vec![],
            entity_tags: vec![],
        }
    }
}

impl ChangeTriggersTopologyStaleness<Topology> for Topology {
    fn triggers_staleness(&self, other: Option<Topology>) -> bool {
        if let Some(other_topology) = other {
            self.base.options.request != other_topology.base.options.request
        } else {
            false
        }
    }
}

impl Display for Topology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Topology {{ id: {}, name: {} }}",
            self.id, self.base.name
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, Hash, ToSchema)]
pub struct TopologyOptions {
    pub local: TopologyLocalOptions,
    pub request: TopologyRequestOptions,
}

/// Filter settings for hiding entities by tag in topology visualization.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default, ToSchema)]
pub struct TopologyTagFilter {
    /// Host tag IDs to hide (hosts with these tags will fade out)
    #[serde(default)]
    pub hidden_host_tag_ids: Vec<Uuid>,
    /// Service tag IDs to hide (services with these tags will be hidden from nodes)
    #[serde(default)]
    pub hidden_service_tag_ids: Vec<Uuid>,
    /// Subnet tag IDs to hide (subnets with these tags will fade out)
    #[serde(default)]
    pub hidden_subnet_tag_ids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, ToSchema)]
pub struct TopologyLocalOptions {
    pub left_zone_title: String,
    pub no_fade_edges: bool,
    pub hide_resize_handles: bool,
    pub hide_edge_types: Vec<EdgeTypeDiscriminants>,
    #[serde(default)]
    pub tag_filter: TopologyTagFilter,
}

impl Default for TopologyLocalOptions {
    fn default() -> Self {
        Self {
            left_zone_title: "Infrastructure".to_string(),
            no_fade_edges: false,
            hide_resize_handles: false,
            hide_edge_types: Vec::new(),
            tag_filter: TopologyTagFilter::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, ToSchema)]
pub struct TopologyRequestOptions {
    pub group_docker_bridges_by_host: bool,
    pub hide_vm_title_on_docker_container: bool,
    pub hide_ports: bool,
    pub left_zone_service_categories: Vec<ServiceCategory>,
    pub hide_service_categories: Vec<ServiceCategory>,
    pub show_gateway_in_left_zone: bool,
}

impl Default for TopologyRequestOptions {
    fn default() -> Self {
        Self {
            group_docker_bridges_by_host: true,
            hide_vm_title_on_docker_container: false,
            hide_ports: false,
            left_zone_service_categories: vec![ServiceCategory::DNS, ServiceCategory::ReverseProxy],
            hide_service_categories: Vec::new(),
            show_gateway_in_left_zone: true,
        }
    }
}

/// Lightweight request type for topology rebuild/refresh operations.
///
/// This type only includes the fields actually needed by the server - entity data
/// (hosts, interfaces, services, etc.) is fetched fresh from the database.
/// Using this instead of the full Topology dramatically reduces payload size
/// for large networks (from MBs to KBs), fixing HTTP 413 errors.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyRebuildRequest {
    /// Network ID for authorization and data fetching
    pub network_id: Uuid,
    /// Topology options for graph building
    pub options: TopologyOptions,
    /// Existing nodes for position preservation during rebuild
    #[serde(default)]
    pub nodes: Vec<Node>,
    /// Existing edges for reference during rebuild
    #[serde(default)]
    pub edges: Vec<Edge>,
}

/// Lightweight request type for updating a single node's position.
///
/// Used for drag operations - instead of sending the entire topology (which can be
/// several megabytes for large networks), only sends the node ID and new position.
/// Fixes HTTP 413 errors on drag operations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyNodePositionUpdate {
    /// Network ID for authorization
    pub network_id: Uuid,
    /// ID of the node to update
    pub node_id: Uuid,
    /// New position for the node
    pub position: Ixy,
}

/// Lightweight request type for updating an edge's handles.
///
/// Used for edge reconnect operations - instead of sending the entire topology,
/// only sends the edge ID and new handle positions.
/// Fixes HTTP 413 errors on edge reconnect operations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyEdgeHandleUpdate {
    /// Network ID for authorization
    pub network_id: Uuid,
    /// ID of the edge to update
    pub edge_id: Uuid,
    /// New source handle position
    pub source_handle: EdgeHandle,
    /// New target handle position
    pub target_handle: EdgeHandle,
}

/// Lightweight request type for updating a node's size and position.
///
/// Used for subnet resize operations - instead of sending the entire topology,
/// only sends the node ID, new size, and new position.
/// Fixes HTTP 413 errors on resize operations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyNodeResizeUpdate {
    /// Network ID for authorization
    pub network_id: Uuid,
    /// ID of the node to update
    pub node_id: Uuid,
    /// New size for the node
    pub size: Uxy,
    /// New position for the node
    pub position: Ixy,
}

/// Lightweight request type for updating topology metadata.
///
/// Used for editing topology name/parent - instead of sending the entire topology
/// (which includes all hosts, interfaces, services, etc.), only sends the metadata fields.
/// Fixes HTTP 413 errors on metadata edit operations.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TopologyMetadataUpdate {
    /// Network ID for authorization
    pub network_id: Uuid,
    /// New name for the topology
    pub name: String,
    /// New parent topology ID (optional)
    pub parent_id: Option<Uuid>,
}
