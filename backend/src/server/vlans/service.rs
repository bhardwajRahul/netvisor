use crate::server::{
    shared::{
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Storable, Storage},
        },
        types::entities::EntitySource,
    },
    vlans::r#impl::{base::Vlan, subnet_vlans::SubnetVlanStorage},
};
use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

pub struct VlanService {
    storage: Arc<GenericPostgresStorage<Vlan>>,
    event_bus: Arc<EventBus>,
    pub subnet_vlan_storage: Arc<SubnetVlanStorage>,
}

impl EventBusService<Vlan> for VlanService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Vlan) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, entity: &Vlan) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

impl CrudService<Vlan> for VlanService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Vlan>> {
        &self.storage
    }

    fn entity_tag_service(
        &self,
    ) -> Option<&Arc<crate::server::tags::entity_tags::EntityTagService>> {
        None
    }
}

impl VlanService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Vlan>>,
        event_bus: Arc<EventBus>,
        subnet_vlan_storage: Arc<SubnetVlanStorage>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            subnet_vlan_storage,
        }
    }

    /// Upsert a VLAN from discovery. Creates if new, updates name if changed.
    /// Returns the VLAN (existing or newly created).
    pub async fn upsert_from_discovery(
        &self,
        network_id: Uuid,
        organization_id: Uuid,
        vlan_number: u16,
        name: String,
    ) -> Result<Vlan> {
        let filter = StorableFilter::<Vlan>::new_from_uuid_column("network_id", &network_id)
            .u16_column("vlan_number", vlan_number);

        if let Some(existing) = self.storage.get_one(filter).await? {
            if existing.base.name != name {
                let mut updated = existing.clone();
                updated.base.name = name;
                updated.updated_at = chrono::Utc::now();
                self.storage.update(&mut updated).await?;
                return Ok(updated);
            }
            return Ok(existing);
        }

        // Create new VLAN
        let vlan = <Vlan as Storable>::new(crate::server::vlans::r#impl::base::VlanBase {
            vlan_number,
            name,
            description: None,
            network_id,
            organization_id,
            source: EntitySource::Discovery,
        });
        self.storage.create(&vlan).await?;
        Ok(vlan)
    }
}
