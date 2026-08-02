use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::Hash;
use strum_macros::{EnumDiscriminants, EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    hosts::r#impl::base::Host,
    shared::{
        concepts::Concept,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
    topology::types::views::{HasFilterValues, MetadataFilterType},
};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumDiscriminants,
    VariantNames,
    ToSchema,
)]
#[strum_discriminants(derive(IntoStaticStr))]
#[schema(title = "HostVirtualization")]
#[serde(tag = "type", content = "details")]
pub enum HostVirtualization {
    #[schema(title = "Proxmox")]
    Proxmox(ProxmoxVirtualization),
    #[schema(title = "VCenter")]
    VCenter(VCenterVirtualization),
    #[schema(title = "ESXi")]
    ESXi(EsxiVirtualization),
}

#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct ProxmoxVirtualization {
    /// Guest name as configured in Proxmox.
    pub vm_name: Option<String>,
    /// Proxmox VMID of the guest.
    pub vm_id: Option<String>,
    /// The service this entity refers to.
    pub service_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct VCenterVirtualization {
    /// Guest name as configured in vCenter.
    pub vm_name: Option<String>,
    /// vCenter managed object ID of the guest.
    pub vm_id: Option<String>,
    /// The service this entity refers to.
    pub service_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Validate, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct EsxiVirtualization {
    /// Guest name as configured on the ESXi host.
    pub vm_name: Option<String>,
    /// ESXi identifier of the guest.
    pub vm_id: Option<String>,
    /// The service this entity refers to.
    pub service_id: Uuid,
}

impl HostVirtualization {
    pub fn service_id(&self) -> Option<Uuid> {
        match self {
            Self::Proxmox(p) => Some(p.service_id),
            Self::VCenter(v) => Some(v.service_id),
            Self::ESXi(e) => Some(e.service_id),
        }
    }

    pub fn set_service_id(&mut self, id: Uuid) {
        match self {
            Self::Proxmox(p) => p.service_id = id,
            Self::VCenter(v) => v.service_id = id,
            Self::ESXi(e) => e.service_id = id,
        }
    }
}

impl HasId for HostVirtualization {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for HostVirtualization {
    fn color(&self) -> Color {
        Concept::Virtualization.color()
    }
    fn icon(&self) -> Icon {
        Concept::Virtualization.icon()
    }
}

impl TypeMetadataProvider for HostVirtualization {
    fn name(&self) -> &'static str {
        match self {
            Self::Proxmox(_) => "Proxmox",
            Self::VCenter(_) => "vCenter",
            Self::ESXi(_) => "ESXi",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Proxmox(_) => "A host running as a Proxmox VM",
            Self::VCenter(_) => "A host running as a vCenter-managed VM",
            Self::ESXi(_) => "A host running as an ESXi VM",
        }
    }
}

/// Coarse virtualization state used by the `Virtualization` metadata filter
/// on Host. Each host resolves to exactly one variant via `HasFilterValues`.
/// Today derived from `host.virtualization.is_some()`; future finer states
/// (e.g. per-hypervisor) can add variants here without breaking persistence
/// of the existing ids.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    IntoStaticStr,
    EnumIter,
    ToSchema,
)]
pub enum HostVirtualizationState {
    Virtualized,
    BareMetal,
}

impl HostVirtualizationState {
    pub fn from_host_virtualization(v: Option<&HostVirtualization>) -> Self {
        match v {
            Some(_) => Self::Virtualized,
            None => Self::BareMetal,
        }
    }
}

impl HasId for HostVirtualizationState {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for HostVirtualizationState {
    fn color(&self) -> Color {
        match self {
            Self::Virtualized => Concept::Virtualization.color(),
            Self::BareMetal => Color::Gray,
        }
    }
    fn icon(&self) -> Icon {
        match self {
            Self::Virtualized => Concept::Virtualization.icon(),
            Self::BareMetal => Icon::Server,
        }
    }
}

impl TypeMetadataProvider for HostVirtualizationState {
    fn name(&self) -> &'static str {
        match self {
            Self::Virtualized => "Virtualized",
            Self::BareMetal => "Bare metal",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Virtualized => "Hosts running as virtual machines",
            Self::BareMetal => "Hosts running on physical hardware",
        }
    }
}

impl HasFilterValues for Host {
    fn filter_values(&self) -> BTreeMap<MetadataFilterType, String> {
        let mut values = BTreeMap::new();
        let state =
            HostVirtualizationState::from_host_virtualization(self.base.virtualization.as_ref());
        values.insert(MetadataFilterType::Virtualization, state.id().to_string());
        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_virtualization_variants_round_trip_by_tag() {
        // The serde "type" tag is what the manual-assignment UI sends (sourced
        // from ServiceDefinition::virtualization_variant). Confirm each tag
        // deserializes and round-trips, and that the tag == HasId::id().
        for tag in ["Proxmox", "VCenter", "ESXi"] {
            let json = format!(
                r#"{{"type":"{tag}","details":{{"vm_name":null,"vm_id":null,"service_id":"00000000-0000-0000-0000-000000000000"}}}}"#
            );
            let v: HostVirtualization =
                serde_json::from_str(&json).unwrap_or_else(|e| panic!("{tag}: {e}"));
            assert_eq!(v.id(), tag, "id() must equal serde tag for {tag}");
            assert_eq!(v.service_id(), Some(Uuid::nil()));
            let reserialized = serde_json::to_value(&v).unwrap();
            assert_eq!(reserialized["type"], tag);
        }
    }

    #[test]
    fn host_virtualization_discriminant_static_str_matches_serde_tag() {
        // The discriminant's IntoStaticStr is what VirtualizationRole::variant_tag
        // derives from; it must equal the serde "type" tag above.
        assert_eq!(
            <&'static str>::from(HostVirtualizationDiscriminants::Proxmox),
            "Proxmox"
        );
        assert_eq!(
            <&'static str>::from(HostVirtualizationDiscriminants::VCenter),
            "VCenter"
        );
        assert_eq!(
            <&'static str>::from(HostVirtualizationDiscriminants::ESXi),
            "ESXi"
        );
    }
}
