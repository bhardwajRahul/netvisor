use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, postgres::PgRow};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;

use super::{
    filter::EntityFilter,
    generic::GenericPostgresStorage,
    traits::{SqlValue, StorableEntity, Storage},
};

/// The base data for an EntityTag junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct EntityTagBase {
    pub entity_id: Uuid,
    pub entity_type: EntityDiscriminants,
    pub tag_id: Uuid,
}

impl EntityTagBase {
    pub fn new(entity_id: Uuid, entity_type: EntityDiscriminants, tag_id: Uuid) -> Self {
        Self {
            entity_id,
            entity_type,
            tag_id,
        }
    }
}

/// A junction record linking an entity to a tag
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct EntityTag {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub base: EntityTagBase,
}

impl EntityTag {
    pub fn new(base: EntityTagBase) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            base,
        }
    }
}

impl Display for EntityTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EntityTag(entity={}, type={}, tag={})",
            self.base.entity_id, self.base.entity_type, self.base.tag_id
        )
    }
}

impl StorableEntity for EntityTag {
    type BaseData = EntityTagBase;

    fn table_name() -> &'static str {
        "entity_tags"
    }

    fn new(base: Self::BaseData) -> Self {
        EntityTag::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn network_id(&self) -> Option<Uuid> {
        None
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
        EntityDiscriminants::EntityTag
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["id", "entity_id", "entity_type", "tag_id", "created_at"],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.entity_id),
                SqlValue::EntityDiscriminant(self.base.entity_type),
                SqlValue::Uuid(self.base.tag_id),
                SqlValue::Timestamp(self.created_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        let entity_type: EntityDiscriminants =
            serde_json::from_str(&row.get::<String, _>("entity_type"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize entity_type: {}", e))?;

        Ok(EntityTag {
            id: row.get("id"),
            created_at: row.get("created_at"),
            base: EntityTagBase {
                entity_id: row.get("entity_id"),
                entity_type,
                tag_id: row.get("tag_id"),
            },
        })
    }
}

/// Storage operations for the entity_tags junction table.
/// Manages tag assignments for all taggable entities.
pub struct EntityTagStorage {
    storage: GenericPostgresStorage<EntityTag>,
}

impl EntityTagStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get all tag IDs for a single entity
    pub async fn get_for_entity(
        &self,
        entity_id: &Uuid,
        entity_type: &EntityDiscriminants,
    ) -> Result<Vec<Uuid>> {
        let filter = EntityFilter::unfiltered()
            .uuid_column("entity_id", entity_id)
            .entity_type(entity_type);
        let records = self.storage.get_all(filter).await?;
        Ok(records.iter().map(|r| r.base.tag_id).collect())
    }

    /// Get tag IDs for multiple entities of the same type (batch loading)
    /// Returns a map of entity_id -> Vec<tag_id>
    pub async fn get_for_entities(
        &self,
        entity_ids: &[Uuid],
        entity_type: &EntityDiscriminants,
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if entity_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = EntityFilter::unfiltered()
            .uuid_columns("entity_id", entity_ids)
            .entity_type(entity_type);
        let records = self.storage.get_all(filter).await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for record in records {
            result
                .entry(record.base.entity_id)
                .or_default()
                .push(record.base.tag_id);
        }

        Ok(result)
    }

    /// Add a tag to an entity
    /// Returns Ok(true) if added, Ok(false) if already existed
    pub async fn add(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<bool> {
        let entity_tag = EntityTag::new(EntityTagBase::new(entity_id, entity_type, tag_id));

        match self.storage.create(&entity_tag).await {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a unique constraint violation (already exists)
                if e.to_string().contains("already exists") {
                    Ok(false)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Remove a tag from an entity
    /// Returns Ok(true) if removed, Ok(false) if didn't exist
    pub async fn remove(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<bool> {
        let filter = EntityFilter::unfiltered()
            .uuid_column("entity_id", &entity_id)
            .entity_type(&entity_type)
            .tag_id(&tag_id);

        let deleted = self.storage.delete_by_filter(filter).await?;
        Ok(deleted > 0)
    }

    /// Replace all tags for an entity with a new set
    pub async fn set(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_ids: Vec<Uuid>,
    ) -> Result<()> {
        // Delete existing tags
        self.remove_all_for_entity(entity_id, entity_type).await?;

        // Insert new tags
        for tag_id in tag_ids {
            let entity_tag = EntityTag::new(EntityTagBase::new(entity_id, entity_type, tag_id));
            self.storage.create(&entity_tag).await?;
        }

        Ok(())
    }

    /// Remove all tags for an entity (used when entity is deleted)
    pub async fn remove_all_for_entity(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
    ) -> Result<()> {
        let filter = EntityFilter::unfiltered()
            .uuid_column("entity_id", &entity_id)
            .entity_type(&entity_type);

        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }

    /// Bulk add a tag to multiple entities
    pub async fn bulk_add(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<usize> {
        if entity_ids.is_empty() {
            return Ok(0);
        }

        let mut added = 0;
        for entity_id in entity_ids {
            let entity_tag = EntityTag::new(EntityTagBase::new(*entity_id, entity_type, tag_id));

            // Ignore unique constraint violations (already exists)
            if self.storage.create(&entity_tag).await.is_ok() {
                added += 1;
            }
        }

        Ok(added)
    }

    /// Bulk remove a tag from multiple entities
    pub async fn bulk_remove(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<usize> {
        if entity_ids.is_empty() {
            return Ok(0);
        }

        let filter = EntityFilter::unfiltered()
            .uuid_columns("entity_id", entity_ids)
            .entity_type(&entity_type)
            .tag_id(&tag_id);

        self.storage.delete_by_filter(filter).await
    }
}
