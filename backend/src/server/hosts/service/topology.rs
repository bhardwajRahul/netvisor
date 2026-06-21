//! LLDP and FDB link resolution.
use super::*;

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
}
