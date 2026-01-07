use std::collections::HashMap;
use std::fmt::Display;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::{
    filter::EntityFilter,
    generic::GenericPostgresStorage,
    traits::{SqlValue, StorableEntity, Storage},
};

/// The base data for a UserApiKeyNetworkAccess junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct UserApiKeyNetworkAccessBase {
    pub api_key_id: Uuid,
    pub network_id: Uuid,
}

impl UserApiKeyNetworkAccessBase {
    pub fn new(api_key_id: Uuid, network_id: Uuid) -> Self {
        Self {
            api_key_id,
            network_id,
        }
    }
}

/// A junction record linking a user API key to a network it has access to
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct UserApiKeyNetworkAccess {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub base: UserApiKeyNetworkAccessBase,
}

impl UserApiKeyNetworkAccess {
    pub fn new(base: UserApiKeyNetworkAccessBase) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            base,
        }
    }

    pub fn api_key_id(&self) -> Uuid {
        self.base.api_key_id
    }

    pub fn network_id(&self) -> Uuid {
        self.base.network_id
    }
}

impl Display for UserApiKeyNetworkAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UserApiKeyNetworkAccess(api_key={}, network={})",
            self.base.api_key_id, self.base.network_id
        )
    }
}

impl StorableEntity for UserApiKeyNetworkAccess {
    type BaseData = UserApiKeyNetworkAccessBase;

    fn table_name() -> &'static str {
        "user_api_key_network_access"
    }

    fn new(base: Self::BaseData) -> Self {
        UserApiKeyNetworkAccess::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn network_id(&self) -> Option<Uuid> {
        Some(self.base.network_id)
    }

    fn organization_id(&self) -> Option<Uuid> {
        None
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.created_at // Junction table doesn't have updated_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    fn set_updated_at(&mut self, _time: DateTime<Utc>) {
        // No-op for junction table
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::UserApiKeyNetworkAccess
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["id", "api_key_id", "network_id", "created_at"],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.api_key_id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::Timestamp(self.created_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(UserApiKeyNetworkAccess {
            id: row.get("id"),
            created_at: row.get("created_at"),
            base: UserApiKeyNetworkAccessBase {
                api_key_id: row.get("api_key_id"),
                network_id: row.get("network_id"),
            },
        })
    }
}

/// Storage operations for user_api_key_network_access junction table.
/// Manages the network access list for each user API key.
pub struct UserApiKeyNetworkAccessStorage {
    storage: GenericPostgresStorage<UserApiKeyNetworkAccess>,
}

impl UserApiKeyNetworkAccessStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get all network IDs for a single API key
    pub async fn get_for_key(&self, api_key_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter = EntityFilter::unfiltered().uuid_column("api_key_id", api_key_id);
        let access_records = self.storage.get_all(filter).await?;
        Ok(access_records.iter().map(|a| a.network_id()).collect())
    }

    /// Get network IDs for multiple API keys (batch loading)
    pub async fn get_for_keys(&self, api_key_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if api_key_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = EntityFilter::unfiltered().uuid_columns("api_key_id", api_key_ids);
        let access_records = self.storage.get_all(filter).await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for access in access_records {
            result
                .entry(access.api_key_id())
                .or_default()
                .push(access.network_id());
        }

        Ok(result)
    }

    /// Save network IDs for an API key (replaces all existing)
    pub async fn save_for_key(&self, api_key_id: &Uuid, network_ids: &[Uuid]) -> Result<()> {
        // Delete existing access for this key
        self.delete_for_key(api_key_id).await?;

        // Insert new access records
        for network_id in network_ids {
            let access = UserApiKeyNetworkAccess::new(UserApiKeyNetworkAccessBase::new(
                *api_key_id,
                *network_id,
            ));
            self.storage.create(&access).await?;
        }

        Ok(())
    }

    /// Delete all network access for an API key
    pub async fn delete_for_key(&self, api_key_id: &Uuid) -> Result<()> {
        let filter = EntityFilter::unfiltered().uuid_column("api_key_id", api_key_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }

    /// Add a single network to an API key's access
    pub async fn add_network(&self, api_key_id: &Uuid, network_id: &Uuid) -> Result<()> {
        let access = UserApiKeyNetworkAccess::new(UserApiKeyNetworkAccessBase::new(
            *api_key_id,
            *network_id,
        ));
        // The storage will handle the unique constraint violation gracefully
        let _ = self.storage.create(&access).await;
        Ok(())
    }

    /// Remove a single network from an API key's access
    pub async fn remove_network(&self, api_key_id: &Uuid, network_id: &Uuid) -> Result<()> {
        let filter = EntityFilter::unfiltered()
            .uuid_column("api_key_id", api_key_id)
            .uuid_column("network_id", network_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }
}
