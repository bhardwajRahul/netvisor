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
    let mut stop = WalkStop::EndOfSubtree;
    let mut stop_detail: Option<String> = None;

    'walk: loop {
        if count >= MAX_WALK_ENTRIES {
            stop = WalkStop::EntryCap;
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
                    stop = WalkStop::Transport;
                    stop_detail = Some(e.to_string());
                    break;
                }
            }
        } else {
            match session.walk_getnext(&current_parts).await {
                Ok(v) => v,
                Err(e) => {
                    stop = WalkStop::Transport;
                    stop_detail = Some(e.to_string());
                    break;
                }
            }
        };

        // Empty response mid-walk is abnormal (getbulk) or an exhausted column
        // (getnext) — treat as partial either way.
        if varbinds.is_empty() {
            stop = WalkStop::EmptyResponse;
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
                stop = WalkStop::EndOfMibView;
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
                stop = if resp_parts <= current_parts {
                    stop_detail = Some(format!("responded with {resp_parts:?}"));
                    WalkStop::StaleResponse
                } else {
                    WalkStop::EndOfSubtree
                };
                done = true;
                break;
            }
            on_entry(&resp_parts[base_parts.len()..], &value);
            count += 1;
            next_parts = Some(resp_parts);
            if count >= MAX_WALK_ENTRIES {
                stop = WalkStop::EntryCap;
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
                    stop_detail = Some(format!("responded with {parts:?}"));
                    stop = WalkStop::NonAdvancingOid;
                    break;
                }
                current_parts = parts;
            }
            None => {
                stop = WalkStop::EmptyResponse;
                break;
            }
        }
    }

    // A truncated column is why interfaces and neighbours go missing, and until now the reason was
    // invisible — a timeout and a session reading stale answers produce identical data. Logged at
    // info because it is both rare on a healthy network and the first thing worth knowing when it
    // is not; a clean walk stays silent.
    if stop.is_truncation() {
        tracing::info!(
            base = base_oid_str,
            ?stop,
            detail = stop_detail.as_deref().unwrap_or(""),
            entries = count,
            "SNMP walk truncated"
        );
    }

    Ok(!stop.is_truncation())
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

/// Records from a multi-column SNMP walk, plus whether the walk actually saw everything.
///
/// Absent data is ambiguous on its own: "this device has no neighbour on that port" and "we failed
/// to read it" are both an empty record, and they call for opposite responses server-side — clear
/// the stored value, or keep it. Only the daemon can tell them apart, so the answer travels with
/// the data.
///
/// `Default` is deliberately `complete: false`. These queries run under `query_or_default`, so a
/// whole-query timeout yields the default — and an empty result from a query that never finished
/// must never be mistaken for a device authoritatively reporting nothing.
#[derive(Debug)]
pub struct SnmpCollection<T> {
    pub records: T,
    pub complete: bool,
}

impl<T: Default> Default for SnmpCollection<T> {
    fn default() -> Self {
        Self {
            records: T::default(),
            complete: false,
        }
    }
}

/// The outcome of walking the ifTable/ifXTable columns.
///
/// Two independent notions of "complete", because they answer different questions and only one of
/// them may gate a destructive operation. `ifIndex` is the table's index column: it alone decides
/// *which* interfaces exist. The other ten carry attributes of interfaces already known.
///
/// Collapsing both into one flag (as this used to) meant a timed-out `ifDescr` read blocked the
/// server-side prune — so stale interfaces lingered on any device with one flaky column — and
/// raised an operator warning about missing interfaces when none were missing.
#[derive(Default)]
pub struct IfTableWalk {
    pub entries: Vec<IfTableEntry>,
    /// Every interface the device listed is present. The set is authoritative, so the server may
    /// prune interfaces absent from it (#649). False whenever the `ifIndex` column itself was cut
    /// short, or a column answered for an interface the device never listed.
    pub set_complete: bool,
    /// Every attribute column also walked to its end. False means some descriptions, speeds or
    /// aliases may be blank — a cosmetic gap, never a reason to withhold pruning.
    pub attributes_complete: bool,
}

// `Default` is the hard-failure outcome (`query_or_default`): no entries, and neither flag set,
// so a walk that never ran can never be mistaken for an authoritative one.

/// Why a column walk stopped.
///
/// Only the first two are a genuine end; the rest are truncation, and telling them apart is the
/// whole diagnostic. "The device is slow" (`Timeout`) and "this session is reading answers to
/// questions it already gave up on" (`SessionDesync`) look identical in the data — both just
/// produce a short column — but they call for completely different responses.
#[derive(Debug, Clone, Copy)]
enum WalkStop {
    /// Responses moved past the requested subtree — the column is finished.
    EndOfSubtree,
    /// Agent signalled end-of-MIB / no-such-object.
    EndOfMibView,
    /// Hit `MAX_WALK_ENTRIES`.
    EntryCap,
    /// getbulk/getnext returned an error. The message distinguishes a timeout from a
    /// request-id or community mismatch.
    Transport,
    /// Agent answered with no varbinds at all mid-walk.
    EmptyResponse,
    /// Agent answered with an OID that did not advance — it would loop for ever.
    NonAdvancingOid,
    /// Left the subtree without advancing: not this walk's continuation at all.
    StaleResponse,
}

impl WalkStop {
    fn is_truncation(self) -> bool {
        !matches!(self, Self::EndOfSubtree | Self::EndOfMibView)
    }
}

/// Walk one column, recording in `complete` whether it reached the end.
///
/// Every multi-column query needs this and none of them had it: `walk_subtree` never returns
/// `Err`, so the `?` these call sites used was dead code and a truncated column was invisible.
async fn walk_column<T, F>(session: &mut T, base_oid_str: &str, complete: &mut bool, on_entry: F)
where
    T: SnmpWalkTransport,
    F: FnMut(&[u64], &Value),
{
    if !walk_subtree(session, base_oid_str, on_entry)
        .await
        .unwrap_or(false)
    {
        *complete = false;
    }
}

/// Walk the ifTable/ifXTable columns.
///
/// See [`IfTableWalk`] for what the two completeness flags mean and why they are separate.
pub async fn walk_if_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<IfTableWalk> {
    let mut entries: HashMap<i32, IfTableEntry> = HashMap::new();
    // Cleared to false the moment any column walk is cut short (error/timeout/limit).
    let mut complete = true;
    // Whether the index column specifically survived. `None` until it has been walked.
    let mut index_column_complete: Option<bool> = None;

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

        if column_name == "ifIndex" {
            index_column_complete = Some(walked);
            // A column cut short still names the indexes it *did* return, and a row outside that
            // set is not an interface this device listed — truncated or not. Only a column that
            // returned nothing leaves no basis to judge, and that is the sole case that falls back
            // to accepting whatever the other columns mint. Requiring the column to have finished
            // let a foreign ifIndex through on exactly the scan where the guard was needed most.
            if !column_indexes.is_empty() {
                known_if_indexes = Some(column_indexes);
            }
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

    // A foreign row means something answered for an interface this device never listed, so the
    // set itself is suspect — not just its attributes.
    let set_complete = match index_column_complete {
        // The index column decides membership, so its own completeness is the set's.
        Some(true) => foreign_rows == 0,
        Some(false) => false,
        // A device that serves no index column at all gives us no independent read on membership;
        // fall back to requiring every column, which is what this did before the split.
        None => complete,
    };

    // `complete` distinguishes an authoritative full ifTable from a partial walk cut short by
    // timeout/error. The server prunes stale interfaces only on a complete walk (GH #649), so
    // surface it at debug level for self-hosted daemon-log triage (enable SCANOPY_LOG_LEVEL=debug).
    tracing::debug!(
        ip = %ip,
        if_count = result.len(),
        set_complete = set_complete,
        attributes_complete = complete,
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

    Ok(IfTableWalk {
        entries: result,
        set_complete,
        attributes_complete: complete,
    })
}

/// Query LLDP remote table for neighbor information
pub async fn query_lldp_neighbors<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<LldpNeighbor>>> {
    let mut neighbors: HashMap<(i32, i32), LldpNeighbor> = HashMap::new();
    let mut complete = true;

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
        walk_column(session, base_oid_str, &mut complete, |suffix, value| {
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
        .await;
    }

    // Resolve remote management addresses from the separate lldpRemManAddrTable.
    // Its index is timeMark.localPortNum.remIndex.addrSubtype.addrLen.addr, so the
    // address lives in the OID *index*, not the column value. We walk an accessible
    // column (lldpRemManAddrIfSubtype) and reconstruct the address from the index.
    let man_base_oid_str = oids::lldp::remote::entry::LLDP_REM_MAN_ADDR_IF_SUBTYPE;
    // Management address is optional enrichment; ignore walk errors (keeps the
    // neighbours already collected above).
    let mut mgmt_complete = true;
    walk_column(
        session,
        man_base_oid_str,
        &mut mgmt_complete,
        |suffix, _value| {
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
        },
    )
    .await;
    // A missing management address never gates resolution (topology.rs matches on chassis/port
    // only), so a truncated walk here is not a reason to withhold the neighbours themselves.
    if !mgmt_complete {
        debug!(ip = %ip, "LLDP management-address walk was cut short");
    }

    // Per IEEE 802.1AB the chassis ID is a mandatory TLV, so a neighbour record without one is
    // malformed by construction — in practice, the tail of a cut-short chassis column while the
    // port-id and sys-name columns completed. Emitting it would overwrite a good chassis ID with
    // NULL, and a row with no chassis ID is excluded from L2 resolution entirely, so it could
    // never recover. Drop it and report the walk as partial instead.
    let before = neighbors.len();
    let result: Vec<LldpNeighbor> = neighbors
        .into_values()
        .filter(|n| n.remote_chassis_id_subtype.is_some() && n.remote_chassis_id_bytes.is_some())
        .collect();
    if result.len() != before {
        complete = false;
        warn!(
            ip = %ip,
            dropped = before - result.len(),
            "LLDP neighbours missing the mandatory chassis ID; discarding them and marking the \
             walk partial"
        );
    }
    debug!("LLDP query from {} returned {} neighbors", ip, result.len());

    Ok(SnmpCollection {
        records: result,
        complete,
    })
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
pub async fn query_cdp_neighbors<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<CdpNeighbor>>> {
    let mut neighbors: HashMap<(i32, i32), CdpNeighbor> = HashMap::new();
    let mut complete = true;

    let columns = [
        (oids::cdp::entry::CDP_CACHE_DEVICE_ID, "deviceId"),
        (oids::cdp::entry::CDP_CACHE_DEVICE_PORT, "devicePort"),
        (oids::cdp::entry::CDP_CACHE_PLATFORM, "platform"),
        (oids::cdp::entry::CDP_CACHE_ADDRESS, "address"),
    ];

    for (base_oid_str, column_name) in columns {
        // CDP index: cdpCacheIfIndex.cdpCacheDeviceIndex
        walk_column(session, base_oid_str, &mut complete, |suffix, value| {
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
        .await;
    }

    // cdpCacheDeviceId is what L2 resolution matches on, so a record without one is the CDP
    // analogue of a chassis-less LLDP neighbour: unusable, and destructive if it overwrites.
    let before = neighbors.len();
    let result: Vec<CdpNeighbor> = neighbors
        .into_values()
        .filter(|n| n.remote_device_id.is_some())
        .collect();
    if result.len() != before {
        complete = false;
        warn!(
            ip = %ip,
            dropped = before - result.len(),
            "CDP neighbours missing a device id; discarding them and marking the walk partial"
        );
    }
    debug!("CDP query from {} returned {} neighbors", ip, result.len());

    Ok(SnmpCollection {
        records: result,
        complete,
    })
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
/// This is the highest-leverage truncation in the file: both FDB and VLAN-membership collection
/// key everything off it, so a cut-short walk here silently empties both for the whole switch.
async fn walk_bridge_port_mapping<T: SnmpWalkTransport>(
    session: &mut T,
) -> Result<SnmpCollection<HashMap<i32, i32>>> {
    let mut port_to_if_index: HashMap<i32, i32> = HashMap::new();
    let mut complete = true;
    // OID suffix is the bridge port number; value is the ifIndex.
    walk_column(
        session,
        oids::bridge::DOT1D_BASE_PORT_IF_INDEX,
        &mut complete,
        |suffix, value| {
            if let Some(&port_u64) = suffix.last()
                && let Some(if_index) = value_to_i32(value)
            {
                port_to_if_index.insert(port_u64 as i32, if_index);
            }
        },
    )
    .await;

    Ok(SnmpCollection {
        records: port_to_if_index,
        complete,
    })
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
pub async fn query_bridge_fdb<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<BridgeFdbEntry>>> {
    // Step 1: Walk dot1dBasePortIfIndex to build bridge_port → ifIndex map.
    // Both FDB tables reference this same dot1dBasePort space.
    let mapping = walk_bridge_port_mapping(session).await?;
    let mut complete = mapping.complete;
    let port_to_if_index = mapping.records;

    // Step 2: Walk legacy dot1dTpFdbTable columns.
    let mut fdb_entries: HashMap<String, FdbBuilder> = HashMap::new();

    let columns = [
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_ADDRESS, "address"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_PORT, "port"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_STATUS, "status"),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix is a 6-octet MAC encoded as 6 sub-ids.
        walk_column(session, base_oid_str, &mut complete, |suffix, value| {
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
        .await;
    }

    // Step 3: Merge in VLAN-aware Q-BRIDGE dot1qTpFdbTable entries. Legacy rows
    // win; Q-BRIDGE fills in MACs the legacy table didn't report (or all of them,
    // on switches that populate only the Q-BRIDGE table).
    let legacy_count = fdb_entries.len();
    let qbridge = walk_qbridge_fdb(session).await.unwrap_or_default();
    if !qbridge.complete {
        complete = false;
    }
    let qbridge = qbridge.records;
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
        complete = complete,
        "Bridge FDB walk finished"
    );

    Ok(SnmpCollection {
        records: result,
        complete,
    })
}

/// Walk the VLAN-aware Q-BRIDGE FDB (`dot1qTpFdbTable`, RFC 4363) for MAC→port
/// mappings, keyed by MAC so results merge with the legacy `dot1dTpFdbTable`.
///
/// Unlike the legacy table, the MAC lives in the table INDEX
/// (`dot1qFdbId` + 6 MAC octets), not a column, so it's derived from the OID
/// suffix. Ports are `dot1dBasePort` numbers, resolved by the caller against the
/// same `dot1dBasePortIfIndex` map. VLAN-aware switches (Aruba/HP ProCurve, etc.)
/// often populate only this table (GH #649).
async fn walk_qbridge_fdb<T: SnmpWalkTransport>(
    session: &mut T,
) -> Result<SnmpCollection<HashMap<String, FdbBuilder>>> {
    let mut entries: HashMap<String, FdbBuilder> = HashMap::new();
    let mut complete = true;

    let columns = [
        (oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_PORT, "port"),
        (oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_STATUS, "status"),
    ];

    for (base_oid_str, column_name) in columns {
        // Q-BRIDGE index = dot1qFdbId (1 sub-id) + MAC (6 octets).
        walk_column(session, base_oid_str, &mut complete, |suffix, value| {
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
        .await;
    }

    Ok(SnmpCollection {
        records: entries,
        complete,
    })
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
pub async fn query_port_vlan_membership<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<PortVlanMembership>>> {
    // Step 1: Get bridge port → ifIndex mapping
    let mapping = walk_bridge_port_mapping(session).await?;
    let mut complete = mapping.complete;
    let port_to_if_index = mapping.records;

    if port_to_if_index.is_empty() {
        debug!(
            "No bridge port mappings from {} — skipping VLAN membership",
            ip
        );
        return Ok(SnmpCollection {
            records: Vec::new(),
            complete,
        });
    }

    // Step 2: Walk dot1qPvid for native VLAN per bridge port. OID suffix is the
    // bridge port number; value is the native VLAN ID.
    let mut native_vlans: HashMap<i32, u16> = HashMap::new();
    walk_column(
        session,
        oids::vlan::q_bridge::DOT1Q_PVID,
        &mut complete,
        |suffix, value| {
            if let Some(&port_u64) = suffix.last()
                && let Some(vlan_id) = value_to_u16(value)
            {
                native_vlans.insert(port_u64 as i32, vlan_id);
            }
        },
    )
    .await;

    // Step 3: Walk dot1qVlanCurrentEgressPorts — PortList bitmap per VLAN, indexed
    // by timeFilter.vlanId (last sub-id is the VLAN ID).
    let mut egress_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_column(
        session,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_EGRESS_PORTS,
        &mut complete,
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
    .await;

    // Step 4: Walk dot1qVlanCurrentUntaggedPorts — same bitmap format.
    let mut untagged_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_column(
        session,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_UNTAGGED_PORTS,
        &mut complete,
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
    .await;

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

    Ok(SnmpCollection {
        records: result,
        complete,
    })
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

        let walk = walk_if_table(&mut agent, ip()).await.unwrap();
        let entries = walk.entries;

        assert!(walk.set_complete && walk.attributes_complete);
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

        let walk = walk_if_table(&mut agent, ip()).await.unwrap();
        let entries = walk.entries;

        assert!(
            !entries.iter().any(|e| e.if_index == 2),
            "an ifIndex the device never listed must not become an interface"
        );
        assert_eq!(
            entries.iter().map(|e| e.if_index).collect::<Vec<_>>(),
            vec![1, 49153, 49154, 49155]
        );
        assert!(
            !walk.set_complete,
            "a table carrying rows the device never listed is not authoritative, so the server \
             must not prune against it"
        );
    }

    /// The gap that let a foreign interface onto switch-exos-01: the guard used to engage only
    /// when the index column *finished*, so on the one scan where that column was cut short — the
    /// scan most likely to be carrying stray responses — it switched itself off and a row for an
    /// ifIndex the device never listed became an interface.
    ///
    /// A truncated column still names the indexes it did return, and those are still the only
    /// interfaces the device claimed.
    #[tokio::test]
    async fn a_truncated_index_column_still_rejects_indexes_it_never_reported() {
        struct TruncatedIndexWithGhost {
            agent: FakeAgent,
        }

        #[async_trait::async_trait]
        impl SnmpWalkTransport for TruncatedIndexWithGhost {
            async fn walk_getbulk<'a>(
                &'a mut self,
                from: &[u64],
                max: u32,
            ) -> Result<WalkPage<'a>> {
                // The index column answers once, then dies — so it reports 1 and 49153 only.
                if from == [1, 3, 6, 1, 2, 1, 2, 2, 1, 1] {
                    return Ok(WalkPage::Varbinds(vec![
                        (vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 1, 1], Value::Integer(1)),
                        (
                            vec![1, 3, 6, 1, 2, 1, 2, 2, 1, 1, 49153],
                            Value::Integer(49153),
                        ),
                    ]));
                }
                if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1]) {
                    return Err(anyhow::anyhow!("getbulk timed out"));
                }
                self.agent.walk_getbulk(from, max).await
            }

            async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
                if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1]) {
                    return Err(anyhow::anyhow!("getnext timed out"));
                }
                self.agent.walk_getnext(from).await
            }
        }

        // The ifDescr column carries a row for ifIndex 2, which the index column never named.
        let mut rows = FakeAgent::omada();
        rows.push(("1.3.6.1.2.1.2.2.1.2.2", Canned::Str("ge-0/0/1")));
        let mut session = TruncatedIndexWithGhost {
            agent: FakeAgent::new(&rows),
        };

        let walk = walk_if_table(&mut session, ip()).await.unwrap();

        assert!(
            !walk.entries.iter().any(|e| e.if_index == 2),
            "a row the index column never named must be rejected even when that column was cut \
             short — that is precisely when stray responses are in play"
        );
        assert_eq!(
            walk.entries.iter().map(|e| e.if_index).collect::<Vec<_>>(),
            vec![1, 49153],
            "only the indexes the device actually reported survive"
        );
        assert!(
            !walk.set_complete,
            "a cut-short index column is never an authoritative set"
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

        let walk = walk_if_table(&mut agent, ip()).await.unwrap();

        assert_eq!(walk.entries.len(), 1);
        assert_eq!(walk.entries[0].if_index, 7);
        assert_eq!(walk.entries[0].if_descr.as_deref(), Some("eth7"));
    }

    /// A flaky attribute column costs descriptions, not interfaces.
    ///
    /// The two used to be one flag, so a timed-out `ifDescr` read both blocked the server-side
    /// prune — leaving stale interfaces on the host forever — and told the operator interfaces
    /// might be missing when every one had been found.
    #[tokio::test]
    async fn a_truncated_attribute_column_keeps_the_interface_set_authoritative() {
        struct FlakyDescr {
            agent: FakeAgent,
        }

        #[async_trait::async_trait]
        impl SnmpWalkTransport for FlakyDescr {
            async fn walk_getbulk<'a>(
                &'a mut self,
                from: &[u64],
                max: u32,
            ) -> Result<WalkPage<'a>> {
                // ifDescr is column 2; cut it short the way a timeout does.
                if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 2]) {
                    return Err(anyhow::anyhow!("getbulk timed out"));
                }
                self.agent.walk_getbulk(from, max).await
            }

            async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
                if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 2]) {
                    return Err(anyhow::anyhow!("getnext timed out"));
                }
                self.agent.walk_getnext(from).await
            }
        }

        let mut session = FlakyDescr {
            agent: FakeAgent::new(&FakeAgent::omada()),
        };

        let walk = walk_if_table(&mut session, ip()).await.unwrap();

        assert_eq!(
            walk.entries.iter().map(|e| e.if_index).collect::<Vec<_>>(),
            vec![1, 49153, 49154, 49155],
            "the interface set comes from the ifIndex column, which was unaffected"
        );
        assert!(
            walk.set_complete,
            "every interface the device listed is present, so the set is prunable"
        );
        assert!(
            !walk.attributes_complete,
            "descriptions are missing and the operator should be told so"
        );
        assert!(walk.entries.iter().all(|e| e.if_descr.is_none()));
    }

    /// The converse: losing the index column loses the set, whatever else succeeded.
    #[tokio::test]
    async fn a_truncated_index_column_makes_the_set_unauthoritative() {
        struct FlakyIndex {
            agent: FakeAgent,
        }

        #[async_trait::async_trait]
        impl SnmpWalkTransport for FlakyIndex {
            async fn walk_getbulk<'a>(
                &'a mut self,
                from: &[u64],
                max: u32,
            ) -> Result<WalkPage<'a>> {
                if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1]) {
                    return Err(anyhow::anyhow!("getbulk timed out"));
                }
                self.agent.walk_getbulk(from, max).await
            }

            async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
                if from.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1]) {
                    return Err(anyhow::anyhow!("getnext timed out"));
                }
                self.agent.walk_getnext(from).await
            }
        }

        let mut session = FlakyIndex {
            agent: FakeAgent::new(&FakeAgent::omada()),
        };

        let walk = walk_if_table(&mut session, ip()).await.unwrap();

        assert!(
            !walk.set_complete,
            "without the index column we cannot know which interfaces exist, so pruning must \
             stay blocked"
        );
    }

    /// A neighbour record with no chassis ID is malformed — IEEE 802.1AB makes the chassis ID a
    /// mandatory TLV — and in practice means the chassis column was cut short while the port-id
    /// and sys-name columns completed. Emitting it overwrote a good chassis ID with NULL, and a
    /// row without one is excluded from L2 resolution entirely, so the link could never recover.
    #[tokio::test]
    async fn a_neighbour_without_a_chassis_id_is_dropped_and_reported_partial() {
        // lldpRemTable index is timeMark.localPortNum.remIndex; port id and sys name are present
        // for remIndex 1, chassis id is not.
        let mut agent = FakeAgent::new(&[
            ("1.0.8802.1.1.2.1.4.1.1.7.0.1.1", Canned::Str("41")),
            (
                "1.0.8802.1.1.2.1.4.1.1.9.0.1.1",
                Canned::Str("switch-core-01"),
            ),
        ]);

        let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

        assert!(
            walk.records.is_empty(),
            "a chassis-less neighbour must not reach the server"
        );
        assert!(
            !walk.complete,
            "dropping a malformed record means this walk is not authoritative, so the server \
             must keep what it already has"
        );
    }

    /// A complete neighbour record still comes through intact.
    #[tokio::test]
    async fn a_complete_neighbour_record_is_collected() {
        let mut agent = FakeAgent::new(&[
            ("1.0.8802.1.1.2.1.4.1.1.4.0.1.1", Canned::Int(4)),
            (
                "1.0.8802.1.1.2.1.4.1.1.5.0.1.1",
                Canned::Str("00:1a:2b:00:10:00"),
            ),
            ("1.0.8802.1.1.2.1.4.1.1.6.0.1.1", Canned::Int(7)),
            ("1.0.8802.1.1.2.1.4.1.1.7.0.1.1", Canned::Str("41")),
            (
                "1.0.8802.1.1.2.1.4.1.1.9.0.1.1",
                Canned::Str("switch-core-01"),
            ),
        ]);

        let walk = query_lldp_neighbors(&mut agent, ip()).await.unwrap();

        assert_eq!(walk.records.len(), 1);
        assert!(walk.complete);
        let n = &walk.records[0];
        assert_eq!(n.local_port_index, 1);
        assert_eq!(n.remote_sys_name.as_deref(), Some("switch-core-01"));
        assert!(n.remote_chassis_id_bytes.is_some());
    }

    /// A whole-query timeout yields the `Default`, and that must not read as a device
    /// authoritatively reporting no neighbours — otherwise one slow switch wipes every link on it.
    #[test]
    fn a_defaulted_collection_is_never_authoritative() {
        let timed_out: SnmpCollection<Vec<LldpNeighbor>> = Default::default();
        assert!(timed_out.records.is_empty());
        assert!(!timed_out.complete);
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
