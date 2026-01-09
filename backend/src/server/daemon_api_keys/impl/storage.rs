use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase},
    shared::{
        entities::EntityDiscriminants,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

impl Storable for DaemonApiKey {
    type BaseData = DaemonApiKeyBase;

    fn table_name() -> &'static str {
        "api_keys"
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
                    key,
                    name,
                    last_used,
                    expires_at,
                    network_id,
                    is_enabled,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "last_used",
                "expires_at",
                "network_id",
                "name",
                "is_enabled",
                "key",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::OptionTimestamp(last_used),
                SqlValue::OptionTimestamp(expires_at),
                SqlValue::Uuid(network_id),
                SqlValue::String(name),
                SqlValue::Bool(is_enabled),
                SqlValue::String(key),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(DaemonApiKey {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: DaemonApiKeyBase {
                last_used: row.get("last_used"),
                expires_at: row.get("expires_at"),
                name: row.get("name"),
                key: row.get("key"),
                is_enabled: row.get("is_enabled"),
                network_id: row.get("network_id"),
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
        })
    }
}

impl Entity for DaemonApiKey {
    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::DaemonApiKey
    }

    fn entity_name_singular() -> &'static str {
        "daemon API key"
    }

    fn entity_name_plural() -> &'static str {
        "daemon-api-keys"
    }

    fn network_id(&self) -> Option<Uuid> {
        Some(self.base.network_id)
    }

    fn organization_id(&self) -> Option<Uuid> {
        None
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // key hash cannot be changed via update (use rotate endpoint instead)
        self.base.key = existing.base.key.clone();
        // last_used is server-set only
        self.base.last_used = existing.base.last_used;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
