use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, Interface, InterfaceBase, Neighbor},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            child::ChildStorableEntity,
            snapshot::{DiscoveryTracked, FkMaps, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
    },
};

/// CSV row representation for Interface export
#[derive(Serialize)]
pub struct InterfaceCsvRow {
    pub id: Uuid,
    pub host_id: Uuid,
    pub network_id: Uuid,
    pub if_index: i32,
    pub if_descr: String,
    pub if_name: Option<String>,
    pub if_alias: Option<String>,
    pub if_type: i32,
    pub speed_bps: Option<i64>,
    pub admin_status: String,
    pub oper_status: String,
    pub mac_address: Option<String>,
    pub ip_address_id: Option<Uuid>,
    pub neighbor: Option<String>,
    pub lldp_chassis_id: Option<String>,
    pub lldp_port_id: Option<String>,
    pub lldp_sys_name: Option<String>,
    pub lldp_port_desc: Option<String>,
    pub lldp_mgmt_addr: Option<String>,
    pub lldp_sys_desc: Option<String>,
    pub cdp_device_id: Option<String>,
    pub cdp_port_id: Option<String>,
    pub cdp_platform: Option<String>,
    pub cdp_address: Option<String>,
    pub fdb_macs: Option<String>,
    pub native_vlan_id: Option<Uuid>,
    pub vlan_ids: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Interface {
    type BaseData = InterfaceBase;

    fn table_name() -> &'static str {
        "interfaces"
    }

    const HAS_SCD2: bool = true;

    fn is_live_row(&self) -> bool {
        self.valid_to.is_none()
    }

    fn new(base: Self::BaseData) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            valid_from,
            valid_to,
            lineage_id,
            last_seen_at,
            last_discovery_id,
            first_discovery_id,
            base:
                Self::BaseData {
                    host_id,
                    network_id,
                    if_index,
                    if_descr,
                    if_name,
                    if_alias,
                    if_type,
                    speed_bps,
                    admin_status,
                    oper_status,
                    mac_address,
                    ip_address_id,
                    neighbor,
                    lldp_chassis_id,
                    lldp_port_id,
                    lldp_sys_name,
                    lldp_port_desc,
                    lldp_mgmt_addr,
                    lldp_sys_desc,
                    cdp_device_id,
                    cdp_port_id,
                    cdp_platform,
                    cdp_address,
                    fdb_macs,
                    native_vlan_id,
                    vlan_ids,
                },
        } = self.clone();

        // Convert Neighbor enum to two mutually exclusive columns
        let (neighbor_interface_id, neighbor_host_id) = match neighbor {
            Some(Neighbor::Interface(id)) => (Some(id), None),
            Some(Neighbor::Host(id)) => (None, Some(id)),
            None => (None, None),
        };

        let columns = vec![
            "id",
            "host_id",
            "network_id",
            "if_index",
            "if_descr",
            "if_name",
            "if_alias",
            "if_type",
            "speed_bps",
            "admin_status",
            "oper_status",
            "mac_address",
            "ip_address_id",
            "neighbor_interface_id",
            "neighbor_host_id",
            "lldp_chassis_id",
            "lldp_port_id",
            "lldp_sys_name",
            "lldp_port_desc",
            "lldp_mgmt_addr",
            "lldp_sys_desc",
            "cdp_device_id",
            "cdp_port_id",
            "cdp_platform",
            "cdp_address",
            "fdb_macs",
            "native_vlan_id",
            "vlan_ids",
            "created_at",
            "updated_at",
            "valid_from",
            "valid_to",
            "lineage_id",
            "last_seen_at",
            "last_discovery_id",
            "first_discovery_id",
        ];

        let values = vec![
            SqlValue::Uuid(id),
            SqlValue::Uuid(host_id),
            SqlValue::Uuid(network_id),
            SqlValue::I32(if_index),
            SqlValue::String(if_descr),
            SqlValue::OptionalString(if_name),
            SqlValue::OptionalString(if_alias),
            SqlValue::I32(if_type),
            SqlValue::OptionalI64(speed_bps),
            SqlValue::I32(i32::from(admin_status)),
            SqlValue::I32(i32::from(oper_status)),
            SqlValue::OptionalMacAddress(mac_address),
            SqlValue::OptionalUuid(ip_address_id),
            SqlValue::OptionalUuid(neighbor_interface_id),
            SqlValue::OptionalUuid(neighbor_host_id),
            SqlValue::OptionalLldpChassisId(lldp_chassis_id),
            SqlValue::OptionalLldpPortId(lldp_port_id),
            SqlValue::OptionalString(lldp_sys_name),
            SqlValue::OptionalString(lldp_port_desc),
            SqlValue::OptionalIpAddr(lldp_mgmt_addr),
            SqlValue::OptionalString(lldp_sys_desc),
            SqlValue::OptionalString(cdp_device_id),
            SqlValue::OptionalString(cdp_port_id),
            SqlValue::OptionalString(cdp_platform),
            SqlValue::OptionalIpAddr(cdp_address),
            SqlValue::OptionalFdbMacs(fdb_macs),
            SqlValue::OptionalUuid(native_vlan_id),
            SqlValue::OptionVecUuid(vlan_ids),
            SqlValue::Timestamp(created_at),
            SqlValue::Timestamp(updated_at),
            SqlValue::Timestamp(valid_from),
            SqlValue::OptionTimestamp(valid_to),
            SqlValue::OptionalUuid(lineage_id),
            SqlValue::Timestamp(last_seen_at),
            SqlValue::OptionalUuid(last_discovery_id),
            SqlValue::OptionalUuid(first_discovery_id),
        ];

        Ok((columns, values))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};

        let admin_status_raw: i32 = row.get("admin_status");
        let oper_status_raw: i32 = row.get("oper_status");

        // Handle speed_bps which might be NULL or a large value
        let speed_bps: Option<i64> = row.get("speed_bps");

        // Read mac_address from MACADDR column
        let mac_address: Option<MacAddress> = row
            .try_get("mac_address")
            .map_err(|e| anyhow::anyhow!("Failed to read mac_address: {}", e))?;

        // Parse neighbor columns into Neighbor enum
        let neighbor_interface_id: Option<Uuid> = row.get("neighbor_interface_id");
        let neighbor_host_id: Option<Uuid> = row.get("neighbor_host_id");
        let neighbor = match (neighbor_interface_id, neighbor_host_id) {
            (Some(id), None) => Some(Neighbor::Interface(id)),
            (None, Some(id)) => Some(Neighbor::Host(id)),
            (None, None) => None,
            // DB constraint should prevent this, but handle gracefully
            (Some(_), Some(_)) => {
                tracing::warn!(
                    "Interface has both neighbor_interface_id and neighbor_host_id set, using neighbor_interface_id"
                );
                Some(Neighbor::Interface(neighbor_interface_id.unwrap()))
            }
        };

        // Parse LLDP JSON fields - they may be null
        let lldp_chassis_json: Option<serde_json::Value> = row.get("lldp_chassis_id");
        let lldp_chassis_id: Option<LldpChassisId> = lldp_chassis_json.and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v).ok()
            }
        });

        let lldp_port_json: Option<serde_json::Value> = row.get("lldp_port_id");
        let lldp_port_id: Option<LldpPortId> = lldp_port_json.and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v).ok()
            }
        });

        Ok(Interface {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: InterfaceBase {
                host_id: row.get("host_id"),
                network_id: row.get("network_id"),
                if_index: row.get("if_index"),
                if_descr: row.get("if_descr"),
                if_name: row.get("if_name"),
                if_alias: row.get("if_alias"),
                if_type: row.get("if_type"),
                speed_bps,
                admin_status: IfAdminStatus::from(admin_status_raw),
                oper_status: IfOperStatus::from(oper_status_raw),
                mac_address,
                ip_address_id: row.get("ip_address_id"),
                neighbor,
                lldp_chassis_id,
                lldp_port_id,
                lldp_sys_name: row.get("lldp_sys_name"),
                lldp_port_desc: row.get("lldp_port_desc"),
                lldp_mgmt_addr: row.try_get("lldp_mgmt_addr").ok().flatten(),
                lldp_sys_desc: row.get("lldp_sys_desc"),
                cdp_device_id: row.get("cdp_device_id"),
                cdp_port_id: row.get("cdp_port_id"),
                cdp_platform: row.get("cdp_platform"),
                cdp_address: row.try_get("cdp_address").ok().flatten(),
                fdb_macs: row
                    .try_get::<Option<serde_json::Value>, _>("fdb_macs")
                    .ok()
                    .flatten()
                    .and_then(|v| serde_json::from_value(v).ok()),
                native_vlan_id: row.get("native_vlan_id"),
                vlan_ids: row
                    .try_get::<Option<serde_json::Value>, _>("vlan_ids")
                    .ok()
                    .flatten()
                    .and_then(|v| serde_json::from_value(v).ok()),
            },
        })
    }
}

impl Entity for Interface {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    type CsvRow = InterfaceCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        InterfaceCsvRow {
            id: self.id,
            host_id: self.base.host_id,
            network_id: self.base.network_id,
            if_index: self.base.if_index,
            if_descr: self.base.if_descr.clone(),
            if_name: self.base.if_name.clone(),
            if_alias: self.base.if_alias.clone(),
            if_type: self.base.if_type,
            speed_bps: self.base.speed_bps,
            admin_status: format!("{:?}", self.base.admin_status),
            oper_status: format!("{:?}", self.base.oper_status),
            mac_address: self.base.mac_address.map(|m| m.to_string()),
            ip_address_id: self.base.ip_address_id,
            neighbor: self.base.neighbor.as_ref().map(|n| match n {
                Neighbor::Interface(id) => format!("Interface:{}", id),
                Neighbor::Host(id) => format!("Host:{}", id),
            }),
            lldp_chassis_id: self
                .base
                .lldp_chassis_id
                .as_ref()
                .and_then(|c| serde_json::to_string(c).ok()),
            lldp_port_id: self
                .base
                .lldp_port_id
                .as_ref()
                .and_then(|p| serde_json::to_string(p).ok()),
            lldp_sys_name: self.base.lldp_sys_name.clone(),
            lldp_port_desc: self.base.lldp_port_desc.clone(),
            lldp_mgmt_addr: self.base.lldp_mgmt_addr.map(|a| a.to_string()),
            lldp_sys_desc: self.base.lldp_sys_desc.clone(),
            cdp_device_id: self.base.cdp_device_id.clone(),
            cdp_port_id: self.base.cdp_port_id.clone(),
            cdp_platform: self.base.cdp_platform.clone(),
            cdp_address: self.base.cdp_address.map(|a| a.to_string()),
            fdb_macs: self
                .base
                .fdb_macs
                .as_ref()
                .and_then(|m| serde_json::to_string(m).ok()),
            native_vlan_id: self.base.native_vlan_id,
            vlan_ids: self
                .base
                .vlan_ids
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok()),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Interface
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Interface";
    const ENTITY_NAME_PLURAL: &'static str = "Interfaces";
    const ENTITY_DESCRIPTION: &'static str =
        "SNMP ifTable entries. Physical and logical interfaces discovered via SNMP on hosts.";

    fn entity_category() -> EntityCategory {
        EntityCategory::NetworkInfrastructure
    }

    fn network_id(&self) -> Option<Uuid> {
        Some(self.base.network_id)
    }

    fn organization_id(&self) -> Option<Uuid> {
        None
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        self.created_at = existing.created_at;
        // MAC address is immutable once set (from SNMP ifPhysAddress)
        if existing.base.mac_address.is_some() {
            self.base.mac_address = existing.base.mac_address;
        }
        // Keep a previously-captured if_name if the current scan happens to lack it
        // (partial SNMP response, or device that stopped reporting ifXTable). Losing
        // if_name silently breaks tier-1 matching on the next scan.
        if existing.base.if_name.is_some() && self.base.if_name.is_none() {
            self.base.if_name = existing.base.if_name.clone();
        }
    }
}

impl ChildStorableEntity for Interface {
    fn parent_column() -> &'static str {
        "host_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.host_id
    }
}

impl Snapshotable for Interface {
    fn id_value(&self) -> Uuid {
        self.id
    }
    fn set_id_value(&mut self, id: Uuid) {
        self.id = id;
    }
    fn valid_from(&self) -> DateTime<Utc> {
        self.valid_from
    }
    fn valid_to(&self) -> Option<DateTime<Utc>> {
        self.valid_to
    }
    fn lineage_id(&self) -> Option<Uuid> {
        self.lineage_id
    }
    fn set_valid_from(&mut self, t: DateTime<Utc>) {
        self.valid_from = t;
    }
    fn set_valid_to(&mut self, t: Option<DateTime<Utc>>) {
        self.valid_to = t;
    }
    fn set_lineage_id(&mut self, id: Option<Uuid>) {
        self.lineage_id = id;
    }

    fn remap_fks_for_clone(&mut self, maps: &FkMaps) {
        if let Some(closed) = maps.hosts.get(&self.base.host_id) {
            self.base.host_id = *closed;
        }
        if let Some(ip_id) = self.base.ip_address_id
            && let Some(closed) = maps.ip_addresses.get(&ip_id)
        {
            self.base.ip_address_id = Some(*closed);
        }
        if let Some(vlan_id) = self.base.native_vlan_id
            && let Some(closed) = maps.vlans.get(&vlan_id)
        {
            self.base.native_vlan_id = Some(*closed);
        }
        // neighbor (Interface(id) | Host(id)) and vlan_ids JSONB array stay
        // as-is — they're cross-host references that may point at entities
        // outside this network's snapshot. Closed rows reference live ids
        // for these; as-of joins handle resolution.
    }
}

impl DiscoveryTracked for Interface {
    fn last_seen_at(&self) -> DateTime<Utc> {
        self.last_seen_at
    }
    fn last_discovery_id(&self) -> Option<Uuid> {
        self.last_discovery_id
    }
    fn first_discovery_id(&self) -> Option<Uuid> {
        self.first_discovery_id
    }
    fn set_last_seen_at(&mut self, t: DateTime<Utc>) {
        self.last_seen_at = t;
    }
    fn set_last_discovery_id(&mut self, id: Option<Uuid>) {
        self.last_discovery_id = id;
    }
    fn set_first_discovery_id(&mut self, id: Option<Uuid>) {
        self.first_discovery_id = id;
    }

    fn scanned_in_session_filter(
        scanned: &crate::server::daemons::r#impl::api::ScannedEntityIds,
    ) -> crate::server::shared::storage::filter::StorableFilter<Self> {
        crate::server::shared::storage::filter::StorableFilter::<Self>::new_from_uuids_column(
            "id",
            &scanned.interface_ids,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mac_address::MacAddress;

    fn make_interface(if_index: i32, if_name: Option<&str>, mac: Option<&str>) -> Interface {
        let mut base = InterfaceBase::default();
        base.host_id = Uuid::new_v4();
        base.network_id = Uuid::new_v4();
        base.if_index = if_index;
        base.if_name = if_name.map(String::from);
        base.mac_address = mac.map(|s| s.parse::<MacAddress>().unwrap());
        Interface::new(base)
    }

    #[test]
    fn preserve_immutable_fields_keeps_existing_if_name_when_incoming_is_none() {
        let existing = make_interface(5, Some("GigabitEthernet0/1"), None);
        let mut incoming = make_interface(5, None, None);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(
            incoming.base.if_name.as_deref(),
            Some("GigabitEthernet0/1"),
            "Existing if_name must survive a scan that dropped it; otherwise tier-1 matching silently breaks next time."
        );
    }

    #[test]
    fn preserve_immutable_fields_allows_if_name_to_be_updated_when_incoming_has_value() {
        let existing = make_interface(5, Some("Old"), None);
        let mut incoming = make_interface(5, Some("New"), None);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.base.if_name.as_deref(), Some("New"));
    }

    #[test]
    fn preserve_immutable_fields_populates_if_name_from_legacy_null_row() {
        // Scenario: pre-existing prod row has if_name = NULL; rescan reports "eth0".
        // Incoming.if_name is Some, existing.if_name is None. Incoming value wins.
        let existing = make_interface(5, None, None);
        let mut incoming = make_interface(5, Some("eth0"), None);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.base.if_name.as_deref(), Some("eth0"));
    }

    #[test]
    fn preserve_immutable_fields_keeps_existing_mac_when_incoming_is_none() {
        let existing = make_interface(5, Some("eth0"), Some("aa:bb:cc:dd:ee:ff"));
        let mut incoming = make_interface(5, Some("eth0"), None);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(
            incoming.base.mac_address, existing.base.mac_address,
            "MAC address should be treated as immutable once captured from SNMP ifPhysAddress."
        );
    }

    #[test]
    fn preserve_immutable_fields_copies_created_at() {
        let existing = make_interface(5, Some("eth0"), None);
        let mut incoming = make_interface(5, Some("eth0"), None);
        // Incoming has a fresh created_at; after preservation it should match the
        // existing row's created_at so the row's age is not reset on every scan.
        assert_ne!(existing.created_at, incoming.created_at);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.created_at, existing.created_at);
    }
}
