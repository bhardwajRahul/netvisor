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
    ///
    /// `scan_ctx` carries the per-submission `scan_time` (see `ScanContext`)
    /// and is used to stamp `valid_from` / `created_at` on freshly inserted
    /// rows so all VLANs in the same daemon submission share a consistent
    /// SCD2 origination timestamp. When `None`, defaults to `Utc::now()` for
    /// each new row (legacy callers + tests).
    pub async fn upsert_from_discovery(
        &self,
        network_id: Uuid,
        organization_id: Uuid,
        vlan_number: u16,
        name: String,
        scan_ctx: Option<&crate::server::shared::services::scan_context::ScanContext>,
    ) -> Result<Vlan> {
        use crate::server::shared::storage::snapshot::DiscoveryTracked;

        // SCD2: natural-key match (network_id + vlan_number) against live rows.
        let filter = StorableFilter::<Vlan>::new_from_uuid_column("network_id", &network_id)
            .u16_column("vlan_number", vlan_number)
            .live();

        if let Some(existing) = self.storage.get_one(filter).await? {
            if existing.base.name != name {
                let mut updated = existing.clone();
                updated.base.name = name;
                updated.updated_at = chrono::Utc::now();
                if let Some(ctx) = scan_ctx {
                    updated.refresh_scan_timestamps(ctx.scan_time);
                }
                self.storage.update(&mut updated).await?;
                return Ok(updated);
            }
            return Ok(existing);
        }

        // Create new VLAN. Stamp originating SCD2 timestamps from scan_ctx so
        // every VLAN in this submission shares a consistent valid_from.
        let mut vlan = <Vlan as Storable>::new(crate::server::vlans::r#impl::base::VlanBase {
            vlan_number,
            name,
            description: None,
            network_id,
            organization_id,
            source: EntitySource::Discovery,
        });
        if let Some(ctx) = scan_ctx {
            vlan.originate_scan_timestamps(ctx.scan_time);
        }
        self.storage.create(&vlan).await?;
        Ok(vlan)
    }
}
