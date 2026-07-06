//! Host creation from API requests and the create-with-children path.
use super::*;

/// Decide whether the end-of-scan interface prune (delete interfaces no longer reported for a
/// host) should run for this upsert. It runs only when BOTH hold:
///   - `interfaces_complete`: the incoming set is an authoritative, complete ifTable. A partial
///     SNMP walk cut short by timeout/error reports fewer interfaces than really exist; pruning
///     against it deletes live interfaces and their server-resolved L2 neighbors (GH #649).
///   - `incoming_count > 0`: a total SNMP failure / credential removal yields zero interfaces,
///     which means "none observed", not "all removed".
///
/// Returning false preserves existing interface rows — the safe direction (stale beats deleted).
pub(crate) fn should_prune_interfaces(interfaces_complete: bool, incoming_count: usize) -> bool {
    interfaces_complete && incoming_count > 0
}

impl HostService {
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
            // A manual API create provides the full, authoritative interface list.
            true,
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
    pub(crate) async fn create_with_children(
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
        // Whether `interfaces` is an authoritative, complete ifTable. When false (a partial SNMP
        // walk), the stale-interface prune is skipped so a transient partial scan cannot delete
        // interfaces and their resolved L2 neighbors (GH #649).
        interfaces_complete: bool,
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
        // Subnet ids known-good because this call just created (or deduped to) them.
        // An ip_address referencing one of these needs no dangling-subnet repair,
        // so the common path skips the live-subnet lookup entirely.
        let mut created_subnet_ids: std::collections::HashSet<Uuid> =
            std::collections::HashSet::new();
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
            created_subnet_ids.insert(created.id);
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

        // Live subnets for this host's network, loaded lazily to repair any
        // ip_address whose subnet_id references no live subnet row — e.g. an old
        // daemon reporting the server-seeded loopback subnet under a UUID this
        // server never minted (see HostService::seed_loopback). Resolve-only:
        // never creates a subnet; on no CIDR match the insert error surfaces.
        let mut network_live_subnets: Option<Vec<Subnet>> = None;

        let mut created_ip_addresses = Vec::new();
        for mut ip_address in ip_addresses {
            ip_address.base.host_id = created_host.id;

            // Remap subnet_id if the subnet was deduped to an existing one
            if let Some(&new_subnet_id) = subnet_id_remap.get(&ip_address.base.subnet_id) {
                ip_address.base.subnet_id = new_subnet_id;
            }

            // Repair a dangling subnet_id (one referencing no live subnet row) by
            // mapping the IP to the most-specific live subnet on the network that
            // contains it. Guards the ip_addresses.subnet_id -> subnets FK against
            // stale references (e.g. an old daemon reporting the server-seeded
            // loopback subnet under a UUID this server re-minted; see seed_loopback).
            //
            // A subnet_id this call just created/deduped is known-good, so the
            // common host (every IP references an in-request subnet) skips the
            // live-subnet lookup entirely. The lookup is lazy and loads once.
            if !created_subnet_ids.contains(&ip_address.base.subnet_id) {
                if network_live_subnets.is_none() {
                    network_live_subnets = Some(
                        self.subnet_service
                            .get_all(
                                StorableFilter::<Subnet>::new_from_network_ids(&[created_host
                                    .base
                                    .network_id])
                                .live(),
                            )
                            .await?,
                    );
                }
                let live_subnets = network_live_subnets.as_ref().expect("loaded above");
                if let Some(resolved) = resolve_dangling_subnet_id(
                    live_subnets,
                    ip_address.base.subnet_id,
                    ip_address.base.ip_address,
                ) {
                    tracing::warn!(
                        ip = %ip_address.base.ip_address,
                        dangling_subnet_id = %ip_address.base.subnet_id,
                        resolved_subnet_id = %resolved,
                        "Repaired ip_address referencing a non-existent subnet via CIDR match"
                    );
                    ip_address.base.subnet_id = resolved;
                }
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
                            let _ = self
                                .service_service
                                .update(&mut updated, authentication.clone())
                                .await;
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
                let _ = self
                    .service_service
                    .update(&mut updated, authentication.clone())
                    .await;
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
                        let _ = self
                            .service_service
                            .update(&mut updated, authentication.clone())
                            .await;
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
        //
        // `claimed` tracks rows already matched/created in this batch so two
        // incoming ifTable entries can't collapse onto one existing row — e.g. an
        // L2 switch whose IP-less ports all share the chassis MAC and lack ifName
        // would otherwise all tier-3 onto the management interface (issue #614).
        let mut created_interfaces = Vec::new();
        let mut claimed: HashSet<Uuid> = HashSet::new();
        for mut entry in interfaces {
            entry.base.host_id = created_host.id;
            entry.base.network_id = created_host.base.network_id;

            let created = self
                .interface_service
                .create_or_update_from_discovery(entry, &claimed, authentication.clone())
                .await?;
            claimed.insert(created.id);
            created_interfaces.push(created);
        }

        // Full-sweep prune of interfaces no longer reported for this host.
        //
        // Only runs when the incoming interface set is an authoritative, COMPLETE ifTable
        // (`interfaces_complete`). An SNMP walk cut short by timeout/error returns a partial,
        // smaller-than-real ifTable (see daemon `walk_if_table`); pruning against it would delete
        // live interfaces and the server-resolved L2 neighbors on them, tearing switches off the
        // L2 topology map on every scan that hiccups (GH #649). Two guards, both required:
        //   - `interfaces_complete`: skip pruning on a partial walk (daemon-signalled).
        //   - non-empty set: a total SNMP failure / credential removal yields zero interfaces,
        //     which is "none observed", not "all removed".
        if should_prune_interfaces(interfaces_complete, created_interfaces.len()) {
            let kept_ids: HashSet<Uuid> = created_interfaces.iter().map(|i| i.id).collect();
            let existing = self
                .interface_service
                .get_for_host(&created_host.id)
                .await?;
            let existing_count = existing.len();
            let mut pruned = 0usize;
            for iface in existing {
                if !kept_ids.contains(&iface.id) {
                    match self
                        .interface_service
                        .delete(&iface.id, authentication.clone())
                        .await
                    {
                        Ok(_) => pruned += 1,
                        Err(e) => tracing::warn!(
                            host_id = %created_host.id,
                            interface_id = %iface.id,
                            error = %e,
                            "Failed to prune orphan interface"
                        ),
                    }
                }
            }
            if pruned > 0 {
                tracing::info!(
                    host_id = %created_host.id,
                    interfaces_complete = true,
                    incoming = created_interfaces.len(),
                    existing = existing_count,
                    pruned = pruned,
                    "Pruned interfaces no longer reported by a complete ifTable scan"
                );
            }
        } else if !created_interfaces.is_empty() {
            // Non-empty incoming set that we chose NOT to prune against ⇒ an incomplete (partial)
            // walk. Surface it so a self-hosted operator can see why stale interfaces persist —
            // and how many interfaces this partial scan would have deleted had the fix not gated it.
            let existing_count = self
                .interface_service
                .get_for_host(&created_host.id)
                .await
                .map(|v| v.len())
                .unwrap_or_default();
            tracing::info!(
                host_id = %created_host.id,
                interfaces_complete = false,
                incoming = created_interfaces.len(),
                existing = existing_count,
                "Skipped interface prune: SNMP ifTable walk was incomplete (partial scan) — preserving existing interfaces and L2 links"
            );
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
        // MERGE (not replace): discovery self-reports only credentials that probed successfully,
        // so replacing would prune user/init-assigned daemon-host creds that didn't probe this
        // scan. Merge adds the freshly-discovered assignments without deleting existing ones.
        if !remapped_assignments.is_empty()
            && let Err(e) = self
                .credential_service
                .merge_host_credentials(&created_host.id, &remapped_assignments)
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
}

/// Resolve a dangling `subnet_id` — one that references no live subnet row — to
/// the most-specific live subnet on the network whose CIDR contains `ip`.
///
/// Returns `None` when `subnet_id` already matches a live subnet (no repair
/// needed) or when no live subnet contains the IP (unresolvable — the caller
/// leaves the reference untouched and lets the FK insert error surface). On
/// overlapping CIDRs the longest prefix wins, so an IP is never mis-attributed
/// to a broader subnet while a more-specific one also contains it.
fn resolve_dangling_subnet_id(
    live_subnets: &[Subnet],
    subnet_id: Uuid,
    ip: std::net::IpAddr,
) -> Option<Uuid> {
    if live_subnets.iter().any(|s| s.id == subnet_id) {
        return None;
    }
    live_subnets
        .iter()
        .filter(|s| s.base.cidr.contains(&ip))
        .max_by_key(|s| s.base.cidr.network_length())
        .map(|s| s.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::subnets::r#impl::base::SubnetBase;

    fn subnet(id: Uuid, cidr: &str) -> Subnet {
        Subnet {
            id,
            base: SubnetBase {
                cidr: cidr.parse().expect("valid test CIDR"),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn ip(s: &str) -> std::net::IpAddr {
        s.parse().expect("valid test IP")
    }

    #[test]
    fn repairs_dangling_loopback_to_seeded_subnet() {
        // The compat case: the daemon reports 127.0.0.1 under a loopback subnet id
        // this server never minted; the live seeded loopback subnet is `seeded`.
        let seeded = Uuid::new_v4();
        let live = vec![
            subnet(seeded, "127.0.0.0/8"),
            subnet(Uuid::new_v4(), "172.25.0.0/28"),
        ];
        let dangling = Uuid::new_v4();
        assert_eq!(
            resolve_dangling_subnet_id(&live, dangling, ip("127.0.0.1")),
            Some(seeded)
        );
    }

    #[test]
    fn valid_subnet_id_is_left_untouched() {
        let valid = Uuid::new_v4();
        let live = vec![subnet(valid, "127.0.0.0/8")];
        // subnet_id already resolves to a live row -> no repair, no re-matching.
        assert_eq!(
            resolve_dangling_subnet_id(&live, valid, ip("127.0.0.1")),
            None
        );
    }

    #[test]
    fn overlapping_cidrs_resolve_to_most_specific() {
        let broad = Uuid::new_v4();
        let specific = Uuid::new_v4();
        // Both contain 10.0.0.5; the /24 must win over the /16.
        let live = vec![
            subnet(broad, "10.0.0.0/16"),
            subnet(specific, "10.0.0.0/24"),
        ];
        let dangling = Uuid::new_v4();
        assert_eq!(
            resolve_dangling_subnet_id(&live, dangling, ip("10.0.0.5")),
            Some(specific)
        );
    }

    #[test]
    fn unresolvable_ip_returns_none() {
        // No live subnet contains the IP -> leave the reference so the insert
        // error surfaces rather than silently mis-attributing the IP.
        let live = vec![subnet(Uuid::new_v4(), "10.0.0.0/24")];
        let dangling = Uuid::new_v4();
        assert_eq!(
            resolve_dangling_subnet_id(&live, dangling, ip("192.168.1.1")),
            None
        );
    }

    // GH #649: the interface prune must NOT run on a partial SNMP walk, or a transient timeout
    // deletes live switch ports and their resolved L2 neighbors every scan. These lock the gate.
    #[test]
    fn partial_walk_never_prunes_even_with_interfaces_present() {
        // The bug: an incomplete walk reported some (but not all) interfaces; pruning against it
        // would delete every interface it missed. It must be skipped.
        assert!(!should_prune_interfaces(false, 5));
    }

    #[test]
    fn complete_walk_with_interfaces_prunes() {
        // An authoritative full ifTable is the only time stale interfaces may be removed.
        assert!(should_prune_interfaces(true, 5));
    }

    #[test]
    fn empty_set_never_prunes_regardless_of_completeness() {
        // Zero interfaces = "none observed" (total SNMP failure / cred removal), not "all removed".
        assert!(!should_prune_interfaces(true, 0));
        assert!(!should_prune_interfaces(false, 0));
    }
}
