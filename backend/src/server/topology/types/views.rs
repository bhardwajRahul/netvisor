use std::collections::{BTreeMap, HashMap};

use crate::server::{
    hosts::r#impl::virtualization::HostVirtualizationState,
    services::r#impl::categories::ServiceCategory,
    shared::{
        concepts::Concept,
        entities::EntityDiscriminants,
        types::entities::EntityFreshness,
        types::{
            Color, Icon,
            metadata::{
                EntityMetadataProvider, HasId, MetadataProvider, TypeMetadata, TypeMetadataProvider,
            },
        },
    },
};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

use super::edges::{
    EdgeDefaultVisibility, EdgeHighlightBehavior, EdgeStroke, EdgeTypeDiscriminants, EdgeViewConfig,
};

// ---------------------------------------------------------------------------
// TopologyView enum
// ---------------------------------------------------------------------------

/// Which topology view is being rendered
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Default,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum TopologyView {
    L2Physical,
    #[default]
    L3Logical,
    Workloads,
    Application,
}

impl HasId for TopologyView {
    fn id(&self) -> &'static str {
        self.into()
    }
}

// ---------------------------------------------------------------------------
// TopologyViewSupport — per-view data-availability flags
// ---------------------------------------------------------------------------

/// Per-view data-availability flags, computed from raw entity tables at
/// share-read time. Decoupled from the persisted topology graph because
/// that graph is view-specific — rebuilt under one view doesn't contain
/// the other views' nodes/edges.
///
/// Add a field per view-specific data requirement. Views that always
/// have the data they need (L3Logical / Workloads today) don't get a
/// field — `TopologyView::is_supported` just returns `true` for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopologyViewSupport {
    /// L2Physical requires interface-level neighbor discovery (LLDP/CDP).
    pub l2_physical: bool,
    /// Application requires at least one application-flagged tag to be
    /// assigned to an entity in the topology's network.
    pub application: bool,
}

impl TopologyView {
    /// Whether a topology with the given support flags has enough data to
    /// render this view. Exhaustive by design — adding a new `TopologyView`
    /// variant will fail compilation here until its eligibility rule is
    /// declared.
    pub fn is_supported(&self, support: &TopologyViewSupport) -> bool {
        match self {
            Self::L3Logical => true,
            Self::L2Physical => support.l2_physical,
            Self::Workloads => true,
            Self::Application => support.application,
        }
    }
}

// ---------------------------------------------------------------------------
// ViewElementConfig — defines how a view structures its elements
// ---------------------------------------------------------------------------

/// Defines the entity hierarchy for a topology view:
/// container (grouping box) → element (rendered as nodes) → inline (shown inside nodes)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ViewElementConfig {
    /// Entity rendered as the container/grouping box (e.g. Subnet in L3, Host in L2/Workloads)
    pub container_entity: Option<EntityDiscriminants>,
    /// Per-element-entity config: the entity types rendered as element nodes and
    /// (for each) which other entity types are shown *inside* those cards.
    /// Replaces the old flat `inline_entities` so views like Workloads — where
    /// Host elements inline services but Service elements inline nothing — can
    /// be expressed correctly.
    pub element_entities: Vec<ViewElementEntityConfig>,
    /// Generic metadata filters keyed by the entity they apply to in this
    /// view. Applied regardless of the entity's role (container/element/
    /// inline) — a Service metadata filter renders under the Services
    /// section whether Service is an element entity (Workloads/Application)
    /// or an inline entity (L3).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata_filters: HashMap<EntityDiscriminants, Vec<MetadataFilter>>,
    /// Single noun spanning all element entities. Used in summaries when the
    /// per-entity breakdown would be confusing (e.g. mixed Host+Service in
    /// Workloads, where everything is conceptually a "workload"). Singular;
    /// the UI pluralizes as needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collective_noun: Option<String>,
}

impl ViewElementConfig {
    /// Attach the staleness filter to every entity in this view that both
    /// carries a freshness verdict and can actually be hidden by one.
    ///
    /// Derived from the view's own hierarchy rather than hand-written per view,
    /// so a new view — or a view that gains a Host/Service slot — picks the
    /// filter up automatically and cannot forget it.
    ///
    /// Scope: Host and Service only. They are the two entities with a
    /// `source` (so a freshness verdict is meaningful — see
    /// `DiscoveryTracked::is_discovery_managed`) and the two the inventory
    /// already badges. Restricted to entities in an element or inline role
    /// because the generic hide path only removes element nodes and inline
    /// services; declaring it for a container-role Host would render a filter
    /// that silently does nothing.
    fn add_staleness_filters(&mut self) {
        let is_element = |entity: EntityDiscriminants| {
            self.element_entities
                .iter()
                .any(|e| e.entity_type == entity)
        };
        let is_inline = |entity: EntityDiscriminants| {
            self.element_entities
                .iter()
                .any(|e| e.inline_entities.contains(&entity))
        };

        for entity in [EntityDiscriminants::Host, EntityDiscriminants::Service] {
            if !is_element(entity) && !is_inline(entity) {
                continue;
            }
            self.metadata_filters
                .entry(entity)
                .or_default()
                .push(MetadataFilter {
                    filter_type: MetadataFilterType::Staleness,
                    label: "By staleness".to_string(),
                    // `New` is a digest-only bucket — it means "created during
                    // the scan window being reported on" and has no meaning in
                    // a live topology — so the values are enumerated rather
                    // than taken from the whole enum.
                    values: [EntityFreshness::Current, EntityFreshness::Stale]
                        .into_iter()
                        .map(|f| {
                            <EntityFreshness as MetadataProvider<TypeMetadata>>::to_metadata(&f)
                                .into()
                        })
                        .collect(),
                });
        }
    }
}

/// An element-entity slot inside a view, plus the set of entity types that
/// render inline on that element's card. Card rendering (what services/ports
/// show up inside an element) and layout re-trigger fingerprinting both read
/// from this list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ViewElementEntityConfig {
    pub entity_type: EntityDiscriminants,
    pub inline_entities: Vec<EntityDiscriminants>,
}

// ---------------------------------------------------------------------------
// MetadataFilter — declared per (view, element entity) on the backend; drives
// both the filter-panel chip UI and the hide-set stored in request options.
// ---------------------------------------------------------------------------

/// The kind of metadata filter. One variant per conceptually-distinct filter
/// across the app — Category (on Service), Virtualization (on Host), and so
/// on. Kept narrow on purpose: adding a new filter means adding a variant
/// here + a `HasFilterValues` impl on the relevant entity.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Ord,
    PartialOrd,
    ToSchema,
    EnumIter,
    IntoStaticStr,
)]
pub enum MetadataFilterType {
    Category,
    Virtualization,
    /// How recently discovery observed the entity. Unlike the others, this
    /// value is not intrinsic to the entity — it depends on the entity's
    /// network staleness window and the current time — so the frontend
    /// extractor for it takes the network as context.
    Staleness,
}

impl HasId for MetadataFilterType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

/// A declared metadata filter on an element entity inside a view.
///
/// `FilterValue::id` is the **stable identifier** — it's what each entity
/// emits via `HasFilterValues`, what the hide-set in request options stores,
/// and what matches entities on toggle. Changing an `id` silently clears
/// users' persisted hide entries for that value; `label` is display-only
/// and can change freely.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MetadataFilter {
    pub filter_type: MetadataFilterType,
    /// User-facing sub-section label (e.g. "By Category", "By Virtualization").
    pub label: String,
    pub values: Vec<FilterValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FilterValue {
    pub id: String,
    pub label: String,
    pub color: Color,
    #[schema(value_type = Option<String>, required)]
    pub icon: Option<Icon>,
}

/// `TypeMetadata` → `FilterValue`. Any enum whose values already implement
/// `TypeMetadataProvider` (ServiceCategory, HostVirtualizationState, etc.)
/// gets a ready-made `FilterValue` for free via `.to_metadata().into()` —
/// no hand-maintained parallel list.
impl From<TypeMetadata> for FilterValue {
    fn from(tm: TypeMetadata) -> Self {
        FilterValue {
            id: tm.id.to_string(),
            label: tm.name.unwrap_or(tm.id).to_string(),
            color: tm.color,
            icon: tm.icon,
        }
    }
}

/// Enumerate a `TypeMetadataProvider` strum enum straight into `FilterValue`s.
pub fn filter_values_from_enum<T>() -> Vec<FilterValue>
where
    T: TypeMetadataProvider + IntoEnumIterator,
{
    T::iter()
        .map(|v| <T as MetadataProvider<TypeMetadata>>::to_metadata(&v).into())
        .collect()
}

/// Entities that can surface metadata filter values. Returns the stable
/// `FilterValue::id` (per `MetadataFilterType`) for each filter the entity
/// participates in. Serialized alongside the entity so the frontend
/// doesn't need any extractor logic.
pub trait HasFilterValues {
    fn filter_values(&self) -> BTreeMap<MetadataFilterType, String>;
}

// ---------------------------------------------------------------------------
// Inspector types
// ---------------------------------------------------------------------------

/// Whether dependencies are tracked by service or by specific binding
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    EnumIter,
    IntoStaticStr,
)]
pub enum DependencyMemberType {
    Services,
    Bindings,
}

/// A section that can appear in the inspector panel
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    ToSchema,
    EnumIter,
    IntoStaticStr,
)]
pub enum InspectorSection {
    Identity,
    IfEntryData,
    Services,
    Dependencies,
    HostDetail,
    Virtualization,
    OtherInterfaces,
    PortBindings,
    SubnetDetail,
    ElementSummary,
    DependencySummary,
    Application,
}

/// View-specific inspector panel configuration.
/// Determines which sections appear and in what order for each view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct ViewInspectorConfig {
    pub element_sections: Vec<InspectorSection>,
    pub container_sections: Vec<InspectorSection>,
    pub dependency_creation: Option<DependencyMemberType>,
    pub show_application_picker: bool,
}

// ---------------------------------------------------------------------------
// TopologyView — metadata impls
// ---------------------------------------------------------------------------

impl EntityMetadataProvider for TopologyView {
    fn color(&self) -> Color {
        match self {
            Self::L2Physical => Concept::L2.color(),
            Self::L3Logical => Concept::L3.color(),
            Self::Workloads => Concept::Workloads.color(),
            Self::Application => Concept::Application.color(),
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::L2Physical => Concept::L2.icon(),
            Self::L3Logical => Concept::L3.icon(),
            Self::Workloads => Concept::Workloads.icon(),
            Self::Application => Concept::Application.icon(),
        }
    }
}

impl TypeMetadataProvider for TopologyView {
    fn name(&self) -> &'static str {
        match self {
            Self::L2Physical => "L2 Physical",
            Self::L3Logical => "L3 Logical",
            Self::Workloads => "Workloads",
            Self::Application => "Applications",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::L2Physical => "Interfaces and physical links between hosts",
            Self::L3Logical => "IP addresses and connectivity across subnets",
            Self::Workloads => "Services, VMs, and containers per host",
            Self::Application => "Services per application",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let edge_view_configs: serde_json::Map<String, serde_json::Value> =
            EdgeTypeDiscriminants::iter()
                .map(|et| {
                    (
                        et.to_string(),
                        serde_json::to_value(self.edge_view_config(et)).unwrap(),
                    )
                })
                .collect();

        serde_json::json!({
            "element_config": self.element_config(),
            "edge_view_configs": edge_view_configs,
            "inspector_config": self.inspector_config()
        })
    }
}

// ---------------------------------------------------------------------------
// TopologyView — typed methods
// ---------------------------------------------------------------------------

impl TopologyView {
    /// The entity hierarchy for this view: parent → element → inline
    pub fn element_config(&self) -> ViewElementConfig {
        let mut config = self.base_element_config();
        config.add_staleness_filters();
        config
    }

    /// Per-view hierarchy without the filters that are derived from it.
    fn base_element_config(&self) -> ViewElementConfig {
        match self {
            Self::L3Logical => ViewElementConfig {
                container_entity: Some(EntityDiscriminants::Subnet),
                element_entities: vec![ViewElementEntityConfig {
                    entity_type: EntityDiscriminants::IPAddress,
                    // Port first so it surfaces above Service in the filter
                    // panel (Service's category chips push it below the fold
                    // otherwise). Card rendering reads this list as a set,
                    // so order is irrelevant there.
                    inline_entities: vec![EntityDiscriminants::Port, EntityDiscriminants::Service],
                }],
                metadata_filters: [(
                    EntityDiscriminants::Service,
                    vec![MetadataFilter {
                        filter_type: MetadataFilterType::Category,
                        label: "By category".to_string(),
                        values: filter_values_from_enum::<ServiceCategory>(),
                    }],
                )]
                .into_iter()
                .collect(),
                collective_noun: None,
            },
            Self::L2Physical => ViewElementConfig {
                container_entity: Some(EntityDiscriminants::Host),
                element_entities: vec![ViewElementEntityConfig {
                    entity_type: EntityDiscriminants::Interface,
                    inline_entities: vec![],
                }],
                metadata_filters: HashMap::new(),
                collective_noun: None,
            },
            Self::Workloads => ViewElementConfig {
                container_entity: Some(EntityDiscriminants::Host),
                element_entities: vec![
                    ViewElementEntityConfig {
                        entity_type: EntityDiscriminants::Service,
                        inline_entities: vec![],
                    },
                    ViewElementEntityConfig {
                        entity_type: EntityDiscriminants::Host,
                        inline_entities: vec![EntityDiscriminants::Service],
                    },
                ],
                metadata_filters: [
                    (
                        EntityDiscriminants::Service,
                        vec![MetadataFilter {
                            filter_type: MetadataFilterType::Category,
                            label: "By category".to_string(),
                            values: filter_values_from_enum::<ServiceCategory>(),
                        }],
                    ),
                    (
                        EntityDiscriminants::Host,
                        vec![MetadataFilter {
                            filter_type: MetadataFilterType::Virtualization,
                            label: "By virtualization".to_string(),
                            values: filter_values_from_enum::<HostVirtualizationState>(),
                        }],
                    ),
                ]
                .into_iter()
                .collect(),
                collective_noun: Some("workload".to_string()),
            },
            Self::Application => ViewElementConfig {
                container_entity: None,
                element_entities: vec![ViewElementEntityConfig {
                    entity_type: EntityDiscriminants::Service,
                    inline_entities: vec![],
                }],
                metadata_filters: [(
                    EntityDiscriminants::Service,
                    vec![MetadataFilter {
                        filter_type: MetadataFilterType::Category,
                        label: "By category".to_string(),
                        values: filter_values_from_enum::<ServiceCategory>(),
                    }],
                )]
                .into_iter()
                .collect(),
                collective_noun: None,
            },
        }
    }

    /// Per-view configuration for each edge type.
    /// All match arms are exhaustive — adding a new EdgeTypeDiscriminants variant
    /// will cause a compile error here, forcing a configuration decision.
    pub fn edge_view_config(&self, edge_type: EdgeTypeDiscriminants) -> EdgeViewConfig {
        use EdgeDefaultVisibility::*;
        use EdgeHighlightBehavior::*;
        use EdgeStroke::*;
        use EdgeTypeDiscriminants::*;

        let active = |affects_layout,
                      visibility,
                      stroke,
                      highlight,
                      will_target_container,
                      show_directionality| {
            EdgeViewConfig::Active {
                affects_layout,
                default_visibility: visibility,
                stroke,
                highlight_behavior: highlight,
                will_target_container,
                show_directionality,
            }
        };

        match self {
            Self::L3Logical => match edge_type {
                SameHost => active(true, Visible, Solid, WhenVisible, false, false),
                ContainerRuntime => active(true, Visible, Solid, WhenVisible, true, false),
                RequestPath => active(false, Visible, Dashed, WhenVisible, false, true),
                HubAndSpoke => active(false, Visible, Dashed, WhenVisible, false, true),
                Hypervisor => active(false, Hidden, Dashed, WhenVisible, true, false),
                PhysicalLink => active(false, Hidden, Dashed, WhenVisible, false, false),
                // Annotates the graph rather than structuring it: no layout influence, off by
                // default, and never elevated to the subnet box — the point is that one
                // container spans them, which is lost if the edge targets the boxes.
                SameContainer => active(false, Hidden, Dotted, WhenVisible, false, false),
            },
            Self::L2Physical => match edge_type {
                PhysicalLink => active(true, Visible, Solid, WhenVisible, false, false),
                SameHost => active(false, Hidden, Dashed, WhenVisible, false, false),
                Hypervisor | ContainerRuntime | RequestPath | HubAndSpoke | SameContainer => {
                    EdgeViewConfig::Disabled
                }
            },
            Self::Workloads => match edge_type {
                PhysicalLink => active(false, Hidden, Dashed, WhenVisible, false, false),
                RequestPath | HubAndSpoke => {
                    active(false, Hidden, Dashed, WhenVisible, false, true)
                }
                Hypervisor | ContainerRuntime | SameHost | SameContainer => {
                    EdgeViewConfig::Disabled
                }
            },
            Self::Application => match edge_type {
                RequestPath => active(true, Visible, Solid, WhenVisible, false, true),
                HubAndSpoke => active(true, Visible, Solid, WhenVisible, false, true),
                ContainerRuntime => active(true, Hidden, Dashed, Always, true, false),
                SameHost | Hypervisor | PhysicalLink | SameContainer => EdgeViewConfig::Disabled,
            },
        }
    }

    /// Inspector panel configuration for this view
    pub fn inspector_config(&self) -> ViewInspectorConfig {
        match self {
            Self::L3Logical => ViewInspectorConfig {
                element_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::HostDetail,
                    InspectorSection::IfEntryData,
                    InspectorSection::Services,
                    InspectorSection::OtherInterfaces,
                ],
                container_sections: vec![
                    InspectorSection::SubnetDetail,
                    InspectorSection::ElementSummary,
                ],
                dependency_creation: Some(DependencyMemberType::Bindings),
                show_application_picker: false,
            },
            Self::Application => ViewInspectorConfig {
                element_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::Dependencies,
                    InspectorSection::Application,
                ],
                container_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::ElementSummary,
                    InspectorSection::DependencySummary,
                ],
                dependency_creation: Some(DependencyMemberType::Services),
                show_application_picker: true,
            },
            Self::Workloads => ViewInspectorConfig {
                element_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::HostDetail,
                    InspectorSection::Virtualization,
                    InspectorSection::Services,
                    InspectorSection::OtherInterfaces,
                ],
                container_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::ElementSummary,
                ],
                dependency_creation: Some(DependencyMemberType::Services),
                show_application_picker: false,
            },
            Self::L2Physical => ViewInspectorConfig {
                element_sections: vec![InspectorSection::Identity, InspectorSection::IfEntryData],
                container_sections: vec![
                    InspectorSection::Identity,
                    InspectorSection::ElementSummary,
                ],
                dependency_creation: None,
                show_application_picker: false,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn topology_view_serde_round_trip() {
        let json = serde_json::to_value(TopologyView::L2Physical).unwrap();
        assert_eq!(json, "L2Physical");
        let deserialized: TopologyView = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized, TopologyView::L2Physical);
    }

    #[test]
    fn all_views_have_non_empty_sections() {
        for view in TopologyView::iter() {
            let config = view.inspector_config();
            assert!(
                !config.element_sections.is_empty(),
                "{:?} has no element sections",
                view
            );
            assert!(
                !config.container_sections.is_empty(),
                "{:?} has no container sections",
                view
            );
        }
    }

    #[test]
    fn serde_roundtrip_inspector_section() {
        let section = InspectorSection::Dependencies;
        let json = serde_json::to_string(&section).unwrap();
        let deserialized: InspectorSection = serde_json::from_str(&json).unwrap();
        assert_eq!(section, deserialized);
    }

    #[test]
    fn serde_roundtrip_config() {
        let config = TopologyView::Application.inspector_config();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ViewInspectorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
