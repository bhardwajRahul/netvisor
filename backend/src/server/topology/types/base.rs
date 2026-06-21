use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::shared::entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants};
use crate::server::shared::types::metadata::HasId;
use crate::server::topology::types::edges::{EdgeHandle, EdgeTypeDiscriminants};
use crate::server::topology::types::grouping::{
    ContainerRule, ElementRule, GraphRule, IdentifiedRule,
};
use crate::server::topology::types::layout::{Ixy, Uxy};
use crate::server::topology::types::views::{
    MetadataFilterType, TopologyView, TopologyViewSupport,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};
use strum::IntoEnumIterator;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, ToSchema, Validate)]
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
    /// Resolve the available views for a share, filtering by data availability.
    /// If `configured` is None or empty, all data-supported views are returned.
    /// If `configured` is Some(non-empty list), returns the intersection preserving list order.
    ///
    /// `support` must be computed from raw entity data (see
    /// `TopologyService::get_view_support`) — NOT from this topology's
    /// persisted graph, because the graph is rebuilt per-view and its
    /// edges/entity_tags reflect only the most recently rendered view.
    pub fn resolve_available_views(
        &self,
        configured: &Option<Vec<TopologyView>>,
        support: &TopologyViewSupport,
    ) -> Vec<TopologyView> {
        let data_supported: Vec<TopologyView> = TopologyView::iter()
            .filter(|v| v.is_supported(support))
            .collect();

        match configured {
            None => data_supported,
            Some(list) if list.is_empty() => data_supported,
            Some(list) => list
                .iter()
                .filter(|v| data_supported.contains(v))
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Default, ToSchema)]
pub struct TopologyBase {
    pub network_id: Uuid,
    pub options: TopologyOptions,
    // The per-view node/edge graph is no longer persisted — it's a pure
    // function of entities + `options` and is built on request by the read
    // path (`build_all_view_graphs`). The row holds only the user's grouping
    // `options`; snapshots build their graph on request from closed copies, so
    // there are no snapshot-pinned topology rows (and no `snapshot_id`).
}

impl TopologyBase {
    pub fn new(network_id: Uuid) -> Self {
        Self {
            network_id,
            options: TopologyOptions::default(),
        }
    }
}

impl ChangeTriggersTopologyStaleness<Topology> for Topology {
    fn triggers_staleness(&self, other: Option<Topology>) -> bool {
        if let Some(other_topology) = other {
            // Switching the active view is a client-side slice selection (all
            // views are pre-built on the row), not an options change — request
            // options no longer carry a view scalar. Grouping/hide-rule edits
            // still trip this.
            self.base.options.request != other_topology.base.options.request
        } else {
            false
        }
    }
}

impl Display for Topology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Topology {{ id: {} }}", self.id)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, ToSchema)]
pub struct TopologyOptions {
    #[serde(default)]
    pub local: TopologyLocalOptions,
    #[serde(default)]
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
#[serde(default)]
pub struct TopologyLocalOptions {
    pub no_fade_edges: bool,
    pub hide_edge_types: Vec<EdgeTypeDiscriminants>,
    #[serde(default)]
    pub tag_filter: TopologyTagFilter,
    #[serde(default = "default_true")]
    pub show_minimap: bool,
    #[serde(default = "default_true")]
    pub bundle_edges: bool,
}

fn default_true() -> bool {
    true
}

impl Default for TopologyLocalOptions {
    fn default() -> Self {
        Self {
            no_fade_edges: false,
            hide_edge_types: vec![EdgeTypeDiscriminants::Hypervisor],
            tag_filter: TopologyTagFilter::default(),
            show_minimap: true,
            bundle_edges: true,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
#[serde(default)]
pub struct TopologyRequestOptions {
    /// Entity types hidden per view. Keyed by TopologyView, values are entity
    /// types (matching those declared as container/element/inline in the
    /// view's element_config). Hides every manifestation of the entity in
    /// that view — element nodes, container nodes, and inline rows on
    /// element cards. Supersedes the old `hide_ports` (L3-only, inline-only).
    #[serde(default)]
    pub hide_entities: HashMap<TopologyView, Vec<EntityDiscriminants>>,
    /// Generic per-(view, entity, filter) hide-set for metadata filters
    /// (Category, Virtualization, etc). Supersedes the old
    /// `hide_service_categories`; nested so JSON keys are strings all the
    /// way down.
    #[serde(default = "default_hide_metadata_values")]
    pub hide_metadata_values: HashMap<
        TopologyView,
        HashMap<EntityDiscriminants, HashMap<MetadataFilterType, Vec<String>>>,
    >,
    #[serde(default = "default_container_rules")]
    pub container_rules: HashMap<TopologyView, Vec<IdentifiedRule<ContainerRule>>>,
    #[serde(default = "default_element_rules")]
    pub element_rules: Vec<IdentifiedRule<ElementRule>>,
}

fn default_hide_metadata_values()
-> HashMap<TopologyView, HashMap<EntityDiscriminants, HashMap<MetadataFilterType, Vec<String>>>> {
    // Hide OpenPorts category on Service in every view — same behaviour as
    // the legacy `default_hide_service_categories`.
    TopologyView::iter()
        .map(|view| {
            let mut by_filter: HashMap<MetadataFilterType, Vec<String>> = HashMap::new();
            by_filter.insert(
                MetadataFilterType::Category,
                vec![ServiceCategory::OpenPorts.id().to_string()],
            );
            let mut by_entity: HashMap<
                EntityDiscriminants,
                HashMap<MetadataFilterType, Vec<String>>,
            > = HashMap::new();
            by_entity.insert(EntityDiscriminants::Service, by_filter);
            (view, by_entity)
        })
        .collect()
}

fn default_container_rules() -> HashMap<TopologyView, Vec<IdentifiedRule<ContainerRule>>> {
    use ContainerRule::*;

    // Build from applicable_views: for each rule type, add it to every view it applies to
    let all_rules: Vec<IdentifiedRule<ContainerRule>> = vec![
        IdentifiedRule::new(BySubnet),
        IdentifiedRule::new(MergeDockerBridges),
        IdentifiedRule::new(ByApplication { tag_ids: vec![] }),
        IdentifiedRule::new(ByHost),
    ];

    let mut map: HashMap<TopologyView, Vec<IdentifiedRule<ContainerRule>>> =
        TopologyView::iter().map(|p| (p, vec![])).collect();

    for gr in all_rules {
        for &view in gr.rule.applicable_views() {
            map.entry(view).or_default().push(gr.clone());
        }
    }

    map
}

fn default_element_rules() -> Vec<IdentifiedRule<ElementRule>> {
    vec![
        IdentifiedRule::new(ElementRule::ByTrunkPort),
        IdentifiedRule::new(ElementRule::ByVLAN),
        IdentifiedRule::new(ElementRule::ByPortOpStatus),
        IdentifiedRule::new(ElementRule::ByServiceCategory {
            categories: ServiceCategory::iter()
                .filter(|c| c.application_relevant_use_cases().is_empty())
                .collect(),
            title: Some("Infrastructure".into()),
            is_infra_rule: true,
        }),
        IdentifiedRule::new(ElementRule::ByTag {
            tag_ids: vec![],
            title: None,
        }),
        IdentifiedRule::new(ElementRule::ByHypervisor),
        IdentifiedRule::new(ElementRule::ByContainerRuntime),
        IdentifiedRule::new(ElementRule::ByStack),
    ]
}

impl Default for TopologyRequestOptions {
    fn default() -> Self {
        Self {
            hide_entities: HashMap::new(),
            hide_metadata_values: default_hide_metadata_values(),
            container_rules: default_container_rules(),
            element_rules: default_element_rules(),
        }
    }
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
    /// View whose node/edge slice this update targets
    pub view: TopologyView,
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
    /// View whose node/edge slice this update targets
    pub view: TopologyView,
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
    /// View whose node/edge slice this update targets
    pub view: TopologyView,
    /// ID of the node to update
    pub node_id: Uuid,
    /// New size for the node
    pub size: Uxy,
    /// New position for the node
    pub position: Ixy,
}
