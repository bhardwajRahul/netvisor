use crate::server::{
    dependencies::r#impl::types::DependencyTypeDiscriminants,
    shared::{
        concepts::Concept,
        entities::EntityDiscriminants,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumDiscriminants, EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;
use uuid::Uuid;

/// Protocol that discovered the physical link between network devices
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema)]
pub enum DiscoveryProtocol {
    /// Link Layer Discovery Protocol (IEEE 802.1AB)
    #[default]
    LLDP,
    /// Cisco Discovery Protocol (Cisco proprietary)
    CDP,
}

/// Whether an edge is visible by default or hidden behind a toggle
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeDefaultVisibility {
    #[default]
    Visible,
    Hidden,
}

/// Visual stroke style for an edge
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeStroke {
    #[default]
    Solid,
    Dashed,
    /// Finer break-up than `Dashed`, for edges that annotate the graph rather than
    /// structure it (see `SameContainer`).
    Dotted,
}

/// Controls when an edge contributes to node highlighting on selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EdgeHighlightBehavior {
    /// Highlights connected nodes when the edge is visible (not hidden by toggle)
    #[default]
    WhenVisible,
    /// Always highlights connected nodes regardless of visibility
    Always,
    /// Never highlights connected nodes
    Never,
}

/// What a click on an edge highlights.
///
/// An edge is one segment of a relation — a dependency's chain, a host's addresses, a
/// container's addresses, a runtime's bridges — and a click either lights up the whole
/// relation or only the segment that was clicked. Generic: any current or future edge type
/// picks one, and the selection code reads the property rather than branching on edge type.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EdgeSelectionScope {
    /// Highlight every node connected by any segment of the same relation. `relation_field`
    /// names the edge payload field holding that relation's id.
    ConnectedNodes { relation_field: &'static str },
    /// Highlight only this edge's own two endpoints.
    #[default]
    Segment,
}

/// Per-view configuration for an edge: disabled (not in this view) or active with properties
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EdgeViewConfig {
    /// Edge is not available in this view
    #[default]
    Disabled,
    /// Edge is active in this view with specific properties
    Active {
        /// Whether ELK should use this edge for layout positioning
        affects_layout: bool,
        /// Whether the edge is shown by default or hidden behind a toggle
        default_visibility: EdgeDefaultVisibility,
        /// Visual stroke style
        stroke: EdgeStroke,
        /// When this edge contributes to node highlighting on selection
        highlight_behavior: EdgeHighlightBehavior,
        /// Whether this edge should be elevated to target an accepting container
        /// instead of the element inside it
        will_target_container: bool,
        /// Whether this edge should show directional animation when highlighted
        show_directionality: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, ToSchema)]
pub struct Edge {
    pub id: Uuid,
    pub source: Uuid,
    pub target: Uuid,
    #[serde(flatten)]
    pub edge_type: EdgeType,
    #[schema(required)]
    pub label: Option<String>,
    pub source_handle: EdgeHandle,
    pub target_handle: EdgeHandle,
    pub is_multi_hop: bool,
    #[serde(default)]
    pub view_config: EdgeViewConfig,
}

#[derive(
    Serialize,
    Copy,
    Deserialize,
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    ToSchema,
)]
pub enum EdgeHandle {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(
    Serialize,
    Copy,
    Deserialize,
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Default,
    IntoStaticStr,
    Display,
    VariantNames,
    ToSchema,
)]
pub enum EdgeStyle {
    Straight,
    #[default]
    #[serde(alias = "Step")]
    SmoothStep,
    #[serde(alias = "SimpleBezier")]
    Bezier,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    EnumDiscriminants,
    IntoStaticStr,
    EnumIter,
    ToSchema,
)]
#[strum_discriminants(derive(Display, Hash, Serialize, Deserialize, EnumIter, ToSchema))]
#[serde(tag = "edge_type")]
pub enum EdgeType {
    SameHost {
        host_id: Uuid,
    },
    Hypervisor {
        hypervisor_service_id: Uuid,
    },
    ContainerRuntime {
        host_id: Uuid,
        service_id: Uuid,
        /// The bridge subnet(s) this edge reaches: one when they render as their own boxes,
        /// all of them when merged into a single box. Resolved here rather than in the
        /// inspector, which cannot tell which subnet an elevated edge landed on.
        subnet_ids: Vec<Uuid>,
        /// The containerized services this edge stands for — the ones on those subnets.
        containerized_service_ids: Vec<Uuid>,
    },
    /// One container reachable at several of its host's container-bridge subnets. Ties the
    /// container's addresses together so a multi-attached container reads as one thing rather
    /// than as unrelated cards in separate subnet boxes.
    SameContainer {
        service_id: Uuid,
    },
    RequestPath {
        dependency_id: Uuid,
        source_id: Uuid,
        target_id: Uuid,
    },
    HubAndSpoke {
        dependency_id: Uuid,
        source_id: Uuid,
        target_id: Uuid,
    },
    /// Physical link discovered via LLDP/CDP neighbor discovery
    PhysicalLink {
        source_entity_id: Uuid,
        target_entity_id: Uuid,
        protocol: DiscoveryProtocol,
    },
}

impl HasId for EdgeType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EdgeType {
    /// What a click on this edge highlights. Edges that stand for one segment of a wider
    /// relation light up the whole relation; edges that are a relationship in their own
    /// right light up their endpoints.
    pub fn selection_scope(&self) -> EdgeSelectionScope {
        use EdgeSelectionScope::*;
        match self {
            // Every segment of the dependency's chain.
            EdgeType::RequestPath { .. } | EdgeType::HubAndSpoke { .. } => ConnectedNodes {
                relation_field: "dependency_id",
            },
            // Every address of the host.
            EdgeType::SameHost { .. } => ConnectedNodes {
                relation_field: "host_id",
            },
            // Every address of the container.
            EdgeType::SameContainer { .. } => ConnectedNodes {
                relation_field: "service_id",
            },
            // A runtime's edges each reach a different bridge, and a hypervisor's each reach a
            // different VM — they are separate connections that happen to share an origin, not
            // segments of one thing, so a click stays on the one that was clicked. A physical
            // link is likewise the whole relationship, not a segment of one.
            EdgeType::ContainerRuntime { .. }
            | EdgeType::Hypervisor { .. }
            | EdgeType::PhysicalLink { .. } => Segment,
        }
    }
}

impl EntityMetadataProvider for EdgeType {
    fn color(&self) -> Color {
        match self {
            EdgeType::RequestPath { .. } => EntityDiscriminants::Dependency.color(),
            EdgeType::HubAndSpoke { .. } => EntityDiscriminants::Dependency.color(),
            EdgeType::SameHost { .. } => EntityDiscriminants::Host.color(),
            EdgeType::Hypervisor { .. } => Concept::Virtualization.color(),
            EdgeType::ContainerRuntime { .. } => Concept::Containerization.color(),
            EdgeType::SameContainer { .. } => Concept::Containerization.color(),
            EdgeType::PhysicalLink { .. } => EntityDiscriminants::Interface.color(),
        }
    }

    fn icon(&self) -> Icon {
        match self {
            EdgeType::RequestPath { .. } => DependencyTypeDiscriminants::RequestPath.icon(),
            EdgeType::HubAndSpoke { .. } => DependencyTypeDiscriminants::HubAndSpoke.icon(),
            EdgeType::SameHost { .. } => EntityDiscriminants::Host.icon(),
            EdgeType::Hypervisor { .. } => Concept::Virtualization.icon(),
            EdgeType::ContainerRuntime { .. } => Concept::Containerization.icon(),
            EdgeType::SameContainer { .. } => Concept::Containerization.icon(),
            EdgeType::PhysicalLink { .. } => EntityDiscriminants::Interface.icon(),
        }
    }
}

impl TypeMetadataProvider for EdgeType {
    fn name(&self) -> &'static str {
        match self {
            EdgeType::RequestPath { .. } => DependencyTypeDiscriminants::RequestPath.name(),
            EdgeType::HubAndSpoke { .. } => DependencyTypeDiscriminants::HubAndSpoke.name(),
            EdgeType::SameHost { .. } => "Same Host",
            EdgeType::Hypervisor { .. } => "Hypervisor",
            EdgeType::ContainerRuntime { .. } => "Container Runtime",
            EdgeType::SameContainer { .. } => "Same Container",
            EdgeType::PhysicalLink { .. } => "Physical Link",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let edge_style: &str = match &self {
            EdgeType::RequestPath { .. } => EdgeStyle::Bezier.into(),
            EdgeType::HubAndSpoke { .. } => EdgeStyle::Bezier.into(),
            EdgeType::SameHost { .. } => EdgeStyle::Bezier.into(),
            EdgeType::Hypervisor { .. } => EdgeStyle::Bezier.into(),
            EdgeType::ContainerRuntime { .. } => EdgeStyle::Bezier.into(),
            EdgeType::SameContainer { .. } => EdgeStyle::Bezier.into(),
            EdgeType::PhysicalLink { .. } => EdgeStyle::Bezier.into(),
        };

        let has_start_marker = false;

        let has_end_marker = match &self {
            EdgeType::RequestPath { .. } => true,
            EdgeType::HubAndSpoke { .. } => true,
            EdgeType::SameHost { .. } => false,
            EdgeType::Hypervisor { .. } => false,
            EdgeType::ContainerRuntime { .. } => false,
            EdgeType::SameContainer { .. } => false,
            EdgeType::PhysicalLink { .. } => false, // No markers - bidirectional link
        };

        let is_host_edge = matches!(
            self,
            EdgeType::SameHost { .. } | EdgeType::ContainerRuntime { .. }
        );
        let is_dependency_edge = matches!(
            self,
            EdgeType::RequestPath { .. } | EdgeType::HubAndSpoke { .. }
        );
        let is_physical_edge = matches!(self, EdgeType::PhysicalLink { .. });

        serde_json::json!({
            "has_start_marker": has_start_marker,
            "has_end_marker": has_end_marker,
            "edge_style": edge_style,
            "is_host_edge": is_host_edge,
            "is_dependency_edge": is_dependency_edge,
            "is_physical_edge": is_physical_edge,
            "selection_scope": self.selection_scope()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::dependencies::r#impl::types::DependencyTypeDiscriminants;
    use strum::IntoEnumIterator;

    #[test]
    fn edge_type_matches_dependency_type() {
        // This will fail to compile if DependencyType adds/removes variants
        // without updating EdgeType
        let dependency_types: Vec<DependencyTypeDiscriminants> =
            DependencyTypeDiscriminants::iter().collect();

        assert_eq!(
            dependency_types.len(),
            2,
            "Update EdgeType to match DependencyType variants!"
        );
        assert!(dependency_types.contains(&DependencyTypeDiscriminants::RequestPath));
        assert!(dependency_types.contains(&DependencyTypeDiscriminants::HubAndSpoke));
    }

    #[test]
    fn edge_view_config_serde_round_trips() {
        // Disabled variant
        let disabled = EdgeViewConfig::Disabled;
        let json = serde_json::to_value(disabled).unwrap();
        assert_eq!(json["type"], "disabled");
        let deserialized: EdgeViewConfig = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, disabled);

        // Active variant
        let active = EdgeViewConfig::Active {
            affects_layout: true,
            default_visibility: EdgeDefaultVisibility::Hidden,
            stroke: EdgeStroke::Dashed,
            highlight_behavior: EdgeHighlightBehavior::Always,
            will_target_container: true,
            show_directionality: true,
        };
        let json = serde_json::to_value(active).unwrap();
        assert_eq!(json["type"], "active");
        assert_eq!(json["affects_layout"], true);
        assert_eq!(json["default_visibility"], "hidden");
        assert_eq!(json["stroke"], "dashed");
        assert_eq!(json["highlight_behavior"], "always");
        assert_eq!(json["will_target_container"], true);
        assert_eq!(json["show_directionality"], true);
        let deserialized: EdgeViewConfig = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, active);
    }

    #[test]
    fn view_config_default_is_disabled() {
        assert_eq!(EdgeViewConfig::default(), EdgeViewConfig::Disabled);
    }

    /// The relation-scoped edges point the selection code at one of their own payload fields
    /// by name. Renaming or dropping that field would silently degrade every click on the
    /// edge to "highlight my two endpoints", so hold the two in step here.
    #[test]
    fn relation_scoped_edges_carry_the_field_they_name() {
        for edge_type in EdgeType::iter() {
            let EdgeSelectionScope::ConnectedNodes { relation_field } = edge_type.selection_scope()
            else {
                continue;
            };
            let payload = serde_json::to_value(&edge_type).unwrap();
            assert!(
                payload.get(relation_field).is_some_and(|v| v.is_string()),
                "{edge_type:?} says it groups by `{relation_field}`, but serializes {payload}"
            );
        }
    }
}
