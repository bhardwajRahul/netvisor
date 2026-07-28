use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        events::{bus::EventBus, types::EntityOperation},
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Storable, Storage},
        },
        types::entities::EntitySource,
    },
    subnets::r#impl::base::Subnet,
    tags::entity_tags::EntityTagService,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

pub struct SubnetService {
    storage: Arc<GenericPostgresStorage<Subnet>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<Subnet> for SubnetService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Subnet) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Subnet) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Subnet> for SubnetService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Subnet>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn create(
        &self,
        subnet: Subnet,
        authentication: AuthenticatedEntity,
    ) -> Result<Subnet, anyhow::Error> {
        // SCD2: natural-key match (CIDR + virtualization) runs against live
        // subnets only; closed historical copies must not match.
        let filter =
            StorableFilter::<Subnet>::new_from_network_ids(&[subnet.base.network_id]).live();
        let all_subnets = self.storage.get_all(filter).await?;

        let subnet = if subnet.id == Uuid::nil() {
            Subnet::new(subnet.base)
        } else {
            subnet
        };

        let subnet_from_storage = match all_subnets.iter().find(|existing_subnet| {
            // CIDR must match first
            if !subnet.eq(existing_subnet) {
                return false;
            }

            // Docker will default to the same subnet range for bridge networks, so we need a way
            // to distinguish docker bridge subnets with the same CIDR but which originate from
            // different hosts. The dedup uses subnet virtualization (which carries service_id
            // for Docker bridges); discovery metadata used to live on EntitySource but moved to
            // FK columns post-terminal.
            match (&existing_subnet.base.source, &subnet.base.source) {
                (EntitySource::Discovery, EntitySource::Discovery) => {
                    // Container-runtime bridge networks (Docker/Podman) are
                    // host-scoped: the same CIDR on different daemons is a
                    // distinct subnet, so they only dedupe when the owning
                    // runtime service matches. Distinct runtimes carry
                    // distinct service ids, so a Docker and a Podman bridge
                    // with the same CIDR never collide.
                    if subnet.base.subnet_type.is_container_bridge()
                        && existing_subnet.base.subnet_type.is_container_bridge()
                    {
                        match (
                            &subnet.base.virtualization,
                            &existing_subnet.base.virtualization,
                        ) {
                            (Some(a), Some(b)) => a.service_id() == b.service_id(),
                            _ => true,
                        }
                    } else {
                        true
                    }
                }
                (EntitySource::System, _) | (_, EntitySource::System) => false,
                _ => true,
            }
        }) {
            Some(existing_subnet) => {
                tracing::info!(
                    existing_subnet_id = %existing_subnet.id,
                    existing_subnet_name = %existing_subnet.base.name,
                    new_subnet_id = %subnet.id,
                    new_subnet_name = %subnet.base.name,
                    subnet_cidr = %subnet.base.cidr,
                    "Duplicate subnet found, refreshing last_seen_at and returning existing"
                );
                // SCD2 semantics: every successful natural-key match advances
                // last_seen_at, even when no field changes. Otherwise
                // unchanged subnets falsely look stale to (future) staleness
                // consumers. The incoming `subnet` was pre-stamped by
                // `HostService::discover_host` when called via discovery (see
                // `ScanContext`) so all entities in one submission share one
                // timestamp; for non-discovery callers the value is whatever
                // they put on the entity.
                let mut refreshed = existing_subnet.clone();
                refreshed.last_seen_at = subnet.last_seen_at;

                // Repair rows left behind by the interface-name heuristic that
                // used to be able to type a subnet as a container bridge (#663).
                if subnet.corrects_container_bridge_guess(existing_subnet) {
                    tracing::info!(
                        subnet_id = %existing_subnet.id,
                        subnet_cidr = %existing_subnet.base.cidr,
                        from = ?existing_subnet.base.subnet_type,
                        to = ?subnet.base.subnet_type,
                        "Reclassifying subnet mistyped as a container bridge"
                    );
                    refreshed.base.subnet_type = subnet.base.subnet_type;
                }

                self.storage.update(&mut refreshed).await?;
                refreshed
            }
            // If there's no existing subnet, create a new one
            None => {
                // SCD2 origin: this row is being inserted for the first
                // time. Stamp created_at + valid_from to the entity's
                // already-refreshed `last_seen_at`. See
                // `DiscoveryTracked::originate_scan_timestamps`.
                use crate::server::shared::storage::snapshot::DiscoveryTracked;
                let mut subnet = subnet;
                subnet.originate_scan_timestamps(subnet.last_seen_at);
                let mut created = self.storage.create(&subnet).await?;

                // Save tags to junction table
                if let Some(tag_service) = self.entity_tag_service()
                    && let Some(org_id) = authentication.organization_id()
                {
                    tag_service
                        .set_tags(
                            created.id,
                            EntityDiscriminants::Subnet,
                            created.base.tags.clone(),
                            org_id,
                        )
                        .await?;
                    created.base.tags = subnet.base.tags.clone();
                }

                let trigger_stale = created.triggers_staleness(None);

                if let Some(scope) = EntityScope::from_ids(
                    created.id,
                    created.clone().into(),
                    self.get_network_id(&created),
                    self.get_organization_id(&created),
                ) {
                    self.event_bus()
                        .publish(
                            Event::new(scope, EntityOperation::Created, authentication).with_flags(
                                EntityEventFlags {
                                    trigger_stale,
                                    ..Default::default()
                                },
                            ),
                        )
                        .await?;
                }

                subnet
            }
        };
        Ok(subnet_from_storage)
    }
}

impl SubnetService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Subnet>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            entity_tag_service,
        }
    }

    /// Update container-runtime bridge subnets (Docker/Podman) that reference an
    /// old service_id to use the new one. Called after host upsert remaps the
    /// runtime service IDs during create_with_children.
    pub async fn patch_container_bridge_virtualization(
        &self,
        network_id: &Uuid,
        old_service_id: &Uuid,
        new_service_id: &Uuid,
    ) -> Result<()> {
        // SCD2: only patch live subnets; closed historical copies retain
        // their as-of state.
        let filter = StorableFilter::<Subnet>::new_from_network_ids(&[*network_id]).live();
        let subnets = self.storage.get_all(filter).await?;

        for mut subnet in subnets {
            if let Some(virt) = subnet.base.virtualization.as_mut()
                && virt.service_id() == Some(*old_service_id)
            {
                tracing::debug!(
                    subnet_id = %subnet.id,
                    subnet_cidr = %subnet.base.cidr,
                    old_service_id = %old_service_id,
                    new_service_id = %new_service_id,
                    "Patching bridge subnet virtualization service_id"
                );
                virt.set_service_id(*new_service_id);
                self.storage.update(&mut subnet).await?;
            }
        }
        Ok(())
    }
}
