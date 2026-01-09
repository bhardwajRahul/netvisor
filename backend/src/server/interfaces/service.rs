use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    interfaces::r#impl::base::Interface,
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{filter::EntityFilter, generic::GenericPostgresStorage, traits::Storage},
        types::api::ValidationError,
    },
    tags::entity_tags::EntityTagService,
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

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
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
            result
                .entry(interface.base.host_id)
                .or_default()
                .push(interface);
        }

        Ok(result)
    }

    /// Get all interfaces for a specific subnet
    pub async fn get_for_subnet(&self, subnet_id: &Uuid) -> Result<Vec<Interface>> {
        let filter = EntityFilter::unfiltered().subnet_id(subnet_id);
        self.storage.get_all(filter).await
    }

    // =========================================================================
    // Position management helpers (for direct API operations)
    // =========================================================================

    /// Get the next available position for a new interface on a host.
    /// Returns the count of existing interfaces (which becomes the next position).
    pub async fn get_next_position_for_host(&self, host_id: &Uuid) -> Result<i32> {
        let existing = self.get_for_host(host_id).await?;
        Ok(existing.len() as i32)
    }

    /// Validate that a position is valid for an interface update.
    /// Position must be within range [0, count-1] and not conflict with other interfaces.
    pub async fn validate_position_for_update(
        &self,
        interface_id: &Uuid,
        host_id: &Uuid,
        new_position: i32,
    ) -> Result<()> {
        let all_interfaces = self.get_for_host(host_id).await?;
        let max_position = (all_interfaces.len() as i32) - 1;

        if new_position < 0 || new_position > max_position {
            return Err(ValidationError::new(format!(
                "Interface position {} is out of range. Valid positions are 0 to {}.",
                new_position, max_position
            ))
            .into());
        }

        // Check for duplicate position (excluding self)
        if all_interfaces
            .iter()
            .any(|i| i.id != *interface_id && i.base.position == new_position)
        {
            return Err(ValidationError::new(format!(
                "Interface position {} is already used by another interface.",
                new_position
            ))
            .into());
        }

        Ok(())
    }

    /// Renumber all interfaces for a host to ensure sequential positions (0, 1, 2, ...).
    /// Called after deleting interface(s) to close gaps.
    pub async fn renumber_interfaces_for_host(
        &self,
        host_id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        let mut interfaces = self.get_for_host(host_id).await?;

        // Interfaces are already ordered by position, so just assign sequential positions
        let mut needs_update = false;
        for (i, iface) in interfaces.iter_mut().enumerate() {
            let expected_position = i as i32;
            if iface.base.position != expected_position {
                needs_update = true;
                iface.base.position = expected_position;
            }
        }

        // Only update if there are gaps to close
        if needs_update {
            for iface in &mut interfaces {
                self.update(iface, authentication.clone()).await?;
            }
        }

        Ok(())
    }
}
