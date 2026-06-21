//! IP/MAC-based host matching, locking, upsert, and consolidation.
use super::*;

impl HostService {
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

    pub(crate) async fn get_host_lock(&self, host_id: &Uuid) -> Arc<Mutex<()>> {
        let mut locks = self.host_locks.lock().await;
        locks
            .entry(*host_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Merge new discovery data with existing host
    pub(crate) async fn upsert_host(
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
}
