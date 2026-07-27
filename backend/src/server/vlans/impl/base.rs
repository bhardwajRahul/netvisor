use std::fmt::Display;

use crate::server::shared::{
    entities::ChangeTriggersTopologyStaleness,
    types::{api::deserialize_empty_string_as_none, entities::EntitySource},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct VlanBase {
    /// The 802.1Q VLAN number (1-4094)
    pub vlan_number: u16,
    #[validate(length(
        min = 1,
        max = 100,
        message = "VLAN name must be between 1 and 100 characters"
    ))]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_empty_string_as_none")]
    pub description: Option<String>,
    pub network_id: Uuid,
    pub organization_id: Uuid,
    #[serde(default)]
    pub source: EntitySource,
}

impl Default for VlanBase {
    fn default() -> Self {
        Self {
            vlan_number: 1,
            name: "Default".to_string(),
            description: None,
            network_id: Uuid::nil(),
            organization_id: Uuid::nil(),
            source: EntitySource::Manual,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema, Validate,
)]
pub struct Vlan {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only)]
    pub valid_from: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only)]
    pub valid_to: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(read_only)]
    pub lineage_id: Option<Uuid>,
    #[serde(default)]
    #[schema(read_only)]
    pub last_seen_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only)]
    pub last_discovery_id: Option<Uuid>,
    #[serde(default)]
    #[schema(read_only)]
    pub first_discovery_id: Option<Uuid>,
    /// Subnets associated with this VLAN, derived from discovered interface
    /// native-VLAN data via the `subnet_vlans` junction. Hydrated by
    /// `VlanService` on read — it is not a column on `vlans`, and it is never
    /// accepted on create/update (which take `VlanBase`).
    #[serde(default)]
    #[schema(read_only)]
    pub subnet_ids: Vec<Uuid>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: VlanBase,
}

impl ChangeTriggersTopologyStaleness<Vlan> for Vlan {
    fn triggers_staleness(&self, other: Option<Vlan>) -> bool {
        match other {
            Some(prev) => self.base.name != prev.base.name,
            None => true,
        }
    }
}

impl Display for Vlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Vlan {} ({}): {}",
            self.base.vlan_number, self.base.name, self.id
        )
    }
}
