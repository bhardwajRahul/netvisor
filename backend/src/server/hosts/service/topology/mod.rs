//! LLDP and FDB link resolution.
use super::*;

/// How many unmatched neighbours the summary names before eliding the rest. Matches the cap the
/// daemon's scan warnings use, for the same reason: a line long enough to scroll is not read.
const MAX_LISTED_UNMATCHED: usize = 10;

/// Why a far end could not be placed, carried alongside the identifiers that were tried.
///
/// The distinction is the whole point of naming these at all: `NotFound` is a gap in what has been
/// scanned and the operator can close it, `Ambiguous` is a device that *is* scanned but reports one
/// identifier for many ports, and `NoStrategy` is a gap on our side. Reporting all three as "did
/// not resolve" is what made a device-level edge un-triagable without a customer snmpwalk (GH #668).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnresolvedReason {
    NoStrategy,
    NotFound,
    Ambiguous,
}

impl UnresolvedReason {
    /// `None` for a resolution that succeeded — there is nothing to report.
    fn from_resolution(resolution: IdentityResolution) -> Option<Self> {
        match resolution {
            IdentityResolution::Resolved(_) => None,
            IdentityResolution::NoStrategy => Some(Self::NoStrategy),
            IdentityResolution::NotFound => Some(Self::NotFound),
            IdentityResolution::Ambiguous => Some(Self::Ambiguous),
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Self::NoStrategy => "no lookup strategy for this port-id subtype",
            Self::NotFound => "no port on that device matches",
            Self::Ambiguous => "several ports on that device match, so it identifies none",
        }
    }
}

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

/// A neighbour whose far-end *device* is known but whose port could not be identified.
///
/// This is the row that draws a device-level edge instead of a port-to-port one — the "attached to
/// the whole switch" outcome. `host_not_found` already names the far ends we have never seen; this
/// names the ones we have, which is the harder case to reason about from a counter alone: the
/// devices are both on the map and the operator has nothing left to scan.
struct UnresolvedPort {
    /// The local device that saw the neighbour, and the port it saw it on.
    host_id: Uuid,
    if_descr: String,
    /// The far-end device, already resolved — this is what makes it distinct from
    /// [`UnmatchedNeighbour`].
    remote_host_id: Uuid,
    /// The advertised port id in `Debug` form, which carries subtype and value together
    /// (`MacAddress("00:ad:24:af:4e:00")`, `InterfaceName("2")`). Both halves are needed: the
    /// subtype says which tier ran and the value says what it looked for.
    port_id: Option<String>,
    /// `lldpRemPortDesc`, the last-resort tier. Present here because "the id failed and the
    /// description was empty" and "both were tried and neither matched" call for different fixes.
    port_desc: Option<String>,
    reason: UnresolvedReason,
}

impl UnresolvedPort {
    fn new(
        interface: &Interface,
        remote_host_id: Uuid,
        port_id: Option<String>,
        reason: UnresolvedReason,
    ) -> Self {
        Self {
            host_id: interface.base.host_id,
            if_descr: interface.base.if_descr.clone(),
            remote_host_id,
            port_id,
            port_desc: interface.base.lldp_port_desc.clone(),
            reason,
        }
    }

    /// `switch7 Gi0/1 -> switch3 via MacAddress("00:ad:…") desc "Port 9": several ports match`
    fn describe(&self, host_name: Option<&String>, remote_name: Option<&String>) -> String {
        let host = host_name.map(String::as_str).unwrap_or("unknown host");
        let remote = remote_name.map(String::as_str).unwrap_or("unknown host");
        let port_id = match &self.port_id {
            Some(id) => format!(" via {id}"),
            None => " with no port id".to_string(),
        };
        let desc = match &self.port_desc {
            Some(desc) if !desc.trim().is_empty() => format!(" desc {desc:?}"),
            _ => String::new(),
        };
        format!(
            "{host} {} -> {remote}{port_id}{desc}: {}",
            self.if_descr,
            self.reason.describe()
        )
    }
}

/// Who names whom across a network's LLDP/CDP adjacencies, and the port pairs that follow from it.
///
/// Built once per resolution pass from *every* interface that names a neighbour — not just the
/// unresolved ones. The count of ports joining two devices is the whole basis of the reciprocal
/// tier, and counting only the unresolved half would pair one leg of a LAG whose other leg happened
/// to resolve, which is exactly the arbitrary-port outcome the shared-MAC guard exists to prevent.
struct NeighborAdjacency {
    /// Every interface in the network that names a neighbour, in whatever resolution state.
    interfaces: Vec<Interface>,
    /// The far-end *device* verdict per interface, computed once here and reused by the resolution
    /// pass so the chassis ladder is not run twice for the same row.
    host_of: HashMap<Uuid, IdentityResolution>,
    /// Interface id -> `(far-end interface, that interface's host)`, for pairs where each side
    /// names the other on exactly one port.
    reciprocal: HashMap<Uuid, (Uuid, Uuid)>,
}

mod reciprocal;
mod reporting;

use reciprocal::PortBinding;

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
    /// Returns the statistics and the operator-facing summary of what could not be placed.
    pub async fn resolve_lldp_links(&self, network_id: Uuid) -> Result<LldpResolutionOutcome> {
        let resolver = LldpResolverImpl::new(
            self.interface_service.clone(),
            self.ip_address_service.clone(),
            self.storage.clone(),
        );

        // Who names whom, and which of those pairs are unambiguous in both directions. Computed
        // before anything is written, because it is the authority both for the reciprocal tier
        // below and for deciding whether an existing MAC-matched binding still stands.
        let mut adjacency = self.build_neighbor_adjacency(network_id, &resolver).await?;
        let reciprocal = std::mem::take(&mut adjacency.reciprocal);
        let host_of = std::mem::take(&mut adjacency.host_of);

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
        // Every far end whose device is known but whose port is not — the device-level edges.
        let mut unresolved_ports: Vec<UnresolvedPort> = Vec::new();
        let mut reopened = 0usize;
        let mut rebound = 0usize;

        for mut interface in std::mem::take(&mut adjacency.interfaces) {
            // What the row held on the way in. The tiers below may downgrade a binding and then
            // re-resolve it to the same value, and only a comparison against the *original* can
            // tell that apart from a change worth writing.
            let original_neighbor = interface.base.neighbor.clone();

            // A row that already names a port is re-examined rather than resolved: it is not a
            // link that failed, it is one that may have been placed on a MAC the far end repeats
            // across every port, which looks authoritative and is not.
            if let Some(Neighbor::Interface(bound_id)) = interface.base.neighbor {
                match self
                    .re_examine_port_binding(&interface, bound_id, &reciprocal, &resolver)
                    .await?
                {
                    PortBinding::Stands => continue,
                    PortBinding::Rebind(paired) => {
                        rebound += 1;
                        interface.base.neighbor = Some(Neighbor::Interface(paired));
                        self.interface_service
                            .update(&mut interface, AuthenticatedEntity::System)
                            .await?;
                        continue;
                    }
                    // Downgraded to the far-end device, which was never in doubt, and then run
                    // through the tiers below in this same pass so a tier that *can* name a port
                    // gets its turn immediately rather than a scan later.
                    PortBinding::Reopen(remote_host_id) => {
                        reopened += 1;
                        interface.base.neighbor = Some(Neighbor::Host(remote_host_id));
                    }
                }
            }

            // Rows admitted only because they already carry a resolved neighbour — an FDB-matched
            // port, say — have no protocol identity to run the tiers against. Persist a downgrade
            // if one just happened and move on.
            if interface.base.lldp_chassis_id.is_none()
                && interface.base.cdp_device_id.is_none()
                && interface.base.cdp_address.is_none()
            {
                self.persist_neighbor(&mut interface, &original_neighbor)
                    .await?;
                continue;
            }

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
                    // Already run once while building the adjacency — reusing the verdict is what
                    // keeps this pass at the same query cost it had before the extra tier.
                    None => host_of
                        .get(&interface.id)
                        .copied()
                        .unwrap_or(IdentityResolution::NoStrategy),
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
                        let port =
                            match Self::pair_reciprocally(port, interface.id, host_id, &reciprocal)
                            {
                                Some(paired) => {
                                    stats.ports_resolved_reciprocal += 1;
                                    paired
                                }
                                None => port,
                            };
                        if let Some(reason) = UnresolvedReason::from_resolution(port) {
                            unresolved_ports.push(UnresolvedPort::new(
                                &interface,
                                host_id,
                                interface
                                    .base
                                    .lldp_port_id
                                    .as_ref()
                                    .map(|p| format!("{p:?}")),
                                reason,
                            ));
                        }
                        Some(stats.record_port(port, host_id))
                    }
                }
            } else if let Some(ref device_id) = interface.base.cdp_device_id {
                // CDP device_id is typically sysName, resolve against sys_name field
                let host = match known_host_id {
                    Some(host_id) => IdentityResolution::Resolved(host_id),
                    None => host_of
                        .get(&interface.id)
                        .copied()
                        .unwrap_or(IdentityResolution::NoStrategy),
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
                        let port =
                            match Self::pair_reciprocally(port, interface.id, host_id, &reciprocal)
                            {
                                Some(paired) => {
                                    stats.ports_resolved_reciprocal += 1;
                                    paired
                                }
                                None => port,
                            };
                        if let Some(reason) = UnresolvedReason::from_resolution(port) {
                            unresolved_ports.push(UnresolvedPort::new(
                                &interface,
                                host_id,
                                interface
                                    .base
                                    .cdp_port_id
                                    .as_ref()
                                    .map(|id| format!("CdpPortId({id:?})")),
                                reason,
                            ));
                        }
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
            if let Some(neighbor) = resolved_neighbor {
                interface.base.neighbor = Some(neighbor);
            }
            self.persist_neighbor(&mut interface, &original_neighbor)
                .await?;
        }

        tracing::info!(
            network_id = %network_id,
            total = stats.total,
            hosts_resolved = stats.hosts_resolved,
            ports_resolved = stats.ports_resolved,
            ports_resolved_reciprocal = stats.ports_resolved_reciprocal,
            host_no_strategy = stats.host_no_strategy,
            host_not_found = stats.host_not_found,
            port_no_strategy = stats.port_no_strategy,
            port_not_found = stats.port_not_found,
            port_ambiguous = stats.port_ambiguous,
            reopened,
            rebound,
            "LLDP/CDP link resolution complete"
        );

        let warnings = self
            .report_unresolved(network_id, &unmatched, &unresolved_ports)
            .await;

        Ok(LldpResolutionOutcome { stats, warnings })
    }

    /// Put a resolution pass's findings on the scan record they belong to.
    ///
    /// Thin by design — the decision of *where* these belong lives in
    /// [`DiscoveryService::append_historical_warnings`]; this only names the session.
    pub async fn append_resolution_warnings(
        &self,
        session_id: Uuid,
        warnings: Vec<String>,
    ) -> Result<()> {
        self.discovery_service
            .append_historical_warnings(session_id, warnings)
            .await
    }

    /// Write `interface` back only when its neighbor actually changed.
    async fn persist_neighbor(
        &self,
        interface: &mut Interface,
        original: &Option<Neighbor>,
    ) -> Result<()> {
        if &interface.base.neighbor == original {
            return Ok(());
        }
        self.interface_service
            .update(interface, AuthenticatedEntity::System)
            .await?;
        Ok(())
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

            // Try full resolution (specific port). A far end that repeats one MAC across its ports
            // names no single port, so the link stays at device level rather than being attached to
            // whichever port the database returned first.
            let neighbor = match resolver.find_if_entry_by_mac(mac, host_id).await {
                IdentityResolution::Resolved(interface_id) => Neighbor::Interface(interface_id),
                _ => Neighbor::Host(host_id),
            };

            interface.base.neighbor = Some(neighbor);
            self.interface_service
                .update(&mut interface, AuthenticatedEntity::System)
                .await?;
            resolved_count += 1;
        }

        if resolved_count > 0 {
            tracing::debug!(
                network_id = %network_id,
                resolved = resolved_count,
                "FDB link resolution complete"
            );
        }

        Ok(resolved_count)
    }
}
