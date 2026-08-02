use std::fmt::Display;

use crate::server::shared::{
    entities::ChangeTriggersTopologyStaleness,
    types::{Color, api::deserialize_empty_string_as_none},
};
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct TagBase {
    /// Human-facing name for this tag.
    #[validate(length(
        min = 1,
        max = 100,
        message = "Tag name must be between 1 and 100 characters"
    ))]
    pub name: String,
    /// Free-text notes about the tag.
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    pub description: Option<String>,
    /// Colour the tag is drawn in.
    pub color: Color,
    /// The organization that owns this record.
    pub organization_id: Uuid,
    /// Whether this tag groups an application, so it drives the application view.
    #[serde(default)]
    pub is_application: bool,
}

impl Default for TagBase {
    fn default() -> Self {
        Self {
            name: "New Tag".to_string(),
            description: None,
            color: Color::Yellow,
            organization_id: Uuid::nil(),
            is_application: false,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema, Validate,
)]
#[schema(example = crate::server::shared::types::examples::tag)]
pub struct Tag {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// Start of the interval this revision was current for (SCD2 history).
    #[serde(default)]
    #[schema(read_only)]
    pub valid_from: DateTime<Utc>,
    /// End of the interval this revision was current for. `null` while it is the live revision.
    #[serde(default)]
    #[schema(read_only)]
    pub valid_to: Option<DateTime<Utc>>,
    /// Stable identifier shared by every revision of the same entity across its history.
    #[serde(default)]
    #[schema(read_only)]
    pub lineage_id: Option<Uuid>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: TagBase,
}

impl ChangeTriggersTopologyStaleness<Tag> for Tag {
    /// Always returns true — the actual determination of whether a Tag change
    /// should mark a topology stale lives downstream in the topology subscriber,
    /// which has the cross-service access needed to check if the tag is an
    /// application tag or is referenced by a ByTag element rule.
    /// See `TopologyService::tag_affects_any_topology`.
    fn triggers_staleness(&self, _other: Option<Tag>) -> bool {
        true
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tag {}: {}", self.base.name, self.id)
    }
}
