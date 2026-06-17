use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    config::AppState,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        handlers::{query::NetworkFilterQuery, traits::CrudHandlers},
    },
    snapshots::service::SnapshotService,
};

#[derive(
    Debug, Clone, Serialize, Deserialize, Validate, PartialEq, Eq, Hash, Default, ToSchema,
)]
pub struct SnapshotBase {
    pub network_id: Uuid,
    pub taken_at: DateTime<Utc>,
    #[serde(default)]
    pub created_by_user_id: Option<Uuid>,
}

impl SnapshotBase {
    pub fn new(
        network_id: Uuid,
        taken_at: DateTime<Utc>,
        created_by_user_id: Option<Uuid>,
    ) -> Self {
        Self {
            network_id,
            taken_at,
            created_by_user_id,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
pub struct Snapshot {
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
    pub base: SnapshotBase,
}

impl Snapshot {
    pub fn new(
        network_id: Uuid,
        taken_at: DateTime<Utc>,
        created_by_user_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SnapshotBase::new(network_id, taken_at, created_by_user_id),
        }
    }
}

impl Display for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Snapshot @ {}: {}", self.base.taken_at, self.id)
    }
}

impl CrudHandlers for Snapshot {
    type Service = SnapshotService;
    type FilterQuery = NetworkFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.snapshot_service
    }
}

impl ChangeTriggersTopologyStaleness<Snapshot> for Snapshot {
    fn triggers_staleness(&self, _other: Option<Snapshot>) -> bool {
        false
    }
}

#[allow(dead_code)]
pub fn snapshot_entity_type() -> EntityDiscriminants {
    EntityDiscriminants::Snapshot
}
