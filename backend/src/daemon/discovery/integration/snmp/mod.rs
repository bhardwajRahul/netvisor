//! SNMP discovery integration.
//!
//! Probe: credentialed SNMP check on UDP ports 161/1161.
//! Execute: walks ifTable, queries LLDP/CDP/ARP/Entity-MIB/Bridge-FDB,
//!          enriches HostData with system info, ip_addresses, and interfaces.
//!
//! Also contains low-level SNMP utilities (queries, session management, OIDs, types).

pub mod oids;
pub mod queries;
pub mod session;
pub mod types;
pub mod values;

// Re-export commonly used items
pub use queries::{
    query_arp_table, query_bridge_fdb, query_bridge_port_mapping, query_cdp_neighbors,
    query_entity_physical, query_ip_addr_table, query_lldp_local, query_lldp_local_ports,
    query_lldp_neighbors, query_port_vlan_membership, query_system_info, query_vlan_table,
    walk_if_table,
};
pub use session::SNMP_WALK_TIMEOUT;
use session::{SNMP_PROBE_TIMEOUT, create_session};
pub use types::{
    ArpEntry, BridgeFdbEntry, CdpNeighbor, DeviceInventory, IfTableEntry, IpAddrEntry,
    LldpLocalInfo, LldpLocalPort, LldpNeighbor, PortVlanMembership, SystemInfo, VlanInfo,
};

use std::net::IpAddr;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use tokio::time::timeout;
use tracing::debug;
use uuid::Uuid;

use crate::{
    daemon::utils::scanner::{SnmpProbeOutcome, try_snmp_with_credential_on_port},
    server::{
        credentials::r#impl::{
            mapping::{
                CredentialQueryPayload, CredentialQueryPayloadDiscriminants, SnmpQueryCredential,
            },
            types::CredentialAssignment,
        },
        hosts::r#impl::base::{Host, HostBase},
        interfaces::r#impl::base::{
            IfAdminStatus, IfOperStatus, Interface, InterfaceBase, InterfaceDataComplete, if_type,
        },
        ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
        ports::r#impl::base::PortType,
        services::r#impl::patterns::ClientProbe,
        shared::types::entities::EntitySource,
        snmp::resolution::lldp::{LldpChassisId, LldpPortId},
        subnets::r#impl::base::Subnet,
    },
};

use super::{
    DiscoveryIntegration, IntegrationContext, IntegrationFailure, ProbeContext, ProbeFailure,
    ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::service::warnings::{
    AttemptOutcome, IncompleteInterfaceWalk, MalformedNeighbours, SnmpCollectionOutcome,
    SnmpGroupOutcome, SnmpWalkGroup, UnresolvedLldpPorts, snmp_walk_shortfalls,
};

/// Handle returned by a successful SNMP probe — carries the working credential and port.
pub struct SnmpProbeHandle {
    pub credential: SnmpQueryCredential,
    pub port: u16,
}

/// Run one SNMP query under `SNMP_WALK_TIMEOUT`, collapsing both a query error and a
/// timeout into `T::default()` — the empty/`None` fallback every call site already used
/// for errors alone.
///
/// Without this, a single query that never returns consumes the whole
/// `SnmpIntegration::timeout()` budget and the integration is aborted mid-sequence,
/// discarding everything collected so far. Observed on Ubiquiti switches, where
/// `query_bridge_fdb` hangs and the host ends up created with zero interfaces.
async fn query_or_default<T, Fut>(ip: IpAddr, query: &str, fut: Fut) -> T
where
    T: Default,
    Fut: std::future::Future<Output = Result<T>>,
{
    match timeout(SNMP_WALK_TIMEOUT, fut).await {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            debug!(ip = %ip, query, error = %e, "SNMP query failed");
            T::default()
        }
        Err(_) => {
            debug!(ip = %ip, query, "SNMP query timed out");
            T::default()
        }
    }
}

/// How much a probe outcome tells us, for picking between the two SNMP ports' answers.
///
/// A device that refuses us on 161 and ignores us on 1161 has told us something on 161; reporting
/// the silence would be reporting the less informative of the two.
fn probe_specificity(outcome: AttemptOutcome) -> u8 {
    match outcome {
        AttemptOutcome::Rejected => 3,
        AttemptOutcome::NotThisService | AttemptOutcome::Malformed => 2,
        _ => 1,
    }
}

pub struct SnmpIntegration;

#[async_trait]
impl DiscoveryIntegration for SnmpIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Snmp
    }

    fn estimated_seconds(&self) -> u32 {
        15
    }

    /// Must exceed the sum of every sequential walk's own timeout, or the outer cap silently
    /// kills the walks that run last — bridge FDB and per-port VLAN membership — which is
    /// exactly the data operators were reporting as missing. 13 walks at
    /// [`session::SNMP_WALK_TIMEOUT`] each is the worst case; this leaves headroom above it.
    fn timeout(&self) -> Duration {
        Duration::from_secs(900)
    }

    // No probe_gate_ports — SNMP does its own UDP port probing.

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let snmp_cred = match ctx.credential {
            CredentialQueryPayload::Snmp(cred) => cred,
            _ => return Err(ProbeFailure::malformed("Expected SNMP credential")),
        };

        let snmp_ports: &[u16] = &[161, 1161];

        // The most specific answer any port gave. A device listening on 161 and silent on 1161
        // should be reported as whatever 161 said, not as the silence from 1161 — so a refusal
        // outranks a timeout, which outranks nothing having been tried.
        let mut best: Option<(AttemptOutcome, String)> = None;

        for &port in snmp_ports {
            if ctx.cancel.is_cancelled() {
                return Err(ProbeFailure::cancelled());
            }

            // Cap the whole probe (create-session + GET) so a non-responder — v3's
            // engine-discovery especially — costs ~2s instead of up to 7s.
            let port_outcome = match timeout(
                SNMP_PROBE_TIMEOUT,
                try_snmp_with_credential_on_port(ctx.ip, snmp_cred, port),
            )
            .await
            {
                Ok(outcome) => outcome,
                Err(_) => SnmpProbeOutcome::Failed(
                    AttemptOutcome::TimedOut,
                    format!("no answer on port {port} within {SNMP_PROBE_TIMEOUT:?}"),
                ),
            };

            match port_outcome {
                SnmpProbeOutcome::Answered(detected_port) => {
                    return Ok(ProbeSuccess {
                        client_probe: ClientProbe::Snmp,
                        ports: vec![PortType::new_udp(detected_port)],
                        handle: Some(Box::new(SnmpProbeHandle {
                            credential: snmp_cred.clone(),
                            port: detected_port,
                        })),
                    });
                }
                SnmpProbeOutcome::Failed(outcome, message) => {
                    tracing::debug!(
                        ip = %ctx.ip,
                        port,
                        ?outcome,
                        error = %message,
                        "SNMP credential probe failed"
                    );
                    if best.as_ref().is_none_or(|(seen, _)| {
                        probe_specificity(outcome) > probe_specificity(*seen)
                    }) {
                        best = Some((outcome, format!("port {port}: {message}")));
                    }
                }
            }
        }

        // No "public" fallback here — the daemon injects a broadcast SNMP credential
        // with community "public" into credential_mappings, so it's tried as its own
        // integration dispatch. No special-casing needed.

        let (outcome, message) = best.unwrap_or((
            AttemptOutcome::TimedOut,
            format!("SNMP not responding on {}", ctx.ip),
        ));
        Err(ProbeFailure::with_outcome(outcome, message))
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
    ) -> Result<(), IntegrationFailure> {
        // Downcast probe handle to get the working credential and port
        let handle = ctx
            .probe_handle
            .and_then(|h| h.downcast_ref::<SnmpProbeHandle>())
            .ok_or_else(|| anyhow::anyhow!("SNMP execute called without SnmpProbeHandle"))?;

        let credential = &handle.credential;
        let port = handle.port;
        let ip = ctx.ip;

        // Open one SNMP session per host and reuse it across every query below.
        // Previously each of the ~12 queries opened its own session — and for v3 each
        // repeated the full engine-discovery handshake — so a single collection did
        // ~12 session setups. Reusing one session removes that per-query cost.
        let mut session = match create_session(ip, credential, port).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    ip = %ip,
                    error = %e,
                    "Failed to open SNMP session; skipping SNMP collection"
                );
                return Ok(());
            }
        };

        // Query system info
        let info = query_or_default(ip, "system_info", query_system_info(&mut session, ip)).await;
        let system_info = if info.sys_descr.is_some()
            || info.sys_name.is_some()
            || info.sys_object_id.is_some()
        {
            tracing::debug!(
                ip = %ip,
                sys_name = ?info.sys_name,
                "SNMP system info retrieved"
            );
            Some(info)
        } else {
            tracing::debug!(ip = %ip, "SNMP system_info returned no data");
            None
        };

        if ctx.cancel.is_cancelled() {
            return Err(IntegrationFailure::cancelled());
        }

        // Walk interface table. `if_table_complete` tells the server whether this is an
        // authoritative full ifTable (safe to prune stale interfaces against) or a partial walk
        // cut short by timeout/error (must NOT prune — see GH #649). A hard failure yields an
        // empty set, which the server's existing empty-set guard already protects.
        let if_table = query_or_default(ip, "if_table", walk_if_table(&mut session, ip)).await;
        let snmp_if_entries = if_table.entries;
        tracing::debug!(
            ip = %ip,
            if_count = snmp_if_entries.len(),
            set_complete = if_table.set_complete,
            attributes_complete = if_table.attributes_complete,
            "SNMP ifTable walked"
        );

        // Persist the interface set before the slower enrichment queries below. `host_data` is
        // `&mut`, and mutations made before a timeout-abort of `execute()` survive in the caller
        // (the integration-timeout wrapper swallows the error but keeps `host_data`), so a hang
        // in any later query can no longer strand the host with zero interfaces. Enrichment data
        // isn't collected yet, so these are bare; the enriched set replaces them at the end.
        let network_id = host_data.host.base.network_id;
        let no_vlan_uuids = std::collections::HashMap::new();
        host_data.replace_interfaces(
            snmp_if_entries
                .iter()
                .map(|entry| {
                    convert_snmp_if_entry(entry, network_id, &[], &[], &[], &[], &no_vlan_uuids)
                })
                .collect(),
        );
        // Pruning acts on the interface *set*, so that is what gates it — not whether every
        // attribute column also finished (#649).
        host_data.set_interfaces_complete(if_table.set_complete);

        // Record an incomplete walk on the session rather than leaving it to debug logs, keeping
        // which kind it was: a short interface list means interfaces are genuinely missing, while
        // a short attribute column only means some fields are blank. Reporting the second as
        // possible data loss sends operators hunting for interfaces that were never absent.
        // Rendered to one line per run at finalize — one paragraph per device drowns the
        // notification on any real network.
        let walk_fell_short = !if_table.set_complete || !if_table.attributes_complete;
        if !snmp_if_entries.is_empty() && walk_fell_short {
            ctx.ops
                .record_interface_shortfall(IncompleteInterfaceWalk {
                    ip,
                    collected: snmp_if_entries.len(),
                    set_complete: if_table.set_complete,
                })
                .await;
        }

        // Query LLDP neighbors
        let lldp = query_or_default(ip, "lldp", query_lldp_neighbors(&mut session, ip)).await;
        // Two different questions, deliberately not one flag.
        //
        // `lldp_complete` — did the walk finish? An agent with no LLDP-MIB answers immediately
        // and completely, so this stays true and no shortfall is reported. Warning about it
        // every scan would be the same noise the bridge-MIB groups used to produce.
        //
        // `lldp_authoritative` — may this result overwrite what the server holds? Only a device
        // that *has* the MIB and reports no neighbours is saying "there are none". Answering
        // `noSuchObject` says nothing about neighbours, and treating it as authority erased the
        // rows the UniFi integration writes for these very switches — the only source of LLDP
        // they have — whenever the SNMP pass happened to land second.
        let lldp_complete = lldp.complete;
        let lldp_reason = lldp.reason;
        let lldp_authoritative = lldp.complete && !lldp.unsupported;
        let lldp_discarded = lldp.discarded;
        let lldp_discard_reason = lldp.discard_reason;
        let mut lldp_neighbors = lldp.records;
        tracing::debug!(
            ip = %ip,
            count = lldp_neighbors.len(),
            complete = lldp_complete,
            unsupported = lldp.unsupported,
            "LLDP neighbors discovered"
        );
        let lldp_count = lldp_neighbors.len();

        // Query CDP neighbors (Cisco devices)
        let cdp = query_or_default(ip, "cdp", query_cdp_neighbors(&mut session, ip)).await;
        let cdp_complete = cdp.complete;
        let cdp_reason = cdp.reason;
        let cdp_discarded = cdp.discarded;
        let cdp_discard_reason = cdp.discard_reason;
        let cdp_neighbors = cdp.records;
        tracing::debug!(
            ip = %ip,
            count = cdp_neighbors.len(),
            complete = cdp_complete,
            "CDP neighbors discovered"
        );
        let cdp_count = cdp_neighbors.len();

        // Records the device served and we could not use. Reported per group because the
        // consequence differs — losing every neighbour on a switch takes it off L2 Physical
        // entirely, losing some leaves it there with holes — and because no rescan will change
        // either, which is the part an operator most needs told (GH #668).
        for (group, discarded, kept, reason) in [
            (
                SnmpWalkGroup::Lldp,
                lldp_discarded,
                lldp_count,
                lldp_discard_reason,
            ),
            (
                SnmpWalkGroup::Cdp,
                cdp_discarded,
                cdp_count,
                cdp_discard_reason,
            ),
        ] {
            // No reason means nothing was thrown away, and there is nothing to report.
            if let Some(reason) = reason {
                ctx.ops
                    .record_malformed_neighbours(MalformedNeighbours {
                        ip,
                        group,
                        discarded,
                        kept,
                        reason,
                    })
                    .await;
            }
        }

        // Translate LLDP local-port indices (which are lldpLocPortNum values, a
        // separate namespace from ifIndex on vendors like ExtremeXOS) to real ifIndex
        // values so neighbours attach to the correct interface. Resolved via
        // lldpLocPortTable; falls back to identity (correct for VOSS and any device
        // that reports lldpLocPortNum == ifIndex or omits the table). CDP is not
        // remapped: cdpCacheIfIndex is already a real ifIndex.
        let lldp_local_ports = if lldp_count > 0 {
            query_or_default(
                ip,
                "lldp_local_ports",
                query_lldp_local_ports(&mut session, ip),
            )
            .await
        } else {
            std::collections::HashMap::new()
        };
        let unresolved_ports =
            remap_lldp_local_ports(&mut lldp_neighbors, &lldp_local_ports, &snmp_if_entries);
        if unresolved_ports > 0 {
            tracing::warn!(
                ip = %ip,
                unresolved = unresolved_ports,
                total = lldp_count,
                "LLDP neighbours could not be matched to a local interface; their links may \
                 attach to the wrong port"
            );
            ctx.ops
                .record_unresolved_lldp_ports(UnresolvedLldpPorts {
                    ip,
                    unresolved: unresolved_ports,
                    total: lldp_count,
                })
                .await;
        }

        // Query ipAddrTable for IP->ifIndex+netMask mappings
        let ip_addr_table =
            query_or_default(ip, "ip_addr_table", query_ip_addr_table(&mut session, ip)).await;

        // Query ARP table for remote host discovery
        let arp_entries = query_or_default(ip, "arp", query_arp_table(&mut session, ip)).await;
        let arp_count = arp_entries.len();
        tracing::info!(ip = %ip, count = arp_count, "ARP table entries collected");

        // Query ENTITY-MIB for hardware inventory
        let device_inventory =
            query_or_default(ip, "entity_mib", query_entity_physical(&mut session, ip)).await;
        let has_entity_inventory = device_inventory.is_some();
        tracing::info!(
            ip = %ip,
            has_inventory = has_entity_inventory,
            "ENTITY-MIB inventory queried"
        );

        // Walk dot1dBasePortIfIndex once and share it. Both the bridge FDB and per-port VLAN
        // membership are keyed by bridge port, and each used to walk this table for itself —
        // so a switch that answers the OID with silence rather than `noSuchObject` (the
        // Ubiquiti USW-Pro-Max does) paid the walk timeout twice per scan for a table that
        // was never going to arrive.
        let bridge_ports = query_or_default(
            ip,
            "bridge_port_mapping",
            query_bridge_port_mapping(&mut session, ip),
        )
        .await;
        tracing::debug!(
            ip = %ip,
            count = bridge_ports.records.len(),
            complete = bridge_ports.complete,
            "Bridge port mappings collected"
        );

        // Query bridge FDB for MAC-to-port mappings
        let fdb = query_or_default(
            ip,
            "bridge_fdb",
            query_bridge_fdb(&mut session, ip, &bridge_ports),
        )
        .await;
        let fdb_complete = fdb.complete;
        let fdb_reason = fdb.reason;
        let bridge_fdb = fdb.records;
        let fdb_count = bridge_fdb.len();
        tracing::info!(
            ip = %ip,
            count = fdb_count,
            complete = fdb_complete,
            "Bridge FDB entries collected"
        );

        // Query VLAN table for VLAN names and persist as VLAN entities
        let vlan_table =
            query_or_default(ip, "vlan_table", query_vlan_table(&mut session, ip)).await;
        let vlan_number_to_uuid: std::collections::HashMap<u16, Uuid> = if !vlan_table.is_empty() {
            tracing::info!(
                ip = %ip,
                count = vlan_table.len(),
                vlans = ?vlan_table.iter().map(|v| format!("{}={}", v.vlan_id, v.name)).collect::<Vec<_>>(),
                "VLAN table entries collected"
            );
            match ctx.ops.upsert_vlans(&vlan_table, network_id).await {
                Ok(mapping) => mapping,
                Err(e) => {
                    tracing::warn!(ip = %ip, error = %e, "Failed to upsert VLANs, VLAN IDs will not be resolved");
                    // The switch answered in full and we could not record it. Silent until now,
                    // and the consequence is not small — every interface on this device loses
                    // its VLAN ids, which looks identical to a switch that reports no VLANs.
                    ctx.ops.record_vlan_recording_failure(ip).await;
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        };

        // Query per-port VLAN membership
        let port_vlan_membership = query_or_default(
            ip,
            "port_vlan_membership",
            query_port_vlan_membership(&mut session, ip, &bridge_ports),
        )
        .await;
        let vlan_membership_complete = port_vlan_membership.complete;
        let vlan_membership_reason = port_vlan_membership.reason;
        let port_vlan_membership = port_vlan_membership.records;
        tracing::info!(
            ip = %ip,
            count = port_vlan_membership.len(),
            complete = vlan_membership_complete,
            "Port VLAN memberships collected"
        );

        // Query local LLDP identity
        let lldp_local =
            query_or_default(ip, "lldp_local", query_lldp_local(&mut session, ip)).await;
        tracing::info!(
            ip = %ip,
            has_lldp_local = lldp_local.is_some(),
            "LLDP local identity queried"
        );

        // --- Hostname enrichment: use SNMP sysName as fallback if DNS didn't provide one ---
        if let Some(ref info) = system_info
            && let Some(ref sys_name) = info.sys_name
        {
            host_data.with_hostname_fallback(sys_name.clone());
        }

        // --- MAC enrichment from ipAddrTable when ARP didn't provide one ---
        if let Some(ip_entry) = ip_addr_table.get(&ip)
            && let Some(entry) = snmp_if_entries
                .iter()
                .find(|e| e.if_index == ip_entry.if_index)
            && let Some(mac) = entry.if_phys_address
        {
            tracing::debug!(
                ip = %ip,
                if_index = ip_entry.if_index,
                mac = ?mac,
                "ipAddrTable MAC enrichment"
            );
            host_data.with_mac_for_ip(ip, mac);
        }

        // --- Enrich host fields from SNMP system info ---
        if let Some(ref info) = system_info {
            if let Some(ref v) = info.sys_descr {
                host_data.with_sys_descr(v.clone());
            }
            if let Some(ref v) = info.sys_object_id {
                host_data.with_sys_object_id(v.clone());
            }
            if let Some(ref v) = info.sys_location {
                host_data.with_sys_location(v.clone());
            }
            if let Some(ref v) = info.sys_contact {
                host_data.with_sys_contact(v.clone());
            }
            if let Some(ref v) = info.sys_name {
                host_data.with_sys_name(v.clone());
            }
        }

        // --- Set chassis_id from LLDP local identity ---
        if let Some(ref local) = lldp_local
            && let Some(chassis) =
                LldpChassisId::from_snmp(local.chassis_id_subtype, &local.chassis_id_bytes)
        {
            // Same canonical form the server matches a *neighbor's* chassis ID against, so a
            // device whose chassis MAC appears on none of its ports is still identifiable.
            host_data.with_chassis_id(chassis.identifier());
        }

        // --- Add ENTITY-MIB hardware inventory ---
        if let Some(ref inventory) = device_inventory {
            if let Some(ref v) = inventory.manufacturer {
                host_data.with_manufacturer(v.clone());
            }
            if let Some(ref v) = inventory.model {
                host_data.with_model(v.clone());
            }
            if let Some(ref v) = inventory.serial_number {
                host_data.with_serial_number(v.clone());
            }
        }

        // --- Credential assignment for the working SNMP credential ---
        if let Some(cred_id) = ctx.credential_id {
            host_data.add_credential_assignment(CredentialAssignment {
                credential_id: cred_id,
                ip_address_ids: None,
            });
        }

        // --- Convert SNMP ifTable entries to Interface entities ---
        // Replaces (not appends to) the bare set persisted right after the ifTable walk, now that
        // the neighbour/FDB/VLAN queries have supplied the enrichment those bare entries lacked.
        host_data.replace_interfaces(
            snmp_if_entries
                .iter()
                .map(|entry| {
                    convert_snmp_if_entry(
                        entry,
                        network_id,
                        &lldp_neighbors,
                        &cdp_neighbors,
                        &bridge_fdb,
                        &port_vlan_membership,
                        &vlan_number_to_uuid,
                    )
                })
                .collect(),
        );
        // Mark whether this SNMP interface set is a complete, authoritative ifTable. The server
        // only prunes interfaces no longer reported when this is true, so a partial walk cannot
        // tear down the host's L2 topology (GH #649).
        host_data.set_interfaces_complete(if_table.set_complete);
        // Tell the server which of these groups it may treat as authoritative. A group we only
        // read partially must not overwrite what is already stored — an empty result from a
        // cut-short walk is indistinguishable from a device reporting nothing, and for the
        // neighbour fields losing them drops the row out of L2 resolution for good.
        host_data.set_interface_data_complete(InterfaceDataComplete {
            lldp: lldp_authoritative,
            cdp: cdp_complete,
            fdb: fdb_complete,
            vlan_membership: vlan_membership_complete,
        });

        // A cut-short neighbour walk used to be entirely silent — it took a database query to
        // discover that a switch had lost its chassis ids. Record it so the run can say so once,
        // with what happened as a result: the previous values are kept, so this is a "no fresh
        // data" notice rather than a loss.
        //
        // `returned_any` is carried per group because it separates two different problems that
        // share the `complete: false` flag: a walk that returned rows and stopped was truncated,
        // while one that returned nothing timed out or errored outright.
        //
        // Which groups are worth reporting — and which are merely downstream of a failure
        // already being reported — is `snmp_walk_shortfalls`'s call, so it can be tested
        // without a live agent.
        let incomplete = snmp_walk_shortfalls(
            ip,
            SnmpCollectionOutcome {
                lldp: SnmpGroupOutcome {
                    complete: lldp_complete,
                    returned_any: lldp_count > 0,
                    reason: lldp_reason,
                },
                cdp: SnmpGroupOutcome {
                    complete: cdp_complete,
                    returned_any: cdp_count > 0,
                    reason: cdp_reason,
                },
                bridge_port_numbering: SnmpGroupOutcome {
                    complete: bridge_ports.complete,
                    returned_any: !bridge_ports.records.is_empty(),
                    reason: bridge_ports.reason,
                },
                bridge_forwarding: SnmpGroupOutcome {
                    complete: fdb_complete,
                    returned_any: fdb_count > 0,
                    reason: fdb_reason,
                },
                vlan_membership: SnmpGroupOutcome {
                    complete: vlan_membership_complete,
                    returned_any: !port_vlan_membership.is_empty(),
                    reason: vlan_membership_reason,
                },
            },
        );
        ctx.ops.record_snmp_shortfalls(incomplete).await;

        // --- Discover remote subnets from ipAddrTable ---
        let scanning_subnet = ctx.scanning_subnet;
        let mut discovered_subnets: Vec<Subnet> = Vec::new();

        for (entry_ip, entry) in &ip_addr_table {
            let mask = match entry.net_mask {
                Some(m) => m,
                None => continue,
            };

            // Only handle IPv4
            let (entry_ipv4, mask_ipv4) = match (entry_ip, mask) {
                (IpAddr::V4(eip), IpAddr::V4(mip)) => (*eip, mip),
                _ => continue,
            };

            // Skip loopback, link-local
            let octets = entry_ipv4.octets();
            if octets[0] == 127 || (octets[0] == 169 && octets[1] == 254) {
                continue;
            }

            // Skip /32 and /0
            let mask_octets = mask_ipv4.octets();
            let mask_u32 = u32::from_be_bytes(mask_octets);
            if mask_u32 == 0xFFFFFFFF || mask_u32 == 0 {
                continue;
            }

            // Build network from IP + mask
            let ipv4_network = match ipnetwork::Ipv4Network::with_netmask(entry_ipv4, mask_ipv4) {
                Ok(n) => n,
                Err(_) => continue,
            };
            let ip_network = ipnetwork::IpNetwork::V4(ipv4_network);

            // Skip if this is the current scanning subnet
            if let Some(subnet) = scanning_subnet {
                let new_cidr_str = format!("{}/{}", ipv4_network.network(), ipv4_network.prefix());
                if new_cidr_str == subnet.base.cidr.to_string() {
                    continue;
                }
            }

            // Get interface name for subnet typing
            let if_name = snmp_if_entries
                .iter()
                .find(|e| e.if_index == entry.if_index)
                .and_then(|e| e.if_name.clone())
                .unwrap_or_default();

            if let Some(new_subnet) = Subnet::from_discovery(if_name, &ip_network, network_id) {
                tracing::info!(
                    ip = %ip,
                    cidr = %new_subnet.base.cidr,
                    "Discovered remote subnet via ipAddrTable"
                );

                match ctx.ops.create_subnet(&new_subnet, ctx.cancel).await {
                    Ok(created_subnet) => {
                        // Build an interface for the host on this subnet
                        let if_mac = snmp_if_entries
                            .iter()
                            .find(|e| e.if_index == entry.if_index)
                            .and_then(|e| e.if_phys_address);

                        host_data.add_ip_address(IPAddress::new(IPAddressBase {
                            network_id,
                            host_id: Uuid::nil(),
                            name: None,
                            subnet_id: created_subnet.id,
                            ip_address: *entry_ip,
                            mac_address: if_mac,
                            position: 0,
                        }));

                        discovered_subnets.push(created_subnet);
                    }
                    Err(e) => {
                        tracing::warn!(
                            ip = %ip,
                            cidr = %new_subnet.base.cidr,
                            error = %e,
                            "Failed to create discovered subnet"
                        );
                    }
                }
            }
        }

        // --- Create loopback interface if this host has a SOFTWARE_LOOPBACK ifEntry ---
        let has_loopback_if_entry = snmp_if_entries
            .iter()
            .any(|e| e.if_type == Some(if_type::SOFTWARE_LOOPBACK));
        if has_loopback_if_entry {
            let loopback_subnet = Subnet::from_discovery(
                "lo".to_string(),
                &ipnetwork::IpNetwork::V4(
                    ipnetwork::Ipv4Network::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 8).unwrap(),
                ),
                network_id,
            );
            if let Some(loopback_subnet) = loopback_subnet {
                match ctx.ops.create_subnet(&loopback_subnet, ctx.cancel).await {
                    Ok(created_loopback) => {
                        host_data.add_ip_address(IPAddress::new(IPAddressBase {
                            network_id,
                            host_id: Uuid::nil(),
                            name: Some("lo".to_string()),
                            subnet_id: created_loopback.id,
                            ip_address: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                            mac_address: None,
                            position: 0,
                        }));
                    }
                    Err(e) => {
                        tracing::debug!(
                            error = %e,
                            "Failed to create loopback subnet for SNMP host"
                        );
                    }
                }
            }
        }

        // --- Discover remote hosts from ARP table ---
        // Only create hosts for ARP entries on SNMP-discovered remote subnets
        for arp_entry in &arp_entries {
            // Skip entries on the current scanning subnet
            if let Some(subnet) = scanning_subnet
                && subnet.base.cidr.contains(&arp_entry.ip_address)
            {
                continue;
            }

            // Find matching SNMP-discovered subnet
            let matching_subnet = discovered_subnets
                .iter()
                .find(|s| s.base.cidr.contains(&arp_entry.ip_address));

            if let Some(remote_subnet) = matching_subnet {
                let arp_interface = IPAddress::new(IPAddressBase {
                    network_id,
                    host_id: Uuid::nil(),
                    name: None,
                    subnet_id: remote_subnet.id,
                    ip_address: arp_entry.ip_address,
                    mac_address: Some(arp_entry.mac_address),
                    position: 0,
                });

                let arp_host = Host::new(HostBase {
                    network_id,
                    source: EntitySource::Discovery,
                    ..Default::default()
                });

                tracing::info!(
                    ip = %arp_entry.ip_address,
                    mac = %arp_entry.mac_address,
                    subnet = %remote_subnet.base.cidr,
                    "Discovered remote host via ARP table"
                );

                if let Err(e) = ctx
                    .ops
                    .create_host(
                        arp_host,
                        vec![arp_interface],
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        // ARP-discovered remote host has no ifTable of its own; nothing to prune.
                        true,
                        // ...and no neighbour data, so nothing to preserve against.
                        InterfaceDataComplete::default(),
                        ctx.cancel,
                    )
                    .await
                {
                    tracing::debug!(
                        ip = %arp_entry.ip_address,
                        error = %e,
                        "Failed to create ARP-discovered host"
                    );
                }
            }
        }

        Ok(())
    }
}

/// Translate each LLDP neighbour's `local_port_index` from an `lldpLocPortNum` to the
/// device's real `ifIndex`, using `lldpLocPortTable` (`loc_ports`) resolved against the
/// interface table (`if_entries`). Neighbours whose port cannot be resolved keep their
/// original index. An empty `loc_ports` is identity — correct for devices where
/// `lldpLocPortNum == ifIndex` (e.g. Extreme VOSS) or that omit the table.
/// Returns how many neighbours could not be resolved and kept their original index.
///
/// The count matters because this failure is *silent and wrong* rather than silent and missing.
/// An unresolved neighbour keeps its `lldpLocPortNum`, which on a device where that is a separate
/// namespace from `ifIndex` — ExtremeXOS reports ports 1..N against ifIndexes 1001+ — attaches
/// the link to whatever interface happens to hold that index, or to none. A missing link is
/// visibly missing; a link drawn to the wrong port is worse, because the map looks complete.
fn remap_lldp_local_ports(
    neighbors: &mut [LldpNeighbor],
    loc_ports: &std::collections::HashMap<i32, LldpLocalPort>,
    if_entries: &[IfTableEntry],
) -> usize {
    // An empty table is the identity mapping, not a failure: devices where `lldpLocPortNum ==
    // ifIndex` (Extreme VOSS, most vendors) legitimately omit it.
    if loc_ports.is_empty() {
        return 0;
    }
    let mut unresolved = 0;
    for neighbor in neighbors.iter_mut() {
        match resolve_lldp_local_port(neighbor.local_port_index, loc_ports, if_entries) {
            Some(if_index) => neighbor.local_port_index = if_index,
            None => unresolved += 1,
        }
    }
    unresolved
}

/// Resolve a single `lldpLocPortNum` to an `ifIndex`. Returns `None` to keep the
/// original value (no confident match).
fn resolve_lldp_local_port(
    local_port_num: i32,
    loc_ports: &std::collections::HashMap<i32, LldpLocalPort>,
    if_entries: &[IfTableEntry],
) -> Option<i32> {
    let entry = loc_ports.get(&local_port_num)?;

    // interfaceIndex(2): the port id is literally the ifIndex.
    if entry.port_id_subtype == Some(2)
        && let Some(id) = entry.port_id.as_deref()
        && let Ok(idx) = id.trim().parse::<i32>()
    {
        return Some(idx);
    }

    let id = entry.port_id.as_deref()?.trim();
    if id.is_empty() {
        return None;
    }

    // Exact match against ifName / ifDescr (VOSS: "1/1" == ifName "1/1").
    for e in if_entries {
        if e.if_name.as_deref() == Some(id) || e.if_descr.as_deref() == Some(id) {
            return Some(e.if_index);
        }
    }

    // Suffix match for vendors whose lldpLocPortId drops the slot prefix (EXOS: id
    // "4" vs ifName "1:4"). Anchor on a ':' or '/' boundary so "4" does not match
    // "14".
    let colon = format!(":{id}");
    let slash = format!("/{id}");
    let ends_at_boundary =
        |name: Option<&str>| name.is_some_and(|n| n.ends_with(&colon) || n.ends_with(&slash));
    for e in if_entries {
        if ends_at_boundary(e.if_name.as_deref()) || ends_at_boundary(e.if_descr.as_deref()) {
            return Some(e.if_index);
        }
    }

    None
}

/// Convert SNMP ifTable entry to Interface entity with LLDP/CDP/FDB neighbor data.
/// Uses Uuid::nil() for host_id as placeholder - server will set correct host_id.
fn convert_snmp_if_entry(
    entry: &IfTableEntry,
    network_id: Uuid,
    lldp_neighbors: &[LldpNeighbor],
    cdp_neighbors: &[CdpNeighbor],
    bridge_fdb: &[BridgeFdbEntry],
    port_vlan_membership: &[PortVlanMembership],
    vlan_number_to_uuid: &std::collections::HashMap<u16, Uuid>,
) -> Interface {
    // Find LLDP neighbor data for this port (match by local_port_index == if_index)
    let lldp_neighbor = lldp_neighbors
        .iter()
        .find(|n| n.local_port_index == entry.if_index);

    // Find CDP neighbor data for this port
    let cdp_neighbor = cdp_neighbors
        .iter()
        .find(|n| n.local_port_index == entry.if_index);

    // Convert LLDP chassis ID using subtype + raw bytes via from_snmp()
    let lldp_chassis_id = lldp_neighbor.and_then(|n| {
        let subtype = n.remote_chassis_id_subtype?;
        let bytes = n.remote_chassis_id_bytes.as_ref()?;
        LldpChassisId::from_snmp(subtype, bytes)
    });

    // Convert LLDP port ID using subtype + raw bytes via from_snmp()
    let lldp_port_id = lldp_neighbor.and_then(|n| {
        let subtype = n.remote_port_id_subtype?;
        let bytes = n.remote_port_id_bytes.as_ref()?;
        LldpPortId::from_snmp(subtype, bytes)
    });

    // Find VLAN membership for this port
    let vlan_membership = port_vlan_membership
        .iter()
        .find(|m| m.if_index == entry.if_index);

    // Collect learned MACs from bridge FDB for this port.
    // Single-MAC ports are used for neighbor resolution server-side;
    // multi-MAC ports indicate uplinks where LLDP/CDP is the better source
    // for direct neighbor identification.
    let fdb_macs: Vec<String> = bridge_fdb
        .iter()
        .filter(|fdb| fdb.if_index == Some(entry.if_index) && fdb.status == 3)
        .map(|fdb| fdb.mac_address.to_string())
        .collect();

    Interface::new(InterfaceBase {
        host_id: Uuid::nil(), // Placeholder - server will set correct host_id
        network_id,
        if_index: entry.if_index,
        if_descr: entry.if_descr.clone().unwrap_or_default(),
        if_name: entry.if_name.clone(),
        if_alias: entry.if_alias.clone(),
        if_type: entry.if_type.unwrap_or(1), // 1 = "other"
        speed_bps: entry.if_speed.map(|s| s as i64),
        admin_status: IfAdminStatus::from(entry.if_admin_status.unwrap_or(1)),
        oper_status: IfOperStatus::from(entry.if_oper_status.unwrap_or(1)),
        mac_address: entry.if_phys_address, // MAC from SNMP ifPhysAddress
        ip_address_id: None,                // Linked server-side via MAC matching
        neighbor: None,                     // Resolved server-side from LLDP/CDP data
        // LLDP raw data
        lldp_chassis_id,
        lldp_port_id,
        lldp_sys_name: lldp_neighbor.and_then(|n| n.remote_sys_name.clone()),
        lldp_port_desc: lldp_neighbor.and_then(|n| n.remote_port_desc.clone()),
        lldp_mgmt_addr: lldp_neighbor.and_then(|n| n.remote_mgmt_addr),
        lldp_sys_desc: lldp_neighbor.and_then(|n| n.remote_sys_desc.clone()),
        // CDP raw data
        cdp_device_id: cdp_neighbor.and_then(|n| n.remote_device_id.clone()),
        cdp_port_id: cdp_neighbor.and_then(|n| n.remote_port_id.clone()),
        cdp_platform: cdp_neighbor.and_then(|n| n.remote_platform.clone()),
        cdp_address: cdp_neighbor.and_then(|n| n.remote_address),
        // Bridge FDB data
        fdb_macs: if fdb_macs.is_empty() {
            None
        } else {
            Some(fdb_macs)
        },
        // VLAN data: resolved to entity UUIDs by caller via vlan_number_to_uuid mapping
        native_vlan_id: vlan_membership
            .and_then(|m| m.native_vlan)
            .and_then(|vid| vlan_number_to_uuid.get(&vid).copied()),
        vlan_ids: vlan_membership
            .map(|m| {
                m.tagged_vlans
                    .iter()
                    .filter_map(|vid| vlan_number_to_uuid.get(vid).copied())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty()),
    })
}

/// Perform a complete SNMP poll of a device.
/// Returns system info, interface table, and neighbor information.
#[allow(dead_code)]
pub async fn poll_device(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<(
    SystemInfo,
    Vec<IfTableEntry>,
    Vec<LldpNeighbor>,
    Vec<CdpNeighbor>,
)> {
    debug!("Starting SNMP poll of {}", ip);

    let mut session = create_session(ip, credential, port).await?;

    let system_info = timeout(SNMP_WALK_TIMEOUT, query_system_info(&mut session, ip))
        .await
        .map_err(|_| anyhow::anyhow!("System info query timeout"))??;

    let interfaces = timeout(SNMP_WALK_TIMEOUT, walk_if_table(&mut session, ip))
        .await
        .map_err(|_| anyhow::anyhow!("ifTable walk timeout"))?
        .map(|walk| walk.entries)
        .unwrap_or_default();

    let lldp_neighbors = timeout(SNMP_WALK_TIMEOUT, query_lldp_neighbors(&mut session, ip))
        .await
        .map(|r| r.map(|c| c.records))
        .unwrap_or(Ok(vec![]))
        .unwrap_or_default();

    let cdp_neighbors = timeout(SNMP_WALK_TIMEOUT, query_cdp_neighbors(&mut session, ip))
        .await
        .map(|r| r.map(|c| c.records))
        .unwrap_or(Ok(vec![]))
        .unwrap_or_default();

    debug!(
        "SNMP poll of {} complete: {} ip_addresses, {} LLDP neighbors, {} CDP neighbors",
        ip,
        interfaces.len(),
        lldp_neighbors.len(),
        cdp_neighbors.len()
    );

    Ok((system_info, interfaces, lldp_neighbors, cdp_neighbors))
}

#[cfg(test)]
mod tests {
    use super::values::{value_to_i32, value_to_mac, value_to_string};
    use snmp2::Value;

    /// The interface set is persisted as soon as the ifTable walk finishes, before the
    /// neighbour/FDB/VLAN queries have run — so it is built with no enrichment available.
    /// Those bare interfaces still have to be complete, usable entities (the host is created
    /// from them if a later query hangs), carrying every ifTable field and simply no
    /// LLDP/CDP/FDB/VLAN data.
    #[test]
    fn interfaces_built_without_enrichment_keep_their_iftable_identity() {
        use super::*;

        let entry = types::IfTableEntry {
            if_index: 7,
            if_descr: Some("Port 7".to_string()),
            if_name: Some("swp7".to_string()),
            if_type: Some(6),
            if_speed: Some(1_000_000_000),
            if_admin_status: Some(1),
            if_oper_status: Some(1),
            ..Default::default()
        };
        let network_id = Uuid::new_v4();

        let interface = convert_snmp_if_entry(
            &entry,
            network_id,
            &[],
            &[],
            &[],
            &[],
            &std::collections::HashMap::new(),
        );

        // ifTable data survives the enrichment-free conversion.
        assert_eq!(interface.base.if_index, 7);
        assert_eq!(interface.base.if_descr, "Port 7");
        assert_eq!(interface.base.if_name.as_deref(), Some("swp7"));
        assert_eq!(interface.base.if_type, 6);
        assert_eq!(interface.base.speed_bps, Some(1_000_000_000));
        assert_eq!(interface.base.network_id, network_id);

        // Enrichment that hasn't been collected yet is absent, not fabricated.
        assert!(interface.base.lldp_chassis_id.is_none());
        assert!(interface.base.cdp_device_id.is_none());
        assert!(interface.base.fdb_macs.is_none());
        assert!(interface.base.native_vlan_id.is_none());
        assert!(interface.base.vlan_ids.is_none());
    }

    #[test]
    fn test_value_to_string() {
        let value = Value::OctetString(b"test string");
        assert_eq!(value_to_string(&value), Some("test string".to_string()));
    }

    #[test]
    fn test_value_to_i32() {
        let value = Value::Integer(42);
        assert_eq!(value_to_i32(&value), Some(42));
    }

    #[test]
    fn test_value_to_mac() {
        let mac_bytes: [u8; 6] = [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34];
        let value = Value::OctetString(&mac_bytes);
        let mac = value_to_mac(&value).unwrap();
        assert_eq!(mac.bytes(), [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34]);
    }

    #[test]
    fn test_convert_snmp_if_entry_with_vlan_data() {
        use super::convert_snmp_if_entry;
        use super::types::{IfTableEntry, PortVlanMembership};
        use uuid::Uuid;

        let entry = IfTableEntry {
            if_index: 5,
            if_descr: Some("GigabitEthernet0/5".to_string()),
            ..Default::default()
        };

        let membership = vec![
            PortVlanMembership {
                if_index: 5,
                native_vlan: Some(10),
                tagged_vlans: vec![20, 30],
            },
            PortVlanMembership {
                if_index: 7,
                native_vlan: Some(20),
                tagged_vlans: vec![],
            },
        ];

        let result = convert_snmp_if_entry(
            &entry,
            Uuid::nil(),
            &[],
            &[],
            &[],
            &membership,
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.base.native_vlan_id, None);
        assert_eq!(result.base.vlan_ids, None);
    }

    #[test]
    fn test_convert_snmp_if_entry_no_vlan_data() {
        use super::convert_snmp_if_entry;
        use super::types::IfTableEntry;
        use uuid::Uuid;

        let entry = IfTableEntry {
            if_index: 3,
            if_descr: Some("Loopback0".to_string()),
            ..Default::default()
        };

        let result = convert_snmp_if_entry(
            &entry,
            Uuid::nil(),
            &[],
            &[],
            &[],
            &[],
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.base.native_vlan_id, None);
        assert_eq!(result.base.vlan_ids, None);
    }

    #[test]
    fn test_convert_snmp_if_entry_empty_tagged_vlans() {
        use super::convert_snmp_if_entry;
        use super::types::{IfTableEntry, PortVlanMembership};
        use uuid::Uuid;

        let entry = IfTableEntry {
            if_index: 1,
            if_descr: Some("FastEthernet0/1".to_string()),
            ..Default::default()
        };

        // Access port: native VLAN only, no tagged VLANs
        let membership = vec![PortVlanMembership {
            if_index: 1,
            native_vlan: Some(10),
            tagged_vlans: vec![],
        }];

        let result = convert_snmp_if_entry(
            &entry,
            Uuid::nil(),
            &[],
            &[],
            &[],
            &membership,
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.base.native_vlan_id, None);
        // Empty tagged_vlans should be stored as None (filtered)
        assert_eq!(result.base.vlan_ids, None);
    }

    // --- LLDP local-port remap (Issue 2: ExtremeXOS vs VOSS) ---

    use super::types::{IfTableEntry, LldpLocalPort, LldpNeighbor};

    /// Minimal LldpNeighbor carrying only a local-port index + a marker sys name.
    fn lldp_neighbor(local_port_index: i32, sys_name: &str) -> LldpNeighbor {
        LldpNeighbor {
            local_port_index,
            remote_chassis_id_subtype: None,
            remote_chassis_id_bytes: None,
            remote_port_id_subtype: None,
            remote_port_id_bytes: None,
            remote_port_desc: None,
            remote_sys_name: Some(sys_name.to_string()),
            remote_sys_desc: None,
            remote_mgmt_addr: None,
        }
    }

    fn if_entry(if_index: i32, if_name: &str) -> IfTableEntry {
        IfTableEntry {
            if_index,
            if_name: Some(if_name.to_string()),
            ..Default::default()
        }
    }

    fn loc_port(subtype: u8, id: &str) -> LldpLocalPort {
        LldpLocalPort {
            port_id_subtype: Some(subtype),
            port_id: Some(id.to_string()),
        }
    }

    #[test]
    fn test_remap_lldp_exos_suffix_match() {
        use super::remap_lldp_local_ports;
        // ExtremeXOS X435: lldpRemTable local-port is an lldpLocPortNum (4, 11) in a
        // 1..N space; real ifIndex is 1001+, ifName "1:N". lldpLocPortId is "N",
        // subtype interfaceName(5) — must suffix-match against ifName "1:N".
        let if_entries = [
            if_entry(1001, "1:1"),
            if_entry(1004, "1:4"),
            if_entry(1011, "1:11"),
        ];
        let mut loc_ports = std::collections::HashMap::new();
        loc_ports.insert(4, loc_port(5, "4"));
        loc_ports.insert(11, loc_port(5, "11"));

        let mut neighbors = vec![lldp_neighbor(4, "peer-a"), lldp_neighbor(11, "peer-b")];
        remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(neighbors[0].local_port_index, 1004);
        assert_eq!(neighbors[1].local_port_index, 1011);
    }

    #[test]
    fn test_remap_lldp_voss_exact_match_identity() {
        use super::remap_lldp_local_ports;
        // Extreme VOSS: lldpLocPortNum == ifIndex and lldpLocPortId ("1/1") matches
        // ifName exactly, so the resolved ifIndex equals the original index.
        let if_entries = [if_entry(192, "1/1"), if_entry(193, "1/2")];
        let mut loc_ports = std::collections::HashMap::new();
        loc_ports.insert(192, loc_port(5, "1/1"));
        loc_ports.insert(193, loc_port(5, "1/2"));

        let mut neighbors = vec![lldp_neighbor(192, "peer")];
        remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(neighbors[0].local_port_index, 192);
    }

    #[test]
    fn test_remap_lldp_no_loc_table_is_identity() {
        use super::remap_lldp_local_ports;
        // No lldpLocPortTable (e.g. devices that report lldpLocPortNum == ifIndex):
        // indices are left untouched so existing behaviour is preserved.
        let if_entries = [if_entry(5, "Gi0/5")];
        let empty = std::collections::HashMap::new();
        let mut neighbors = vec![lldp_neighbor(5, "peer")];
        remap_lldp_local_ports(&mut neighbors, &empty, &if_entries);
        assert_eq!(neighbors[0].local_port_index, 5);
    }

    #[test]
    fn test_remap_lldp_interface_index_subtype() {
        use super::remap_lldp_local_ports;
        // lldpLocPortId subtype interfaceIndex(2): the id is literally the ifIndex.
        let if_entries = [if_entry(1007, "1:7")];
        let mut loc_ports = std::collections::HashMap::new();
        loc_ports.insert(7, loc_port(2, "1007"));
        let mut neighbors = vec![lldp_neighbor(7, "peer")];
        remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);
        assert_eq!(neighbors[0].local_port_index, 1007);
    }

    #[test]
    fn test_remap_then_convert_attaches_exos_neighbor() {
        use super::{convert_snmp_if_entry, remap_lldp_local_ports};
        use uuid::Uuid;
        // End-to-end at the convert layer: after remap, the EXOS neighbour attaches
        // to the correct interface (which it would NOT before the fix, since
        // local_port_index 4 != ifIndex 1004).
        let if_entries = [if_entry(1004, "1:4")];
        let mut loc_ports = std::collections::HashMap::new();
        loc_ports.insert(4, loc_port(5, "4"));

        let mut neighbors = vec![lldp_neighbor(4, "switch-peer")];
        remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        let result = convert_snmp_if_entry(
            &if_entries[0],
            Uuid::nil(),
            &neighbors,
            &[],
            &[],
            &[],
            &std::collections::HashMap::new(),
        );
        assert_eq!(result.base.lldp_sys_name, Some("switch-peer".to_string()));
    }
}
