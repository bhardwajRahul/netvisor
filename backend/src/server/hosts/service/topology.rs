//! LLDP and FDB link resolution.
use super::*;

/// How many unmatched neighbours the summary names before eliding the rest. Matches the cap the
/// daemon's scan warnings use, for the same reason: a line long enough to scroll is not read.
const MAX_LISTED_UNMATCHED: usize = 10;

/// A neighbour advertised by a local interface whose far end no strategy could place.
///
/// Holds what identifies both ends — which of our devices saw it, on which port, and the
/// identifier the far end advertised — because those are the three things needed to decide whether
/// an unresolved neighbour is a device that should have been scanned or one that never will be.
struct UnmatchedNeighbour {
    /// The local device that saw the neighbour, not the far end — the far end is what we failed
    /// to identify.
    host_id: Uuid,
    if_descr: String,
    /// The chassis ID (LLDP) or device id (CDP) that matched nothing.
    identifier: String,
    sys_name: Option<String>,
}

impl UnmatchedNeighbour {
    fn new(interface: &Interface, identifier: String, sys_name: Option<String>) -> Self {
        Self {
            host_id: interface.base.host_id,
            if_descr: interface.base.if_descr.clone(),
            identifier,
            sys_name,
        }
    }

    /// `switch7 ten-gigabitEthernet 1/0/1 -> 00:ad:24:89:cc:f0 (core-sw)`, with the sysName only
    /// when the device sent one — it is often the only human-readable clue to what the far end is.
    fn describe(&self, host_name: Option<&String>) -> String {
        let host = host_name.map(String::as_str).unwrap_or("unknown host");
        let sys_name = match &self.sys_name {
            Some(name) if !name.trim().is_empty() => format!(" ({name})"),
            _ => String::new(),
        };
        format!("{host} {} -> {}{sys_name}", self.if_descr, self.identifier)
    }
}

impl HostService {
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
        let resolver = LldpResolverImpl::new(
            self.interface_service.clone(),
            self.ip_address_service.clone(),
            self.storage.clone(),
        );

        // Every interface in this network whose remote *port* isn't known yet — including ones
        // already resolved as far as the remote host, whose port half is retried below.
        let filter =
            StorableFilter::<Interface>::new_for_unresolved_lldp_port_in_network(network_id);
        let unresolved = self.interface_service.get_all(filter).await?;

        let mut stats = LldpResolutionStats::default();
        // Every far end no strategy could place, kept so the summary below can name them.
        //
        // `host_not_found` on its own says only how many there were, and it is the one counter
        // that does not move between scans: an unresolvable row keeps `neighbor_interface_id`
        // NULL, so the filter re-selects it every pass and the count is a standing population
        // rather than a per-run delta. A reporter seeing the same figure twice cannot tell a
        // stable set of genuinely-unknown neighbours (endpoints, phones, unmanaged gear — the
        // expected case) from a resolution defect without knowing *which* devices they are
        // (GH #668).
        let mut unmatched: Vec<UnmatchedNeighbour> = Vec::new();

        for mut interface in unresolved {
            stats.total += 1;

            // A previous pass may already have identified the remote host but not the port. Keep
            // that result and retry only the port, so a partial can never regress to nothing.
            let known_host_id = match interface.base.neighbor {
                Some(Neighbor::Host(host_id)) => Some(host_id),
                _ => None,
            };

            // Only chassis_id and port_id are used for neighbor resolution — they represent
            // actual physical connections. lldp_mgmt_addr / cdp_address are where you manage the
            // device, not necessarily the physical connection point.
            let resolved_neighbor = if let Some(ref chassis_id) = interface.base.lldp_chassis_id {
                let host = match known_host_id {
                    Some(host_id) => IdentityResolution::Resolved(host_id),
                    None => {
                        chassis_id
                            .resolve_host_id(
                                &resolver,
                                network_id,
                                interface.base.lldp_sys_name.as_deref(),
                            )
                            .await
                    }
                };
                if matches!(host, IdentityResolution::NotFound) {
                    unmatched.push(UnmatchedNeighbour::new(
                        &interface,
                        chassis_id.identifier(),
                        interface.base.lldp_sys_name.clone(),
                    ));
                }
                match stats.record_host(host) {
                    None => None,
                    Some(host_id) => {
                        let port = match interface.base.lldp_port_id {
                            Some(ref port_id) => {
                                port_id.resolve_if_entry_id(&resolver, host_id).await
                            }
                            None => IdentityResolution::NoStrategy,
                        };
                        // Last resort: the port *description*. Distinct from the port id and
                        // sometimes the only one that matches — a D-Link DGS-1210-48 advertises
                        // the id as a bare port number but describes the port as
                        // "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 9", which is byte-identical to
                        // that switch's own ifDescr (GH #668). Only consulted once the id has
                        // failed, so a device whose id resolves is unaffected, and scoped to the
                        // already-resolved host like every other tier.
                        let port = match port {
                            IdentityResolution::Resolved(id) => IdentityResolution::Resolved(id),
                            unresolved => match interface.base.lldp_port_desc.as_deref() {
                                Some(desc) if !desc.trim().is_empty() => {
                                    match resolver.find_if_entry_by_name(desc, host_id).await {
                                        Some(id) => IdentityResolution::Resolved(id),
                                        // Keep the port id's own verdict rather than overwriting
                                        // it: `NoStrategy` and `NotFound` are counted separately
                                        // and mean different things to whoever reads the stats.
                                        None => unresolved,
                                    }
                                }
                                _ => unresolved,
                            },
                        };
                        Some(stats.record_port(port, host_id))
                    }
                }
            } else if let Some(ref device_id) = interface.base.cdp_device_id {
                // CDP device_id is typically sysName, resolve against sys_name field
                let host = match known_host_id {
                    Some(host_id) => IdentityResolution::Resolved(host_id),
                    None => IdentityResolution::found(
                        resolver.find_host_by_sys_name(device_id, network_id).await,
                    ),
                };
                if matches!(host, IdentityResolution::NotFound) {
                    unmatched.push(UnmatchedNeighbour::new(&interface, device_id.clone(), None));
                }
                match stats.record_host(host) {
                    None => None,
                    Some(host_id) => {
                        // CDP port ids are the long ifDescr form
                        let port = match interface.base.cdp_port_id {
                            Some(ref port_id) => IdentityResolution::found(
                                resolver.find_if_entry_by_name(port_id, host_id).await,
                            ),
                            None => IdentityResolution::NoStrategy,
                        };
                        Some(stats.record_port(port, host_id))
                    }
                }
            } else {
                // Admitted by the filter on cdp_address alone, which is a management address and
                // never a physical connection — there is nothing here to resolve.
                stats.host_no_strategy += 1;
                None
            };

            // Persist the resolved neighbor. `None` leaves the row as it was: an existing partial
            // is preserved, and an unresolved row stays eligible for the next pass.
            if let Some(neighbor) = resolved_neighbor
                && Some(&neighbor) != interface.base.neighbor.as_ref()
            {
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
            host_no_strategy = stats.host_no_strategy,
            host_not_found = stats.host_not_found,
            port_no_strategy = stats.port_no_strategy,
            port_not_found = stats.port_not_found,
            "LLDP/CDP link resolution complete"
        );

        self.log_unmatched_neighbours(network_id, &unmatched).await;

        Ok(stats)
    }

    /// Name the neighbours no strategy could place, so `host_not_found` can be checked rather
    /// than inferred.
    ///
    /// One line for the whole network, capped like the daemon's scan warnings and saying how many
    /// were elided — a list that simply stops reads as though that was all of them. This is a log
    /// rather than a scan warning because neighbour resolution runs in a debounced subscriber that
    /// fires after the historical Discovery row and its warning list are already written.
    async fn log_unmatched_neighbours(&self, network_id: Uuid, unmatched: &[UnmatchedNeighbour]) {
        if unmatched.is_empty() {
            return;
        }

        // One fetch for the whole batch: this runs after every scan, and the list is dominated by
        // a handful of local devices reporting many unknown far ends each.
        let host_ids: Vec<Uuid> = unmatched
            .iter()
            .map(|u| u.host_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let names: HashMap<Uuid, String> = match self
            .get_all(StorableFilter::<Host>::new_from_entity_ids(&host_ids))
            .await
        {
            Ok(hosts) => hosts.into_iter().map(|h| (h.id, h.base.name)).collect(),
            // The identifiers below are the point of the line; losing the local host's name makes
            // it harder to read, not useless.
            Err(e) => {
                tracing::debug!(network_id = %network_id, error = %e, "Could not name the local hosts for unmatched LLDP neighbours");
                HashMap::new()
            }
        };

        let listed: Vec<String> = unmatched
            .iter()
            .take(MAX_LISTED_UNMATCHED)
            .map(|u| u.describe(names.get(&u.host_id)))
            .collect();
        let elided = unmatched.len().saturating_sub(listed.len());

        tracing::warn!(
            network_id = %network_id,
            unmatched = unmatched.len(),
            elided,
            neighbours = %listed.join("; "),
            "LLDP/CDP neighbours identify devices this network has not discovered, so they draw \
             no links. Expected where the far end is an endpoint or unmanaged device; a device \
             that should have been scanned means its identifier is not one we hold."
        );
    }

    /// Resolve FDB (bridge forwarding database) single-MAC ports to neighbor links.
    /// Called after resolve_lldp_links — only processes ports without LLDP/CDP data
    /// that have exactly one learned MAC address (direct physical connection).
    pub async fn resolve_fdb_links(&self, network_id: Uuid) -> Result<u32> {
        let resolver = LldpResolverImpl::new(
            self.interface_service.clone(),
            self.ip_address_service.clone(),
            self.storage.clone(),
        );

        let filter = StorableFilter::<Interface>::new_for_unresolved_fdb_in_network(network_id);
        let unresolved = self.interface_service.get_all(filter).await?;

        let mut resolved_count: u32 = 0;
        // GH #649 diagnostics: track why FDB resolution produced (or didn't produce) L2 links.
        let total_candidates = unresolved.len();
        let mut single_mac = 0usize;
        let mut host_matched = 0usize;

        for mut interface in unresolved {
            let mac = match &interface.base.fdb_macs {
                Some(macs) if macs.len() == 1 => &macs[0],
                _ => continue,
            };
            single_mac += 1;

            // Try to find host by MAC
            let host_id = match resolver.find_host_by_mac(mac, network_id).await {
                Some(id) => id,
                None => continue,
            };
            host_matched += 1;

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

        // Always log (even at zero) so a "no L2 links" report shows where resolution fell off:
        // no candidates (nothing collected FDB), candidates but none single-MAC (shared/uplink
        // ports), single-MAC but no host owns that MAC, or resolved.
        tracing::debug!(
            network_id = %network_id,
            total_candidates = total_candidates,
            single_mac = single_mac,
            host_matched = host_matched,
            resolved = resolved_count,
            "FDB link resolution complete"
        );

        Ok(resolved_count)
    }

    /// GH #649 diagnostics: after a scan's neighbor resolution completes, summarize the L2 edge
    /// state of the whole network in one line. `full_edges` is how many interfaces the L2 map will
    /// actually render (`Neighbor::Interface`); `partial` are host-only resolutions; `dangling`
    /// are interfaces whose neighbor points to an interface id that no longer exists — data that
    /// survived but silently produces no edge, a distinct failure mode from an over-eager prune.
    pub async fn log_l2_topology_summary(&self, network_id: Uuid) {
        let filter = StorableFilter::<Interface>::new_from_network_ids(&[network_id]).live();
        let interfaces = match self.interface_service.get_all(filter).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(network_id = %network_id, error = %e, "L2 summary: failed to load interfaces");
                return;
            }
        };

        let present_ids: HashSet<Uuid> = interfaces.iter().map(|i| i.id).collect();
        let mut full_edges = 0usize;
        let mut partial = 0usize;
        let mut dangling = 0usize;
        for iface in &interfaces {
            match &iface.base.neighbor {
                Some(Neighbor::Interface(id)) => {
                    if present_ids.contains(id) {
                        full_edges += 1;
                    } else {
                        dangling += 1;
                    }
                }
                Some(Neighbor::Host(_)) => partial += 1,
                None => {}
            }
        }

        tracing::debug!(
            network_id = %network_id,
            total_interfaces = interfaces.len(),
            full_edges = full_edges,
            partial = partial,
            dangling = dangling,
            "L2 topology summary after discovery"
        );
    }
}
