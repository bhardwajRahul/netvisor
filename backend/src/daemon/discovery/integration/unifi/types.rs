//! Raw UniFi controller JSON shapes.
//!
//! **This module is deliberately free of Scanopy entity types.** Everything here mirrors what
//! the controller actually puts on the wire; the translation into hosts/interfaces lives in
//! [`super::mapping`]. When a field shape turns out to be wrong against real hardware, this is
//! the only file that needs correcting.
//!
//! # Provenance
//!
//! Ubiquiti does not officially document the `stat/device` sub-tables (`port_table`,
//! `lldp_table`, `mac_table`, `uplink`, `downlink_table`). The de-facto reference is the
//! **unpoller/unifi** Go library, whose structs are cited per field below. Where unpoller
//! defines one struct per device class (`USW`, `UAP`, `USG`, `UDM`), Scanopy uses a single
//! permissive struct instead — every field is optional, so a device class that omits one
//! simply leaves it `None`.
//!
//! No `deny_unknown_fields` anywhere: newer firmware adds keys routinely, and rejecting them
//! would turn a cosmetic firmware change into total topology loss.

use serde::Deserialize;

/// Standard UniFi response envelope: `{"meta": {"rc": "ok"}, "data": [...]}`.
#[derive(Debug, Deserialize)]
pub struct UnifiEnvelope<T> {
    #[serde(default)]
    pub meta: UnifiMeta,
    #[serde(default = "Vec::new")]
    pub data: Vec<T>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UnifiMeta {
    /// `"ok"` on success; `"error"` with `msg` populated otherwise.
    #[serde(default)]
    pub rc: String,
    #[serde(default)]
    pub msg: Option<String>,
}

impl UnifiMeta {
    pub fn is_ok(&self) -> bool {
        // Older controllers omit `meta` entirely on success, so an empty `rc` is not a failure.
        self.rc.is_empty() || self.rc == "ok"
    }
}

/// One device from `GET /stat/device`.
///
/// unpoller: `unifi.USW` / `unifi.UAP` / `unifi.USG` / `unifi.UDM`.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct UnifiDevice {
    /// unpoller: `USW.Mac`. The device's chassis MAC — its stable identity.
    pub mac: Option<String>,
    /// unpoller: `USW.IP`. Management IP. Absent for a device that has not checked in.
    pub ip: Option<String>,
    /// unpoller: `USW.Name`. Admin-assigned name; absent until the device is named.
    pub name: Option<String>,
    /// unpoller: `USW.Model` (e.g. `"USL24P"`).
    pub model: Option<String>,
    /// unpoller: `USW.Serial`.
    pub serial: Option<String>,
    /// unpoller: `USW.Version`. Running firmware version.
    pub version: Option<String>,
    /// unpoller: `USW.Type` — the device class discriminator (`"usw"`, `"uap"`, `"ugw"`,
    /// `"udm"`). Feeds `Pattern::ManagedDeviceType` for service matching.
    #[serde(rename = "type")]
    pub device_type: Option<String>,
    /// unpoller: `USW.PortTable []Port`. Empty on devices with no switch ports (e.g. an AP).
    #[serde(default)]
    pub port_table: Vec<UnifiPort>,
    /// unpoller: `USW.LLDPTable []LLDPTable`. Neighbors the firmware learned via LLDP —
    /// the payload SNMP cannot reach on these switches.
    #[serde(default)]
    pub lldp_table: Vec<UnifiLldpEntry>,
    /// unpoller: `USW.DownlinkTable []DownlinkTable`. Devices adopted *below* this one.
    #[serde(default)]
    pub downlink_table: Vec<UnifiDownlink>,
    /// unpoller: `USW.Uplink Uplink`. How this device reaches its parent.
    pub uplink: Option<UnifiUplink>,
}

/// One switch port. unpoller: `unifi.Port`.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct UnifiPort {
    /// unpoller: `Port.PortIDX`. 1-based, matches the physical port label.
    pub port_idx: Option<FlexInt>,
    /// unpoller: `Port.Name` (e.g. `"Port 1"`, or an admin-set label).
    pub name: Option<String>,
    /// unpoller: `Port.Up` — link state.
    pub up: Option<FlexBool>,
    /// unpoller: `Port.Enable` — admin state.
    pub enable: Option<FlexBool>,
    /// unpoller: `Port.Speed`, in Mbit/s (0 when down).
    pub speed: Option<FlexInt>,
    /// unpoller: `Port.Mac`. Frequently the device's chassis MAC repeated on every port
    /// rather than a per-port address — see `mapping::port_to_interface` for why that
    /// distinction matters.
    pub mac: Option<String>,
    /// unpoller: `Port.IsUplink`.
    pub is_uplink: Option<FlexBool>,
    /// unpoller: `Port.MacTable []MacTable`. Learned MACs on this port (the bridge FDB).
    #[serde(default)]
    pub mac_table: Vec<UnifiMacTableEntry>,
}

/// One learned MAC on a port. unpoller: `unifi.MacTable`.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct UnifiMacTableEntry {
    /// unpoller: `MacTable.Mac`.
    pub mac: Option<String>,
    /// unpoller: `MacTable.Vlan`.
    pub vlan: Option<FlexInt>,
    /// unpoller: `MacTable.Age`.
    pub age: Option<FlexInt>,
    /// unpoller: `MacTable.Static`.
    #[serde(rename = "static")]
    pub is_static: Option<FlexBool>,
}

/// One LLDP neighbor. unpoller: `unifi.LLDPTable`.
///
/// Note the controller reports already-decoded strings and **no IEEE subtype byte**, which is
/// why mapping uses `LldpChassisId::from_identifier_str` rather than `from_snmp`.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct UnifiLldpEntry {
    /// unpoller: `LLDPTable.ChassisID`. Usually the neighbor's MAC; sometimes a hostname.
    pub chassis_id: Option<String>,
    /// unpoller: `LLDPTable.PortID`. The neighbor's port identifier, vendor-formatted.
    pub port_id: Option<String>,
    /// unpoller: `LLDPTable.LocalPortIdx`. Which of *our* ports saw this neighbor.
    pub local_port_idx: Option<FlexInt>,
    /// unpoller: `LLDPTable.LocalPortName`.
    pub local_port_name: Option<String>,
    /// unpoller: `LLDPTable.IsWired`.
    pub is_wired: Option<FlexBool>,
}

/// A device adopted below this one. unpoller: `unifi.DownlinkTable`.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct UnifiDownlink {
    /// unpoller: `DownlinkTable.Mac` — the downstream device's chassis MAC.
    pub mac: Option<String>,
    /// unpoller: `DownlinkTable.PortIdx` — our port it is attached to.
    pub port_idx: Option<FlexInt>,
    /// unpoller: `DownlinkTable.SpeedMbps`.
    pub speed: Option<FlexInt>,
}

/// How a device reaches its parent. unpoller: `unifi.Uplink`.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct UnifiUplink {
    /// unpoller: `Uplink.Name` — our local uplink interface name (e.g. `"eth0"`).
    pub name: Option<String>,
    /// unpoller: `Uplink.Mac` — *our* uplink interface MAC. On an AP this is a genuinely
    /// distinct per-interface address, unlike the repeated port MACs on a switch.
    pub mac: Option<String>,
    /// unpoller: `Uplink.PortIdx` — our local port index, when we have a port table.
    pub port_idx: Option<FlexInt>,
    /// unpoller: `Uplink.UplinkMac` — the *parent's* chassis MAC.
    pub uplink_mac: Option<String>,
    /// unpoller: `Uplink.UplinkRemotePort` — the parent's port index we plug into.
    pub uplink_remote_port: Option<FlexInt>,
    /// unpoller: `Uplink.UplinkDeviceName` — the parent's name, when reported.
    pub uplink_device_name: Option<String>,
}

// `FlexInt`/`FlexBool` moved to `super::super::flex` when Instant On needed the same tolerance
// against its own loosely-typed vendor JSON. Re-exported here so the wire structs above (and
// anything importing them from this module) are unaffected.
pub use crate::daemon::discovery::integration::flex::{FlexBool, FlexInt};
