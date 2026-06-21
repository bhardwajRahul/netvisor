//! Daemon-discovery upsert path: interface linking and subnet/VLAN reconciliation.
use super::*;

impl HostService {
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
}
