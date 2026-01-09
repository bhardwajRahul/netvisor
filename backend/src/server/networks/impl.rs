use std::fmt::Display;

use crate::server::{
    config::AppState,
    networks::service::NetworkService,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        handlers::{query::NoFilterQuery, traits::CrudHandlers},
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::PgRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::storage::traits::{Entity, SqlValue, Storable};

#[derive(
    Debug, Clone, Serialize, Deserialize, Validate, PartialEq, Eq, Hash, Default, ToSchema,
)]
pub struct NetworkBase {
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    pub organization_id: Uuid,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

impl NetworkBase {
    pub fn new(organization_id: Uuid) -> Self {
        Self {
            name: "My Network".to_string(),
            organization_id,
            tags: Vec::new(),
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
#[schema(example = crate::server::shared::types::examples::network)]
pub struct Network {
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
    pub base: NetworkBase,
}

impl Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.base.name, self.id)
    }
}

impl CrudHandlers for Network {
    type Service = NetworkService;
    type FilterQuery = NoFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.network_service
    }
}

impl ChangeTriggersTopologyStaleness<Network> for Network {
    fn triggers_staleness(&self, _other: Option<Network>) -> bool {
        false
    }
}

impl Storable for Network {
    type BaseData = NetworkBase;

    fn table_name() -> &'static str {
        "networks"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    name,
                    organization_id,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();

        Ok((
            vec!["id", "created_at", "updated_at", "name", "organization_id"],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::Uuid(organization_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(Network {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: NetworkBase {
                name: row.get("name"),
                organization_id: row.get("organization_id"),
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
        })
    }
}

impl Entity for Network {
    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Network
    }

    fn entity_name_singular() -> &'static str {
        "network"
    }

    fn entity_name_plural() -> &'static str {
        "networks"
    }

    fn network_id(&self) -> Option<Uuid> {
        None
    }

    fn organization_id(&self) -> Option<Uuid> {
        Some(self.base.organization_id)
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
