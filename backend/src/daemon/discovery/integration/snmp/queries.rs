//! SNMP Query Functions
//!
//! Functions for querying SNMP data from devices.

use anyhow::{Result, anyhow};
use snmp2::{Oid, Value};
use std::collections::HashMap;
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

/// Query system MIB information from a device
pub async fn query_system_info(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) ->Result<SystemInfo> {
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
pub async fn walk_if_table(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) ->Result<(Vec<IfTableEntry>, bool)> {
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

    // Walk each column
    for (base_oid_str, column_name) in columns {
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                warn!("Failed to parse OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                warn!("Walk limit reached for {} on {}", column_name, ip);
                complete = false;
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        // EndOfMibView/NoSuchObject/NoSuchInstance = no more data
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        // Check if we're still in the same subtree
                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            // We've walked past the column
                            break;
                        }

                        // Extract ifIndex from OID (last component)
                        if let Some(&if_index_u64) = response_parts.last() {
                            let if_index = if_index_u64 as i32;
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
                                "ifIndex" => {
                                    // Already set above
                                }
                                "ifDescr" => entry.if_descr = value_to_string(&value),
                                "ifType" => entry.if_type = value_to_i32(&value),
                                "ifMtu" => entry.if_mtu = value_to_i32(&value),
                                "ifSpeed" => {
                                    // Only set if ifHighSpeed not already set
                                    if entry.if_speed.is_none() {
                                        entry.if_speed = value_to_u64(&value);
                                    }
                                }
                                "ifPhysAddress" => entry.if_phys_address = value_to_mac(&value),
                                "ifAdminStatus" => entry.if_admin_status = value_to_i32(&value),
                                "ifOperStatus" => entry.if_oper_status = value_to_i32(&value),
                                "ifName" => entry.if_name = value_to_string(&value),
                                "ifHighSpeed" => {
                                    // ifHighSpeed is in Mbps, convert to bps for consistency
                                    if let Some(mbps) = value_to_u64(&value) {
                                        entry.if_speed = Some(mbps * 1_000_000);
                                    }
                                }
                                "ifAlias" => entry.if_alias = value_to_string(&value),
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        // getnext returned no varbind — abnormal termination, treat as partial.
                        complete = false;
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!("Walk {} failed on {}: {:?}", column_name, ip, e);
                    complete = false;
                    break;
                }
                Err(_) => {
                    debug!("Walk {} timeout on {}", column_name, ip);
                    complete = false;
                    break;
                }
            }
        }

        trace!("Walked {} entries for {} from {}", count, column_name, ip);
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
) ->Result<Vec<LldpNeighbor>> {
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
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to parse LLDP OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        // EndOfMibView/NoSuchObject/NoSuchInstance = no more data
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // Extract index components from OID suffix
                        // Format: base.timeMark.localPortNum.remIndex
                        let suffix = &response_parts[base_parts.len()..];
                        if suffix.len() >= 3 {
                            let local_port = suffix[1] as i32;
                            let rem_index = suffix[2] as i32;

                            let neighbor =
                                neighbors.entry((local_port, rem_index)).or_insert_with(|| {
                                    LldpNeighbor {
                                        local_port_index: local_port,
                                        remote_chassis_id_subtype: None,
                                        remote_chassis_id_bytes: None,
                                        remote_port_id_subtype: None,
                                        remote_port_id_bytes: None,
                                        remote_port_desc: None,
                                        remote_sys_name: None,
                                        remote_sys_desc: None,
                                        remote_mgmt_addr: None,
                                    }
                                });

                            match column_name {
                                "remChassisIdSubtype" => {
                                    neighbor.remote_chassis_id_subtype =
                                        value_to_i32(&value).map(|v| v as u8)
                                }
                                "remChassisId" => {
                                    if let Value::OctetString(bytes) = &value {
                                        neighbor.remote_chassis_id_bytes = Some(bytes.to_vec());
                                    }
                                }
                                "remPortIdSubtype" => {
                                    neighbor.remote_port_id_subtype =
                                        value_to_i32(&value).map(|v| v as u8)
                                }
                                "remPortId" => {
                                    if let Value::OctetString(bytes) = &value {
                                        neighbor.remote_port_id_bytes = Some(bytes.to_vec());
                                    }
                                }
                                "remPortDesc" => {
                                    neighbor.remote_port_desc = value_to_string(&value)
                                }
                                "remSysName" => neighbor.remote_sys_name = value_to_string(&value),
                                "remSysDesc" => neighbor.remote_sys_desc = value_to_string(&value),
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
    }

    // Resolve remote management addresses from the separate lldpRemManAddrTable.
    // Its index is timeMark.localPortNum.remIndex.addrSubtype.addrLen.addr, so the
    // address lives in the OID *index*, not the column value. We walk an accessible
    // column (lldpRemManAddrIfSubtype) and reconstruct the address from the index.
    let man_base_oid_str = oids::lldp::remote::entry::LLDP_REM_MAN_ADDR_IF_SUBTYPE;
    if let Ok(man_base_oid) = parse_oid(man_base_oid_str) {
        let man_base_parts: Vec<u64> = man_base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        let mut current_oid = man_base_oid.clone();
        let mut count = 0;
        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }
            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    let Some((resp_oid, value)) = response.varbinds.next() else {
                        break;
                    };
                    if matches!(
                        value,
                        Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                    ) {
                        break;
                    }
                    let response_parts = oid_to_vec(&resp_oid);
                    if response_parts.len() <= man_base_parts.len()
                        || !response_parts.starts_with(&man_base_parts)
                    {
                        break;
                    }
                    // suffix = timeMark, localPortNum, remIndex, addrSubtype, addrLen, addr...
                    let suffix = &response_parts[man_base_parts.len()..];
                    if suffix.len() >= 5 {
                        let local_port = suffix[1] as i32;
                        let rem_index = suffix[2] as i32;
                        let addr_subtype = suffix[3];
                        let addr_len = suffix[4] as usize;
                        if suffix.len() >= 5 + addr_len && addr_len > 0 {
                            // parse_lldp_mgmt_addr expects [ianaFamily, addr bytes...]
                            let mut buf = Vec::with_capacity(1 + addr_len);
                            buf.push(addr_subtype as u8);
                            buf.extend(suffix[5..5 + addr_len].iter().map(|&b| b as u8));
                            if let Some(addr) = parse_lldp_mgmt_addr(&buf)
                                && let Some(neighbor) = neighbors.get_mut(&(local_port, rem_index))
                            {
                                neighbor.remote_mgmt_addr = Some(addr);
                            }
                        }
                    }
                    current_oid = match Oid::from(response_parts.as_slice()) {
                        Ok(o) => o,
                        Err(_) => break,
                    };
                    count += 1;
                }
                Ok(Err(e)) => {
                    debug!(oid = %man_base_oid_str, error = %e, "LLDP mgmt-addr walk error");
                    break;
                }
                Err(_) => break,
            }
        }
    }

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
) ->Result<HashMap<i32, LldpLocalPort>> {
    let mut ports: HashMap<i32, LldpLocalPort> = HashMap::new();

    let columns = [
        (oids::lldp::local::LLDP_LOC_PORT_ID_SUBTYPE, "subtype"),
        (oids::lldp::local::LLDP_LOC_PORT_ID, "id"),
    ];

    for (base_oid_str, column_name) in columns {
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to parse lldpLocPort OID {}: {}", base_oid_str, e);
                continue;
            }
        };
        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;
        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }
            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    let Some((resp_oid, value)) = response.varbinds.next() else {
                        break;
                    };
                    if matches!(
                        value,
                        Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                    ) {
                        break;
                    }
                    let response_parts = oid_to_vec(&resp_oid);
                    if response_parts.len() <= base_parts.len()
                        || !response_parts.starts_with(&base_parts)
                    {
                        break;
                    }
                    // Index is a single sub-id: lldpLocPortNum
                    let local_port_num = response_parts[base_parts.len()] as i32;
                    let entry = ports.entry(local_port_num).or_default();
                    match column_name {
                        "subtype" => {
                            entry.port_id_subtype = value_to_i32(&value).map(|v| v as u8)
                        }
                        "id" => entry.port_id = value_to_string(&value),
                        _ => {}
                    }
                    current_oid = match Oid::from(response_parts.as_slice()) {
                        Ok(o) => o,
                        Err(_) => break,
                    };
                    count += 1;
                }
                Ok(Err(e)) => {
                    debug!(oid = %base_oid_str, error = %e, "lldpLocPortTable walk error");
                    break;
                }
                Err(_) => break,
            }
        }
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
) ->Result<HashMap<IpAddr, IpAddrEntry>> {
    let mut if_index_map: HashMap<IpAddr, i32> = HashMap::new();
    let mut net_mask_map: HashMap<IpAddr, IpAddr> = HashMap::new();

    // Walk ipAdEntIfIndex
    let base_oid_str = oids::ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX;
    let base_oid = parse_oid(base_oid_str)?;
    let base_parts: Vec<u64> = base_oid_str
        .split('.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut current_oid = base_oid.clone();
    let mut count = 0;

    loop {
        if count >= MAX_WALK_ENTRIES {
            warn!("Walk limit reached for ipAddrTable ifIndex on {}", ip);
            break;
        }

        match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
            Ok(Ok(mut response)) => {
                if let Some((resp_oid, value)) = response.varbinds.next() {
                    // EndOfMibView/NoSuchObject/NoSuchInstance = no more data
                    if matches!(
                        value,
                        Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                    ) {
                        break;
                    }

                    let response_parts = oid_to_vec(&resp_oid);
                    if response_parts.len() <= base_parts.len()
                        || !response_parts.starts_with(&base_parts)
                    {
                        break;
                    }

                    let suffix = &response_parts[base_parts.len()..];
                    if suffix.len() == 4 {
                        let addr = IpAddr::from([
                            suffix[0] as u8,
                            suffix[1] as u8,
                            suffix[2] as u8,
                            suffix[3] as u8,
                        ]);
                        if let Some(if_index) = value_to_i32(&value) {
                            if_index_map.insert(addr, if_index);
                        }
                    }

                    current_oid = Oid::from(response_parts.as_slice())
                        .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                    count += 1;
                } else {
                    break;
                }
            }
            Ok(Err(e)) => {
                debug!("ipAddrTable ifIndex walk failed on {}: {:?}", ip, e);
                break;
            }
            Err(_) => {
                debug!("ipAddrTable ifIndex walk timeout on {}", ip);
                break;
            }
        }
    }

    // Walk ipAdEntNetMask
    let mask_oid_str = oids::ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK;
    let mask_base_oid = parse_oid(mask_oid_str)?;
    let mask_base_parts: Vec<u64> = mask_oid_str
        .split('.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut current_oid = mask_base_oid.clone();
    let mut count = 0;

    loop {
        if count >= MAX_WALK_ENTRIES {
            warn!("Walk limit reached for ipAddrTable netMask on {}", ip);
            break;
        }

        match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
            Ok(Ok(mut response)) => {
                if let Some((resp_oid, value)) = response.varbinds.next() {
                    // EndOfMibView/NoSuchObject/NoSuchInstance = no more data
                    if matches!(
                        value,
                        Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                    ) {
                        break;
                    }

                    let response_parts = oid_to_vec(&resp_oid);
                    if response_parts.len() <= mask_base_parts.len()
                        || !response_parts.starts_with(&mask_base_parts)
                    {
                        break;
                    }

                    let suffix = &response_parts[mask_base_parts.len()..];
                    if suffix.len() == 4 {
                        let addr = IpAddr::from([
                            suffix[0] as u8,
                            suffix[1] as u8,
                            suffix[2] as u8,
                            suffix[3] as u8,
                        ]);
                        if let Some(mask) = value_to_ip(&value) {
                            net_mask_map.insert(addr, mask);
                        }
                    }

                    current_oid = Oid::from(response_parts.as_slice())
                        .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                    count += 1;
                } else {
                    break;
                }
            }
            Ok(Err(e)) => {
                debug!("ipAddrTable netMask walk failed on {}: {:?}", ip, e);
                break;
            }
            Err(_) => {
                debug!("ipAddrTable netMask walk timeout on {}", ip);
                break;
            }
        }
    }

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
) ->Result<Vec<CdpNeighbor>> {
    let mut neighbors: HashMap<(i32, i32), CdpNeighbor> = HashMap::new();

    let columns = [
        (oids::cdp::entry::CDP_CACHE_DEVICE_ID, "deviceId"),
        (oids::cdp::entry::CDP_CACHE_DEVICE_PORT, "devicePort"),
        (oids::cdp::entry::CDP_CACHE_PLATFORM, "platform"),
        (oids::cdp::entry::CDP_CACHE_ADDRESS, "address"),
    ];

    for (base_oid_str, column_name) in columns {
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to parse CDP OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        // EndOfMibView/NoSuchObject/NoSuchInstance = no more data
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // CDP index: base.cdpCacheIfIndex.cdpCacheDeviceIndex
                        let suffix = &response_parts[base_parts.len()..];
                        if suffix.len() >= 2 {
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
                                "deviceId" => neighbor.remote_device_id = value_to_string(&value),
                                "devicePort" => neighbor.remote_port_id = value_to_string(&value),
                                "platform" => neighbor.remote_platform = value_to_string(&value),
                                "address" => {
                                    // CDP address is encoded as 4 bytes for IPv4
                                    if let Value::OctetString(bytes) = &value
                                        && bytes.len() == 4
                                    {
                                        neighbor.remote_address = Some(IpAddr::from([
                                            bytes[0], bytes[1], bytes[2], bytes[3],
                                        ]));
                                    }
                                }
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
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
) ->Result<Vec<ArpEntry>> {

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
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to parse ARP OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        // EndOfMibView/NoSuchObject/NoSuchInstance = no more data
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // OID suffix: ifIndex.A.B.C.D
                        let suffix = &response_parts[base_parts.len()..];
                        if suffix.len() >= 5 {
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
                                "ifIndex" => entry.if_index = value_to_i32(&value),
                                "physAddress" => entry.mac_address = value_to_mac(&value),
                                "netAddress" => entry.ip_address = value_to_ip(&value),
                                "type" => entry.entry_type = value_to_i32(&value),
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
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
) ->Result<Option<DeviceInventory>> {

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
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to parse ENTITY OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        // EndOfMibView/NoSuchObject/NoSuchInstance = no more data
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // OID suffix is entPhysicalIndex (single integer)
                        if let Some(&index_u64) = response_parts.last() {
                            let index = index_u64 as i32;
                            let entry = entries.entry(index).or_insert_with(|| PhysicalEntry {
                                description: None,
                                class: None,
                                name: None,
                                serial_number: None,
                                manufacturer: None,
                                model: None,
                            });

                            match column_name {
                                "descr" => entry.description = value_to_string(&value),
                                "class" => entry.class = value_to_i32(&value),
                                "name" => entry.name = value_to_string(&value),
                                "serialNum" => {
                                    entry.serial_number =
                                        value_to_string(&value).filter(|s| !s.is_empty())
                                }
                                "mfgName" => {
                                    entry.manufacturer =
                                        value_to_string(&value).filter(|s| !s.is_empty())
                                }
                                "modelName" => {
                                    entry.model = value_to_string(&value).filter(|s| !s.is_empty())
                                }
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
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
    let port_oid_str = oids::bridge::DOT1D_BASE_PORT_IF_INDEX;
    let port_base_oid = parse_oid(port_oid_str)?;
    let port_base_parts: Vec<u64> = port_oid_str
        .split('.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut port_to_if_index: HashMap<i32, i32> = HashMap::new();
    let mut current_oid = port_base_oid.clone();
    let mut count = 0;

    loop {
        if count >= MAX_WALK_ENTRIES {
            break;
        }

        match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
            Ok(Ok(mut response)) => {
                if let Some((resp_oid, value)) = response.varbinds.next() {
                    if matches!(
                        value,
                        Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                    ) {
                        break;
                    }

                    let response_parts = oid_to_vec(&resp_oid);
                    if response_parts.len() <= port_base_parts.len()
                        || !response_parts.starts_with(&port_base_parts)
                    {
                        break;
                    }

                    // OID suffix is bridge port number, value is ifIndex
                    if let Some(&port_u64) = response_parts.last() {
                        let bridge_port = port_u64 as i32;
                        if let Some(if_index) = value_to_i32(&value) {
                            port_to_if_index.insert(bridge_port, if_index);
                        }
                    }

                    current_oid = Oid::from(response_parts.as_slice())
                        .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                    count += 1;
                } else {
                    break;
                }
            }
            Ok(Err(e)) => {
                debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                break;
            }
            Err(_) => {
                debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                break;
            }
        }
    }

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
) ->Result<Vec<BridgeFdbEntry>> {

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
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to parse bridge FDB OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        // EndOfMibView/NoSuchObject/NoSuchInstance = no more data
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // OID suffix is 6-octet MAC encoded as 6 integers
                        let suffix = &response_parts[base_parts.len()..];
                        if suffix.len() == 6 {
                            let key = suffix
                                .iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(".");

                            let entry = fdb_entries.entry(key).or_default();

                            match column_name {
                                "address" => entry.mac_address = value_to_mac(&value),
                                "port" => entry.port = value_to_i32(&value),
                                "status" => entry.status = value_to_i32(&value),
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
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
        let base_oid = match parse_oid(base_oid_str) {
            Ok(o) => o,
            Err(e) => {
                debug!("Failed to parse Q-BRIDGE FDB OID {}: {}", base_oid_str, e);
                continue;
            }
        };

        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // Q-BRIDGE index = dot1qFdbId (1 sub-id) + MAC (6 octets).
                        let suffix = &response_parts[base_parts.len()..];
                        if let Some(mac) = qbridge_fdb_index_to_mac(suffix) {
                            // Key by MAC alone (drop fdb_id) so the same MAC learned
                            // across VLANs collapses to one entry and merges with the
                            // legacy table's MAC key.
                            let key = suffix[1..7]
                                .iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(".");

                            let entry = entries.entry(key).or_default();
                            entry.mac_address = Some(mac);
                            match column_name {
                                "port" => entry.port = value_to_i32(&value),
                                "status" => entry.status = value_to_i32(&value),
                                _ => {}
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
    }

    Ok(entries)
}

/// Query local LLDP chassis ID (scalar GETs, not walks).
/// Returns the device's own LLDP identity.
pub async fn query_lldp_local(
    session: &mut Box<snmp2::AsyncSession>,
    ip: IpAddr,
) ->Result<Option<LldpLocalInfo>> {

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
) ->Result<Vec<VlanInfo>> {
    let mut vlans: Vec<VlanInfo> = Vec::new();

    // Try Q-BRIDGE dot1qVlanStaticName first
    let base_oid_str = oids::vlan::q_bridge::DOT1Q_VLAN_STATIC_NAME;
    let base_oid = parse_oid(base_oid_str)?;
    let base_parts: Vec<u64> = base_oid_str
        .split('.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut current_oid = base_oid.clone();
    let mut count = 0;

    loop {
        if count >= MAX_WALK_ENTRIES {
            break;
        }

        match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
            Ok(Ok(mut response)) => {
                if let Some((resp_oid, value)) = response.varbinds.next() {
                    if matches!(
                        value,
                        Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                    ) {
                        break;
                    }

                    let response_parts = oid_to_vec(&resp_oid);
                    if response_parts.len() <= base_parts.len()
                        || !response_parts.starts_with(&base_parts)
                    {
                        break;
                    }

                    // OID suffix is VLAN ID
                    if let Some(&vlan_u64) = response_parts.last() {
                        let vlan_id = vlan_u64 as u16;
                        if let Some(name) = value_to_string(&value) {
                            vlans.push(VlanInfo { vlan_id, name });
                        }
                    }

                    current_oid = Oid::from(response_parts.as_slice())
                        .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                    count += 1;
                } else {
                    break;
                }
            }
            Ok(Err(e)) => {
                debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                break;
            }
            Err(_) => {
                debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                break;
            }
        }
    }

    // Fall back to Cisco VTP if Q-BRIDGE returned nothing
    if vlans.is_empty() {
        let vtp_oid_str = oids::vlan::cisco_vtp::VTP_VLAN_NAME;
        let vtp_base_oid = parse_oid(vtp_oid_str)?;
        let vtp_base_parts: Vec<u64> = vtp_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = vtp_base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= vtp_base_parts.len()
                            || !response_parts.starts_with(&vtp_base_parts)
                        {
                            break;
                        }

                        // VTP index is mgmtDomainIndex.vlanId — use last component as VLAN ID
                        if let Some(&vlan_u64) = response_parts.last() {
                            let vlan_id = vlan_u64 as u16;
                            if let Some(name) = value_to_string(&value) {
                                vlans.push(VlanInfo { vlan_id, name });
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
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
) ->Result<Vec<PortVlanMembership>> {

    // Step 1: Get bridge port → ifIndex mapping
    let port_to_if_index = walk_bridge_port_mapping(session).await?;

    if port_to_if_index.is_empty() {
        debug!(
            "No bridge port mappings from {} — skipping VLAN membership",
            ip
        );
        return Ok(Vec::new());
    }

    // Step 2: Walk dot1qPvid for native VLAN per bridge port
    let mut native_vlans: HashMap<i32, u16> = HashMap::new();
    {
        let base_oid_str = oids::vlan::q_bridge::DOT1Q_PVID;
        let base_oid = parse_oid(base_oid_str)?;
        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // OID suffix is bridge port number, value is native VLAN ID
                        if let Some(&port_u64) = response_parts.last() {
                            let bridge_port = port_u64 as i32;
                            if let Some(vlan_id) = value_to_u16(&value) {
                                native_vlans.insert(bridge_port, vlan_id);
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
    }

    // Step 3: Walk dot1qVlanCurrentEgressPorts — PortList bitmap per VLAN
    // Indexed by timeFilter.vlanId (timeFilter is typically 0)
    let mut egress_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    {
        let base_oid_str = oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_EGRESS_PORTS;
        let base_oid = parse_oid(base_oid_str)?;
        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        // Suffix is timeFilter.vlanId — last component is VLAN ID
                        let suffix = &response_parts[base_parts.len()..];
                        if let Some(&vlan_u64) = suffix.last() {
                            let vlan_id = vlan_u64 as u16;

                            // Value is PortList bitmap (OCTET STRING)
                            if let Value::OctetString(bytes) = &value {
                                let bridge_ports = parse_portlist_bitmap(bytes);
                                for bp in bridge_ports {
                                    egress_by_port.entry(bp).or_default().push(vlan_id);
                                }
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
    }

    // Step 4: Walk dot1qVlanCurrentUntaggedPorts — same bitmap format
    let mut untagged_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    {
        let base_oid_str = oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_UNTAGGED_PORTS;
        let base_oid = parse_oid(base_oid_str)?;
        let base_parts: Vec<u64> = base_oid_str
            .split('.')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut current_oid = base_oid.clone();
        let mut count = 0;

        loop {
            if count >= MAX_WALK_ENTRIES {
                break;
            }

            match timeout(SNMP_TIMEOUT, session.getnext(&current_oid)).await {
                Ok(Ok(mut response)) => {
                    if let Some((resp_oid, value)) = response.varbinds.next() {
                        if matches!(
                            value,
                            Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
                        ) {
                            break;
                        }

                        let response_parts = oid_to_vec(&resp_oid);
                        if response_parts.len() <= base_parts.len()
                            || !response_parts.starts_with(&base_parts)
                        {
                            break;
                        }

                        let suffix = &response_parts[base_parts.len()..];
                        if let Some(&vlan_u64) = suffix.last() {
                            let vlan_id = vlan_u64 as u16;

                            if let Value::OctetString(bytes) = &value {
                                let bridge_ports = parse_portlist_bitmap(bytes);
                                for bp in bridge_ports {
                                    untagged_by_port.entry(bp).or_default().push(vlan_id);
                                }
                            }
                        }

                        current_oid = Oid::from(response_parts.as_slice())
                            .map_err(|e| anyhow!("Invalid response OID: {:?}", e))?;
                        count += 1;
                    } else {
                        break;
                    }
                }
                Ok(Err(e)) => {
                    debug!(oid = %current_oid, error = %e, "SNMP walk column stopped on error");
                    break;
                }
                Err(_) => {
                    debug!(oid = %current_oid, "SNMP walk column stopped on timeout");
                    break;
                }
            }
        }
    }

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
