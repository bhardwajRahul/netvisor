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
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        inference::{infer_range_for, placeable_subnet},
        types::{SubnetCidrSource, SubnetType},
    },
    tags::entity_tags::EntityTagService,
};
use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::Utc;
use cidr::IpCidr;
use std::net::IpAddr;
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

    /// Counts only the subnets the user curates, so the dashboard's per-network
    /// subnet count agrees with the rows the management lists show.
    ///
    /// The default counts every live row (`shared/services/traits.rs`), which is
    /// how the reporter's dashboard total came to include a subnet no page would
    /// display (GH #677).
    async fn count_for_networks(&self, network_ids: &[Uuid]) -> Result<u64, anyhow::Error> {
        let filter = StorableFilter::<Subnet>::new_from_network_ids(network_ids)
            .live()
            .user_managed();
        self.storage().count(filter).await
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
                            subnet.base.virtualization_service_id,
                            existing_subnet.base.virtualization_service_id,
                        ) {
                            (Some(a), Some(b)) => a == b,
                            // An owner-less bridge row predates this scoping, or had its owner
                            // quarantined as dangling by the migration. Either way it merges on
                            // CIDR alone, which is what collapses the duplicates it accumulated.
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

/// Where an address belongs, and how sure we are of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// A subnet this network already holds contains it.
    Existing(Uuid),
    /// Nothing held it, so a range was inferred and created. The row carries
    /// [`SubnetCidrSource::Inferred`], so it badges and asks an operator to confirm it.
    Inferred(Uuid),
    /// Nothing holds it and nothing may be invented for it — a public address, or IPv6 global
    /// unicast, neither of which is a segment of this network to create. The caller decides.
    Unplaceable,
}

impl SubnetService {
    /// Place an address, inferring a range for it when nothing this network holds contains it.
    ///
    /// The single automatic-placement entry point. Every source of a discovered address goes
    /// through it — an LLDP far end, a controller-reported device, a repaired daemon payload — so
    /// they cannot disagree about which subnet an address belongs to, and so that "nothing holds
    /// this" is answered the same way once rather than per integration.
    ///
    /// Deliberately not the placement rule for a *container* endpoint: the runtime API already says
    /// which network the endpoint is on, and re-deriving that by address would throw away an
    /// identity for a guess — see `get_container_interfaces`.
    pub async fn place_address(&self, network_id: Uuid, ip: IpAddr) -> Result<Placement, Error> {
        let live = self
            .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
            .await?;

        if let Some(subnet) = placeable_subnet(&live, ip) {
            return Ok(Placement::Existing(subnet.id));
        }

        let live_cidrs: Vec<IpCidr> = live.iter().map(|s| s.base.cidr).collect();
        let Some(cidr) = infer_range_for(ip, &live_cidrs) else {
            return Ok(Placement::Unplaceable);
        };

        let mut subnet = Subnet::new(SubnetBase {
            cidr,
            // A range nothing read, only inferred — the whole reason this rung exists.
            cidr_source: SubnetCidrSource::Inferred,
            network_id,
            name: cidr.to_string(),
            description: None,
            // Not `Management` even where a management address produced it: on a flat network that
            // address is just the device's LAN address, and typing the subnet would be a second
            // guess stacked on the first.
            subnet_type: SubnetType::Unknown,
            virtualization_service_id: None,
            source: EntitySource::Discovery,
            tags: Vec::new(),
        });
        subnet.last_seen_at = Utc::now();

        let created = self.create(subnet, AuthenticatedEntity::System).await?;
        tracing::info!(
            network_id = %network_id,
            ip = %ip,
            cidr = %cidr,
            subnet_id = %created.id,
            "Inferred a subnet for an address nothing on this network holds"
        );
        Ok(Placement::Inferred(created.id))
    }

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
}
