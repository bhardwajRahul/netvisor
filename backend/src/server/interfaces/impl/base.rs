use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};
use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Which groups of per-interface data the daemon read in full during a scan.
///
/// Each group comes from its own SNMP walk, and a walk cut short by a timeout yields exactly the
/// same empty result as a device that genuinely has nothing to report. Without knowing which
/// happened, the server overwrote good data with NULL on every truncation — and for the neighbour
/// fields that also dropped the row out of L2 resolution permanently, since the resolution filter
/// requires a chassis id or CDP device id to be present.
///
/// Every field defaults to `true`, so a daemon predating this behaves exactly as before: it
/// reports everything as authoritative and the server overwrites.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct InterfaceDataComplete {
    /// `lldp_chassis_id`, `lldp_port_id`, `lldp_sys_name`, `lldp_port_desc`, `lldp_mgmt_addr`,
    /// `lldp_sys_desc`
    #[serde(default = "crate::server::interfaces::r#impl::base::complete_default")]
    pub lldp: bool,
    /// `cdp_device_id`, `cdp_port_id`, `cdp_platform`, `cdp_address`
    #[serde(default = "crate::server::interfaces::r#impl::base::complete_default")]
    pub cdp: bool,
    /// `fdb_macs`
    #[serde(default = "crate::server::interfaces::r#impl::base::complete_default")]
    pub fdb: bool,
    /// `native_vlan_id`, `vlan_ids`
    #[serde(default = "crate::server::interfaces::r#impl::base::complete_default")]
    pub vlan_membership: bool,
}

pub(crate) fn complete_default() -> bool {
    true
}

impl Default for InterfaceDataComplete {
    fn default() -> Self {
        Self {
            lldp: true,
            cdp: true,
            fdb: true,
            vlan_membership: true,
        }
    }
}

impl InterfaceDataComplete {
    /// Whether every group was read in full.
    pub fn all(&self) -> bool {
        self.lldp && self.cdp && self.fdb && self.vlan_membership
    }

    /// No group has been read yet, so the server keeps everything it already holds.
    ///
    /// The counterpart to [`Default`], which claims every group is authoritative. That default is
    /// right for the wire (an old daemon that never sends the field behaved that way), and wrong
    /// for a checkpoint written partway through a collection: SNMP persists its interface set as
    /// soon as the ifTable walk finishes, long before the neighbour and VLAN walks run, and
    /// shipping the all-`true` default alongside it told the server those columns were
    /// authoritatively empty. It cleared them — and an interface with no chassis id drops out of
    /// L2 resolution for good.
    pub fn none() -> Self {
        Self {
            lldp: false,
            cdp: false,
            fdb: false,
            vlan_membership: false,
        }
    }
}

/// Resolved LLDP/CDP neighbor connection.
///
/// Represents the remote endpoint this port connects to, discovered via LLDP or CDP.
/// The two variants are mutually exclusive and represent different resolution states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "type", content = "id")]
pub enum Neighbor {
    /// Full resolution - the specific remote port was identified
    #[schema(title = "Interface")]
    Interface(Uuid),
    /// Partial resolution - the remote device was identified but not the specific port
    #[schema(title = "Host")]
    Host(Uuid),
}

impl Neighbor {
    /// Get the Interface ID if this is a full resolution
    pub fn interface_id(&self) -> Option<Uuid> {
        match self {
            Neighbor::Interface(id) => Some(*id),
            Neighbor::Host(_) => None,
        }
    }

    /// Returns true if this is a full resolution (specific port known)
    pub fn is_full_resolution(&self) -> bool {
        matches!(self, Neighbor::Interface(_))
    }

    /// Returns true if this is a partial resolution (only host known)
    pub fn is_partial_resolution(&self) -> bool {
        matches!(self, Neighbor::Host(_))
    }
}

/// SNMP ifAdminStatus values per IF-MIB RFC 2863
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema)]
#[repr(i32)]
pub enum IfAdminStatus {
    #[default]
    Up = 1,
    Down = 2,
    Testing = 3,
}

impl From<i32> for IfAdminStatus {
    fn from(value: i32) -> Self {
        match value {
            1 => IfAdminStatus::Up,
            2 => IfAdminStatus::Down,
            3 => IfAdminStatus::Testing,
            _ => IfAdminStatus::Up,
        }
    }
}

impl From<IfAdminStatus> for i32 {
    fn from(value: IfAdminStatus) -> Self {
        value as i32
    }
}

/// SNMP ifOperStatus values per IF-MIB RFC 2863
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    Default,
    ToSchema,
    strum_macros::Display,
)]
#[repr(i32)]
pub enum IfOperStatus {
    #[default]
    Up = 1,
    Down = 2,
    Testing = 3,
    Unknown = 4,
    Dormant = 5,
    NotPresent = 6,
    LowerLayerDown = 7,
}

impl From<i32> for IfOperStatus {
    fn from(value: i32) -> Self {
        match value {
            1 => IfOperStatus::Up,
            2 => IfOperStatus::Down,
            3 => IfOperStatus::Testing,
            4 => IfOperStatus::Unknown,
            5 => IfOperStatus::Dormant,
            6 => IfOperStatus::NotPresent,
            7 => IfOperStatus::LowerLayerDown,
            _ => IfOperStatus::Unknown,
        }
    }
}

impl From<IfOperStatus> for i32 {
    fn from(value: IfOperStatus) -> Self {
        value as i32
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct InterfaceBase {
    /// The host this entity belongs to.
    pub host_id: Uuid,
    /// The network this entity belongs to.
    pub network_id: Uuid,
    /// SNMP ifIndex - stable identifier within device
    pub if_index: i32,
    /// SNMP ifDescr - interface description (e.g., GigabitEthernet0/1)
    #[validate(length(min = 1, message = "Interface description is required"))]
    pub if_descr: String,
    /// SNMP ifName - short interface name (e.g., Gi1/0/1)
    pub if_name: Option<String>,
    /// SNMP ifAlias - user-configured description
    pub if_alias: Option<String>,
    /// SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)
    pub if_type: i32,
    /// Interface speed from ifSpeed/ifHighSpeed in bits per second
    pub speed_bps: Option<i64>,
    /// SNMP ifAdminStatus: 1=up, 2=down, 3=testing
    pub admin_status: IfAdminStatus,
    /// SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown
    pub oper_status: IfOperStatus,

    // Local links
    /// MAC address from SNMP ifPhysAddress - immutable once set
    #[serde(default)]
    #[schema(value_type = Option<String>, pattern = r"^(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$", example = "a4:bb:6d:12:34:56")]
    pub mac_address: Option<MacAddress>,
    /// FK to IPAddress entity - this port's IP assignment (must be on same host).
    /// Old daemons send this as "interface_id".
    #[serde(alias = "interface_id")]
    pub ip_address_id: Option<Uuid>,

    // Neighbor resolution (LLDP/CDP) - remote endpoint this port connects to
    /// Resolved neighbor connection (mutually exclusive: either Interface or Host)
    pub neighbor: Option<Neighbor>,

    // Raw LLDP data (from SNMP lldpRemTable, used for resolution and display)
    /// Remote chassis identifier from LLDP neighbor (globally/locally unique)
    pub lldp_chassis_id: Option<LldpChassisId>,
    /// Remote port identifier from LLDP neighbor
    pub lldp_port_id: Option<LldpPortId>,
    /// Remote system name from LLDP neighbor (lldpRemSysName)
    pub lldp_sys_name: Option<String>,
    /// Remote port description from LLDP neighbor (lldpRemPortDesc)
    pub lldp_port_desc: Option<String>,
    /// Remote management IP from LLDP neighbor (lldpRemManAddr). IPv4 or IPv6.
    #[schema(value_type = Option<String>, example = "192.168.1.1")]
    pub lldp_mgmt_addr: Option<std::net::IpAddr>,
    /// Remote system description from LLDP neighbor (lldpRemSysDesc) - platform info
    pub lldp_sys_desc: Option<String>,

    // Raw CDP data (from SNMP cdpCacheTable, Cisco devices)
    /// Remote device ID from CDP (typically hostname, locally unique)
    pub cdp_device_id: Option<String>,
    /// Remote port ID from CDP
    pub cdp_port_id: Option<String>,
    /// Remote platform from CDP (e.g., "Cisco IOS")
    pub cdp_platform: Option<String>,
    /// Remote management IP from CDP (cdpCacheAddress). IPv4 or IPv6.
    #[schema(value_type = Option<String>, example = "192.168.1.1")]
    pub cdp_address: Option<std::net::IpAddr>,

    /// Bridge FDB: learned MAC addresses on this switch port.
    /// Single-MAC ports can be resolved to neighbor links server-side.
    /// Multi-MAC ports indicate uplinks where LLDP/CDP is the better source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fdb_macs: Option<Vec<String>>,

    /// Native/untagged VLAN entity ID on this port (resolved from Q-BRIDGE dot1qPvid)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_vlan_id: Option<Uuid>,

    /// Tagged VLAN entity IDs on this port (resolved from Q-BRIDGE dot1qVlanCurrentEgressPorts)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vlan_ids: Option<Vec<Uuid>>,
}

impl Default for InterfaceBase {
    fn default() -> Self {
        Self {
            host_id: Uuid::nil(),
            network_id: Uuid::nil(),
            if_index: 0,
            if_descr: String::new(),
            if_name: None,
            if_alias: None,
            if_type: 1, // other
            speed_bps: None,
            admin_status: IfAdminStatus::Up,
            oper_status: IfOperStatus::Up,
            mac_address: None,
            ip_address_id: None,
            neighbor: None,
            lldp_chassis_id: None,
            lldp_port_id: None,
            lldp_sys_name: None,
            lldp_port_desc: None,
            lldp_mgmt_addr: None,
            lldp_sys_desc: None,
            cdp_device_id: None,
            cdp_port_id: None,
            cdp_platform: None,
            cdp_address: None,
            fdb_macs: None,
            native_vlan_id: None,
            vlan_ids: None,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash, Default, ToSchema, Validate,
)]
pub struct Interface {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// Start of the interval this revision was current for (SCD2 history).
    #[serde(default)]
    #[schema(read_only)]
    pub valid_from: DateTime<Utc>,
    /// End of the interval this revision was current for. `null` while it is the live revision.
    #[serde(default)]
    #[schema(read_only)]
    pub valid_to: Option<DateTime<Utc>>,
    /// Stable identifier shared by every revision of the same entity across its history.
    #[serde(default)]
    #[schema(read_only)]
    pub lineage_id: Option<Uuid>,
    /// When a discovery last observed this entity.
    #[serde(default)]
    #[schema(read_only)]
    pub last_seen_at: DateTime<Utc>,
    /// The most recent discovery that observed this entity.
    #[serde(default)]
    #[schema(read_only)]
    pub last_discovery_id: Option<Uuid>,
    /// The discovery that first observed this entity.
    #[serde(default)]
    #[schema(read_only)]
    pub first_discovery_id: Option<Uuid>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: InterfaceBase,
}

impl ChangeTriggersTopologyStaleness<Interface> for Interface {
    fn triggers_staleness(&self, other: Option<Interface>) -> bool {
        if let Some(other_entry) = other {
            // Topology changes if neighbor changes (link discovery)
            self.base.neighbor != other_entry.base.neighbor
                || self.base.ip_address_id != other_entry.base.ip_address_id
                || self.base.host_id != other_entry.base.host_id
        } else {
            true // New or deleted entry triggers staleness
        }
    }
}

impl Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Interface {} (ifIndex {}): {}",
            self.id, self.base.if_index, self.base.if_descr
        )
    }
}

impl Interface {
    pub fn new(base: InterfaceBase) -> Self {
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

    /// Returns true if interface is operationally up
    pub fn is_up(&self) -> bool {
        self.base.oper_status == IfOperStatus::Up
    }

    /// Returns true if interface is administratively up
    pub fn is_admin_up(&self) -> bool {
        self.base.admin_status == IfAdminStatus::Up
    }

    /// Get display name - prefer ifAlias if set, otherwise ifDescr
    pub fn display_name(&self) -> &str {
        self.base.if_alias.as_deref().unwrap_or(&self.base.if_descr)
    }

    /// Returns true if this port has a resolved neighbor connection
    pub fn has_neighbor(&self) -> bool {
        self.base.neighbor.is_some()
    }

    /// Returns true if neighbor is fully resolved (remote port known)
    pub fn has_full_neighbor_resolution(&self) -> bool {
        self.base
            .neighbor
            .as_ref()
            .map(|n| n.is_full_resolution())
            .unwrap_or(false)
    }

    /// Returns true if this port has raw LLDP data (may or may not be resolved)
    pub fn has_lldp_data(&self) -> bool {
        self.base.lldp_chassis_id.is_some() || self.base.lldp_port_id.is_some()
    }

    /// Returns true if this port has raw CDP data (may or may not be resolved)
    pub fn has_cdp_data(&self) -> bool {
        self.base.cdp_device_id.is_some() || self.base.cdp_port_id.is_some()
    }

    /// Returns true if this port has any neighbor discovery data (LLDP or CDP)
    pub fn has_neighbor_discovery_data(&self) -> bool {
        self.has_lldp_data() || self.has_cdp_data()
    }

    /// Keep the stored values for any group of data this scan did not finish reading.
    ///
    /// Each group of neighbour/VLAN fields comes from its own SNMP walk, and a walk cut short by a
    /// timeout returns exactly what a device with nothing to report returns: nothing. Overwriting
    /// on that is destructive rather than merely stale — losing `lldp_chassis_id` also drops the
    /// row out of L2 resolution (the pass requires a chassis id or CDP device id), so the link
    /// freezes at whatever it last resolved to and no rescan can repair it.
    ///
    /// A group the daemon *did* read in full is authoritative in both directions: absence means
    /// the neighbour is genuinely gone and must be cleared, or a decommissioned link lingers for
    /// ever.
    pub fn preserve_uncollected_data(&mut self, existing: &Self, collected: InterfaceDataComplete) {
        if !collected.lldp {
            self.base.lldp_chassis_id = existing.base.lldp_chassis_id.clone();
            self.base.lldp_port_id = existing.base.lldp_port_id.clone();
            self.base.lldp_sys_name = existing.base.lldp_sys_name.clone();
            self.base.lldp_port_desc = existing.base.lldp_port_desc.clone();
            self.base.lldp_mgmt_addr = existing.base.lldp_mgmt_addr;
            self.base.lldp_sys_desc = existing.base.lldp_sys_desc.clone();
        }
        if !collected.cdp {
            self.base.cdp_device_id = existing.base.cdp_device_id.clone();
            self.base.cdp_port_id = existing.base.cdp_port_id.clone();
            self.base.cdp_platform = existing.base.cdp_platform.clone();
            self.base.cdp_address = existing.base.cdp_address;
        }
        if !collected.fdb {
            self.base.fdb_macs = existing.base.fdb_macs.clone();
        }
        if !collected.vlan_membership {
            self.base.native_vlan_id = existing.base.native_vlan_id;
            self.base.vlan_ids = existing.base.vlan_ids.clone();
        }
    }

    /// Drop identity fields the device reported blank, so absence is recorded as absence.
    ///
    /// A zero-length ifXTable `ifName` is a legitimate SNMP answer meaning "this device has no
    /// name for this port", but it reaches the server as `Some("")` and from there is treated as
    /// a real name: the tiered discovery match keys on `if_name.is_some()`, and the partial unique
    /// index `(host_id, if_name) WHERE if_name IS NOT NULL` counts `""` as a value. A switch that
    /// answers `""` for every port then hits a duplicate-key violation on its second port —
    /// truncating that host's whole ingest. `if_index` remains as the identity for such ports,
    /// which is the correct one anyway.
    ///
    /// Called on the discovery ingest path so devices behind older daemons are covered too.
    pub fn normalize_blank_identity(&mut self) {
        if self
            .base
            .if_name
            .as_deref()
            .is_some_and(|name| name.trim().is_empty())
        {
            self.base.if_name = None;
        }
        if self
            .base
            .if_alias
            .as_deref()
            .is_some_and(|alias| alias.trim().is_empty())
        {
            self.base.if_alias = None;
        }
    }
}

/// Common IANAifType values for reference
/// Full list: https://www.iana.org/assignments/ianaiftype-mib/ianaiftype-mib
pub mod if_type {
    pub const OTHER: i32 = 1;
    pub const ETHERNET_CSMA_CD: i32 = 6;
    pub const ISO88023_CSMA_CD: i32 = 7;
    pub const FAST_ETHERNET: i32 = 62;
    pub const GIGABIT_ETHERNET: i32 = 117;
    pub const SOFTWARE_LOOPBACK: i32 = 24;
    pub const TUNNEL: i32 = 131;
    pub const PROP_VIRTUAL: i32 = 53;
    pub const IEEE8023AD_LAG: i32 = 161; // Link Aggregation Group
    pub const BRIDGE: i32 = 209;
    pub const VLAN: i32 = 135;
    pub const L2_VLAN: i32 = 136;
    pub const L3_IPVLAN: i32 = 137;
}
