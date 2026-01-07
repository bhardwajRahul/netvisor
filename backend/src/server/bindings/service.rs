use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::Binding,
    shared::{
        events::bus::EventBus,
        services::{
            entity_tags::EntityTagService,
            traits::{ChildCrudService, CrudService, EventBusService},
        },
        storage::generic::GenericPostgresStorage,
    },
};

pub struct BindingService {
    storage: Arc<GenericPostgresStorage<Binding>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<Binding> for BindingService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Binding) -> Option<Uuid> {
        Some(entity.network_id())
    }

    fn get_organization_id(&self, _entity: &Binding) -> Option<Uuid> {
        None
    }
}

impl CrudService<Binding> for BindingService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Binding>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl ChildCrudService<Binding> for BindingService {}

impl BindingService {
    pub fn new(storage: Arc<GenericPostgresStorage<Binding>>, event_bus: Arc<EventBus>) -> Self {
        Self { storage, event_bus }
    }
}
