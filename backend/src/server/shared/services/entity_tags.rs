use crate::server::{
    shared::{
        entities::EntityDiscriminants,
        storage::{entity_tags::EntityTagStorage, traits::StorableEntity},
        types::api::ApiError,
    },
    tags::service::TagService,
};
use anyhow::anyhow;
use anyhow::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Service for managing tag assignments to entities.
///
/// Provides:
/// - Tag hydration for entities retrieved from the database
/// - Tag assignment/removal with validation
/// - Bulk operations for efficient multi-entity updates
pub struct EntityTagService {
    storage: Arc<EntityTagStorage>,
    tag_service: Arc<TagService>,
}

impl EntityTagService {
    pub fn new(storage: Arc<EntityTagStorage>, tag_service: Arc<TagService>) -> Self {
        Self {
            storage,
            tag_service,
        }
    }

    // =========================================================================
    // Hydration Methods
    // =========================================================================

    /// Get tags for a single entity.
    pub async fn get_tags(
        &self,
        entity_id: &Uuid,
        entity_type: &EntityDiscriminants,
    ) -> Result<Vec<Uuid>, Error> {
        self.storage.get_for_entity(entity_id, entity_type).await
    }

    /// Hydrate tags for a single entity.
    pub async fn hydrate_tags<T: StorableEntity>(&self, entity: &mut T) -> Result<()> {
        let tags = self
            .storage
            .get_for_entity(&entity.id(), &T::entity_type())
            .await?;
        entity.set_tags(tags);
        Ok(())
    }

    /// Hydrate tags for a batch of entities (single database query).
    ///
    /// This is the preferred method for list endpoints to avoid N+1 queries.
    pub async fn hydrate_tags_batch<T: StorableEntity>(&self, entities: &mut [T]) -> Result<()> {
        if entities.is_empty() {
            return Ok(());
        }

        let ids: Vec<Uuid> = entities.iter().map(|e| e.id()).collect();
        let tags_map = self
            .storage
            .get_for_entities(&ids, &T::entity_type())
            .await?;

        for entity in entities {
            let tags = tags_map.get(&entity.id()).cloned().unwrap_or_default();
            entity.set_tags(tags);
        }

        Ok(())
    }

    /// Get tags for multiple entities as a map (useful when building response types).
    pub async fn get_tags_map(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        self.storage
            .get_for_entities(entity_ids, &entity_type)
            .await
    }

    // =========================================================================
    // Assignment Methods
    // =========================================================================

    /// Add a tag to an entity.
    ///
    /// Validates that:
    /// - The tag exists
    /// - The tag belongs to the specified organization
    pub async fn add_tag(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
        organization_id: Uuid,
    ) -> Result<(), Error> {
        // Validate tag exists and belongs to organization
        self.validate_tag(tag_id, organization_id).await?;

        // Add to junction table
        self.storage
            .add(entity_id, entity_type, tag_id)
            .await
            .map_err(|e| anyhow!("Failed to add tag: {}", e))?;

        Ok(())
    }

    /// Remove a tag from an entity.
    pub async fn remove_tag(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<(), Error> {
        self.storage
            .remove(entity_id, entity_type, tag_id)
            .await
            .map_err(|e| anyhow!("Failed to remove tag: {}", e))?;

        Ok(())
    }

    /// Replace all tags for an entity.
    ///
    /// Validates all new tags before making changes.
    pub async fn set_tags(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
        tag_ids: Vec<Uuid>,
        organization_id: Uuid,
    ) -> Result<(), Error> {
        if tag_ids.is_empty() {
            return Ok(());
        }

        // Validate all tags
        for tag_id in &tag_ids {
            self.validate_tag(*tag_id, organization_id).await?;
        }

        // Replace tags
        self.storage
            .set(entity_id, entity_type, tag_ids)
            .await
            .map_err(|e| anyhow!("Failed to set tags: {}", e))?;

        Ok(())
    }

    /// Remove all tags when an entity is deleted.
    pub async fn remove_all_for_entity(
        &self,
        entity_id: Uuid,
        entity_type: EntityDiscriminants,
    ) -> Result<()> {
        self.storage
            .remove_all_for_entity(entity_id, entity_type)
            .await
    }

    // =========================================================================
    // Bulk Operations
    // =========================================================================

    /// Add a tag to multiple entities.
    ///
    /// Validates the tag once, then adds to all entities.
    pub async fn bulk_add_tag(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
        organization_id: Uuid,
    ) -> Result<usize, ApiError> {
        if entity_ids.is_empty() {
            return Ok(0);
        }

        // Validate tag exists and belongs to organization
        self.validate_tag(tag_id, organization_id).await?;

        // Bulk add
        let count = self
            .storage
            .bulk_add(entity_ids, entity_type, tag_id)
            .await
            .map_err(|e| ApiError::internal_error(&format!("Failed to bulk add tag: {}", e)))?;

        Ok(count)
    }

    /// Remove a tag from multiple entities.
    pub async fn bulk_remove_tag(
        &self,
        entity_ids: &[Uuid],
        entity_type: EntityDiscriminants,
        tag_id: Uuid,
    ) -> Result<usize, Error> {
        if entity_ids.is_empty() {
            return Ok(0);
        }

        let count = self
            .storage
            .bulk_remove(entity_ids, entity_type, tag_id)
            .await
            .map_err(|e| anyhow!("Failed to bulk remove tag: {}", e))?;

        Ok(count)
    }

    // =========================================================================
    // Validation Helpers
    // =========================================================================

    /// Validate that a tag exists and belongs to the specified organization.
    async fn validate_tag(&self, tag_id: Uuid, organization_id: Uuid) -> Result<(), Error> {
        use crate::server::shared::services::traits::CrudService;

        match self.tag_service.get_by_id(&tag_id).await {
            Ok(Some(tag)) => {
                if tag.base.organization_id != organization_id {
                    return Err(anyhow!(
                        "Tag {} does not belong to this organization",
                        tag_id
                    ));
                }
                Ok(())
            }
            Ok(None) => Err(anyhow!("Tag {} not found", tag_id)),
            Err(e) => Err(anyhow!("Failed to validate tag {}: {}", tag_id, e)),
        }
    }
}
