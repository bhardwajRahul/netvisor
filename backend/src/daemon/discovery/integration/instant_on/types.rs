//! Raw Instant On portal JSON shapes.
//!
//! **This module is deliberately free of Scanopy entity types.** Everything here mirrors what the
//! portal puts on the wire; the translation into hosts/interfaces lives in [`super::mapping`].
//! When a field turns out to be shaped differently against real hardware, this is the only file
//! that needs correcting.
//!
//! # Provenance
//!
//! HPE publishes no API for Instant On. These shapes are read from the portal's own web client —
//! resource names from its route table, field names from its models — and confirmed against a
//! live site. Treat every one of them as provisional until it has been seen in a real payload.
//!
//! # Absence is normal
//!
//! Every field is optional, and that is load-bearing rather than lazy. One `inventory` response
//! mixes device classes with genuinely different shapes: an access point has no port table, a
//! non-PoE switch reports no PoE state, an 1830 carries less than a 1960. Only one operator's
//! 1960s were available to validate against, so the other models' payloads are unseen — a missing
//! field must cost that one datum, never the device and never the collection.
//!
//! No `deny_unknown_fields` anywhere, for the same reason it is absent from the UniFi types:
//! a cloud service adds keys whenever it likes, and rejecting them would turn a cosmetic change
//! into total topology loss.

use serde::Deserialize;

pub use crate::daemon::discovery::integration::flex::{FlexBool, FlexInt};

/// Standard Instant On list envelope: `{"elements": [...]}`.
///
/// No status/`meta` block to check, unlike UniFi — the portal signals failure with the HTTP
/// status, which [`super::client`] handles before decoding.
#[derive(Debug, Default, Deserialize)]
pub struct InstantOnEnvelope<T> {
    #[serde(default = "Vec::new")]
    pub elements: Vec<T>,
}

/// One site from `GET /api/sites/`.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstantOnSite {
    pub id: Option<String>,
    pub name: Option<String>,
}

/// One device from `GET /api/sites/{id}/inventory`.
///
/// Covers all four `device-type-enum` classes with a single permissive struct, the same choice
/// the UniFi types make across `USW`/`UAP`/`USG`/`UDM`.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstantOnDevice {
    /// Portal-assigned device id. Stable across renames, unlike `name`.
    pub id: Option<String>,
    /// The device's MAC — its stable hardware identity.
    pub mac_address: Option<String>,
    /// Management IP. Absent for a device that has never checked in.
    pub ip_address: Option<String>,
    /// Admin-assigned name.
    pub name: Option<String>,
    /// Model string (e.g. `"1960-48G-4SFP"`).
    pub model: Option<String>,
    pub serial_number: Option<String>,
    /// Running firmware version.
    pub firmware_version: Option<String>,
    /// `ACCESS_POINT` | `GATEWAY` | `STACK` | `SWITCH`. Feeds `Pattern::ManagedDeviceType`.
    pub device_type: Option<String>,
    /// Ports, when the class has any. Empty for an access point.
    #[serde(default)]
    pub ports: Vec<InstantOnPort>,
    /// How this device reaches its parent. Instant On derives this itself rather than from LLDP,
    /// so it is authoritative for the site even where LLDP is not running.
    pub uplink: Option<InstantOnUplink>,
    /// Stack members, when `device_type` is `STACK`.
    ///
    /// A stack is one host with one management IP, so members are not hosts of their own — this
    /// is carried for the serial/model detail it adds, not to split the device up.
    #[serde(default)]
    pub members: Vec<InstantOnStackMember>,
}

/// One switch port.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstantOnPort {
    /// Portal port identifier. On a stack this is member-qualified (`"1/1/1"`), on a standalone
    /// switch it is not — which is why mapping keeps it opaque instead of assuming an index.
    pub id: Option<String>,
    /// Human port label as the portal shows it.
    pub name: Option<String>,
    /// Physical port number *within its member*. Not unique across a stack on its own.
    pub port_number: Option<FlexInt>,
    /// Link state.
    pub up: Option<FlexBool>,
    /// Admin state.
    pub enabled: Option<FlexBool>,
    /// Negotiated speed in Mbit/s (0 or absent when down).
    pub speed: Option<FlexInt>,
    /// VLAN assigned to the port, when the portal reports one.
    pub vlan_id: Option<FlexInt>,
}

/// One member switch of a stack.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstantOnStackMember {
    pub serial_number: Option<String>,
    pub model: Option<String>,
    pub mac_address: Option<String>,
    /// Member number within the stack — the leading component of a member-qualified port id.
    pub member_id: Option<FlexInt>,
}

/// How a device reaches its parent.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstantOnUplink {
    /// The parent device's id, as it appears in the same inventory response.
    pub connected_to: Option<String>,
    /// The parent's MAC, when reported.
    pub connected_to_mac_address: Option<String>,
    /// Our local port id the uplink leaves from.
    pub port_id: Option<String>,
    /// The parent's port id we plug into.
    pub remote_port_id: Option<String>,
}

/// One client from `GET /api/sites/{id}/clientSummary`.
///
/// Wired clients are the interesting ones: they name the switch port they are on, which is what
/// becomes a bridge-FDB entry. Wireless clients are carried by the same endpoint and are
/// distinguished by `wireless_network_id` being present.
///
/// Client IP addresses are frequently absent here even for connected wired clients, which is one
/// reason mapping never turns these into hosts — host identity is IP-based.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstantOnClient {
    pub mac_address: Option<String>,
    pub name: Option<String>,
    pub ip_address: Option<String>,
    /// Present ⇒ this is a wireless client, and there is no switch port to attach it to.
    pub wireless_network_id: Option<String>,
    /// Device id of the switch this client is attached to.
    pub connected_to: Option<String>,
    /// Port id on that switch.
    pub port_id: Option<String>,
    pub vlan_id: Option<FlexInt>,
}

impl InstantOnClient {
    /// Whether this client is attached to a switch port rather than an SSID.
    pub fn is_wired(&self) -> bool {
        self.wireless_network_id.is_none()
    }
}
