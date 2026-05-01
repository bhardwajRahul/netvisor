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
        let filter = StorableFilter::<Subnet>::new_from_network_ids(&[subnet.base.network_id]);
        let all_subnets = self.storage.get_all(filter).await?;

        let subnet = if subnet.id == Uuid::nil() {
            Subnet::new(subnet.base)
        } else {
            subnet
        };

        tracing::debug!(
            subnet_id = %subnet.id,
            subnet_name = %subnet.base.name,
            subnet_cidr = %subnet.base.cidr,
            network_id = %subnet.base.network_id,
            "Creating subnet"
        );

        // Validate discovery metadata upfront — a discovered subnet must have at least one metadata entry
        if let EntitySource::Discovery { metadata } = &subnet.base.source
            && metadata.is_empty()
        {
            return Err(anyhow::anyhow!(
                "Error comparing discovered subnets during creation: subnet missing discovery metadata"
            ));
        }

        let subnet_from_storage = match all_subnets.iter().find(|existing_subnet| {
            // CIDR must match first
            if !subnet.eq(existing_subnet) {
                return false;
            }

            // Docker will default to the same subnet range for bridge networks, so we need a way
            // to distinguish docker bridge subnets with the same CIDR but which originate from
            // different hosts. This returns true for docker bridge subnets created from the same
            // host (same service_id), and true for all other sources provided CIDRs match.
            match (&existing_subnet.base.source, &subnet.base.source) {
                (
                    EntitySource::Discovery {
                        metadata: existing_metadata,
                    },
                    EntitySource::Discovery { .. },
                ) => {
                    existing_metadata.iter().any(|_other_m| {
                        use crate::server::subnets::r#impl::virtualization::SubnetVirtualization;

                        // Docker bridge subnets need per-service dedup: same CIDR on
                        // different Docker daemons are distinct subnets.
                        if subnet.base.subnet_type.is_docker_bridge()
                            && existing_subnet.base.subnet_type.is_docker_bridge()
                        {
                            match (
                                &subnet.base.virtualization,
                                &existing_subnet.base.virtualization,
                            ) {
                                (
                                    Some(SubnetVirtualization::Docker(a)),
                                    Some(SubnetVirtualization::Docker(b)),
                                ) => a.service_id == b.service_id,
                                // One or both missing virtualization — treat as same
                                _ => true,
                            }
                        } else {
                            // Non-DockerBridge: always deduplicate by CIDR
                            true
                        }
                    })
                }
                // System subnets are never going to be upserted to or from
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
                    "Duplicate subnet found, returning existing"
                );
                existing_subnet.clone()
            }
            // If there's no existing subnet, create a new one
            None => {
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

    /// Update DockerBridge subnets that reference an old service_id to use the new one.
    /// Called after host upsert remaps Docker service IDs during create_with_children.
    pub async fn patch_docker_bridge_virtualization(
        &self,
        network_id: &Uuid,
        old_service_id: &Uuid,
        new_service_id: &Uuid,
    ) -> Result<()> {
        use crate::server::subnets::r#impl::virtualization::SubnetVirtualization;

        let filter = StorableFilter::<Subnet>::new_from_network_ids(&[*network_id]);
        let subnets = self.storage.get_all(filter).await?;

        for mut subnet in subnets {
            if let Some(SubnetVirtualization::Docker(ref mut d)) = subnet.base.virtualization
                && d.service_id == *old_service_id
            {
                tracing::debug!(
                    subnet_id = %subnet.id,
                    subnet_cidr = %subnet.base.cidr,
                    old_service_id = %old_service_id,
                    new_service_id = %new_service_id,
                    "Patching bridge subnet virtualization service_id"
                );
                d.service_id = *new_service_id;
                self.storage.update(&mut subnet).await?;
            }
        }
        Ok(())
    }
}
