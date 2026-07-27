//! SNMP Query Functions
//!
//! Functions for querying SNMP data from devices.

use anyhow::Result;
use snmp2::{Oid, Value};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use tokio::time::timeout;
use tracing::{debug, trace, warn};

use super::oids::{self, oid_to_vec, parse_oid};
use super::session::{MAX_WALK_ENTRIES, SNMP_TIMEOUT};
use super::types::{
    ArpEntry, BridgeFdbEntry, CdpNeighbor, DeviceInventory, IfTableEntry, IpAddrEntry,
    LldpLocalInfo, LldpLocalPort, LldpNeighbor, PortVlanMembership, SystemInfo, VlanInfo,
};
use super::values::{
    parse_lldp_mgmt_addr, parse_portlist_bitmap, qbridge_fdb_index_to_mac, value_to_i32,
    value_to_ip, value_to_mac, value_to_string, value_to_u16, value_to_u64,
};

/// Varbinds requested per getbulk round when walking a table subtree.
const BULK_MAX_REPETITIONS: u32 = 20;

/// A single `getbulk` round-trip's non-error outcome. Transport failures (timeouts,
/// session errors) are the `Err` arm of the returned `Result`; the one legitimate
/// non-error signal is an agent that refuses getbulk, which the walk retries via getnext.
/// Varbinds borrow the session's response buffer (`snmp2::Value<'a>` holds `&'a [u8]`
/// for octet strings), so a page is only valid while the session stays borrowed.
pub type Varbinds<'a> = Vec<(Vec<u64>, Value<'a>)>;

pub enum WalkPage<'a> {
    /// Decoded varbinds in wire order, OIDs as sub-id vectors.
    Varbinds(Varbinds<'a>),
    /// Agent rejected getbulk (e.g. SNMPv1) — retry from the same OID with getnext.
    BulkUnsupported,
}

/// The two SNMP operations `walk_subtree` needs. Abstracting them keeps the walk loop
/// transport-agnostic so its termination logic is unit-testable without a live UDP
/// socket. Two implementors only: `Box<AsyncSession>` in production (below) and a
/// canned-page mock under `#[cfg(test)]`.
#[async_trait::async_trait]
pub trait SnmpWalkTransport: Send {
    async fn walk_getbulk<'a>(
        &'a mut self,
        from: &[u64],
        max_repetitions: u32,
    ) -> Result<WalkPage<'a>>;
    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>>;
}

#[async_trait::async_trait]
impl SnmpWalkTransport for Box<snmp2::AsyncSession> {
    async fn walk_getbulk<'a>(
        &'a mut self,
        from: &[u64],
        max_repetitions: u32,
    ) -> Result<WalkPage<'a>> {
        let oid = Oid::from(from).map_err(|_| anyhow::anyhow!("invalid walk OID"))?;
        match timeout(SNMP_TIMEOUT, self.getbulk(&[&oid], 0, max_repetitions)).await {
            Ok(Ok(pdu)) => Ok(WalkPage::Varbinds(
                pdu.varbinds.map(|(o, v)| (oid_to_vec(&o), v)).collect(),
            )),
            // A response that fails request-id or community validation is a session that has lost
            // sync with its own traffic, not an agent declining getbulk. Retrying the same OID with
            // getnext on a desynced session just reads the next stale answer, so the walk has to
            // end and be reported as truncated — treating it as "no bulk support" produced a
            // silently short table that still claimed to be complete.
            Ok(Err(e @ (snmp2::Error::RequestIdMismatch | snmp2::Error::CommunityMismatch))) => {
                Err(anyhow::anyhow!("SNMP session desynchronized: {e}"))
            }
            Ok(Err(_)) => Ok(WalkPage::BulkUnsupported),
            Err(_) => Err(anyhow::anyhow!("getbulk timed out")),
        }
    }

    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
        let oid = Oid::from(from).map_err(|_| anyhow::anyhow!("invalid walk OID"))?;
        match timeout(SNMP_TIMEOUT, self.getnext(&oid)).await {
            Ok(Ok(pdu)) => Ok(pdu.varbinds.map(|(o, v)| (oid_to_vec(&o), v)).collect()),
            Ok(Err(e)) => Err(anyhow::anyhow!("getnext failed: {e}")),
            Err(_) => Err(anyhow::anyhow!("getnext timed out")),
        }
    }
}

/// Walk the OID subtree rooted at `base_oid_str`, invoking `on_entry(suffix, value)`
/// for every varbind under it, where `suffix` is the OID sub-ids after the base.
///
/// Uses SNMP `getbulk` for throughput (one round returns up to `BULK_MAX_REPETITIONS`
/// varbinds instead of one per round-trip) and transparently falls back to `getnext`
/// if the agent rejects getbulk (e.g. SNMPv1).
///
/// Returns `Ok(true)` when the subtree was walked to its natural end (or `EndOfMibView`)
/// and `Ok(false)` when it was cut short by `MAX_WALK_ENTRIES`, a session error, a
/// timeout, a non-advancing OID, or an abnormal empty response — callers that prune
/// against a full table (see `walk_if_table`, GH #649) must treat `false` as a partial
/// walk.
async fn walk_subtree<T, F>(session: &mut T, base_oid_str: &str, mut on_entry: F) -> Result<bool>
where
    T: SnmpWalkTransport,
    F: FnMut(&[u64], &Value),
{
    let base_parts: Vec<u64> = base_oid_str
        .split('.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut current_parts = base_parts.clone();
    let mut count = 0usize;
    let mut use_bulk = true;
    let mut truncated = false;

    'walk: loop {
        if count >= MAX_WALK_ENTRIES {
            truncated = true;
            break;
        }

        let varbinds = if use_bulk {
            match session
                .walk_getbulk(&current_parts, BULK_MAX_REPETITIONS)
                .await
            {
                Ok(WalkPage::Varbinds(v)) => v,
                Ok(WalkPage::BulkUnsupported) => {
                    // Agent rejected getbulk (e.g. v1) — retry from the same OID with
                    // getnext and stay on getnext for the rest of this walk.
                    use_bulk = false;
                    continue 'walk;
                }
                Err(e) => {
                    debug!(?current_parts, error = %e, "SNMP walk getbulk stopped");
                    truncated = true;
                    break;
                }
            }
        } else {
            match session.walk_getnext(&current_parts).await {
                Ok(v) => v,
                Err(e) => {
                    debug!(?current_parts, error = %e, "SNMP walk column stopped");
                    truncated = true;
                    break;
                }
            }
        };

        // Empty response mid-walk is abnormal (getbulk) or an exhausted column
        // (getnext) — treat as partial either way.
        if varbinds.is_empty() {
            truncated = true;
            break;
        }

        // Process the response, remembering the last in-subtree OID to continue from.
        let mut next_parts: Option<Vec<u64>> = None;
        let mut done = false;
        for (resp_parts, value) in varbinds {
            if matches!(
                value,
                Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
            ) {
                done = true;
                break;
            }
            if resp_parts.len() <= base_parts.len() || !resp_parts.starts_with(&base_parts) {
                // Out of the subtree. That is the natural end of a column *if* the agent moved
                // forward past it — a walk always advances. An OID that doesn't exceed where we
                // asked from is not a continuation of this walk at all (a stale response left
                // over from a cancelled request reads exactly like this), and calling it a
                // natural end would report a column that stopped early as authoritative, which
                // then re-enables the server-side prune #649 exists to suppress.
                if resp_parts <= current_parts {
                    debug!(
                        ?resp_parts,
                        ?current_parts,
                        base = base_oid_str,
                        "SNMP walk stopped: response left the subtree without advancing"
                    );
                    truncated = true;
                }
                done = true;
                break;
            }
            on_entry(&resp_parts[base_parts.len()..], &value);
            count += 1;
            next_parts = Some(resp_parts);
            if count >= MAX_WALK_ENTRIES {
                truncated = true;
                done = true;
                break;
            }
        }
        if done {
            break;
        }

        match next_parts {
            Some(parts) => {
                // The walk must strictly advance. A device that answers with a tail OID
                // that doesn't lexicographically exceed the one we asked from (observed
                // on Ubiquiti bridge-FDB) would otherwise have us re-request the same
                // page until MAX_WALK_ENTRIES or the integration timeout. `Vec<u64>`
                // compares lexicographically, matching SNMP OID ordering.
                if parts <= current_parts {
                    debug!(
                        ?parts,
                        ?current_parts,
                        "SNMP walk stopped: OID did not advance"
                    );
                    truncated = true;
                    break;
                }
                current_parts = parts;
            }
            None => {
                truncated = true;
                break;
            }
        }
    }

    Ok(!truncated)
}

/// Query system MIB information from a device
pub async fn query_system_info(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<SystemInfo> {
    let mut info = SystemInfo::default();

    // Query each system OID
    let oids_to_query = [
        (oids::system::SYS_DESCR, "sysDescr"),
        (oids::system::SYS_OBJECT_ID, "sysObjectID"),
        (oids::system::SYS_NAME, "sysName"),
        (oids::system::SYS_LOCATION, "sysLocation"),
        (oids::system::SYS_CONTACT, "sysContact"),
        (oids::system::SYS_UPTIME, "sysUpTime"),
    ];

    for (oid_str, name) in oids_to_query {
        let oid = match parse_oid(oid_str) {
            Ok(o) => o,
            Err(e) => {
                warn!("Failed to parse OID {}: {}", oid_str, e);
                continue;
            }
        };

        match timeout(SNMP_TIMEOUT, session.get(&oid)).await {
            Ok(Ok(mut response)) => {
                if let Some((resp_oid, value)) = response.varbinds.next() {
                    trace!("SNMP {} from {}: {:?} = {:?}", name, ip, resp_oid, value);
                    match name {
                        "sysDescr" => info.sys_descr = value_to_string(&value),
                        "sysObjectID" => info.sys_object_id = value_to_string(&value),
                        "sysName" => info.sys_name = value_to_string(&value),
                        "sysLocation" => info.sys_location = value_to_string(&value),
                        "sysContact" => info.sys_contact = value_to_string(&value),
                        "sysUpTime" => info.sys_uptime = value_to_u64(&value),
                        _ => {}
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("SNMP GET {} failed from {}: {:?}", name, ip, e);
            }
            Err(_) => {
                debug!("SNMP GET {} timeout from {}", name, ip);
            }
        }
    }

    Ok(info)
}

/// Walk the ifTable and ifXTable to get interface information
/// Walk the ifTable/ifXTable columns.
///
/// Returns the collected entries plus a `complete` flag: `true` only when every column walked
/// cleanly to its end-of-subtree, `false` if any column was cut short by an SNMP error, a
/// per-getnext timeout, or the `MAX_WALK_ENTRIES` cap. A `false` here means the entry set may be
/// a partial view of the host's real ifTable — the server uses it to skip the interface prune so
/// a transient partial walk cannot delete interfaces (and their resolved L2 neighbors). See #649.
pub async fn walk_if_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<(Vec<IfTableEntry>, bool)> {
    let mut entries: HashMap<i32, IfTableEntry> = HashMap::new();
    // Cleared to false the moment any column walk is cut short (error/timeout/limit).
    let mut complete = true;

    // Define the columns we want to walk
    let columns = [
        (oids::if_mib::columns::IF_INDEX, "ifIndex"),
        (oids::if_mib::columns::IF_DESCR, "ifDescr"),
        (oids::if_mib::columns::IF_TYPE, "ifType"),
        (oids::if_mib::columns::IF_MTU, "ifMtu"),
        (oids::if_mib::columns::IF_SPEED, "ifSpeed"),
        (oids::if_mib::columns::IF_PHYS_ADDRESS, "ifPhysAddress"),
        (oids::if_mib::columns::IF_ADMIN_STATUS, "ifAdminStatus"),
        (oids::if_mib::columns::IF_OPER_STATUS, "ifOperStatus"),
        (oids::if_mib::if_x_table::IF_NAME, "ifName"),
        (oids::if_mib::if_x_table::IF_HIGH_SPEED, "ifHighSpeed"),
        (oids::if_mib::if_x_table::IF_ALIAS, "ifAlias"),
    ];

    // ifIndex is walked first and is the table's index column, so once it has returned a
    // non-empty set every later column must land inside it. A row appearing only in a later
    // column is not an interface this device reported — it is a response that doesn't belong to
    // this walk — and minting an interface from it is how a foreign port ended up on a switch.
    // Only trusted when the ifIndex column itself completed; a device that doesn't serve it at
    // all still gets the old permissive behaviour.
    let mut known_if_indexes: Option<HashSet<i32>> = None;
    let mut foreign_rows = 0usize;

    // Walk each column. ifTable/ifXTable are indexed by a single sub-id (ifIndex).
    for (base_oid_str, column_name) in columns {
        let known = known_if_indexes.clone();
        let mut column_indexes: HashSet<i32> = HashSet::new();
        let mut column_foreign = 0usize;
        let walked = walk_subtree(session, base_oid_str, |suffix, value| {
            let Some(&if_index_u64) = suffix.last() else {
                return;
            };
            let if_index = if_index_u64 as i32;
            column_indexes.insert(if_index);
            if let Some(known) = &known
                && !known.contains(&if_index)
            {
                column_foreign += 1;
                return;
            }
            let entry = entries.entry(if_index).or_insert_with(|| IfTableEntry {
                if_index,
                if_descr: None,
                if_type: None,
                if_mtu: None,
                if_speed: None,
                if_phys_address: None,
                if_admin_status: None,
                if_oper_status: None,
                if_name: None,
                if_alias: None,
            });
            match column_name {
                "ifIndex" => {} // already set above
                "ifDescr" => entry.if_descr = value_to_string(value),
                "ifType" => entry.if_type = value_to_i32(value),
                "ifMtu" => entry.if_mtu = value_to_i32(value),
                "ifSpeed" => {
                    // Only set if ifHighSpeed not already set
                    if entry.if_speed.is_none() {
                        entry.if_speed = value_to_u64(value);
                    }
                }
                "ifPhysAddress" => entry.if_phys_address = value_to_mac(value),
                "ifAdminStatus" => entry.if_admin_status = value_to_i32(value),
                "ifOperStatus" => entry.if_oper_status = value_to_i32(value),
                "ifName" => entry.if_name = value_to_string(value),
                "ifHighSpeed" => {
                    // ifHighSpeed is in Mbps, convert to bps for consistency
                    if let Some(mbps) = value_to_u64(value) {
                        entry.if_speed = Some(mbps * 1_000_000);
                    }
                }
                "ifAlias" => entry.if_alias = value_to_string(value),
                _ => {}
            }
        })
        .await
        .unwrap_or(false);

        // A column cut short (timeout/error/limit) means this is NOT an authoritative
        // full ifTable — the server must not prune stale interfaces against it (#649).
        if !walked {
            complete = false;
        }

        if column_name == "ifIndex" && walked && !column_indexes.is_empty() {
            known_if_indexes = Some(column_indexes);
        }

        if column_foreign > 0 {
            // Something answered for an interface this device never listed. Whatever the cause,
            // what we hold is not a faithful copy of its ifTable.
            foreign_rows += column_foreign;
            complete = false;
            tracing::warn!(
                ip = %ip,
                column = column_name,
                rows = column_foreign,
                "SNMP ifTable column returned rows for unknown ifIndexes; discarding them and \
                 marking the walk partial"
            );
        }
    }

    let mut result: Vec<IfTableEntry> = entries.into_values().collect();
    result.sort_by_key(|e| e.if_index);

    // `complete` distinguishes an authoritative full ifTable from a partial walk cut short by
    // timeout/error. The server prunes stale interfaces only on a complete walk (GH #649), so
    // surface it at debug level for self-hosted daemon-log triage (enable SCANOPY_LOG_LEVEL=debug).
    tracing::debug!(
        ip = %ip,
        if_count = result.len(),
        complete = complete,
        foreign_rows = foreign_rows,
        "SNMP ifTable walk finished"
    );
    // Diagnostic for issue #614 (high-ifIndex interfaces missing): log the full set of
    // collected ifIndex values, not just the count, so we can tell whether a high-ifIndex
    // switch (e.g. ifIndex 49153-49168) is dropped at walk time or later during ingestion.
    debug!(
        ip = %ip,
        if_indexes = ?result.iter().map(|e| e.if_index).collect::<Vec<_>>(),
        "SNMP ifTable walk ifIndex set"
    );

    Ok((result, complete))
}

/// Query LLDP remote table for neighbor information
pub async fn query_lldp_neighbors(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<Vec<LldpNeighbor>> {
    let mut neighbors: HashMap<(i32, i32), LldpNeighbor> = HashMap::new();

    // LLDP remote table uses a complex index: lldpRemTimeMark.lldpRemLocalPortNum.lldpRemIndex
    // We'll walk the columns and extract the local port from the OID

    let columns = [
        (
            oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID_SUBTYPE,
            "remChassisIdSubtype",
        ),
        (
            oids::lldp::remote::entry::LLDP_REM_CHASSIS_ID,
            "remChassisId",
        ),
        (
            oids::lldp::remote::entry::LLDP_REM_PORT_ID_SUBTYPE,
            "remPortIdSubtype",
        ),
        (oids::lldp::remote::entry::LLDP_REM_PORT_ID, "remPortId"),
        (oids::lldp::remote::entry::LLDP_REM_PORT_DESC, "remPortDesc"),
        (oids::lldp::remote::entry::LLDP_REM_SYS_NAME, "remSysName"),
        (oids::lldp::remote::entry::LLDP_REM_SYS_DESC, "remSysDesc"),
        // NOTE: lldpRemManAddr is intentionally NOT walked here. It lives in the
        // separate lldpRemManAddrTable whose index carries extra trailing sub-ids
        // (addrSubtype.addrLen.addr), so the 3-element index parse below does not
        // apply. It is resolved by walk_lldp_rem_man_addr() after this loop.
    ];

    for (base_oid_str, column_name) in columns {
        // lldpRemEntry index: timeMark.localPortNum.remIndex
        walk_subtree(session, base_oid_str, |suffix, value| {
            if suffix.len() < 3 {
                return;
            }
            let local_port = suffix[1] as i32;
            let rem_index = suffix[2] as i32;
            let neighbor =
                neighbors
                    .entry((local_port, rem_index))
                    .or_insert_with(|| LldpNeighbor {
                        local_port_index: local_port,
                        remote_chassis_id_subtype: None,
                        remote_chassis_id_bytes: None,
                        remote_port_id_subtype: None,
                        remote_port_id_bytes: None,
                        remote_port_desc: None,
                        remote_sys_name: None,
                        remote_sys_desc: None,
                        remote_mgmt_addr: None,
                    });
            match column_name {
                "remChassisIdSubtype" => {
                    neighbor.remote_chassis_id_subtype = value_to_i32(value).map(|v| v as u8)
                }
                "remChassisId" => {
                    if let Value::OctetString(bytes) = value {
                        neighbor.remote_chassis_id_bytes = Some(bytes.to_vec());
                    }
                }
                "remPortIdSubtype" => {
                    neighbor.remote_port_id_subtype = value_to_i32(value).map(|v| v as u8)
                }
                "remPortId" => {
                    if let Value::OctetString(bytes) = value {
                        neighbor.remote_port_id_bytes = Some(bytes.to_vec());
                    }
                }
                "remPortDesc" => neighbor.remote_port_desc = value_to_string(value),
                "remSysName" => neighbor.remote_sys_name = value_to_string(value),
                "remSysDesc" => neighbor.remote_sys_desc = value_to_string(value),
                _ => {}
            }
        })
        .await?;
    }

    // Resolve remote management addresses from the separate lldpRemManAddrTable.
    // Its index is timeMark.localPortNum.remIndex.addrSubtype.addrLen.addr, so the
    // address lives in the OID *index*, not the column value. We walk an accessible
    // column (lldpRemManAddrIfSubtype) and reconstruct the address from the index.
    let man_base_oid_str = oids::lldp::remote::entry::LLDP_REM_MAN_ADDR_IF_SUBTYPE;
    // Management address is optional enrichment; ignore walk errors (keeps the
    // neighbours already collected above).
    let _ = walk_subtree(session, man_base_oid_str, |suffix, _value| {
        // suffix = timeMark, localPortNum, remIndex, addrSubtype, addrLen, addr...
        if suffix.len() < 5 {
            return;
        }
        let local_port = suffix[1] as i32;
        let rem_index = suffix[2] as i32;
        let addr_subtype = suffix[3];
        let addr_len = suffix[4] as usize;
        if suffix.len() < 5 + addr_len || addr_len == 0 {
            return;
        }
        // parse_lldp_mgmt_addr expects [ianaFamily, addr bytes...]
        let mut buf = Vec::with_capacity(1 + addr_len);
        buf.push(addr_subtype as u8);
        buf.extend(suffix[5..5 + addr_len].iter().map(|&b| b as u8));
        if let Some(addr) = parse_lldp_mgmt_addr(&buf)
            && let Some(neighbor) = neighbors.get_mut(&(local_port, rem_index))
        {
            neighbor.remote_mgmt_addr = Some(addr);
        }
    })
    .await;

    let result: Vec<LldpNeighbor> = neighbors.into_values().collect();
    debug!("LLDP query from {} returned {} neighbors", ip, result.len());

    Ok(result)
}

/// Walk lldpLocPortTable, returning `lldpLocPortNum -> LldpLocalPort`.
///
/// The local-port index reported in `lldpRemTable` is an `lldpLocPortNum`, which on
/// some vendors (e.g. ExtremeXOS) is a separate namespace from `ifIndex`. This table
/// maps that number to a textual port id (`lldpLocPortId`), which the caller resolves
/// back to the real ifIndex. Returns an empty map if the device does not expose the
/// table (callers fall back to treating the local-port number as the ifIndex).
pub async fn query_lldp_local_ports(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<HashMap<i32, LldpLocalPort>> {
    let mut ports: HashMap<i32, LldpLocalPort> = HashMap::new();

    let columns = [
        (oids::lldp::local::LLDP_LOC_PORT_ID_SUBTYPE, "subtype"),
        (oids::lldp::local::LLDP_LOC_PORT_ID, "id"),
    ];

    for (base_oid_str, column_name) in columns {
        // Index is a single sub-id: lldpLocPortNum.
        walk_subtree(session, base_oid_str, |suffix, value| {
            let Some(&local_port_num) = suffix.first() else {
                return;
            };
            let entry = ports.entry(local_port_num as i32).or_default();
            match column_name {
                "subtype" => entry.port_id_subtype = value_to_i32(value).map(|v| v as u8),
                "id" => entry.port_id = value_to_string(value),
                _ => {}
            }
        })
        .await?;
    }

    debug!(
        "lldpLocPortTable from {} returned {} local ports",
        ip,
        ports.len()
    );
    Ok(ports)
}

/// Query ipAddrTable for IP address to ifIndex + subnet mask mappings.
/// Walks ipAdEntIfIndex and ipAdEntNetMask columns where the OID suffix
/// encodes the IP address as A.B.C.D.
pub async fn query_ip_addr_table(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<HashMap<IpAddr, IpAddrEntry>> {
    let mut if_index_map: HashMap<IpAddr, i32> = HashMap::new();
    let mut net_mask_map: HashMap<IpAddr, IpAddr> = HashMap::new();

    // Walk ipAdEntIfIndex — OID suffix encodes the IP address as A.B.C.D.
    walk_subtree(
        session,
        oids::ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
        |suffix, value| {
            if suffix.len() == 4
                && let Some(if_index) = value_to_i32(value)
            {
                let addr = IpAddr::from([
                    suffix[0] as u8,
                    suffix[1] as u8,
                    suffix[2] as u8,
                    suffix[3] as u8,
                ]);
                if_index_map.insert(addr, if_index);
            }
        },
    )
    .await?;

    // Walk ipAdEntNetMask
    walk_subtree(
        session,
        oids::ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
        |suffix, value| {
            if suffix.len() == 4
                && let Some(mask) = value_to_ip(value)
            {
                let addr = IpAddr::from([
                    suffix[0] as u8,
                    suffix[1] as u8,
                    suffix[2] as u8,
                    suffix[3] as u8,
                ]);
                net_mask_map.insert(addr, mask);
            }
        },
    )
    .await?;

    // Combine ifIndex and netMask results
    let result: HashMap<IpAddr, IpAddrEntry> = if_index_map
        .into_iter()
        .map(|(addr, if_index)| {
            let net_mask = net_mask_map.get(&addr).copied();
            (addr, IpAddrEntry { if_index, net_mask })
        })
        .collect();

    debug!(
        "ipAddrTable walk from {} returned {} entries",
        ip,
        result.len()
    );

    Ok(result)
}

/// Query CDP cache table for neighbor information (Cisco devices)
pub async fn query_cdp_neighbors(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<Vec<CdpNeighbor>> {
    let mut neighbors: HashMap<(i32, i32), CdpNeighbor> = HashMap::new();

    let columns = [
        (oids::cdp::entry::CDP_CACHE_DEVICE_ID, "deviceId"),
        (oids::cdp::entry::CDP_CACHE_DEVICE_PORT, "devicePort"),
        (oids::cdp::entry::CDP_CACHE_PLATFORM, "platform"),
        (oids::cdp::entry::CDP_CACHE_ADDRESS, "address"),
    ];

    for (base_oid_str, column_name) in columns {
        // CDP index: cdpCacheIfIndex.cdpCacheDeviceIndex
        walk_subtree(session, base_oid_str, |suffix, value| {
            if suffix.len() < 2 {
                return;
            }
            let if_index = suffix[0] as i32;
            let device_index = suffix[1] as i32;
            let neighbor = neighbors
                .entry((if_index, device_index))
                .or_insert_with(|| CdpNeighbor {
                    local_port_index: if_index,
                    remote_device_id: None,
                    remote_port_id: None,
                    remote_platform: None,
                    remote_address: None,
                });
            match column_name {
                "deviceId" => neighbor.remote_device_id = value_to_string(value),
                "devicePort" => neighbor.remote_port_id = value_to_string(value),
                "platform" => neighbor.remote_platform = value_to_string(value),
                "address" => {
                    // CDP address is encoded as 4 bytes for IPv4
                    if let Value::OctetString(bytes) = value
                        && bytes.len() == 4
                    {
                        neighbor.remote_address =
                            Some(IpAddr::from([bytes[0], bytes[1], bytes[2], bytes[3]]));
                    }
                }
                _ => {}
            }
        })
        .await?;
    }

    let result: Vec<CdpNeighbor> = neighbors.into_values().collect();
    debug!("CDP query from {} returned {} neighbors", ip, result.len());

    Ok(result)
}

/// Query ARP table (ipNetToMediaTable) for IP-to-MAC mappings.
/// Returns entries with ifIndex, MAC, and IP for each ARP cache entry.
pub async fn query_arp_table(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<Vec<ArpEntry>> {
    // We need to walk 4 columns: ifIndex, physAddress, netAddress, type
    // OID suffix format: ifIndex.A.B.C.D
    struct ArpEntryBuilder {
        if_index: Option<i32>,
        mac_address: Option<mac_address::MacAddress>,
        ip_address: Option<IpAddr>,
        entry_type: Option<i32>,
    }

    let mut entries: HashMap<String, ArpEntryBuilder> = HashMap::new();

    let columns = [
        (oids::arp::entry::IP_NET_TO_MEDIA_IF_INDEX, "ifIndex"),
        (
            oids::arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
            "physAddress",
        ),
        (oids::arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS, "netAddress"),
        (oids::arp::entry::IP_NET_TO_MEDIA_TYPE, "type"),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix: ifIndex.A.B.C.D
        walk_subtree(session, base_oid_str, |suffix, value| {
            if suffix.len() < 5 {
                return;
            }
            let key = suffix
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(".");
            let entry = entries.entry(key).or_insert_with(|| ArpEntryBuilder {
                if_index: None,
                mac_address: None,
                ip_address: None,
                entry_type: None,
            });
            match column_name {
                "ifIndex" => entry.if_index = value_to_i32(value),
                "physAddress" => entry.mac_address = value_to_mac(value),
                "netAddress" => entry.ip_address = value_to_ip(value),
                "type" => entry.entry_type = value_to_i32(value),
                _ => {}
            }
        })
        .await?;
    }

    // Filter out invalid entries (type==2) and entries missing required fields
    let result: Vec<ArpEntry> = entries
        .into_values()
        .filter_map(|e| {
            let entry_type = e.entry_type.unwrap_or(0);
            // Skip invalid entries (type 2)
            if entry_type == 2 {
                return None;
            }
            Some(ArpEntry {
                if_index: e.if_index?,
                mac_address: e.mac_address?,
                ip_address: e.ip_address?,
            })
        })
        .collect();

    debug!(
        "ARP table walk from {} returned {} entries",
        ip,
        result.len()
    );

    Ok(result)
}

/// Query ENTITY-MIB entPhysicalTable for hardware inventory.
/// Returns the best-match physical entity (chassis > stack > module).
pub async fn query_entity_physical(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<Option<DeviceInventory>> {
    struct PhysicalEntry {
        description: Option<String>,
        class: Option<i32>,
        name: Option<String>,
        serial_number: Option<String>,
        manufacturer: Option<String>,
        model: Option<String>,
    }

    let mut entries: HashMap<i32, PhysicalEntry> = HashMap::new();

    let columns = [
        (oids::entity::entry::ENT_PHYSICAL_DESCR, "descr"),
        (oids::entity::entry::ENT_PHYSICAL_CLASS, "class"),
        (oids::entity::entry::ENT_PHYSICAL_NAME, "name"),
        (oids::entity::entry::ENT_PHYSICAL_SERIAL_NUM, "serialNum"),
        (oids::entity::entry::ENT_PHYSICAL_MFG_NAME, "mfgName"),
        (oids::entity::entry::ENT_PHYSICAL_MODEL_NAME, "modelName"),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix is entPhysicalIndex (single integer).
        walk_subtree(session, base_oid_str, |suffix, value| {
            let Some(&index_u64) = suffix.last() else {
                return;
            };
            let entry = entries
                .entry(index_u64 as i32)
                .or_insert_with(|| PhysicalEntry {
                    description: None,
                    class: None,
                    name: None,
                    serial_number: None,
                    manufacturer: None,
                    model: None,
                });
            match column_name {
                "descr" => entry.description = value_to_string(value),
                "class" => entry.class = value_to_i32(value),
                "name" => entry.name = value_to_string(value),
                "serialNum" => {
                    entry.serial_number = value_to_string(value).filter(|s| !s.is_empty())
                }
                "mfgName" => entry.manufacturer = value_to_string(value).filter(|s| !s.is_empty()),
                "modelName" => entry.model = value_to_string(value).filter(|s| !s.is_empty()),
                _ => {}
            }
        })
        .await?;
    }

    // Select best match: prefer chassis (3), fallback to stack (11), then module (9)
    let best = entries
        .values()
        .find(|e| e.class == Some(3))
        .or_else(|| entries.values().find(|e| e.class == Some(11)))
        .or_else(|| entries.values().find(|e| e.class == Some(9)));

    let result = best.map(|e| DeviceInventory {
        description: e.description.clone().or_else(|| e.name.clone()),
        manufacturer: e.manufacturer.clone(),
        model: e.model.clone(),
        serial_number: e.serial_number.clone(),
    });

    debug!(
        "ENTITY-MIB query from {} returned {} physical entries, best match: {}",
        ip,
        entries.len(),
        result.is_some()
    );

    Ok(result)
}

/// Walk dot1dBasePortIfIndex to build bridge_port → ifIndex mapping.
/// Shared by query_bridge_fdb() and query_port_vlan_membership().
async fn walk_bridge_port_mapping(
    session: &mut Box<snmp2::AsyncSession>,
) -> Result<HashMap<i32, i32>> {
    let mut port_to_if_index: HashMap<i32, i32> = HashMap::new();
    // OID suffix is the bridge port number; value is the ifIndex.
    walk_subtree(
        session,
        oids::bridge::DOT1D_BASE_PORT_IF_INDEX,
        |suffix, value| {
            if let Some(&port_u64) = suffix.last()
                && let Some(if_index) = value_to_i32(value)
            {
                port_to_if_index.insert(port_u64 as i32, if_index);
            }
        },
    )
    .await?;

    Ok(port_to_if_index)
}

/// In-progress FDB row assembled column-by-column across an SNMP walk, keyed by
/// its MAC. Shared by the legacy (dot1dTpFdbTable) and VLAN-aware (dot1qTpFdbTable)
/// collectors so their results can be merged by MAC.
#[derive(Default)]
struct FdbBuilder {
    mac_address: Option<mac_address::MacAddress>,
    port: Option<i32>,
    status: Option<i32>,
}

/// Query bridge FDB for MAC-to-port mappings, resolving bridge ports to ifIndex
/// values via dot1dBasePortIfIndex. Collects both the legacy `dot1dTpFdbTable`
/// (RFC 4188) and the VLAN-aware `dot1qTpFdbTable` (Q-BRIDGE, RFC 4363) — many
/// VLAN-aware switches (Aruba/HP ProCurve, etc.) populate only the latter and
/// leave the legacy table empty, so relying on dot1d alone silently produced no
/// L2 adjacency for them (GH #649).
pub async fn query_bridge_fdb(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<Vec<BridgeFdbEntry>> {
    // Step 1: Walk dot1dBasePortIfIndex to build bridge_port → ifIndex map.
    // Both FDB tables reference this same dot1dBasePort space.
    let port_to_if_index = walk_bridge_port_mapping(session).await?;

    // Step 2: Walk legacy dot1dTpFdbTable columns.
    let mut fdb_entries: HashMap<String, FdbBuilder> = HashMap::new();

    let columns = [
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_ADDRESS, "address"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_PORT, "port"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_STATUS, "status"),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix is a 6-octet MAC encoded as 6 sub-ids.
        walk_subtree(session, base_oid_str, |suffix, value| {
            if suffix.len() != 6 {
                return;
            }
            let key = suffix
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(".");
            let entry = fdb_entries.entry(key).or_default();
            match column_name {
                "address" => entry.mac_address = value_to_mac(value),
                "port" => entry.port = value_to_i32(value),
                "status" => entry.status = value_to_i32(value),
                _ => {}
            }
        })
        .await?;
    }

    // Step 3: Merge in VLAN-aware Q-BRIDGE dot1qTpFdbTable entries. Legacy rows
    // win; Q-BRIDGE fills in MACs the legacy table didn't report (or all of them,
    // on switches that populate only the Q-BRIDGE table).
    let legacy_count = fdb_entries.len();
    let qbridge = walk_qbridge_fdb(session).await.unwrap_or_default();
    let qbridge_count = qbridge.len();
    for (key, builder) in qbridge {
        fdb_entries.entry(key).or_insert(builder);
    }

    // Filter: keep learned (3) and self (5), resolve bridge port to ifIndex
    let result: Vec<BridgeFdbEntry> = fdb_entries
        .into_values()
        .filter_map(|e| {
            let status = e.status.unwrap_or(0);
            if status != 3 && status != 5 {
                return None;
            }
            let bridge_port = e.port?;
            Some(BridgeFdbEntry {
                mac_address: e.mac_address?,
                bridge_port,
                if_index: port_to_if_index.get(&bridge_port).copied(),
                status,
            })
        })
        .collect();

    // Debug-level (enable SCANOPY_LOG_LEVEL=debug) with the legacy-vs-Q-BRIDGE split: on a
    // VLAN-aware switch, legacy=0 with qbridge>0 confirms the daemon has (and is using) the
    // Q-BRIDGE FDB collection; legacy=0 and qbridge=0 on a switch that snmpwalk shows has FDB data
    // points at an un-upgraded daemon or a MIB the switch doesn't expose (GH #649).
    tracing::debug!(
        ip = %ip,
        entries = result.len(),
        legacy_dot1d = legacy_count,
        qbridge_dot1q = qbridge_count,
        port_mappings = port_to_if_index.len(),
        "Bridge FDB walk finished"
    );

    Ok(result)
}

/// Walk the VLAN-aware Q-BRIDGE FDB (`dot1qTpFdbTable`, RFC 4363) for MAC→port
/// mappings, keyed by MAC so results merge with the legacy `dot1dTpFdbTable`.
///
/// Unlike the legacy table, the MAC lives in the table INDEX
/// (`dot1qFdbId` + 6 MAC octets), not a column, so it's derived from the OID
/// suffix. Ports are `dot1dBasePort` numbers, resolved by the caller against the
/// same `dot1dBasePortIfIndex` map. VLAN-aware switches (Aruba/HP ProCurve, etc.)
/// often populate only this table (GH #649).
async fn walk_qbridge_fdb(
    session: &mut Box<snmp2::AsyncSession>,
) -> Result<HashMap<String, FdbBuilder>> {
    let mut entries: HashMap<String, FdbBuilder> = HashMap::new();

    let columns = [
        (oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_PORT, "port"),
        (oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_STATUS, "status"),
    ];

    for (base_oid_str, column_name) in columns {
        // Q-BRIDGE index = dot1qFdbId (1 sub-id) + MAC (6 octets).
        walk_subtree(session, base_oid_str, |suffix, value| {
            let Some(mac) = qbridge_fdb_index_to_mac(suffix) else {
                return;
            };
            if suffix.len() < 7 {
                return;
            }
            // Key by MAC alone (drop fdb_id) so the same MAC learned across VLANs
            // collapses to one entry and merges with the legacy table's MAC key.
            let key = suffix[1..7]
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(".");
            let entry = entries.entry(key).or_default();
            entry.mac_address = Some(mac);
            match column_name {
                "port" => entry.port = value_to_i32(value),
                "status" => entry.status = value_to_i32(value),
                _ => {}
            }
        })
        .await?;
    }

    Ok(entries)
}

/// Query local LLDP chassis ID (scalar GETs, not walks).
/// Returns the device's own LLDP identity.
pub async fn query_lldp_local(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<Option<LldpLocalInfo>> {
    // GET lldpLocChassisIdSubtype
    let subtype_oid = parse_oid(oids::lldp::local::LLDP_LOC_CHASSIS_ID_SUBTYPE)?;
    let subtype = match timeout(SNMP_TIMEOUT, session.get(&subtype_oid)).await {
        Ok(Ok(mut response)) => response
            .varbinds
            .next()
            .and_then(|(_, value)| value_to_i32(&value))
            .map(|v| v as u8),
        Ok(Err(e)) => {
            debug!(
                "LLDP local chassis ID subtype GET failed from {}: {:?}",
                ip, e
            );
            None
        }
        Err(_) => {
            debug!("LLDP local chassis ID subtype GET timeout from {}", ip);
            None
        }
    };

    // GET lldpLocChassisId
    let chassis_oid = parse_oid(oids::lldp::local::LLDP_LOC_CHASSIS_ID)?;
    let chassis_bytes = match timeout(SNMP_TIMEOUT, session.get(&chassis_oid)).await {
        Ok(Ok(mut response)) => response.varbinds.next().and_then(|(_, value)| {
            if let Value::OctetString(bytes) = &value {
                Some(bytes.to_vec())
            } else {
                None
            }
        }),
        Ok(Err(e)) => {
            debug!("LLDP local chassis ID GET failed from {}: {:?}", ip, e);
            None
        }
        Err(_) => {
            debug!("LLDP local chassis ID GET timeout from {}", ip);
            None
        }
    };

    match (subtype, chassis_bytes) {
        (Some(subtype), Some(bytes)) => {
            debug!(
                "LLDP local info from {}: subtype={}, bytes_len={}",
                ip,
                subtype,
                bytes.len()
            );
            Ok(Some(LldpLocalInfo {
                chassis_id_subtype: subtype,
                chassis_id_bytes: bytes,
            }))
        }
        _ => {
            debug!("LLDP local info incomplete from {}", ip);
            Ok(None)
        }
    }
}

/// Query VLAN table for VLAN IDs and names.
/// Tries Q-BRIDGE dot1qVlanStaticName first, falls back to Cisco VTP vtpVlanName.
pub async fn query_vlan_table(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<Vec<VlanInfo>> {
    let mut vlans: Vec<VlanInfo> = Vec::new();

    // Try Q-BRIDGE dot1qVlanStaticName first. OID suffix is the VLAN ID.
    walk_subtree(
        session,
        oids::vlan::q_bridge::DOT1Q_VLAN_STATIC_NAME,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last()
                && let Some(name) = value_to_string(value)
            {
                vlans.push(VlanInfo {
                    vlan_id: vlan_u64 as u16,
                    name,
                });
            }
        },
    )
    .await?;

    // Fall back to Cisco VTP if Q-BRIDGE returned nothing. VTP index is
    // mgmtDomainIndex.vlanId — use the last sub-id as the VLAN ID.
    if vlans.is_empty() {
        walk_subtree(
            session,
            oids::vlan::cisco_vtp::VTP_VLAN_NAME,
            |suffix, value| {
                if let Some(&vlan_u64) = suffix.last()
                    && let Some(name) = value_to_string(value)
                {
                    vlans.push(VlanInfo {
                        vlan_id: vlan_u64 as u16,
                        name,
                    });
                }
            },
        )
        .await?;
    }

    debug!(
        "VLAN table query from {} returned {} entries (Q-BRIDGE or VTP)",
        ip,
        vlans.len()
    );

    Ok(vlans)
}

/// Query per-port VLAN membership from Q-BRIDGE-MIB.
/// Uses dot1qPvid for native VLANs and dot1qVlanCurrentEgressPorts/UntaggedPorts
/// for tagged VLAN membership. Resolves bridge ports to ifIndex.
pub async fn query_port_vlan_membership(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) -> Result<Vec<PortVlanMembership>> {
    // Step 1: Get bridge port → ifIndex mapping
    let port_to_if_index = walk_bridge_port_mapping(session).await?;

    if port_to_if_index.is_empty() {
        debug!(
            "No bridge port mappings from {} — skipping VLAN membership",
            ip
        );
        return Ok(Vec::new());
    }

    // Step 2: Walk dot1qPvid for native VLAN per bridge port. OID suffix is the
    // bridge port number; value is the native VLAN ID.
    let mut native_vlans: HashMap<i32, u16> = HashMap::new();
    walk_subtree(
        session,
        oids::vlan::q_bridge::DOT1Q_PVID,
        |suffix, value| {
            if let Some(&port_u64) = suffix.last()
                && let Some(vlan_id) = value_to_u16(value)
            {
                native_vlans.insert(port_u64 as i32, vlan_id);
            }
        },
    )
    .await?;

    // Step 3: Walk dot1qVlanCurrentEgressPorts — PortList bitmap per VLAN, indexed
    // by timeFilter.vlanId (last sub-id is the VLAN ID).
    let mut egress_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_subtree(
        session,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_EGRESS_PORTS,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last()
                && let Value::OctetString(bytes) = value
            {
                let vlan_id = vlan_u64 as u16;
                for bp in parse_portlist_bitmap(bytes) {
                    egress_by_port.entry(bp).or_default().push(vlan_id);
                }
            }
        },
    )
    .await?;

    // Step 4: Walk dot1qVlanCurrentUntaggedPorts — same bitmap format.
    let mut untagged_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_subtree(
        session,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_UNTAGGED_PORTS,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last()
                && let Value::OctetString(bytes) = value
            {
                let vlan_id = vlan_u64 as u16;
                for bp in parse_portlist_bitmap(bytes) {
                    untagged_by_port.entry(bp).or_default().push(vlan_id);
                }
            }
        },
    )
    .await?;

    // Step 5: Assemble per-port membership, resolving bridge port → ifIndex
    let mut result: Vec<PortVlanMembership> = Vec::new();

    for (&bridge_port, &if_index) in &port_to_if_index {
        let native_vlan = native_vlans.get(&bridge_port).copied();
        let egress_vlans = egress_by_port.get(&bridge_port);
        let untagged_vlans = untagged_by_port.get(&bridge_port);

        // Tagged VLANs = egress VLANs minus untagged VLANs for this port
        let tagged_vlans: Vec<u16> = match egress_vlans {
            Some(egress) => {
                let untagged_set: std::collections::HashSet<u16> = untagged_vlans
                    .map(|v| v.iter().copied().collect())
                    .unwrap_or_default();
                egress
                    .iter()
                    .copied()
                    .filter(|v| !untagged_set.contains(v))
                    .collect()
            }
            None => Vec::new(),
        };

        // Only include ports that have some VLAN data
        if native_vlan.is_some() || !tagged_vlans.is_empty() {
            result.push(PortVlanMembership {
                if_index,
                native_vlan,
                tagged_vlans,
            });
        }
    }

    debug!(
        "VLAN membership query from {} returned {} port memberships ({} bridge port mappings)",
        ip,
        result.len(),
        port_to_if_index.len()
    );

    Ok(result)
}

#[cfg(test)]
mod walk_tests {
    use super::*;
    use std::collections::VecDeque;

    const BASE: &str = "1.3.6.1.2.1.2.2.1.1";

    fn page(oids: &[&str]) -> Vec<Vec<u64>> {
        oids.iter()
            .map(|s| s.split('.').map(|p| p.parse().unwrap()).collect())
            .collect()
    }

    /// Serves canned pages of OIDs to `walk_subtree`. Once `pages` is drained it repeats
    /// `repeat` forever, which is how a device that never advances its OID is modelled.
    /// Only OIDs are stored — `Value` isn't `Clone`, and the walk's termination logic
    /// only cares about OID progression — so each page mints fresh integer values.
    struct MockTransport {
        pages: VecDeque<Vec<Vec<u64>>>,
        repeat: Option<Vec<Vec<u64>>>,
    }

    impl MockTransport {
        fn scripted(pages: &[Vec<Vec<u64>>]) -> Self {
            Self {
                pages: pages.iter().cloned().collect(),
                repeat: None,
            }
        }

        fn stalling(p: Vec<Vec<u64>>) -> Self {
            Self {
                pages: VecDeque::new(),
                repeat: Some(p),
            }
        }

        fn next_page(&mut self) -> Varbinds<'static> {
            self.pages
                .pop_front()
                .or_else(|| self.repeat.clone())
                .unwrap_or_default()
                .into_iter()
                .map(|o| (o, Value::Integer(1)))
                .collect()
        }
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for MockTransport {
        async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
            Ok(WalkPage::Varbinds(self.next_page()))
        }

        async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
            Ok(self.next_page())
        }
    }

    /// A device that keeps answering with the same in-subtree OID must not walk to the
    /// entry cap (which on a live host burns the whole integration budget) — the
    /// strict-advance guard has to cut it short and report a partial walk.
    #[tokio::test]
    async fn walk_terminates_when_oid_does_not_advance() {
        let mut session = MockTransport::stalling(page(&["1.3.6.1.2.1.2.2.1.1.5"]));

        let mut seen = 0usize;
        let complete = walk_subtree(&mut session, BASE, |_suffix, _v| seen += 1)
            .await
            .unwrap();

        assert!(!complete, "a non-advancing walk must report as partial");
        assert!(
            seen < MAX_WALK_ENTRIES,
            "guard must stop the walk, not run to the cap (saw {seen} entries)"
        );
    }

    /// The guard must not fire on a normal multi-page walk: each page's tail OID
    /// strictly exceeds the OID it was requested from.
    #[tokio::test]
    async fn walk_completes_across_advancing_pages() {
        let mut session = MockTransport::scripted(&[
            page(&["1.3.6.1.2.1.2.2.1.1.1", "1.3.6.1.2.1.2.2.1.1.2"]),
            page(&["1.3.6.1.2.1.2.2.1.1.3", "1.3.6.1.2.1.2.2.1.1.4"]),
            // Next column — outside the base subtree, so the walk ends naturally.
            page(&["1.3.6.1.2.1.2.2.1.2.1"]),
        ]);

        let mut suffixes = Vec::new();
        let complete = walk_subtree(&mut session, BASE, |suffix, _v| {
            suffixes.push(suffix.to_vec())
        })
        .await
        .unwrap();

        assert!(
            complete,
            "a walk that reaches the end of the subtree is complete"
        );
        assert_eq!(suffixes, vec![vec![1], vec![2], vec![3], vec![4]]);
    }
}

/// `walk_if_table` assembles one interface per ifIndex across eleven separate column walks, and
/// until now had no test at all — the multi-column assembly, the row-minting and the `complete`
/// aggregation were all uncovered, which is how a foreign interface ended up on a switch and was
/// still reported as an authoritative full ifTable.
#[cfg(test)]
mod if_table_tests {
    use super::*;

    const IF_INDEX: &str = "1.3.6.1.2.1.2.2.1.1";
    const IF_DESCR: &str = "1.3.6.1.2.1.2.2.1.2";

    /// A value an agent can return. `Value` borrows, so the test data is `'static`.
    #[derive(Clone)]
    enum Canned {
        Int(i64),
        Str(&'static str),
    }

    /// An agent backed by a sorted OID table, answering GETNEXT/GETBULK the way a real one does:
    /// every row strictly greater than the requested OID, in order. That is what makes the
    /// multi-column walk behave as it does in production — each column walk asks from its own
    /// base and stops when the responses leave that subtree.
    struct FakeAgent {
        rows: Vec<(Vec<u64>, Canned)>,
    }

    impl FakeAgent {
        fn new(rows: &[(&str, Canned)]) -> Self {
            let mut rows: Vec<(Vec<u64>, Canned)> = rows
                .iter()
                .map(|(oid, v)| {
                    (
                        oid.split('.').map(|p| p.parse().unwrap()).collect(),
                        v.clone(),
                    )
                })
                .collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            Self { rows }
        }

        /// The ifTable of a switch whose 16 ports live at high ifIndexes, like the Omada
        /// TL-SG3216 in the SNMP sim.
        fn omada() -> Vec<(&'static str, Canned)> {
            let mut rows = vec![
                ("1.3.6.1.2.1.2.2.1.1.1", Canned::Int(1)),
                ("1.3.6.1.2.1.2.2.1.2.1", Canned::Str("Vlan-interface1")),
            ];
            // A handful of ports is enough to prove the assembly; the real device has 16.
            for (idx, oid, descr) in [
                (
                    49153u64,
                    "1.3.6.1.2.1.2.2.1.1.49153",
                    "gigabitEthernet 1/0/1",
                ),
                (49154, "1.3.6.1.2.1.2.2.1.1.49154", "gigabitEthernet 1/0/2"),
                (49155, "1.3.6.1.2.1.2.2.1.1.49155", "gigabitEthernet 1/0/3"),
            ] {
                rows.push((oid, Canned::Int(idx as i64)));
                rows.push((
                    match idx {
                        49153 => "1.3.6.1.2.1.2.2.1.2.49153",
                        49154 => "1.3.6.1.2.1.2.2.1.2.49154",
                        _ => "1.3.6.1.2.1.2.2.1.2.49155",
                    },
                    Canned::Str(descr),
                ));
            }
            rows
        }

        fn page(&self, from: &[u64]) -> Varbinds<'_> {
            let page: Varbinds<'_> = self
                .rows
                .iter()
                .filter(|(oid, _)| oid.as_slice() > from)
                .take(BULK_MAX_REPETITIONS as usize)
                .map(|(oid, v)| {
                    let value = match v {
                        Canned::Int(i) => Value::Integer(*i),
                        Canned::Str(s) => Value::OctetString(s.as_bytes()),
                    };
                    (oid.clone(), value)
                })
                .collect();

            // Past the last row a real agent says so rather than answering with nothing — an
            // empty response is abnormal and the walk rightly treats it as truncation. Columns
            // this device doesn't implement have to end this way or every walk reads as partial.
            if page.is_empty() {
                return vec![(from.to_vec(), Value::EndOfMibView)];
            }
            page
        }
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for FakeAgent {
        async fn walk_getbulk<'a>(&'a mut self, from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
            Ok(WalkPage::Varbinds(self.page(from)))
        }

        async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
            Ok(self.page(from))
        }
    }

    fn ip() -> IpAddr {
        "192.0.2.1".parse().unwrap()
    }

    #[tokio::test]
    async fn assembles_one_entry_per_if_index_across_columns() {
        let mut agent = FakeAgent::new(&FakeAgent::omada());

        let (entries, complete) = walk_if_table(&mut agent, ip()).await.unwrap();

        assert!(complete, "a device answering every column is authoritative");
        assert_eq!(
            entries.iter().map(|e| e.if_index).collect::<Vec<_>>(),
            vec![1, 49153, 49154, 49155],
            "every ifIndex appears exactly once, in order"
        );
        assert_eq!(entries[0].if_descr.as_deref(), Some("Vlan-interface1"));
        assert_eq!(
            entries[1].if_descr.as_deref(),
            Some("gigabitEthernet 1/0/1"),
            "a high ifIndex must keep the description from its own column"
        );
    }

    /// The reported defect: a switch came back with an interface belonging to a different device.
    ///
    /// Every column mints a row on sight, so a single varbind under `ifDescr` for an ifIndex the
    /// device never listed in `ifIndex` was enough to invent an interface — and the walk still
    /// reported itself complete, which lets the server prune real interfaces against a table it
    /// should not trust (#649). The row must be discarded and the walk must admit it is partial.
    #[tokio::test]
    async fn a_row_for_an_unlisted_if_index_is_discarded_and_makes_the_walk_partial() {
        let mut rows = FakeAgent::omada();
        // ifIndex 2 exists only in the ifDescr column — the shape of the foreign row.
        rows.push(("1.3.6.1.2.1.2.2.1.2.2", Canned::Str("ge-0/0/1")));
        let mut agent = FakeAgent::new(&rows);

        let (entries, complete) = walk_if_table(&mut agent, ip()).await.unwrap();

        assert!(
            !entries.iter().any(|e| e.if_index == 2),
            "an ifIndex the device never listed must not become an interface"
        );
        assert_eq!(
            entries.iter().map(|e| e.if_index).collect::<Vec<_>>(),
            vec![1, 49153, 49154, 49155]
        );
        assert!(
            !complete,
            "a table carrying rows the device never listed is not authoritative, so the server \
             must not prune against it"
        );
    }

    /// The guard only applies once the device has actually told us its ifIndex set. An agent that
    /// serves no ifIndex column at all still gets its other columns, as before.
    #[tokio::test]
    async fn a_device_serving_no_if_index_column_still_yields_interfaces() {
        let mut agent = FakeAgent::new(&[
            ("1.3.6.1.2.1.2.2.1.2.7", Canned::Str("eth7")),
            ("1.3.6.1.2.1.2.2.1.3.7", Canned::Int(6)),
        ]);

        let (entries, _) = walk_if_table(&mut agent, ip()).await.unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].if_index, 7);
        assert_eq!(entries[0].if_descr.as_deref(), Some("eth7"));
    }

    /// A response that leaves the subtree *without advancing* is not this walk's natural end — it
    /// is an answer to some other question. Reporting it as a finished column is what let a
    /// silently short ifTable claim to be complete.
    #[tokio::test]
    async fn a_non_advancing_out_of_subtree_response_reports_partial() {
        struct StaleAgent;

        #[async_trait::async_trait]
        impl SnmpWalkTransport for StaleAgent {
            async fn walk_getbulk<'a>(
                &'a mut self,
                _from: &[u64],
                _max: u32,
            ) -> Result<WalkPage<'a>> {
                // Below the requested base, so it neither belongs to the subtree nor advances.
                Ok(WalkPage::Varbinds(vec![(
                    vec![1, 3, 6, 1, 2, 1, 1, 1, 0],
                    Value::Integer(1),
                )]))
            }

            async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
                unreachable!("getbulk answers first")
            }
        }

        let complete = walk_subtree(&mut StaleAgent, IF_DESCR, |_, _| {})
            .await
            .unwrap();
        assert!(!complete);
    }

    /// The other side of that rule: a genuine end-of-column response *does* advance past the
    /// subtree, and must still count as a complete walk.
    #[tokio::test]
    async fn a_natural_end_of_column_still_reports_complete() {
        let mut agent = FakeAgent::new(&[
            ("1.3.6.1.2.1.2.2.1.1.1", Canned::Int(1)),
            ("1.3.6.1.2.1.2.2.1.2.1", Canned::Str("eth0")),
        ]);

        let mut seen = 0usize;
        let complete = walk_subtree(&mut agent, IF_INDEX, |_, _| seen += 1)
            .await
            .unwrap();

        assert!(complete, "walking off the end of a column is a natural end");
        assert_eq!(seen, 1);
    }
}
