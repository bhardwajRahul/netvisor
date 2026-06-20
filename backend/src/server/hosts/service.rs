use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::r#impl::base::{Binding, BindingType},
    credentials::service::CredentialService,
    daemons::{r#impl::base::Daemon, service::DaemonService},
    hosts::r#impl::{
        api::{
            BindingInput, ConflictBehavior, CreateHostRequest, HostResponse, IPAddressInput,
            PortInput, ServiceInput, UpdateHostRequest,
        },
        base::{Host, HostBase},
    },
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    ip_addresses::{r#impl::base::IPAddress, service::IPAddressService},
    ports::{r#impl::base::Port, service::PortService},
    services::{
        r#impl::{base::Service, definitions::ServiceDefinitionExt},
        service::ServiceService,
    },
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        events::{bus::EventBus, types::EntityOperation},
        position::resolve_and_validate_input_positions,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, PaginatedResult, Storable, Storage},
        },
        types::{
            api::ValidationError,
            entities::{EntitySource, EntitySourceDiscriminants},
        },
    },
    snmp::resolution::{lldp::LldpResolver, resolver::LldpResolverImpl},
    subnets::{r#impl::base::Subnet, service::SubnetService},
    tags::entity_tags::EntityTagService,
    vlans::service::VlanService,
};
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    sync::Arc,
};
use strum::IntoDiscriminant;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct HostLimitContext {
    pub limit: u64,
    pub org_id: Uuid,
    pub org_network_ids: Vec<Uuid>,
    pub plan: crate::server::billing::types::base::BillingPlan,
}

pub struct HostService {
    storage: Arc<GenericPostgresStorage<Host>>,
    ip_address_service: Arc<IPAddressService>,
    port_service: Arc<PortService>,
    service_service: Arc<ServiceService>,
    interface_service: Arc<InterfaceService>,
    pub daemon_service: Arc<DaemonService>,
    credential_service: Arc<CredentialService>,
    subnet_service: Arc<SubnetService>,
    vlan_service: Arc<VlanService>,
    host_locks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<Host> for HostService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Host) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Host) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Host> for HostService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Host>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    /// Create a new host, or upsert if a matching host exists.
    ///
    /// This method uses `Host::eq` (ID comparison) to find existing hosts.
    /// For discovery workflows, `create_with_children` sets the incoming host's ID
    /// to match an existing host found via IP-address comparison, so this method
    /// will find the match and trigger `upsert_host()`.
    ///
    /// Upsert conditions:
    /// - Both hosts are from discovery (merges discovery metadata)
    /// - OR the IDs already match (handles re-discovery of known hosts)
    async fn create(&self, host: Host, authentication: AuthenticatedEntity) -> Result<Host> {
        let host = if host.id == Uuid::nil() {
            Host::new(host.base.clone())
        } else {
            host
        };

        let lock = self.get_host_lock(&host.id).await;
        let _guard = lock.lock().await;

        tracing::trace!("Creating host {:?}", host);

        // SCD2: only live hosts are eligible for the natural-key match.
        let filter = StorableFilter::<Host>::new_from_network_ids(&[host.base.network_id]).live();
        let all_hosts = self.get_all(filter).await?;

        // Find existing host by ID (Host::eq only compares IDs)
        // For discovery, create_with_children already set host.id to the existing host's ID
        // if an IP-address match was found, so this will find the match
        let host_from_storage = match all_hosts.into_iter().find(|h| host.eq(h)) {
            // Upsert if both are discovery sources, or if IDs match exactly
            Some(existing_host)
                if (host.base.source.discriminant() == EntitySourceDiscriminants::Discovery
                    && existing_host.base.source.discriminant()
                        == EntitySourceDiscriminants::Discovery)
                    || host.id == existing_host.id =>
            {
                if host.id != existing_host.id {
                    tracing::warn!(
                        incoming_host_id = %host.id,
                        matched_host_id = %existing_host.id,
                        matched_host_name = %existing_host.base.name,
                        "Host matched via MAC/IP address but discovery reported a different host ID. \
                         This may indicate a daemon is using a stale configuration. \
                         To fix, update the daemon's config file with: host_id = \"{}\"",
                        existing_host.id
                    );
                }

                tracing::debug!(
                    "Duplicate host for {}: {} found, {}: {} - upserting discovery data...",
                    host.base.name,
                    host.id,
                    existing_host.base.name,
                    existing_host.id
                );

                self.upsert_host(existing_host, host, authentication)
                    .await?
            }
            _ => {
                if let Some(existing_host) = self.get_by_id(&host.id).await? {
                    return Err(ValidationError::new(format!(
                        "Network mismatch: Daemon is trying to update host '{}' (id: {}) but cannot proceed. \
                        The host belongs to network {} while the daemon is assigned to network {}. \
                        To resolve this, either reassign the daemon to the correct network or delete the mismatched host.",
                        existing_host.base.name,
                        host.id,
                        existing_host.base.network_id,
                        host.base.network_id
                    )).into());
                }

                // SCD2 origin: this row is being inserted for the first
                // time. Stamp created_at + valid_from to the entity's
                // already-refreshed `last_seen_at` so all four temporal
                // columns line up at one canonical scan_time. See
                // `DiscoveryTracked::originate_scan_timestamps`.
                use crate::server::shared::storage::snapshot::DiscoveryTracked;
                let mut host = host;
                host.originate_scan_timestamps(host.last_seen_at);
                let created = self.storage().create(&host).await?;
                let trigger_stale = created.triggers_staleness(None);

                if let Some(scope) = EntityScope::from_ids(
                    created.id(),
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

                host
            }
        };

        Ok(host_from_storage)
    }

    async fn update(
        &self,
        updates: &mut Host,
        authentication: AuthenticatedEntity,
    ) -> Result<Host, Error> {
        let lock = self.get_host_lock(&updates.id).await;
        let _guard = lock.lock().await;

        let current_host = self
            .get_by_id(&updates.id)
            .await?
            .ok_or_else(|| anyhow!("Host '{}' not found", updates.id))?;

        let updated = self.storage().update(updates).await?;
        let trigger_stale = updated.triggers_staleness(Some(current_host));

        if let Some(scope) = EntityScope::from_ids(
            updated.id(),
            updated.clone().into(),
            self.get_network_id(&updated),
            self.get_organization_id(&updated),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Updated, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(updated)
    }
}

impl HostService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<GenericPostgresStorage<Host>>,
        ip_address_service: Arc<IPAddressService>,
        port_service: Arc<PortService>,
        service_service: Arc<ServiceService>,
        interface_service: Arc<InterfaceService>,
        daemon_service: Arc<DaemonService>,
        credential_service: Arc<CredentialService>,
        subnet_service: Arc<SubnetService>,
        vlan_service: Arc<VlanService>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            ip_address_service,
            port_service,
            service_service,
            interface_service,
            daemon_service,
            credential_service,
            subnet_service,
            vlan_service,
            host_locks: Arc::new(Mutex::new(HashMap::new())),
            event_bus,
            entity_tag_service,
        }
    }

    /// Get ports for a specific host
    pub async fn get_ports_for_host(&self, host_id: &Uuid) -> Result<Vec<Port>> {
        self.port_service.get_for_host(host_id).await
    }

    /// Get ip_addresses for a specific host
    pub async fn get_ip_addresses_for_host(&self, host_id: &Uuid) -> Result<Vec<IPAddress>> {
        self.ip_address_service.get_for_host(host_id).await
    }

    // =========================================================================
    // HostResponse builders (load children for API responses)
    // =========================================================================

    /// Get a single host with all children hydrated for API response
    pub async fn get_host_response(&self, id: &Uuid) -> Result<Option<HostResponse>> {
        let mut host = match self.get_by_id(id).await? {
            Some(h) => h,
            None => return Ok(None),
        };

        // Hydrate tags from junction table
        let tags = self
            .entity_tag_service
            .get_tags(id, &EntityDiscriminants::Host)
            .await?;
        host.base.tags = tags;

        let (ip_addresses, ports, services, interfaces) =
            self.load_children_for_host(&host.id).await?;
        Ok(Some(HostResponse::from_host_with_children(
            host,
            ip_addresses,
            ports,
            services,
            interfaces,
        )))
    }

    /// Get all hosts with all children hydrated for API response
    pub async fn get_all_host_responses(
        &self,
        filter: StorableFilter<Host>,
    ) -> Result<Vec<HostResponse>> {
        let hosts = self.get_all(filter).await?;
        if hosts.is_empty() {
            return Ok(vec![]);
        }

        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let (ip_addresses_map, ports_map, services_map, interfaces_map) =
            self.load_children_for_hosts(&host_ids, None).await?;

        // Hydrate tags from junction table
        let tags_map = self
            .entity_tag_service
            .get_tags_map(&host_ids, EntityDiscriminants::Host)
            .await?;

        let responses = hosts
            .into_iter()
            .map(|mut host| {
                // Apply hydrated tags
                if let Some(tags) = tags_map.get(&host.id) {
                    host.base.tags = tags.clone();
                }
                let ip_addresses = ip_addresses_map.get(&host.id).cloned().unwrap_or_default();
                let ports = ports_map.get(&host.id).cloned().unwrap_or_default();
                let services = services_map.get(&host.id).cloned().unwrap_or_default();
                let interfaces = interfaces_map.get(&host.id).cloned().unwrap_or_default();
                HostResponse::from_host_with_children(
                    host,
                    ip_addresses,
                    ports,
                    services,
                    interfaces,
                )
            })
            .collect();

        Ok(responses)
    }

    /// Get paginated hosts with all children hydrated for API response.
    /// Supports custom ordering via the `order_by` parameter.
    pub async fn get_all_host_responses_paginated(
        &self,
        filter: StorableFilter<Host>,
        order_by: &str,
        at: Option<DateTime<Utc>>,
    ) -> Result<PaginatedResult<HostResponse>> {
        let result = self.storage().get_paginated(filter, order_by).await?;

        if result.items.is_empty() {
            return Ok(PaginatedResult {
                items: vec![],
                total_count: result.total_count,
            });
        }

        let host_ids: Vec<Uuid> = result.items.iter().map(|h| h.id).collect();
        // Hydrate children as-of the same instant as the host rows so a snapshot
        // view shows a coherent point-in-time host + children bundle.
        let (ip_addresses_map, ports_map, services_map, interfaces_map) =
            self.load_children_for_hosts(&host_ids, at).await?;

        // Hydrate tags from junction table
        let tags_map = self
            .entity_tag_service
            .get_tags_map(&host_ids, EntityDiscriminants::Host)
            .await?;

        let responses = result
            .items
            .into_iter()
            .map(|mut host| {
                // Apply hydrated tags
                if let Some(tags) = tags_map.get(&host.id) {
                    host.base.tags = tags.clone();
                }
                let ip_addresses = ip_addresses_map.get(&host.id).cloned().unwrap_or_default();
                let ports = ports_map.get(&host.id).cloned().unwrap_or_default();
                let services = services_map.get(&host.id).cloned().unwrap_or_default();
                let interfaces = interfaces_map.get(&host.id).cloned().unwrap_or_default();
                HostResponse::from_host_with_children(
                    host,
                    ip_addresses,
                    ports,
                    services,
                    interfaces,
                )
            })
            .collect();

        Ok(PaginatedResult {
            items: responses,
            total_count: result.total_count,
        })
    }

    /// Load all children for a single host.
    async fn load_children_for_host(
        &self,
        host_id: &Uuid,
    ) -> Result<(Vec<IPAddress>, Vec<Port>, Vec<Service>, Vec<Interface>)> {
        let ip_addresses = self.ip_address_service.get_for_host(host_id).await?;
        let ports = self.port_service.get_for_host(host_id).await?;
        // SCD2: live services only.
        let services = self
            .service_service
            .get_all_ordered(
                StorableFilter::<Service>::new_from_host_ids(&[*host_id]).live(),
                "position ASC",
            )
            .await?;
        let interfaces = self.interface_service.get_for_host(host_id).await?;

        Ok((ip_addresses, ports, services, interfaces))
    }

    /// Batch load all children for multiple hosts.
    /// `at = None` loads live children; `Some(t)` loads SCD2 state as of `t`.
    async fn load_children_for_hosts(
        &self,
        host_ids: &[Uuid],
        at: Option<DateTime<Utc>>,
    ) -> Result<(
        HashMap<Uuid, Vec<IPAddress>>,
        HashMap<Uuid, Vec<Port>>,
        HashMap<Uuid, Vec<Service>>,
        HashMap<Uuid, Vec<Interface>>,
    )> {
        let ip_addresses_map = self.ip_address_service.get_for_hosts(host_ids, at).await?;
        let ports_map = self.port_service.get_for_hosts(host_ids, at).await?;

        // Load services ordered by position and group by host_id.
        let services = self
            .service_service
            .get_all_ordered(
                StorableFilter::<Service>::new_from_host_ids(host_ids).live_or_as_of(at),
                "position ASC",
            )
            .await?;

        let mut services_map: HashMap<Uuid, Vec<Service>> = HashMap::new();
        for service in services {
            services_map
                .entry(service.base.host_id)
                .or_default()
                .push(service);
        }

        // Load interfaces and group by host_id
        let mut interfaces_map = self.interface_service.get_for_hosts(host_ids, at).await?;
        // Sort each host's entries by if_index
        for entries in interfaces_map.values_mut() {
            entries.sort_by_key(|e| e.base.if_index);
        }

        Ok((ip_addresses_map, ports_map, services_map, interfaces_map))
    }

    // =========================================================================
    // Host creation with children
    // =========================================================================

    /// Create a host with all its children (ip_addresses, ports, services, interfaces) from API request.
    /// Client provides UUIDs for all entities, enabling services to reference ip_addresses/ports.
    /// For API users: errors if a host with matching ip_addresses already exists.
    pub async fn create_from_request(
        &self,
        request: CreateHostRequest,
        authentication: AuthenticatedEntity,
    ) -> Result<HostResponse> {
        // Destructure request to ensure compile error if fields change
        let CreateHostRequest {
            name,
            network_id,
            hostname,
            description,
            virtualization,
            hidden,
            tags,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            credential_assignments,
            ip_addresses: ip_address_inputs,
            ports: port_inputs,
            services: service_inputs,
            interfaces: interface_inputs,
        } = request;

        // Resolve and validate positions (no existing entities for create)
        let empty_ip_addresses: Vec<IPAddress> = vec![];
        let empty_services: Vec<Service> = vec![];
        let mut ip_address_inputs = ip_address_inputs;
        let mut service_inputs = service_inputs;
        resolve_and_validate_input_positions(
            &mut ip_address_inputs,
            &empty_ip_addresses,
            "ip_address",
        )
        .map_err(|e| ValidationError::new(e.message))?;
        resolve_and_validate_input_positions(&mut service_inputs, &empty_services, "service")
            .map_err(|e| ValidationError::new(e.message))?;

        // Auto-set source to Manual for API-created entities
        let source = EntitySource::Manual;

        // Create host base with SNMP fields
        let host_base = HostBase {
            name: name.clone(),
            network_id,
            hostname,
            description,
            source: source.clone(),
            virtualization,
            hidden,
            tags,
            sys_descr,
            sys_object_id,
            sys_location,
            sys_contact,
            management_url,
            chassis_id,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            credential_assignments,
        };
        let host = Host::new(host_base);

        // Build ip_addresses with client-provided IDs
        let ip_addresses: Vec<IPAddress> = ip_address_inputs
            .into_iter()
            .map(|input| input.into_ip_address(host.id, network_id))
            .collect();

        // Build ports with client-provided IDs
        let ports: Vec<Port> = port_inputs
            .into_iter()
            .map(|input| input.into_port(host.id, network_id))
            .collect();

        // Build services with client-provided IDs
        let services: Vec<Service> = service_inputs
            .into_iter()
            .map(|input| input.into_service(host.id, network_id, source.clone()))
            .collect();

        // Build interfaces (server assigns UUIDs)
        let interfaces: Vec<Interface> = interface_inputs
            .into_iter()
            .map(|input| input.into_interface(host.id, network_id))
            .collect();

        // Use unified creation with Error behavior for API users
        self.create_with_children(
            host,
            ip_addresses,
            ports,
            services,
            interfaces,
            vec![], // No integration-derived subnets for API creates
            ConflictBehavior::Error,
            authentication,
            None, // limit checked in handler
        )
        .await
    }

    /// Create a host with all children, handling conflicts according to behavior.
    /// This is the unified internal method used by both API and discovery paths.
    ///
    /// ## Host Deduplication Flow
    ///
    /// Host deduplication happens in two stages:
    ///
    /// 1. **IP-address-based matching** (this method): `find_matching_host_by_ip_addresses` compares
    ///    incoming IP addresses against existing hosts using MAC address or subnet+IP matching.
    ///    - For API users (ConflictBehavior::Error): Returns an error telling them to edit the existing host.
    ///    - For discovery (ConflictBehavior::Upsert): Sets `host.id = existing_host.id` so the
    ///      subsequent create() call will recognize this as an existing host.
    ///
    /// 2. **ID-based matching** (in `create()`): Uses `Host::eq` which only compares IDs.
    ///    Since we set `host.id = existing_host.id` in step 1, the create() method will find
    ///    a match and call `upsert_host()` to merge discovery data.
    ///
    /// This two-stage approach means:
    /// - IP-address matching handles the "is this the same physical host?" question
    /// - ID matching handles the "should we upsert?" question (relies on ID being set correctly)
    /// - Discovery always upserts when IP addresses match, even if daemon reported a different host ID
    #[allow(clippy::too_many_arguments)]
    async fn create_with_children(
        &self,
        mut host: Host,
        ip_addresses: Vec<IPAddress>,
        ports: Vec<Port>,
        services: Vec<Service>,
        interfaces: Vec<Interface>,
        subnets: Vec<Subnet>,
        conflict_behavior: ConflictBehavior,
        authentication: AuthenticatedEntity,
        limit_ctx: Option<&HostLimitContext>,
    ) -> Result<HostResponse> {
        // Stage 1: IP-address-based collision detection
        // Compares MAC addresses and subnet+IP to find hosts that represent the same physical machine
        let matching_result = self
            .find_matching_host_by_ip_addresses(&host.base.network_id, &ip_addresses)
            .await?;

        let is_new_host = matching_result.is_none();

        if let Some((existing_host, _)) = matching_result {
            match conflict_behavior {
                ConflictBehavior::Error => {
                    // API users should edit the existing host rather than create a duplicate
                    return Err(ValidationError::new(format!(
                        "A host with matching IP addresses already exists: '{}' (id: {}). \
                         Edit the existing host instead of creating a new one.",
                        existing_host.base.name, existing_host.id
                    ))
                    .into());
                }
                ConflictBehavior::Upsert => {
                    // For discovery: align the incoming host ID with the existing host
                    // This ensures create() will match via Host::eq (which compares IDs)
                    // and trigger upsert_host() to merge discovery metadata
                    if host.id != existing_host.id {
                        tracing::debug!(
                            incoming_host_id = %host.id,
                            matched_host_id = %existing_host.id,
                            matched_host_name = %existing_host.base.name,
                            "Setting host ID to match existing host found via IP-address comparison"
                        );
                        host.id = existing_host.id;
                    }
                }
            }
        }

        // Check host limit for new hosts (not upserts)
        if is_new_host && let Some(ctx) = limit_ctx {
            // SCD2: limit applies to live hosts only; closed historical copies
            // don't count toward plan limits.
            let filter = StorableFilter::<Host>::new_from_network_ids(&ctx.org_network_ids).live();
            let current_hosts = self.get_all(filter).await?.len() as u64;
            if current_hosts >= ctx.limit {
                return Err(anyhow!(
                    "Host limit reached ({}/{}). Upgrade your plan for unlimited hosts.",
                    current_hosts,
                    ctx.limit
                ));
            }
        }

        // Store original entities for binding reassignment (discovery case)
        // These are needed because interface/port IDs may change during creation,
        // and service bindings need to be remapped to the new IDs
        let original_host = host.clone();
        let original_ip_addresses = ip_addresses.clone();
        let original_ports = ports.clone();

        // Stage 2: Create or upsert host via ID matching
        // If host.id was set to an existing host's ID above, this will trigger upsert_host()
        let mut created_host = self.create(host, authentication.clone()).await?;

        // Capture daemon interface ID → IP mapping before ip_addresses are consumed.
        // Used later to remap credential assignment ip_address_ids to server-assigned IDs.
        let daemon_ip_address_ips: Vec<(Uuid, IpAddr)> = ip_addresses
            .iter()
            .map(|i| (i.id, i.base.ip_address))
            .collect();

        // Order: ports → subnets → ip_addresses → services
        // Subnets before ip_addresses (FK: ip_addresses.subnet_id → subnets.id).
        // Interfaces before services (binding validation queries host ip_addresses).
        // Subnet virtualization.service_id needs the real (deduped) service ID, but
        // services aren't created yet. Solved by pre-computing service_id_remap from
        // existing_services_for_match before creating subnets.

        // Create ports with correct host_id
        // For Upsert: deduplicate by checking existing ports first
        // For Error: just create (will fail on duplicate constraint)
        let mut created_ports = Vec::new();
        for port in ports {
            let port_with_host = port.with_host(created_host.id, created_host.base.network_id);

            if matches!(conflict_behavior, ConflictBehavior::Upsert) {
                // Check if port already exists by ID
                if let Some(existing_port) = self.port_service.get_by_id(&port_with_host.id).await?
                {
                    created_ports.push(existing_port);
                    continue;
                }

                // Check by unique constraint (host_id, port_number, protocol)
                let existing_ports = self.port_service.get_for_host(&created_host.id).await?;
                let port_config = port_with_host.base.port_type.config();
                if let Some(existing_port) = existing_ports.into_iter().find(|p| {
                    let existing_config = p.base.port_type.config();
                    existing_config.number == port_config.number
                        && existing_config.protocol == port_config.protocol
                }) {
                    created_ports.push(existing_port);
                    continue;
                }
            }

            // SCD2 origin: dedup didn't match, so this row is being
            // inserted. Stamp created_at + valid_from to the entity's
            // already-refreshed `last_seen_at` so all four temporal
            // columns line up at one canonical scan_time.
            use crate::server::shared::storage::snapshot::DiscoveryTracked;
            let mut port_with_host = port_with_host;
            port_with_host.originate_scan_timestamps(port_with_host.last_seen_at);
            let created = self
                .port_service
                .create(port_with_host, authentication.clone())
                .await?;
            created_ports.push(created);
        }

        // Order: subnets → ip_addresses → services
        // Subnets before ip_addresses (FK constraint: ip_addresses.subnet_id → subnets.id).
        // Interfaces before services (binding validation queries host ip_addresses from DB).
        // Subnets need the real service_id for dedup, but services haven't been created yet.
        // Solution: pre-compute service_id_remap by matching incoming services against
        // existing ones, then apply it to subnet virtualization before creation.

        // Pre-fetch existing services for ID alignment and service_id pre-computation.
        // SCD2: only live services are candidates for natural-key match.
        let mut existing_services_for_match =
            if matches!(conflict_behavior, ConflictBehavior::Upsert) {
                self.service_service
                    .get_all(
                        StorableFilter::<Service>::new_from_host_ids(&[created_host.id]).live(),
                    )
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            };

        // Pre-compute service_id_remap: match incoming services to existing ones
        // using the same PartialEq logic (host_id + service_definition) that the
        // service creation loop uses for ID alignment. This lets us patch subnet
        // virtualization.service_id before creating subnets.
        let mut service_id_remap: std::collections::HashMap<Uuid, Uuid> =
            std::collections::HashMap::new();
        for svc in &services {
            let mut probe = svc.clone();
            probe.base.host_id = created_host.id;
            if let Some(existing) = existing_services_for_match.iter().find(|e| **e == probe)
                && existing.id != svc.id
            {
                service_id_remap.insert(svc.id, existing.id);
            }
        }

        // Create integration-derived subnets (e.g., Docker bridges)
        // Patch virtualization.service_id using the pre-computed remap
        let mut subnet_id_remap: std::collections::HashMap<Uuid, Uuid> =
            std::collections::HashMap::new();
        for mut subnet in subnets {
            if let Some(ref mut virt) = subnet.base.virtualization
                && let Some(old_id) = virt.service_id()
                && let Some(&new_id) = service_id_remap.get(&old_id)
            {
                virt.set_service_id(new_id);
            }
            let original_id = subnet.id;
            let created = self
                .subnet_service
                .create(subnet, authentication.clone())
                .await?;
            if created.id != original_id {
                subnet_id_remap.insert(original_id, created.id);
            }
        }

        // Create ip_addresses with correct host_id
        // For Upsert: deduplicate by checking existing ip_addresses first
        // Apply subnet_id_remap for virtualized subnets whose IDs changed during dedup

        // Count how many incoming ip_addresses share each MAC address.
        // Multiple incoming ip_addresses with the same MAC = VLAN sub-interfaces (or bridge/bond
        // members) sharing a parent's MAC. These are distinct ip_addresses and must not be
        // collapsed via MAC matching. A unique MAC (count == 1) indicates a standalone interface
        // that may have moved subnets (e.g., Docker container with DHCP, subnet reconfiguration).
        let incoming_mac_counts: HashMap<MacAddress, usize> = ip_addresses
            .iter()
            .filter_map(|i| i.base.mac_address)
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            });

        let mut created_ip_addresses = Vec::new();
        for mut ip_address in ip_addresses {
            ip_address.base.host_id = created_host.id;

            // Remap subnet_id if the subnet was deduped to an existing one
            if let Some(&new_subnet_id) = subnet_id_remap.get(&ip_address.base.subnet_id) {
                ip_address.base.subnet_id = new_subnet_id;
            }

            if matches!(conflict_behavior, ConflictBehavior::Upsert) {
                // Check if interface already exists by ID
                if let Some(existing_iface) =
                    self.ip_address_service.get_by_id(&ip_address.id).await?
                {
                    created_ip_addresses.push(existing_iface);
                    continue;
                }

                // Check by unique constraint (host_id, subnet_id, ip_address).
                // SCD2: only live rows; the partial unique index is also
                // `WHERE valid_to IS NULL`, so this matches index semantics.
                let filter =
                    StorableFilter::<IPAddress>::new_from_host_ids(&[ip_address.base.host_id])
                        .subnet_id(&ip_address.base.subnet_id)
                        .live();
                let existing_by_key: Vec<IPAddress> =
                    self.ip_address_service.get_all(filter).await?;
                if let Some(existing_iface) = existing_by_key
                    .into_iter()
                    .find(|i| i.base.ip_address == ip_address.base.ip_address)
                {
                    created_ip_addresses.push(existing_iface);
                    continue;
                }

                // MAC fallback: find by (host_id, mac_address) when subnet differs.
                // Designed for the case where an interface moved between subnets across
                // discovery runs (e.g., Docker container with DHCP, subnet reconfiguration).
                //
                // Dual guard to prevent VLAN sub-interface collapse:
                // - incoming_mac_counts == 1: this MAC is unique in the incoming batch,
                //   so it's a standalone ip_address, not a VLAN sub-interface
                // - existing_by_mac.len() == 1: only one existing interface has this MAC,
                //   so there's an unambiguous 1:1 match (not a N:1 VLAN consolidation)
                if let Some(mac) = &ip_address.base.mac_address
                    && incoming_mac_counts.get(mac).copied().unwrap_or(0) == 1
                {
                    let mac_filter =
                        StorableFilter::<IPAddress>::new_from_host_ids(&[ip_address.base.host_id])
                            .mac_address(mac)
                            .live();
                    let existing_by_mac: Vec<IPAddress> =
                        self.ip_address_service.get_all(mac_filter).await?;
                    if existing_by_mac.len() == 1 {
                        let existing_iface = existing_by_mac.into_iter().next().unwrap();
                        tracing::debug!(
                            interface_ip = %ip_address.base.ip_address,
                            interface_mac = %mac,
                            existing_subnet_id = %existing_iface.base.subnet_id,
                            incoming_subnet_id = %ip_address.base.subnet_id,
                            "Found existing ip_address by MAC address (subnet_id differs, 1:1 MAC match)"
                        );
                        created_ip_addresses.push(existing_iface);
                        continue;
                    }
                }
            }

            // SCD2 origin: dedup didn't match, so this row is being
            // inserted. Stamp created_at + valid_from to the entity's
            // already-refreshed `last_seen_at`.
            use crate::server::shared::storage::snapshot::DiscoveryTracked;
            let mut ip_address = ip_address;
            ip_address.originate_scan_timestamps(ip_address.last_seen_at);
            let created = self
                .ip_address_service
                .create(ip_address, authentication.clone())
                .await?;
            created_ip_addresses.push(created);
        }

        // Build scanner→DB ip_address ID mapping using positional correspondence.
        // original_ip_addresses and created_ip_addresses are 1:1 in order (the ip_address
        // creation loop produces exactly one entry per input ip_address).
        let ip_address_id_remap: std::collections::HashMap<Uuid, Uuid> = original_ip_addresses
            .iter()
            .zip(created_ip_addresses.iter())
            .filter(|(orig, created)| orig.id != created.id)
            .map(|(orig, created)| (orig.id, created.id))
            .collect();

        // Create services with bindings reassigned (for discovery where IDs may change)
        // Track claimed bindings in this batch to detect in-batch conflicts
        let mut batch_claimed: Vec<(Uuid, Option<Uuid>)> = Vec::new();
        // Collect orphaned bindings from dropped services to assign to OpenPorts
        let mut orphaned_bindings: Vec<Binding> = Vec::new();
        let mut created_services = Vec::new();

        for service in services {
            let mut reassigned = self
                .service_service
                .reassign_service_interface_bindings(
                    service,
                    &original_host,
                    &original_ip_addresses,
                    &original_ports,
                    &created_host,
                    &created_ip_addresses,
                    &created_ports,
                    &ip_address_id_remap,
                )
                .await;

            // Align service ID with existing match so conflict check excludes its bindings
            let original_service_id = reassigned.id;
            if let Some(existing) = existing_services_for_match
                .iter()
                .find(|e| **e == reassigned)
            {
                reassigned.id = existing.id;
            }

            // Track service ID remapping for subnet virtualization patching
            if reassigned.id != original_service_id {
                service_id_remap.insert(original_service_id, reassigned.id);
            }

            // Check for binding conflicts with other services (DB + batch)
            let (valid_bindings, conflicting_bindings) = self
                .service_service
                .partition_conflicting_bindings(
                    &created_host.id,
                    &reassigned.id,
                    reassigned.base.bindings.clone(),
                    &batch_claimed,
                )
                .await?;

            if !conflicting_bindings.is_empty() {
                // Check if this service matches an existing one on this host (ID was
                // aligned earlier to enable upsert). When true, partial conflicts are
                // expected — the service is being re-discovered from a different scan
                // phase (e.g., Docker scan after network scan) and some of its new
                // bindings may conflict with other services like Unclaimed Open Ports.
                // We proceed with non-conflicting bindings so the upsert can merge
                // metadata (e.g., Docker virtualization).
                let matches_existing_service = existing_services_for_match
                    .iter()
                    .any(|e| e.id == reassigned.id);

                if matches_existing_service {
                    tracing::debug!(
                        service_name = %reassigned.base.name,
                        service_definition = %reassigned.base.service_definition.name(),
                        conflicting_count = conflicting_bindings.len(),
                        valid_count = valid_bindings.len(),
                        "Re-discovered service has partial binding conflicts - proceeding with valid bindings for upsert"
                    );
                    reassigned.base.bindings = valid_bindings;
                } else if reassigned.base.virtualization.is_some()
                    && ServiceDefinitionExt::is_generic(&reassigned.base.service_definition)
                {
                    // Safety net for Docker container → specific service reconciliation.
                    //
                    // When the Docker scan can't identify a container's specific service
                    // (e.g., exec-based and external endpoint probing both fail to match),
                    // it creates a generic "Docker Container" service. This conflicts with
                    // the specific service already found by the network scan (same port).
                    //
                    // Rather than dropping the Docker Container and losing its virtualization
                    // metadata (container name, container ID, Docker daemon linkage), we find
                    // the specific service that claims the conflicting port and set the Docker
                    // virtualization on it directly. The network scan already correctly
                    // identified the service; we're just adding the Docker container metadata.
                    let conflicting_port_ids: Vec<Uuid> = conflicting_bindings
                        .iter()
                        .filter_map(|b| b.port_id())
                        .collect();

                    // Find non-generic services on this host that claim the conflicting ports
                    let enrichable_services: Vec<&Service> = existing_services_for_match
                        .iter()
                        .filter(|s| {
                            !ServiceDefinitionExt::is_generic(&s.base.service_definition)
                                && s.base.virtualization.is_none()
                                && s.base.bindings.iter().any(|b| {
                                    b.port_id()
                                        .is_some_and(|pid| conflicting_port_ids.contains(&pid))
                                })
                        })
                        .collect();

                    if !enrichable_services.is_empty() {
                        for existing_svc in enrichable_services {
                            tracing::info!(
                                service_name = %existing_svc.base.name,
                                service_definition = %existing_svc.base.service_definition.name(),
                                container_service = %reassigned.base.name,
                                "Setting Docker virtualization on existing service from conflicting Docker Container"
                            );
                            let mut updated = existing_svc.clone();
                            updated.base.virtualization = reassigned.base.virtualization.clone();
                            let _ = self.service_service.storage().update(&mut updated).await;
                        }
                    }

                    // Still drop the generic Docker Container service itself
                    continue;
                } else {
                    // Check if all conflicts are with the Unclaimed Open Ports service.
                    // When a new service definition is added and a host is re-scanned,
                    // the new service's ports conflict with OpenPorts from the prior scan.
                    // The specific service should reclaim those ports.
                    let conflicting_claims: Vec<(Uuid, Option<Uuid>)> = conflicting_bindings
                        .iter()
                        .filter_map(|b| {
                            if let BindingType::Port {
                                port_id,
                                ip_address_id,
                            } = &b.base.binding_type
                            {
                                Some((*port_id, *ip_address_id))
                            } else {
                                None
                            }
                        })
                        .collect();

                    // Check each conflicting claim has a matching Open Ports binding
                    // using the same overlap logic as partition_conflicting_bindings:
                    // None overlaps anything, Some(a) overlaps Some(a)
                    let all_conflicts_from_open_ports = !conflicting_claims.is_empty()
                        && conflicting_claims.iter().all(|(port_id, claim_iface)| {
                            existing_services_for_match.iter().any(|s| {
                                ServiceDefinitionExt::is_open_ports(&s.base.service_definition)
                                    && s.base.bindings.iter().any(|b| {
                                        let Some(op_port) = b.port_id() else {
                                            return false;
                                        };
                                        op_port == *port_id
                                            && bindings_overlap(claim_iface, &b.ip_address_id())
                                    })
                            })
                        });

                    if all_conflicts_from_open_ports {
                        // Find the OpenPorts service and remove the conflicting bindings.
                        // The daemon's OpenPorts upsert later in the batch sets the
                        // authoritative final state — this just clears DB conflicts
                        // so the new service can be created.
                        if let Some(open_ports_svc) = existing_services_for_match.iter().find(|s| {
                            ServiceDefinitionExt::is_open_ports(&s.base.service_definition)
                        }) {
                            let open_ports_id = open_ports_svc.id;

                            // Count bindings that would remain after removing overlapping ones
                            let remaining_binding_count = open_ports_svc
                                .base
                                .bindings
                                .iter()
                                .filter(|b| {
                                    let Some(port_id) = b.port_id() else {
                                        return true;
                                    };
                                    let bind_iface = b.ip_address_id();
                                    !conflicting_claims.iter().any(|(cp, ci)| {
                                        *cp == port_id && bindings_overlap(ci, &bind_iface)
                                    })
                                })
                                .count();

                            if remaining_binding_count == 0 {
                                tracing::info!(
                                    service_name = %reassigned.base.service_definition.name(),
                                    reclaimed_ports = ?conflicting_claims,
                                    "Deleting Unclaimed Open Ports service after all ports reclaimed"
                                );
                                let _ = self
                                    .service_service
                                    .delete(&open_ports_id, authentication.clone())
                                    .await;
                            } else {
                                tracing::info!(
                                    service_name = %reassigned.base.service_definition.name(),
                                    reclaimed_ports = ?conflicting_claims,
                                    remaining_bindings = remaining_binding_count,
                                    "Reclaiming ports from Unclaimed Open Ports service"
                                );
                                let _ = self
                                    .service_service
                                    .remove_port_bindings(
                                        &open_ports_id,
                                        &conflicting_claims,
                                        authentication.clone(),
                                    )
                                    .await;
                            }

                            // Update in-memory state so later iterations see the change
                            if let Some(svc) = existing_services_for_match
                                .iter_mut()
                                .find(|s| s.id == open_ports_id)
                            {
                                svc.base.bindings.retain(|b| {
                                    let Some(port_id) = b.port_id() else {
                                        return true;
                                    };
                                    let bind_iface = b.ip_address_id();
                                    !conflicting_claims.iter().any(|(cp, ci)| {
                                        *cp == port_id && bindings_overlap(ci, &bind_iface)
                                    })
                                });
                            }
                        }

                        // Restore full bindings on the incoming service
                        let mut full_bindings = valid_bindings;
                        full_bindings.extend(conflicting_bindings);
                        reassigned.base.bindings = full_bindings;

                    // Fall through to service creation below
                    } else {
                        let conflicting_ports: Vec<_> = conflicting_bindings
                            .iter()
                            .filter_map(|b| {
                                if let BindingType::Port { port_id, .. } = &b.base.binding_type {
                                    created_ports
                                        .iter()
                                        .find(|p| p.id == *port_id)
                                        .map(|p| p.to_string())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        tracing::warn!(
                            service_name = %reassigned.base.name,
                            service_definition = %reassigned.base.service_definition.name(),
                            host_id = %created_host.id,
                            conflicting_ports = ?conflicting_ports,
                            valid_binding_count = valid_bindings.len(),
                            "Discovery found service with conflicting port bindings - dropping service"
                        );

                        orphaned_bindings.extend(valid_bindings);
                        continue;
                    }
                }
            }

            // Track this service's port bindings for in-batch conflict detection
            for binding in &reassigned.base.bindings {
                if let BindingType::Port {
                    port_id,
                    ip_address_id,
                } = &binding.base.binding_type
                {
                    batch_claimed.push((*port_id, *ip_address_id));
                }
            }

            let created = self
                .service_service
                .create(reassigned, authentication.clone())
                .await?;
            // Add to existing_services_for_match so subsequent services in this batch
            // can find it for ID alignment and Docker Container → specific service reconciliation
            existing_services_for_match.push(created.clone());
            created_services.push(created);
        }

        // If we have orphaned bindings, assign them to OpenPorts service
        if !orphaned_bindings.is_empty() {
            use crate::server::services::definitions::open_ports::OpenPorts as OpenPortsDef;
            use crate::server::services::r#impl::base::ServiceBase;

            tracing::info!(
                host_id = %created_host.id,
                orphaned_binding_count = orphaned_bindings.len(),
                "Assigning orphaned bindings to OpenPorts service"
            );

            let open_ports_service = Service::new(ServiceBase {
                host_id: created_host.id,
                network_id: created_host.base.network_id,
                service_definition: Box::new(OpenPortsDef),
                name: "Unclaimed Open Ports".to_string(),
                bindings: orphaned_bindings,
                virtualization: None,
                source: EntitySource::Discovery,
                tags: Vec::new(),
                position: 0,
            });

            // The singleton upsert in service.create() will merge bindings
            // if an OpenPorts service already exists on this host
            let created = self
                .service_service
                .create(open_ports_service, authentication.clone())
                .await?;
            created_services.push(created);
        }

        // Patch service virtualization.service_id for container services
        // whose parent service ID was remapped during dedup.
        // Uses service_id_remap which was pre-computed before subnet creation
        // and may have additional entries from in-batch service creation above.
        for svc in &created_services {
            if let Some(ref virt) = svc.base.virtualization
                && let Some(old_id) = virt.service_id()
                && let Some(&new_id) = service_id_remap.get(&old_id)
            {
                let mut updated = svc.clone();
                updated
                    .base
                    .virtualization
                    .as_mut()
                    .unwrap()
                    .set_service_id(new_id);
                let _ = self.service_service.storage().update(&mut updated).await;
            }
        }

        // Binding fixup: remap provisional daemon interface/port IDs to server-assigned IDs.
        // This handles the case where interface or port UUIDs changed during dedup (upsert).
        // Idempotent — no-op if no IDs changed.
        // Reuse the zip-based ip_address_id_remap built before service creation.
        // The old structural-matching approach (ip_address + subnet_id) failed on second
        // scan because original_ip_addresses retain scanner subnet_ids while created_ip_addresses
        // have DB subnet_ids.
        {
            let port_id_remap: std::collections::HashMap<Uuid, Uuid> = original_ports
                .iter()
                .filter_map(|orig| {
                    created_ports
                        .iter()
                        .find(|c| c.base.port_type == orig.base.port_type)
                        .and_then(|created| {
                            if created.id != orig.id {
                                Some((orig.id, created.id))
                            } else {
                                None
                            }
                        })
                })
                .collect();

            if !ip_address_id_remap.is_empty() || !port_id_remap.is_empty() {
                for svc in &created_services {
                    let needs_update = svc.base.bindings.iter().any(|b| {
                        b.ip_address_id()
                            .is_some_and(|id| ip_address_id_remap.contains_key(&id))
                            || b.port_id()
                                .is_some_and(|id| port_id_remap.contains_key(&id))
                    });
                    if needs_update {
                        let mut updated = svc.clone();
                        for binding in &mut updated.base.bindings {
                            match &mut binding.base.binding_type {
                                BindingType::IPAddress { ip_address_id } => {
                                    if let Some(&new_id) = ip_address_id_remap.get(ip_address_id) {
                                        *ip_address_id = new_id;
                                    }
                                }
                                BindingType::Port {
                                    port_id,
                                    ip_address_id,
                                } => {
                                    if let Some(&new_id) = port_id_remap.get(port_id) {
                                        *port_id = new_id;
                                    }
                                    if let Some(iface_id) = ip_address_id
                                        && let Some(&new_id) = ip_address_id_remap.get(iface_id)
                                    {
                                        *iface_id = new_id;
                                    }
                                }
                            }
                        }
                        let _ = self.service_service.storage().update(&mut updated).await;
                    }
                }
            }
        }

        tracing::info!(
            host_id = %created_host.id,
            host_name = %created_host.base.name,
            interface_count = %created_ip_addresses.len(),
            port_count = %created_ports.len(),
            service_count = %created_services.len(),
            "Created host with children"
        );

        if let Some(org_id) = authentication.organization_id() {
            self.entity_tag_service
                .set_tags(
                    created_host.id,
                    EntityDiscriminants::Host,
                    created_host.base.tags.clone(),
                    org_id,
                )
                .await?;
        }

        // Create interfaces with correct host_id
        // Uses create_or_update_from_discovery for tiered dedup on
        // (host_id, if_name) → (host_id, if_index) → (host_id, mac_address).
        let mut created_interfaces = Vec::new();
        for mut entry in interfaces {
            entry.base.host_id = created_host.id;
            entry.base.network_id = created_host.base.network_id;

            let created = self
                .interface_service
                .create_or_update_from_discovery(entry, authentication.clone())
                .await?;
            created_interfaces.push(created);
        }

        // Full-sweep prune of interfaces no longer reported for this host.
        //
        // Scanopy has no partial-scan mode; a successful SNMP query returns the
        // host's complete ifTable. Zero incoming interfaces means discovery
        // couldn't run (network error, credentials revoked), NOT "all removed" —
        // the empty-set guard below preserves existing rows in that case.
        if !created_interfaces.is_empty() {
            let kept_ids: HashSet<Uuid> = created_interfaces.iter().map(|i| i.id).collect();
            let existing = self
                .interface_service
                .get_for_host(&created_host.id)
                .await?;
            for iface in existing {
                if !kept_ids.contains(&iface.id)
                    && let Err(e) = self
                        .interface_service
                        .delete(&iface.id, authentication.clone())
                        .await
                {
                    tracing::warn!(
                        host_id = %created_host.id,
                        interface_id = %iface.id,
                        error = %e,
                        "Failed to prune orphan interface"
                    );
                }
            }
        }

        // Remap credential assignment ip_address_ids from daemon UUIDs to server UUIDs
        // and persist to the host_credentials junction table.
        // credential_assignments is transient on HostBase (not stored in hosts table),
        // so we must persist via set_host_credentials and use the original input host's
        // assignments (created_host.base.credential_assignments is empty after DB round-trip).
        let mut remapped_assignments = original_host.base.credential_assignments.clone();
        for assignment in &mut remapped_assignments {
            if let Some(ref mut ids) = assignment.ip_address_ids {
                *ids = ids
                    .iter()
                    .filter_map(|daemon_id| {
                        let ip = daemon_ip_address_ips
                            .iter()
                            .find(|(id, _)| id == daemon_id)
                            .map(|(_, ip)| *ip)?;
                        created_ip_addresses
                            .iter()
                            .find(|i| i.base.ip_address == ip)
                            .map(|i| i.id)
                    })
                    .collect();
            }
        }
        if !remapped_assignments.is_empty()
            && let Err(e) = self
                .credential_service
                .set_host_credentials(&created_host.id, &remapped_assignments)
                .await
        {
            tracing::warn!(
                host_id = %created_host.id,
                error = ?e,
                "Failed to persist credential assignments during discover_host"
            );
        }
        created_host.base.credential_assignments = remapped_assignments;

        Ok(HostResponse::from_host_with_children(
            created_host,
            created_ip_addresses,
            created_ports,
            created_services,
            created_interfaces,
        ))
    }

    /// Update a host from an UpdateHostRequest
    /// Optionally syncs ip_addresses and ports if provided in the request.
    pub async fn update_from_request(
        &self,
        request: UpdateHostRequest,
        authentication: AuthenticatedEntity,
    ) -> Result<HostResponse> {
        // Get existing host
        let existing = self
            .get_by_id(&request.id)
            .await?
            .ok_or_else(|| anyhow!("Host '{}' not found", request.id))?;

        let network_id = existing.base.network_id;
        let UpdateHostRequest {
            id,
            name,
            hostname,
            description,
            virtualization,
            hidden,
            tags,
            expected_updated_at: _,
            ip_addresses,
            ports,
            services,
            credential_assignments,
        } = request;

        // Optimistic locking: check if host was modified since user loaded it
        // Compare at microsecond precision since PostgreSQL TIMESTAMPTZ truncates nanoseconds
        if let Some(expected) = request.expected_updated_at
            && existing.updated_at.timestamp_micros() != expected.timestamp_micros()
        {
            tracing::warn!(
                host_id = %id,
                expected = %expected,
                actual = %existing.updated_at,
                "Host update conflict - host was modified since user loaded it"
            );
            return Err(ValidationError::new(format!(
                "Host was modified by another process (possibly discovery). \
                     Please reload and try again. Expected: {}, Actual: {}",
                expected, existing.updated_at
            ))
            .into());
        }

        let mut updated_host = Host {
            id,
            created_at: existing.created_at,
            updated_at: Utc::now(),
            valid_from: existing.valid_from,
            valid_to: existing.valid_to,
            lineage_id: existing.lineage_id,
            last_seen_at: existing.last_seen_at,
            last_discovery_id: existing.last_discovery_id,
            first_discovery_id: existing.first_discovery_id,
            base: HostBase {
                name,
                network_id,
                source: existing.base.source,
                hostname,
                description,
                virtualization,
                hidden,
                tags: tags.clone(),
                // Preserve existing SNMP fields on update
                sys_descr: existing.base.sys_descr.clone(),
                sys_object_id: existing.base.sys_object_id.clone(),
                sys_location: existing.base.sys_location.clone(),
                sys_contact: existing.base.sys_contact.clone(),
                management_url: existing.base.management_url.clone(),
                chassis_id: existing.base.chassis_id.clone(),
                sys_name: existing.base.sys_name.clone(),
                manufacturer: existing.base.manufacturer.clone(),
                model: existing.base.model.clone(),
                serial_number: existing.base.serial_number.clone(),
                credential_assignments: credential_assignments
                    .unwrap_or_else(|| existing.base.credential_assignments.clone()),
            },
        };

        if let Some(org_id) = authentication.organization_id() {
            self.entity_tag_service
                .set_tags(id, EntityDiscriminants::Host, tags, org_id)
                .await?;
        }

        let updated = self
            .update(&mut updated_host, authentication.clone())
            .await?;

        // Sync ip_addresses only if provided (None means preserve existing)
        if let Some(ip_addresses) = ip_addresses {
            self.sync_ip_addresses(
                &updated.id,
                &network_id,
                ip_addresses,
                authentication.clone(),
            )
            .await?;
        }

        // Sync ports only if provided (None means preserve existing)
        if let Some(ports) = ports {
            self.sync_ports(&updated.id, &network_id, ports, authentication.clone())
                .await?;
        }

        // Sync services only if provided (None means preserve existing)
        if let Some(services) = services {
            self.sync_services(&updated.id, &network_id, services, authentication.clone())
                .await?;
        }

        // Load fresh children after sync
        let (ip_addresses, ports, services, interfaces) =
            self.load_children_for_host(&updated.id).await?;

        Ok(HostResponse::from_host_with_children(
            updated,
            ip_addresses,
            ports,
            services,
            interfaces,
        ))
    }

    /// Sync ip_addresses for a host: delete removed, update existing, create new.
    /// Client provides UUIDs - if ID exists for this host, update; if not, create.
    async fn sync_ip_addresses(
        &self,
        host_id: &Uuid,
        network_id: &Uuid,
        inputs: Vec<IPAddressInput>,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use std::collections::HashSet;

        // Get existing ip_addresses for this host (needed for position resolution)
        let existing = self.ip_address_service.get_for_host(host_id).await?;
        let existing_ids: HashSet<Uuid> = existing.iter().map(|i| i.id).collect();

        // Resolve and validate positions
        let mut inputs = inputs;
        resolve_and_validate_input_positions(&mut inputs, &existing, "ip_address")
            .map_err(|e| ValidationError::new(e.message))?;

        // All input IDs (client-provided)
        let input_ids: HashSet<Uuid> = inputs.iter().map(|i| i.id).collect();

        // Delete ip_addresses that are not in the input list
        let to_delete: Vec<Uuid> = existing_ids.difference(&input_ids).copied().collect();
        if !to_delete.is_empty() {
            self.ip_address_service
                .delete_many(&to_delete, authentication.clone())
                .await?;
        }

        // Process each input - create or update based on whether ID exists for this host
        for input in inputs {
            let id = input.id;
            let mut ip_address = input.into_ip_address(*host_id, *network_id);

            if existing_ids.contains(&id) {
                // Update existing interface - preserve created_at from existing
                if let Some(existing_iface) = existing.iter().find(|i| i.id == id) {
                    ip_address.preserve_immutable_fields(existing_iface);
                }

                self.ip_address_service
                    .update(&mut ip_address, authentication.clone())
                    .await?;
            } else {
                // Create new interface with client-provided ID
                self.ip_address_service
                    .create(ip_address, authentication.clone())
                    .await?;
            }
        }

        Ok(())
    }

    /// Sync ports for a host: delete removed, create new, update existing.
    /// Client provides UUIDs - if ID exists for this host, update; if not, create.
    async fn sync_ports(
        &self,
        host_id: &Uuid,
        network_id: &Uuid,
        inputs: Vec<PortInput>,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use std::collections::HashSet;

        // Get existing ports for this host
        let existing = self.port_service.get_for_host(host_id).await?;
        let existing_ids: HashSet<Uuid> = existing.iter().map(|p| p.id).collect();

        // All input IDs (client-provided)
        let input_ids: HashSet<Uuid> = inputs.iter().map(|p| p.id).collect();

        // Delete ports that are not in the input list
        let to_delete: Vec<Uuid> = existing_ids.difference(&input_ids).copied().collect();
        if !to_delete.is_empty() {
            self.port_service
                .delete_many(&to_delete, authentication.clone())
                .await?;
        }

        // Process each input - create or update based on whether ID exists for this host
        for input in inputs {
            let id = input.id;
            let mut port = input.into_port(*host_id, *network_id);

            if existing_ids.contains(&id) {
                // Update existing port - preserve created_at from existing
                if let Some(existing_port) = existing.iter().find(|p| p.id == id) {
                    port.preserve_immutable_fields(existing_port);
                }

                self.port_service
                    .update(&mut port, authentication.clone())
                    .await?;
            } else {
                // Create new port with client-provided ID
                self.port_service
                    .create(port, authentication.clone())
                    .await?;
            }
        }

        Ok(())
    }

    /// Sync services for a host: delete removed, update existing, create new.
    /// Client provides UUIDs - if ID exists for this host, update; if not, create.
    async fn sync_services(
        &self,
        host_id: &Uuid,
        network_id: &Uuid,
        inputs: Vec<ServiceInput>,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use std::collections::HashSet;

        // Get existing services for this host (needed for position resolution)
        let existing = self.service_service.get_for_parent(host_id).await?;
        let existing_ids: HashSet<Uuid> = existing.iter().map(|s| s.id).collect();

        // Resolve and validate positions
        let mut inputs = inputs;
        resolve_and_validate_input_positions(&mut inputs, &existing, "service")
            .map_err(|e| ValidationError::new(e.message))?;

        // All input IDs (client-provided)
        let input_ids: HashSet<Uuid> = inputs.iter().map(|s| s.id).collect();

        // Delete services that are not in the input list
        let to_delete: Vec<Uuid> = existing_ids.difference(&input_ids).copied().collect();
        if !to_delete.is_empty() {
            self.service_service
                .delete_many(&to_delete, authentication.clone())
                .await?;
        }

        // Partition inputs: services losing port bindings must be processed first.
        // This ensures bindings are freed in DB before other services try to claim them,
        // which is required for port transfers between services to work correctly.
        let (losing_bindings, others): (Vec<_>, Vec<_>) = inputs.into_iter().partition(|input| {
            if let Some(existing_svc) = existing.iter().find(|s| s.id == input.id) {
                // Get current port binding keys (port_id, ip_address_id)
                let current_ports: HashSet<_> = existing_svc
                    .base
                    .bindings
                    .iter()
                    .filter_map(|b| match &b.base.binding_type {
                        BindingType::Port {
                            port_id,
                            ip_address_id,
                        } => Some((*port_id, *ip_address_id)),
                        _ => None,
                    })
                    .collect();

                // Get input port binding keys
                let input_ports: HashSet<_> = input
                    .bindings
                    .iter()
                    .filter_map(|b| match b {
                        BindingInput::Port {
                            port_id,
                            ip_address_id,
                            ..
                        } => Some((*port_id, *ip_address_id)),
                        _ => None,
                    })
                    .collect();

                // Service is "losing" if it has ports in DB that aren't in input
                current_ports.difference(&input_ports).next().is_some()
            } else {
                false // New service, not losing anything
            }
        });

        // Process losing-bindings services first, then others
        let ordered_inputs = losing_bindings.into_iter().chain(others);

        // Process each input - create or update based on whether ID exists for this host
        for input in ordered_inputs {
            let id = input.id;
            // For new services, source is Manual (API-created)
            // For existing services, we'll preserve their source below
            let mut service = input.into_service(*host_id, *network_id, EntitySource::Manual);

            if existing_ids.contains(&id) {
                // Update existing service - preserve immutable fields
                if let Some(existing_svc) = existing.iter().find(|s| s.id == id) {
                    service.preserve_immutable_fields(existing_svc);
                    // Also preserve source - can't change via API
                    service.base.source = existing_svc.base.source.clone();
                }

                self.service_service
                    .update(&mut service, authentication.clone())
                    .await?;
            } else {
                // Create new service with client-provided ID
                self.service_service
                    .create(service, authentication.clone())
                    .await?;
            }
        }

        Ok(())
    }

    // =========================================================================
    // Discovery support (internal API)
    // =========================================================================

    /// Create or update a host from daemon discovery data.
    /// This handles IP-address and port matching for host deduplication and upserts on conflict.
    #[allow(clippy::too_many_arguments)]
    pub async fn discover_host(
        &self,
        mut host: Host,
        mut ip_addresses: Vec<IPAddress>,
        mut ports: Vec<Port>,
        mut services: Vec<Service>,
        mut interfaces: Vec<crate::server::interfaces::r#impl::base::Interface>,
        mut subnets: Vec<Subnet>,
        scan_ctx: Option<&crate::server::shared::services::scan_context::ScanContext>,
        authentication: AuthenticatedEntity,
        limit_ctx: Option<&HostLimitContext>,
    ) -> Result<HostResponse> {
        // SCD2 scan-time normalization: stamp every entity in this
        // submission with the same `scan_time` so per-scan diff queries
        // (Added/Removed/Modified/Refreshed-unchanged buckets keyed on
        // session window) and freshness reads see one consistent timestamp.
        // Without this, each entity's per-call `Utc::now()` drifts across
        // the host+children tree by microseconds-to-milliseconds, blurring
        // session boundaries.
        //
        // Only refresh-style fields (last_seen_at, updated_at) are stamped
        // here — they advance on every observation regardless of new vs.
        // upsert. Origin-style fields (created_at, valid_from) are stamped
        // by the new-insert sites themselves (HostService::create,
        // SubnetService::create, ServiceService::create, and the no-match
        // branches in this function for Port / IPAddress / Interface) so
        // they only fire when a row is actually being inserted for the
        // first time. Each site reads scan_time off the entity's
        // already-refreshed `last_seen_at`.
        if let Some(ctx) = scan_ctx {
            use crate::server::shared::storage::snapshot::DiscoveryTracked;
            host.refresh_scan_timestamps(ctx.scan_time);
            for ip in ip_addresses.iter_mut() {
                ip.refresh_scan_timestamps(ctx.scan_time);
            }
            for p in ports.iter_mut() {
                p.refresh_scan_timestamps(ctx.scan_time);
            }
            for s in services.iter_mut() {
                s.refresh_scan_timestamps(ctx.scan_time);
            }
            for i in interfaces.iter_mut() {
                i.refresh_scan_timestamps(ctx.scan_time);
            }
            for s in subnets.iter_mut() {
                s.refresh_scan_timestamps(ctx.scan_time);
            }
        }

        // Capture the subnets the matched host touched before the upsert. If the
        // host migrates subnets (or an IP address stops reporting), we need to
        // revisit the old subnet during reconciliation so its stale VLAN links
        // can drop. Post-upsert `get_for_host` alone would miss subnets that
        // disappeared entirely from the host.
        let previous_subnets: HashSet<Uuid> = self
            .find_matching_host_by_ip_addresses(&host.base.network_id, &ip_addresses)
            .await?
            .map(|(_, existing_ips)| existing_ips.iter().map(|i| i.base.subnet_id).collect())
            .unwrap_or_default();

        let host_response = self
            .create_with_children(
                host,
                ip_addresses,
                ports,
                services,
                interfaces.clone(),
                subnets,
                ConflictBehavior::Upsert,
                authentication.clone(),
                limit_ctx,
            )
            .await?;

        // Link Interfaces to IPAddresses via MAC address matching (if any were created)
        if !interfaces.is_empty()
            && let Err(e) = self
                .link_interfaces_to_ip_addresses(&host_response.id, authentication)
                .await
        {
            tracing::warn!(error = %e, "Failed to link Interfaces to IPAddresses");
        }

        // Reconcile subnet↔VLAN junction records across previous ∪ current subnets.
        // Aggregates native_vlan_id observations from all hosts so stale links drop
        // when nobody reports them anymore.
        if !interfaces.is_empty()
            && let Err(e) = self
                .reconcile_subnet_vlans_for_host(&host_response.id, &previous_subnets)
                .await
        {
            tracing::warn!(error = %e, "Failed to reconcile subnet_vlans");
        }

        Ok(host_response)
    }

    /// Link Interface records (SNMP if-entries) to IPAddress records for a host by matching MAC addresses.
    ///
    /// For each Interface with a MAC address, finds an IPAddress on the same host with
    /// the same MAC address and sets `interface.ip_address_id = ip_address.id`.
    /// This enables PhysicalLink topology edges to have source/target Interface IDs.
    async fn link_interfaces_to_ip_addresses(
        &self,
        host_id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        use crate::server::interfaces::r#impl::base::if_type;

        // Get all ip_addresses for this host
        let ip_addresses = self.ip_address_service.get_for_host(host_id).await?;

        // Build MAC -> ip_address_id lookup
        let mac_to_interface: std::collections::HashMap<_, _> = ip_addresses
            .iter()
            .filter_map(|iface| iface.base.mac_address.map(|mac| (mac, iface.id)))
            .collect();

        // Find loopback interface (by IP address)
        let loopback_interface_id = ip_addresses
            .iter()
            .find(|iface| iface.base.ip_address.is_loopback())
            .map(|iface| iface.id);

        // Get all IfEntries for this host
        let interfaces = self.interface_service.get_for_host(host_id).await?;

        let mut linked_count = 0;
        for mut interface in interfaces {
            // Skip if already linked
            if interface.base.ip_address_id.is_some() {
                continue;
            }

            // Try loopback linking by if_type
            let matched_interface_id = if interface.base.if_type == if_type::SOFTWARE_LOOPBACK {
                loopback_interface_id
            } else {
                // Try MAC-based linking
                interface
                    .base
                    .mac_address
                    .and_then(|mac| mac_to_interface.get(&mac).copied())
            };

            if let Some(ip_address_id) = matched_interface_id {
                interface.base.ip_address_id = Some(ip_address_id);
                if let Err(e) = self
                    .interface_service
                    .update(&mut interface, authentication.clone())
                    .await
                {
                    tracing::warn!(
                        interface_id = %interface.id,
                        error = %e,
                        "Failed to link Interface to IPAddress"
                    );
                } else {
                    linked_count += 1;
                }
            }
        }

        if linked_count > 0 {
            tracing::debug!(
                host_id = %host_id,
                linked = linked_count,
                "Linked Interfaces to IPAddresses via MAC address and loopback type"
            );
        }

        Ok(())
    }

    /// Reconcile subnet↔VLAN junction records after a host's discovery.
    ///
    /// For each subnet in `previous_subnets ∪ current_subnets`, aggregates
    /// `native_vlan_id` observations across ALL hosts' Interfaces on that subnet
    /// and replaces the junction set via `save_for_subnet`. The union ensures
    /// that a host which migrated off a subnet still triggers cleanup for that
    /// old subnet; post-upsert `get_for_host` captures "current" subnets.
    ///
    /// Inside each reconciled subnet, aggregating across all hosts means host A's
    /// rescan preserves host B's contribution — stale (subnet, vlan) pairs drop
    /// only when NO host reports them anymore.
    ///
    /// NOTE: the reads of fresh data and the `save_for_subnet` calls are not
    /// transactionally linked. Two hosts on the same subnet discovering
    /// concurrently can briefly produce a wrong VLAN set; the next scan of any
    /// host on that subnet corrects it. Eventually consistent.
    async fn reconcile_subnet_vlans_for_host(
        &self,
        host_id: &Uuid,
        previous_subnets: &HashSet<Uuid>,
    ) -> Result<()> {
        let current: HashSet<Uuid> = self
            .ip_address_service
            .get_for_host(host_id)
            .await?
            .iter()
            .map(|i| i.base.subnet_id)
            .collect();

        let to_reconcile: HashSet<Uuid> = previous_subnets.union(&current).copied().collect();

        let mut reconciled = 0usize;
        for subnet_id in to_reconcile {
            // All IPAddresses on this subnet across all hosts
            let subnet_ip_addresses = self.ip_address_service.get_for_subnet(&subnet_id).await?;
            if subnet_ip_addresses.is_empty() {
                // Subnet has no ip_addresses at all; drop any leftover links.
                self.vlan_service
                    .subnet_vlan_storage
                    .save_for_subnet(&subnet_id, &[])
                    .await?;
                reconciled += 1;
                continue;
            }

            let ip_address_ids: Vec<Uuid> = subnet_ip_addresses.iter().map(|i| i.id).collect();

            // All Interface rows linked to those IPAddresses across all hosts
            let linked_interfaces = self
                .interface_service
                .get_by_ip_address_ids(&ip_address_ids)
                .await?;

            let mut fresh_vlan_ids: Vec<Uuid> = linked_interfaces
                .iter()
                .filter_map(|iface| iface.base.native_vlan_id)
                .collect();
            fresh_vlan_ids.sort();
            fresh_vlan_ids.dedup();

            self.vlan_service
                .subnet_vlan_storage
                .save_for_subnet(&subnet_id, &fresh_vlan_ids)
                .await?;
            reconciled += 1;
        }

        if reconciled > 0 {
            tracing::debug!(
                host_id = %host_id,
                subnets_reconciled = reconciled,
                "Reconciled subnet_vlans links from Interface data"
            );
        }

        Ok(())
    }

    /// Find an existing host that matches based on IP-address data (subnet+IP or MAC address).
    ///
    /// **Known limitation — VRRP/HSRP:** Routers sharing a virtual IP+subnet via VRRP or HSRP
    /// could false-match on the IP+subnet branch. Virtual router MAC addresses are filtered
    /// out (see `is_virtual_router_mac`), but the shared virtual IP on a real interface could
    /// still cause incorrect dedup. Full VRRP awareness would require tracking group membership.
    pub async fn find_matching_host_by_ip_addresses(
        &self,
        network_id: &Uuid,
        incoming_ip_addresses: &[IPAddress],
    ) -> Result<Option<(Host, Vec<IPAddress>)>> {
        if incoming_ip_addresses.is_empty() {
            return Ok(None);
        }

        // SCD2: only match against live rows. Closed historical copies
        // (set when a snapshot fires) must not influence reconciliation.
        let filter = StorableFilter::<Host>::new_from_network_ids(&[*network_id]).live();
        let all_hosts = self.get_all(filter).await?;

        if all_hosts.is_empty() {
            return Ok(None);
        }

        let host_ids: Vec<Uuid> = all_hosts.iter().map(|h| h.id).collect();
        let ip_addresses_by_host = self
            .ip_address_service
            .get_for_hosts(&host_ids, None)
            .await?;

        // Exclude loopback and virtual router (VRRP/HSRP) IP addresses from matching.
        // Loopbacks: every host has 127.0.0.1, so they would falsely match all hosts.
        // Virtual router MACs: shared across physical routers, would falsely merge peers.
        let should_skip_for_matching = |ip: &IPAddress| {
            ip.base.ip_address.is_loopback()
                || ip
                    .base
                    .mac_address
                    .map(|m| is_virtual_router_mac(&m))
                    .unwrap_or(false)
        };

        let matchable_incoming: Vec<_> = incoming_ip_addresses
            .iter()
            .filter(|i| !should_skip_for_matching(i))
            .collect();

        if matchable_incoming.is_empty() {
            return Ok(None);
        }

        // Count incoming IP addresses per MAC to detect VLAN sub-interfaces.
        // Same approach as the upsert MAC fallback — shared MAC (count > 1) means
        // VLAN/bridge/bond sub-interfaces that must not trigger MAC-based host matching.
        // Unique MAC (count == 1) means a standalone IP address safe for MAC matching
        // (e.g., Docker container whose IP changed via DHCP).
        let incoming_mac_counts: HashMap<MacAddress, usize> = matchable_incoming
            .iter()
            .filter_map(|i| i.base.mac_address)
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            });

        for host in all_hosts {
            let host_interfaces = ip_addresses_by_host
                .get(&host.id)
                .cloned()
                .unwrap_or_default();

            for incoming_iface in &matchable_incoming {
                for existing_iface in &host_interfaces {
                    if should_skip_for_matching(existing_iface) {
                        continue;
                    }
                    if ip_addresses_match(incoming_iface, existing_iface, &incoming_mac_counts) {
                        tracing::debug!(
                            incoming_ip = %incoming_iface.base.ip_address,
                            existing_ip = %existing_iface.base.ip_address,
                            existing_host_id = %host.id,
                            existing_host_name = %host.base.name,
                            "Found matching host via IP-address comparison"
                        );
                        return Ok(Some((host, host_interfaces)));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn get_host_lock(&self, host_id: &Uuid) -> Arc<Mutex<()>> {
        let mut locks = self.host_locks.lock().await;
        locks
            .entry(*host_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Merge new discovery data with existing host
    async fn upsert_host(
        &self,
        mut existing_host: Host,
        new_host_data: Host,
        authentication: AuthenticatedEntity,
    ) -> Result<Host> {
        let host_before_updates = existing_host.clone();
        let mut has_updates = false;

        // SCD2 semantics: every successful natural-key match advances
        // last_seen_at to the scan time, regardless of whether other fields
        // change. The incoming `new_host_data` was pre-stamped by
        // `discover_host` (see `ScanContext`) so all entities from one
        // submission share one timestamp; without ScanContext the value is
        // whatever the caller put on the entity (likely a per-entity
        // Utc::now()).
        existing_host.last_seen_at = new_host_data.last_seen_at;

        tracing::trace!(
            "Upserting new host data {:?} to host {:?}",
            new_host_data,
            existing_host
        );

        // Update hostname if not set
        if existing_host.base.hostname.is_none()
            && new_host_data
                .base
                .hostname
                .as_ref()
                .is_some_and(|h| !h.is_empty())
        {
            has_updates = true;
            existing_host.base.hostname = new_host_data.base.hostname.clone();

            // Also update display name if it was auto-set to IP (not manually renamed)
            if existing_host.base.name.parse::<std::net::IpAddr>().is_ok()
                && let Some(ref hostname) = existing_host.base.hostname
            {
                existing_host.base.name = hostname.clone();
            }
        }

        // Update SNMP fields if not set
        if existing_host.base.sys_descr.is_none() && new_host_data.base.sys_descr.is_some() {
            has_updates = true;
            existing_host.base.sys_descr = new_host_data.base.sys_descr;
        }
        if existing_host.base.sys_object_id.is_none() && new_host_data.base.sys_object_id.is_some()
        {
            has_updates = true;
            existing_host.base.sys_object_id = new_host_data.base.sys_object_id;
        }
        if existing_host.base.sys_location.is_none() && new_host_data.base.sys_location.is_some() {
            has_updates = true;
            existing_host.base.sys_location = new_host_data.base.sys_location;
        }
        if existing_host.base.sys_contact.is_none() && new_host_data.base.sys_contact.is_some() {
            has_updates = true;
            existing_host.base.sys_contact = new_host_data.base.sys_contact;
        }
        if existing_host.base.management_url.is_none()
            && new_host_data.base.management_url.is_some()
        {
            has_updates = true;
            existing_host.base.management_url = new_host_data.base.management_url;
        }
        if existing_host.base.chassis_id.is_none() && new_host_data.base.chassis_id.is_some() {
            has_updates = true;
            existing_host.base.chassis_id = new_host_data.base.chassis_id;
        }
        if existing_host.base.sys_name.is_none() && new_host_data.base.sys_name.is_some() {
            has_updates = true;
            existing_host.base.sys_name = new_host_data.base.sys_name;
        }
        if existing_host.base.manufacturer.is_none() && new_host_data.base.manufacturer.is_some() {
            has_updates = true;
            existing_host.base.manufacturer = new_host_data.base.manufacturer;
        }
        if existing_host.base.model.is_none() && new_host_data.base.model.is_some() {
            has_updates = true;
            existing_host.base.model = new_host_data.base.model;
        }
        if existing_host.base.serial_number.is_none() && new_host_data.base.serial_number.is_some()
        {
            has_updates = true;
            existing_host.base.serial_number = new_host_data.base.serial_number;
        }

        // EntitySource merge: previously concatenated discovery metadata vecs
        // here. With the metadata field removed, source is just the variant
        // discriminant — propagate the new value if it's Discovery, else keep
        // existing. Discovery context is now tracked via FK columns
        // (last_discovery_id / first_discovery_id) populated post-terminal.
        existing_host.base.source = match (existing_host.base.source, new_host_data.base.source) {
            (EntitySource::Discovery, EntitySource::Discovery) => EntitySource::Discovery,
            (_, EntitySource::Discovery) => {
                has_updates = true;
                EntitySource::Discovery
            }
            (existing_source, _) => existing_source,
        };

        // Always write — even when no field changes, last_seen_at was
        // advanced above and must persist. The Updated event is only
        // published when there's a substantive field change, so consumers
        // that listen for "real" updates aren't woken by every refresh.
        self.storage().update(&mut existing_host).await?;

        if has_updates {
            let trigger_stale = existing_host.triggers_staleness(Some(host_before_updates));

            if let Some(scope) = EntityScope::from_ids(
                existing_host.id(),
                existing_host.clone().into(),
                self.get_network_id(&existing_host),
                self.get_organization_id(&existing_host),
            ) {
                self.event_bus()
                    .publish(
                        Event::new(scope, EntityOperation::Updated, authentication).with_flags(
                            EntityEventFlags {
                                trigger_stale,
                                ..Default::default()
                            },
                        ),
                    )
                    .await?;
            }
        } else {
            tracing::debug!(
                "No new data to upsert from host {} to {} (last_seen_at refresh only)",
                new_host_data.base.name,
                existing_host.base.name
            );
        }

        Ok(existing_host)
    }

    pub async fn consolidate_hosts(
        &self,
        destination_host: Host,
        other_host: Host,
        authentication: AuthenticatedEntity,
    ) -> Result<HostResponse> {
        if destination_host.id == other_host.id {
            return Err(ValidationError::new("Can't consolidate a host with itself").into());
        }

        let daemon_filter = StorableFilter::<Daemon>::new_from_host_ids(&[other_host.id]);

        if self.daemon_service.get_one(daemon_filter).await?.is_some() {
            return Err(ValidationError::new(
                "Can't consolidate a host that has a daemon associated with it. \
                 Consolidate the other host into the host with the daemon instead.",
            )
            .into());
        }

        let lock = self.get_host_lock(&destination_host.id).await;
        let _guard1 = lock.lock().await;

        tracing::trace!(
            "Consolidating host {:?} into host {:?}",
            other_host,
            destination_host
        );

        // Get ip_addresses and ports for both hosts
        let dest_interfaces = self
            .ip_address_service
            .get_for_host(&destination_host.id)
            .await?;
        let other_interfaces = self.ip_address_service.get_for_host(&other_host.id).await?;

        let dest_ports = self.port_service.get_for_host(&destination_host.id).await?;
        let other_ports = self.port_service.get_for_host(&other_host.id).await?;

        // Build interface ID mapping: source interface ID -> dest interface ID
        // Transfer non-conflicting ip_addresses to destination

        // Count MACs per host to detect VLAN sub-interfaces. MAC-based conflict detection
        // is only safe when both sides have a unique MAC (count == 1). If either host has
        // multiple ip_addresses sharing a MAC (VLANs/bridges/bonds), MAC matching would
        // incorrectly collapse distinct sub-interfaces during the merge.
        let dest_mac_counts: HashMap<MacAddress, usize> = dest_interfaces
            .iter()
            .filter_map(|i| i.base.mac_address)
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            });
        let other_mac_counts: HashMap<MacAddress, usize> = other_interfaces
            .iter()
            .filter_map(|i| i.base.mac_address)
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            });

        let mut interface_id_map: HashMap<Uuid, Uuid> = HashMap::new();
        for other_iface in &other_interfaces {
            // Check for conflict: same (subnet_id + ip_address) or same MAC (when 1:1)
            let matching_dest_iface = dest_interfaces.iter().find(|dest_iface| {
                // Match by subnet + IP (always safe — same logical ip_address)
                (dest_iface.base.subnet_id == other_iface.base.subnet_id
                    && dest_iface.base.ip_address == other_iface.base.ip_address)
                    // Match by MAC only when both hosts have a single interface with this MAC.
                    // Multiple ip_addresses sharing a MAC = VLAN sub-interfaces that should
                    // be preserved separately, not collapsed during merge.
                    || (dest_iface.base.mac_address.is_some()
                        && dest_iface.base.mac_address == other_iface.base.mac_address
                        && dest_iface
                            .base
                            .mac_address
                            .map(|mac| {
                                dest_mac_counts.get(&mac).copied().unwrap_or(0) == 1
                                    && other_mac_counts.get(&mac).copied().unwrap_or(0) == 1
                            })
                            .unwrap_or(false))
            });

            if let Some(dest_iface) = matching_dest_iface {
                // Conflict: map source ID to destination ID
                tracing::debug!(
                    source_interface_id = %other_iface.id,
                    dest_interface_id = %dest_iface.id,
                    ip = %other_iface.base.ip_address,
                    "IP address conflict - mapping to existing destination ip_address"
                );
                interface_id_map.insert(other_iface.id, dest_iface.id);
            } else {
                // No conflict: transfer interface to destination host
                let mut transferred = other_iface.clone();
                transferred.base.host_id = destination_host.id;
                self.ip_address_service
                    .update(&mut transferred, authentication.clone())
                    .await?;
                tracing::debug!(
                    ip_address_id = %other_iface.id,
                    ip = %other_iface.base.ip_address,
                    "Transferred ip_address to destination host"
                );
                // Map to itself (ID unchanged, just host_id changed)
                interface_id_map.insert(other_iface.id, other_iface.id);
            }
        }

        // Build port ID mapping: source_port_id -> dest_port_id
        // Transfer non-conflicting ports to destination
        let mut port_id_map: HashMap<Uuid, Uuid> = HashMap::new();
        for other_port in &other_ports {
            let other_config = other_port.base.port_type.config();

            // Check for conflict: same (number + protocol)
            let matching_dest_port = dest_ports.iter().find(|dest_port| {
                let dest_config = dest_port.base.port_type.config();
                dest_config.number == other_config.number
                    && dest_config.protocol == other_config.protocol
            });

            if let Some(dest_port) = matching_dest_port {
                // Conflict: map source ID to destination ID
                tracing::debug!(
                    source_port_id = %other_port.id,
                    dest_port_id = %dest_port.id,
                    port = %other_config.number,
                    "Port conflict - mapping to existing destination port"
                );
                port_id_map.insert(other_port.id, dest_port.id);
            } else {
                // No conflict: transfer port to destination host
                let mut transferred =
                    other_port.with_host(destination_host.id, destination_host.base.network_id);
                self.port_service
                    .update(&mut transferred, authentication.clone())
                    .await?;
                tracing::debug!(
                    port_id = %other_port.id,
                    port = %other_config.number,
                    "Transferred port to destination host"
                );
                // Map to itself (ID unchanged, just host_id changed)
                port_id_map.insert(other_port.id, other_port.id);
            }
        }

        // Upsert host data (metadata merge)
        let updated_host = self
            .upsert_host(
                destination_host.clone(),
                other_host.clone(),
                authentication.clone(),
            )
            .await?;

        // Get services for both hosts. SCD2: live rows only.
        let destination_services = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_host_ids(&[destination_host.id]).live())
            .await?;

        let other_services = self
            .service_service
            .get_all(StorableFilter::<Service>::new_from_host_ids(&[other_host.id]).live())
            .await?;

        // Transfer services, updating binding IDs using the maps
        for mut service in other_services {
            // Check for duplicate by name + service_definition
            let is_duplicate = destination_services.iter().any(|dest_svc| {
                dest_svc.base.name == service.base.name
                    && dest_svc.base.service_definition.id() == service.base.service_definition.id()
            });

            if is_duplicate {
                tracing::debug!(
                    service_name = %service.base.name,
                    service_def = %service.base.service_definition.id(),
                    "Skipping duplicate service during consolidation"
                );
                continue;
            }

            // Update host_id
            service.base.host_id = updated_host.id;
            service.base.network_id = updated_host.base.network_id;

            // Remap binding IDs using our maps
            for binding in &mut service.base.bindings {
                match &mut binding.base.binding_type {
                    BindingType::IPAddress { ip_address_id } => {
                        if let Some(&new_id) = interface_id_map.get(ip_address_id) {
                            *ip_address_id = new_id;
                        } else {
                            tracing::warn!(
                                service = %service.base.name,
                                ip_address_id = %ip_address_id,
                                "IP address not found in mapping during consolidation"
                            );
                        }
                    }
                    BindingType::Port {
                        port_id,
                        ip_address_id,
                    } => {
                        if let Some(&new_port_id) = port_id_map.get(port_id) {
                            *port_id = new_port_id;
                        } else {
                            tracing::warn!(
                                service = %service.base.name,
                                port_id = %port_id,
                                "Port not found in mapping during consolidation"
                            );
                        }
                        if let Some(iface_id) = ip_address_id {
                            if let Some(&new_iface_id) = interface_id_map.get(iface_id) {
                                *ip_address_id = Some(new_iface_id);
                            } else {
                                tracing::warn!(
                                    service = %service.base.name,
                                    ip_address_id = %iface_id,
                                    "IP address not found in mapping, falling back to all-ip_addresses"
                                );
                                *ip_address_id = None;
                            }
                        }
                    }
                }
            }

            self.service_service
                .update(&mut service, authentication.clone())
                .await
                .map_err(|e| {
                    tracing::error!(
                        service_id = %service.id,
                        service_name = %service.base.name,
                        "Failed to update service during consolidation: {}",
                        e
                    );
                    anyhow!(
                        "Failed to update service '{}' during consolidation: {}",
                        service.base.name,
                        e
                    )
                })?;
        }

        // Migrate credential assignments from other host to destination host
        let other_assignments = self
            .credential_service
            .get_credential_assignments_for_host(&other_host.id)
            .await?;

        if !other_assignments.is_empty() {
            use crate::server::credentials::r#impl::types::CredentialAssignment;

            let dest_assignments = self
                .credential_service
                .get_credential_assignments_for_host(&updated_host.id)
                .await?;

            let dest_cred_map: HashMap<Uuid, &CredentialAssignment> = dest_assignments
                .iter()
                .map(|a| (a.credential_id, a))
                .collect();

            let mut merged: Vec<CredentialAssignment> = dest_assignments.clone();
            let mut migrated_count = 0usize;

            for other in &other_assignments {
                // Remap ip_address_ids if present
                let remapped_iface_ids = match &other.ip_address_ids {
                    None => None,
                    Some(ids) => {
                        let remapped: Vec<Uuid> = ids
                            .iter()
                            .filter_map(|id| interface_id_map.get(id).copied())
                            .collect();
                        if remapped.is_empty() {
                            // All ip_addresses were dropped — skip this assignment
                            continue;
                        }
                        Some(remapped)
                    }
                };

                if let Some(dest_assignment) = dest_cred_map.get(&other.credential_id) {
                    // Both hosts have this credential — merge with broadest-scope-wins
                    let merged_iface_ids =
                        match (&dest_assignment.ip_address_ids, &remapped_iface_ids) {
                            (None, _) | (_, None) => None, // Either is all-interfaces → all
                            (Some(dest_ids), Some(other_ids)) => {
                                let mut union = dest_ids.clone();
                                for id in other_ids {
                                    if !union.contains(id) {
                                        union.push(*id);
                                    }
                                }
                                Some(union)
                            }
                        };

                    // Update the existing dest entry in merged list
                    if let Some(entry) = merged
                        .iter_mut()
                        .find(|a| a.credential_id == other.credential_id)
                    {
                        entry.ip_address_ids = merged_iface_ids;
                    }
                } else {
                    // Only on other host — add to merged list
                    merged.push(CredentialAssignment {
                        credential_id: other.credential_id,
                        ip_address_ids: remapped_iface_ids,
                    });
                }
                migrated_count += 1;
            }

            self.credential_service
                .set_host_credentials(&updated_host.id, &merged)
                .await?;

            tracing::info!(
                migrated = migrated_count,
                source_host_id = %other_host.id,
                dest_host_id = %updated_host.id,
                "Migrated credential assignments during consolidation"
            );
        }

        // Delete other host (remaining children that weren't transferred will cascade)
        self.delete_host(&other_host.id, authentication).await?;

        tracing::info!(
            source_host_id = %other_host.id,
            source_host_name = %other_host.base.name,
            dest_host_id = %updated_host.id,
            dest_host_name = %updated_host.base.name,
            ip_addresses_mapped = %interface_id_map.len(),
            ports_mapped = %port_id_map.len(),
            "Hosts consolidated"
        );

        // Return response with hydrated children
        let (ip_addresses, ports, services, interfaces) =
            self.load_children_for_host(&updated_host.id).await?;
        Ok(HostResponse::from_host_with_children(
            updated_host,
            ip_addresses,
            ports,
            services,
            interfaces,
        ))
    }

    // =========================================================================
    // LLDP link resolution
    // =========================================================================

    /// Resolve LLDP links for all interfaces in a network.
    ///
    /// Called by DiscoveryService when a discovery session completes successfully.
    /// This resolves LLDP neighbor data (chassis ID, port ID) to actual database
    /// entity references via the Neighbor enum.
    ///
    /// Resolution states:
    /// - Full resolution: Both host and port identified → `Neighbor::Interface(id)`
    /// - Partial resolution: Only host identified → `Neighbor::Host(id)`
    ///
    /// Returns statistics about the resolution process.
    pub async fn resolve_lldp_links(&self, network_id: Uuid) -> Result<LldpResolutionStats> {
        use crate::server::interfaces::r#impl::base::Neighbor;

        let resolver = LldpResolverImpl::new(
            self.interface_service.clone(),
            self.ip_address_service.clone(),
            self.storage.clone(),
        );

        // Get all interfaces with unresolved LLDP/CDP neighbors in this network
        let filter = StorableFilter::<Interface>::new_for_unresolved_lldp_in_network(network_id);
        let unresolved = self.interface_service.get_all(filter).await?;

        let mut stats = LldpResolutionStats::default();

        for mut interface in unresolved {
            stats.total += 1;

            // Try LLDP resolution first (more detailed data)
            // Only use chassis_id and port_id for neighbor resolution - these represent
            // actual physical connections. lldp_mgmt_addr is where you manage the device,
            // not necessarily the physical connection point.
            let resolved_neighbor = if let Some(ref chassis_id) = interface.base.lldp_chassis_id {
                // Resolve host from LLDP chassis ID
                if let Some(host_id) = chassis_id.resolve_host_id(&resolver, network_id).await {
                    stats.hosts_resolved += 1;

                    // Try to resolve specific port
                    if let Some(ref port_id) = interface.base.lldp_port_id
                        && let Some(remote_if_entry_id) =
                            port_id.resolve_if_entry_id(&resolver, host_id).await
                    {
                        stats.ports_resolved += 1;
                        Some(Neighbor::Interface(remote_if_entry_id))
                    } else {
                        Some(Neighbor::Host(host_id))
                    }
                } else {
                    None
                }
            } else if let Some(ref device_id) = interface.base.cdp_device_id {
                // CDP device_id is typically sysName, resolve against sys_name field
                // Don't fall back to cdp_address - it's management address, not physical connection
                if let Some(host_id) = resolver.find_host_by_sys_name(device_id, network_id).await {
                    stats.hosts_resolved += 1;

                    // Try CDP port resolution using cdp_port_id (long ifDescr format)
                    if let Some(ref port_id) = interface.base.cdp_port_id
                        && let Some(remote_if_entry_id) =
                            resolver.find_if_entry_by_name(port_id, host_id).await
                    {
                        stats.ports_resolved += 1;
                        Some(Neighbor::Interface(remote_if_entry_id))
                    } else {
                        Some(Neighbor::Host(host_id))
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // Persist resolved neighbor
            if let Some(neighbor) = resolved_neighbor {
                interface.base.neighbor = Some(neighbor);
                self.interface_service
                    .update(&mut interface, AuthenticatedEntity::System)
                    .await?;
            }
        }

        tracing::info!(
            network_id = %network_id,
            total = stats.total,
            hosts_resolved = stats.hosts_resolved,
            ports_resolved = stats.ports_resolved,
            "LLDP/CDP link resolution complete"
        );

        Ok(stats)
    }

    /// Resolve FDB (bridge forwarding database) single-MAC ports to neighbor links.
    /// Called after resolve_lldp_links — only processes ports without LLDP/CDP data
    /// that have exactly one learned MAC address (direct physical connection).
    pub async fn resolve_fdb_links(&self, network_id: Uuid) -> Result<u32> {
        use crate::server::interfaces::r#impl::base::Neighbor;

        let resolver = LldpResolverImpl::new(
            self.interface_service.clone(),
            self.ip_address_service.clone(),
            self.storage.clone(),
        );

        let filter = StorableFilter::<Interface>::new_for_unresolved_fdb_in_network(network_id);
        let unresolved = self.interface_service.get_all(filter).await?;

        let mut resolved_count: u32 = 0;

        for mut interface in unresolved {
            let mac = match &interface.base.fdb_macs {
                Some(macs) if macs.len() == 1 => &macs[0],
                _ => continue,
            };

            // Try to find host by MAC
            let host_id = match resolver.find_host_by_mac(mac, network_id).await {
                Some(id) => id,
                None => continue,
            };

            // Try full resolution (specific port)
            let neighbor =
                if let Some(interface_id) = resolver.find_if_entry_by_mac(mac, host_id).await {
                    Neighbor::Interface(interface_id)
                } else {
                    Neighbor::Host(host_id)
                };

            interface.base.neighbor = Some(neighbor);
            self.interface_service
                .update(&mut interface, AuthenticatedEntity::System)
                .await?;
            resolved_count += 1;
        }

        if resolved_count > 0 {
            tracing::info!(
                network_id = %network_id,
                resolved = resolved_count,
                "FDB link resolution complete"
            );
        }

        Ok(resolved_count)
    }

    /// Delete a host (children cascade via FK)
    pub async fn delete_host(&self, id: &Uuid, authentication: AuthenticatedEntity) -> Result<()> {
        // Can't delete host with daemon
        if self
            .daemon_service
            .get_one(StorableFilter::<Daemon>::new_from_host_ids(&[*id]))
            .await?
            .is_some()
        {
            return Err(ValidationError::new(
                "Can't delete a host with an associated daemon. Delete the daemon first.",
            )
            .into());
        }

        let host = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Host {} not found", id))?;

        let lock = self.get_host_lock(id).await;
        let _guard = lock.lock().await;

        // Remove tags from junction table
        if let Some(tag_service) = self.entity_tag_service() {
            tag_service
                .remove_all_for_entity(*id, EntityDiscriminants::Host)
                .await?;
        }

        // Delete host - children cascade via ON DELETE CASCADE
        self.storage().delete(id).await?;

        let trigger_stale = host.triggers_staleness(None);

        if let Some(scope) = EntityScope::from_ids(
            host.id(),
            host.clone().into(),
            self.get_network_id(&host),
            self.get_organization_id(&host),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Deleted, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(())
    }
}

/// Statistics from LLDP link resolution.
#[derive(Default, Debug)]
pub struct LldpResolutionStats {
    /// Total number of interfaces with unresolved LLDP data
    pub total: usize,
    /// Number of interfaces where remote host was resolved
    pub hosts_resolved: usize,
    /// Number of interfaces where remote port (interface) was resolved
    pub ports_resolved: usize,
}

/// Check whether a claimer's `(port_id, ip_address_id)` overlaps with an
/// Open Ports binding's `(port_id, ip_address_id)`.
/// Uses the same semantics as `partition_conflicting_bindings`:
/// None (all ip_addresses) overlaps with anything, Some(a) overlaps Some(a).
fn bindings_overlap(claim_iface: &Option<Uuid>, op_iface: &Option<Uuid>) -> bool {
    match (claim_iface, op_iface) {
        (None, _) | (_, None) => true,
        (Some(a), Some(b)) => a == b,
    }
}

/// Detect VRRP/HSRP virtual router MAC addresses by their well-known prefixes.
///
/// Virtual router protocols assign deterministic MACs shared across physical router peers.
/// These must be excluded from host identity matching to prevent different physical routers
/// in the same redundancy group from being deduped into a single host.
///
/// The VRRP/HSRP group ID is encoded in the last byte(s) of the MAC itself, so detection
/// requires only the MAC prefix — no SNMP MIB query needed.
fn is_virtual_router_mac(mac: &MacAddress) -> bool {
    let bytes = mac.bytes();
    // VRRP (RFC 5798): 00:00:5e:00:01:XX where XX = VRRP group ID (0-255)
    (bytes[0..5] == [0x00, 0x00, 0x5e, 0x00, 0x01])
    // HSRP v1 (Cisco): 00:00:0c:07:ac:XX where XX = HSRP group ID (0-255)
    || (bytes[0..5] == [0x00, 0x00, 0x0c, 0x07, 0xac])
    // HSRP v2 (Cisco): 00:00:0c:9f:fX:XX where X:XX = HSRP group ID (0-4095)
    || (bytes[0..4] == [0x00, 0x00, 0x0c, 0x9f] && (bytes[4] & 0xf0) == 0xf0)
}

/// Compare two ip_addresses for host dedup matching.
///
/// Three match branches, checked in order:
/// 1. **IP+subnet** (primary): same IP on the same subnet = same logical interface
/// 2. **ID** (secondary): same non-nil database UUID = known same record
/// 3. **MAC** (tertiary, conditional): same MAC address, but only when the MAC is unique
///    among incoming ip_addresses (count == 1). Shared MACs (count > 1) indicate VLAN
///    sub-interfaces, bridge members, or bond members — distinct ip_addresses that must
///    not be collapsed. Unique MACs indicate a standalone interface (e.g., a Docker
///    container whose IP changed via DHCP) where MAC is a valid identity anchor.
fn ip_addresses_match(
    incoming: &IPAddress,
    existing: &IPAddress,
    incoming_mac_counts: &HashMap<MacAddress, usize>,
) -> bool {
    // Primary: same IP on same subnet
    (incoming.base.ip_address == existing.base.ip_address
        && incoming.base.subnet_id == existing.base.subnet_id)
    // Secondary: same non-nil ID
    || (incoming.id == existing.id
        && incoming.id != Uuid::nil()
        && existing.id != Uuid::nil())
    // Tertiary: MAC match, gated on incoming MAC uniqueness
    || (incoming.base.mac_address.is_some()
        && incoming.base.mac_address == existing.base.mac_address
        && incoming
            .base
            .mac_address
            .map(|mac| incoming_mac_counts.get(&mac).copied().unwrap_or(0) == 1)
            .unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;

    fn make_interface(ip: IpAddr, subnet_id: Uuid, mac: Option<MacAddress>) -> IPAddress {
        IPAddress {
            id: Uuid::new_v4(),
            base: IPAddressBase {
                ip_address: ip,
                subnet_id,
                mac_address: mac,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    // --- is_virtual_router_mac tests ---

    #[test]
    fn vrrp_mac_detected() {
        // VRRP (RFC 5798): 00:00:5e:00:01:XX
        let mac = MacAddress::new([0x00, 0x00, 0x5e, 0x00, 0x01, 0x01]);
        assert!(is_virtual_router_mac(&mac), "VRRP MAC should be detected");
    }

    #[test]
    fn hsrp_v1_mac_detected() {
        // HSRP v1: 00:00:0c:07:ac:XX
        let mac = MacAddress::new([0x00, 0x00, 0x0c, 0x07, 0xac, 0x0a]);
        assert!(
            is_virtual_router_mac(&mac),
            "HSRP v1 MAC should be detected"
        );
    }

    #[test]
    fn hsrp_v2_mac_detected() {
        // HSRP v2: 00:00:0c:9f:fX:XX
        let mac = MacAddress::new([0x00, 0x00, 0x0c, 0x9f, 0xf0, 0x0a]);
        assert!(
            is_virtual_router_mac(&mac),
            "HSRP v2 MAC should be detected"
        );
    }

    #[test]
    fn normal_mac_not_virtual_router() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        assert!(
            !is_virtual_router_mac(&mac),
            "Regular MAC should not be detected as virtual router"
        );
    }

    // --- ip_addresses_match tests ---

    #[test]
    fn match_by_ip_subnet() {
        let subnet = Uuid::new_v4();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let a = make_interface(ip, subnet, None);
        let b = make_interface(ip, subnet, None);
        let counts = HashMap::new();
        assert!(ip_addresses_match(&a, &b, &counts));
    }

    #[test]
    fn no_match_different_ip_subnet() {
        let a = make_interface("10.0.0.1".parse().unwrap(), Uuid::new_v4(), None);
        let b = make_interface("20.0.0.1".parse().unwrap(), Uuid::new_v4(), None);
        let counts = HashMap::new();
        assert!(!ip_addresses_match(&a, &b, &counts));
    }

    #[test]
    fn mac_match_when_unique_in_batch() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let a = make_interface("10.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let b = make_interface("20.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        // MAC appears only once in the incoming batch — standalone ip_address, safe to match
        let counts = HashMap::from([(mac, 1)]);
        assert!(
            ip_addresses_match(&a, &b, &counts),
            "Unique MAC in batch should allow MAC matching (Docker/DHCP case)"
        );
    }

    #[test]
    fn mac_no_match_when_shared_in_batch() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let a = make_interface("10.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let b = make_interface("20.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        // MAC appears 3 times in the incoming batch — VLAN sub-interfaces, must not match
        let counts = HashMap::from([(mac, 3)]);
        assert!(
            !ip_addresses_match(&a, &b, &counts),
            "Shared MAC in batch (VLANs) must not match"
        );
    }

    // --- #600 characterization: multi-IP host on a single MAC ---
    //
    // These document the *actual* dedup decision for a host that responds on multiple
    // IP addresses behind one MAC (multi-homed server, IP aliases). They are evidence,
    // not a fix: they pin down what the current heuristic does so we can tell whether
    // the reported "tied to lowest IP" behavior is a real defect or expected dedup.

    /// Mirror how `find_matching_host_by_ip_addresses` builds `incoming_mac_counts`:
    /// the count is computed from the IP addresses of a *single incoming payload*.
    fn mac_counts_for_payload(payload: &[IPAddress]) -> HashMap<MacAddress, usize> {
        payload
            .iter()
            .filter_map(|i| i.base.mac_address)
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            })
    }

    #[test]
    fn mac_match_same_subnet_when_unique_in_batch() {
        // Multi-homed / IP-alias case: two IPs on the SAME subnet sharing one MAC.
        // Primary (ip+subnet) branch fails (different IP); MAC branch (count==1) must
        // carry the match so both IPs collapse onto one host.
        let subnet = Uuid::new_v4();
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let a = make_interface("192.168.1.10".parse().unwrap(), subnet, Some(mac));
        let b = make_interface("192.168.1.11".parse().unwrap(), subnet, Some(mac));
        let counts = HashMap::from([(mac, 1)]);
        assert!(
            ip_addresses_match(&a, &b, &counts),
            "Same MAC + same subnet, unique in batch, should match (IP-alias multi-homing)"
        );
    }

    #[test]
    fn multi_homed_host_separate_payloads_merge() {
        // The real discovery flow: each scanned IP arrives as its own single-IP payload,
        // so `incoming_mac_counts` for that payload is always {MAC: 1}. Therefore the
        // VLAN guard never trips across payloads, and the second IP merges into the host
        // created by the first — one Host, multiple IPAddress children. This is the
        // model-(a) outcome, contradicting "a separate host per IP".
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02]);

        // Payload 1: first IP (different subnet to exercise the MAC branch, not ip+subnet)
        let first = make_interface("192.168.1.50".parse().unwrap(), Uuid::new_v4(), Some(mac));
        // Payload 2: second IP, scanned independently
        let second = make_interface("10.0.0.50".parse().unwrap(), Uuid::new_v4(), Some(mac));

        // Counts are per-payload; each single-IP payload yields {mac: 1}.
        let counts_for_second = mac_counts_for_payload(std::slice::from_ref(&second));
        assert_eq!(counts_for_second.get(&mac), Some(&1));

        assert!(
            ip_addresses_match(&second, &first, &counts_for_second),
            "Independently scanned same-MAC IPs must merge into one host (model a)"
        );
    }

    #[test]
    fn multi_ip_single_payload_does_not_mac_merge() {
        // Contrast: when one payload carries BOTH same-MAC IPs (count==2), the MAC branch
        // is intentionally disabled (treated as VLAN/bridge sub-interfaces). They are not
        // merged *via MAC*; they remain distinct IPAddress rows under whatever host owns
        // them. Guards the existing behavior so a future fix can't regress it silently.
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x03]);
        let a = make_interface("172.16.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let b = make_interface("172.16.5.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let counts = mac_counts_for_payload(&[a.clone(), b.clone()]);
        assert_eq!(counts.get(&mac), Some(&2));
        assert!(
            !ip_addresses_match(&a, &b, &counts),
            "Two same-MAC IPs in one payload must not MAC-merge (VLAN sub-interface guard)"
        );
    }
}
