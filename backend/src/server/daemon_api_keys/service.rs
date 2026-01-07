use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    daemon_api_keys::r#impl::base::DaemonApiKey,
    shared::{
        api_key_common::ApiKeyService,
        events::bus::EventBus,
        services::{
            entity_tags::EntityTagService,
            traits::{CrudService, EventBusService},
        },
        storage::generic::GenericPostgresStorage,
    },
};

pub struct DaemonApiKeyService {
    storage: Arc<GenericPostgresStorage<DaemonApiKey>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<DaemonApiKey> for DaemonApiKeyService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &DaemonApiKey) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &DaemonApiKey) -> Option<Uuid> {
        None
    }

    fn suppress_logs(
        &self,
        current: Option<&DaemonApiKey>,
        updated: Option<&DaemonApiKey>,
    ) -> bool {
        match (current, updated) {
            (Some(current), Some(updated)) => updated.suppress_logs(current),
            _ => false,
        }
    }
}

#[async_trait]
impl CrudService<DaemonApiKey> for DaemonApiKeyService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<DaemonApiKey>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }
}

impl DaemonApiKeyService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<DaemonApiKey>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            entity_tag_service,
        }
    }
}

impl ApiKeyService for DaemonApiKeyService {
    type Key = DaemonApiKey;

    fn api_key_event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn validate_access(&self, key: &DaemonApiKey, entity: &AuthenticatedEntity) -> Result<()> {
        // User must have access to the network this key belongs to
        if !entity.network_ids().contains(&key.base.network_id) {
            return Err(anyhow!(
                "You don't have access to the network for this daemon API key"
            ));
        }
        Ok(())
    }
}
