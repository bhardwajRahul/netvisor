use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        api_key_common::ApiKeyService,
        events::bus::EventBus,
        services::{
            entity_tags::EntityTagService,
            traits::{CrudService, EventBusService},
        },
        storage::generic::GenericPostgresStorage,
    },
    user_api_keys::r#impl::{base::UserApiKey, network_access::UserApiKeyNetworkAccessStorage},
    users::r#impl::permissions::UserOrgPermissions,
};

pub struct UserApiKeyService {
    storage: Arc<GenericPostgresStorage<UserApiKey>>,
    network_access_storage: Arc<UserApiKeyNetworkAccessStorage>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<UserApiKey> for UserApiKeyService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &UserApiKey) -> Option<Uuid> {
        // User API keys use junction table, not a single network_id
        None
    }

    fn get_organization_id(&self, entity: &UserApiKey) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }

    fn suppress_logs(&self, current: Option<&UserApiKey>, updated: Option<&UserApiKey>) -> bool {
        match (current, updated) {
            (Some(current), Some(updated)) => updated.suppress_logs(current),
            _ => false,
        }
    }
}

#[async_trait]
impl CrudService<UserApiKey> for UserApiKeyService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<UserApiKey>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }
}

impl UserApiKeyService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<UserApiKey>>,
        network_access_storage: Arc<UserApiKeyNetworkAccessStorage>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            network_access_storage,
            event_bus,
            entity_tag_service,
        }
    }

    /// Get the network access storage for junction table operations
    pub fn network_access_storage(&self) -> &Arc<UserApiKeyNetworkAccessStorage> {
        &self.network_access_storage
    }

    /// Get a user API key by its hashed key value
    pub async fn get_by_key(&self, hashed_key: &str) -> Result<Option<UserApiKey>> {
        use crate::server::shared::storage::{filter::EntityFilter, traits::Storage};

        let filter = EntityFilter::unfiltered().api_key(hashed_key.to_string());
        if let Some(mut key) = self.storage.get_one(filter).await? {
            // Hydrate network_ids from junction table
            key.base.network_ids = self.network_access_storage.get_for_key(&key.id).await?;
            self.hydrate_tags(&mut key).await?;
            return Ok(Some(key));
        }
        Ok(None)
    }

    /// Get all API keys for a specific user, with network_ids hydrated
    pub async fn get_for_user(&self, user_id: &Uuid) -> Result<Vec<UserApiKey>> {
        use crate::server::shared::storage::{filter::EntityFilter, traits::Storage};

        let filter = EntityFilter::unfiltered().user_id(user_id);
        let mut keys = self.storage.get_all(filter).await?;

        // Batch hydrate network_ids
        let key_ids: Vec<Uuid> = keys.iter().map(|k| k.id).collect();
        let network_map = self.network_access_storage.get_for_keys(&key_ids).await?;

        for key in &mut keys {
            key.base.network_ids = network_map.get(&key.id).cloned().unwrap_or_default();
        }

        self.bulk_hydrate_tags(&mut keys).await?;

        Ok(keys)
    }

    /// Validate that the requested permissions don't exceed the user's permissions
    pub fn validate_permissions(
        key_permissions: UserOrgPermissions,
        user_permissions: UserOrgPermissions,
    ) -> Result<(), String> {
        if key_permissions > user_permissions {
            return Err(format!(
                "API key permissions ({}) cannot exceed your permissions ({})",
                key_permissions, user_permissions
            ));
        }
        Ok(())
    }

    /// Validate that the requested network access is a subset of the user's network access
    pub fn validate_network_access(
        key_network_ids: &[Uuid],
        user_network_ids: &[Uuid],
    ) -> Result<(), String> {
        for network_id in key_network_ids {
            if !user_network_ids.contains(network_id) {
                return Err(format!("You don't have access to network {}", network_id));
            }
        }
        Ok(())
    }

    /// Get network IDs for an API key from the junction table
    pub async fn get_network_ids(&self, api_key_id: &Uuid) -> Result<Vec<Uuid>> {
        self.network_access_storage.get_for_key(api_key_id).await
    }

    /// Create a new user API key with network access
    pub async fn create_with_networks(
        &self,
        api_key: UserApiKey,
        network_ids: Vec<Uuid>,
        authentication: AuthenticatedEntity,
    ) -> Result<UserApiKey> {
        // Create the key first
        let created = self.create(api_key.clone(), authentication).await?;

        // Then save network access
        if !network_ids.is_empty() {
            self.network_access_storage
                .save_for_key(&created.id, &network_ids)
                .await?;
        }

        // Return with hydrated network_ids
        let mut result = created;
        result.base.network_ids = network_ids;
        Ok(result)
    }

    /// Update network access for an existing key
    pub async fn update_network_access(
        &self,
        api_key_id: &Uuid,
        network_ids: &[Uuid],
    ) -> Result<()> {
        self.network_access_storage
            .save_for_key(api_key_id, network_ids)
            .await
    }
}

impl ApiKeyService for UserApiKeyService {
    type Key = UserApiKey;

    fn api_key_event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn validate_access(&self, key: &UserApiKey, entity: &AuthenticatedEntity) -> Result<()> {
        // User must own this key
        if let Some(user_id) = entity.user_id() {
            if key.base.user_id != user_id {
                return Err(anyhow!("You don't own this API key"));
            }
            Ok(())
        } else {
            Err(anyhow!("User context required to validate API key access"))
        }
    }
}
