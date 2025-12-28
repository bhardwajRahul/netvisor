use uuid::Uuid;

use crate::server::{
    interfaces::r#impl::base::Interface,
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{
            filter::EntityFilter,
            generic::GenericPostgresStorage,
            traits::Storage,
        },
    },
};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub struct InterfaceService {
    storage: Arc<GenericPostgresStorage<Interface>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<Interface> for InterfaceService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Interface) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &Interface) -> Option<Uuid> {
        None
    }
}

impl CrudService<Interface> for InterfaceService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Interface>> {
        &self.storage
    }
}

impl ChildCrudService<Interface> for InterfaceService {}

impl InterfaceService {
    pub fn new(storage: Arc<GenericPostgresStorage<Interface>>, event_bus: Arc<EventBus>) -> Self {
        Self { storage, event_bus }
    }

    /// Get all interfaces for a specific host, ordered by position
    pub async fn get_for_host(&self, host_id: &Uuid) -> Result<Vec<Interface>> {
        let filter = EntityFilter::unfiltered().uuid_column("host_id", host_id);
        self.storage.get_all_ordered(filter, "position ASC").await
    }

    /// Get interfaces for multiple hosts, ordered by position within each host
    pub async fn get_for_hosts(&self, host_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<Interface>>> {
        if host_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = EntityFilter::unfiltered().uuid_columns("host_id", host_ids);
        let interfaces = self.storage.get_all_ordered(filter, "position ASC").await?;

        let mut result: HashMap<Uuid, Vec<Interface>> = HashMap::new();
        for interface in interfaces {
            result.entry(interface.base.host_id).or_default().push(interface);
        }

        Ok(result)
    }

    /// Get all interfaces for a specific subnet
    pub async fn get_for_subnet(&self, subnet_id: &Uuid) -> Result<Vec<Interface>> {
        let filter = EntityFilter::unfiltered().subnet_id(subnet_id);
        self.storage.get_all(filter).await
    }
}
