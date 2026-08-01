//! LLDP (Link Layer Discovery Protocol) types and resolution.
//!
//! This module provides enums for LLDP identifier types per IEEE 802.1AB,
//! along with resolution methods to convert LLDP neighbor data into
//! database entity references.

use crate::server::shared::storage::pg_value::strip_nuls;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use strum_macros::VariantNames;
use utoipa::ToSchema;
use uuid::Uuid;

/// LLDP Chassis ID subtypes per IEEE 802.1AB.
///
/// The chassis ID identifies the remote device. Different network equipment
/// may use different subtypes depending on configuration and capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames, ToSchema)]
#[serde(tag = "subtype", content = "value")]
pub enum LldpChassisId {
    /// Subtype 1: Chassis component (e.g., backplane serial number)
    #[schema(title = "ChassisComponent")]
    ChassisComponent(String),
    /// Subtype 2: Interface alias (ifAlias from IF-MIB)
    #[schema(title = "InterfaceAlias")]
    InterfaceAlias(String),
    /// Subtype 3: Port component (e.g., backplane port number)
    #[schema(title = "PortComponent")]
    PortComponent(String),
    /// Subtype 4: MAC address (most common)
    #[schema(title = "MacAddress")]
    MacAddress(String),
    /// Subtype 5: Network address (IP address stored as string)
    #[schema(value_type = String)]
    #[schema(title = "NetworkAddress")]
    NetworkAddress(#[serde(with = "ip_addr_serde")] IpAddr),
    /// Subtype 6: Interface name (ifName from IF-MIB)
    #[schema(title = "InterfaceName")]
    InterfaceName(String),
    /// Subtype 7: Locally assigned (device-specific identifier)
    #[schema(title = "LocallyAssigned")]
    LocallyAssigned(String),
}

/// LLDP Port ID subtypes per IEEE 802.1AB.
///
/// The port ID identifies the specific port on the remote device.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, VariantNames, ToSchema)]
#[serde(tag = "subtype", content = "value")]
pub enum LldpPortId {
    /// Subtype 1: Interface alias (ifAlias from IF-MIB)
    #[schema(title = "InterfaceAlias")]
    InterfaceAlias(String),
    /// Subtype 2: Port component (e.g., backplane port number)
    #[schema(title = "PortComponent")]
    PortComponent(String),
    /// Subtype 3: MAC address
    #[schema(title = "MacAddress")]
    MacAddress(String),
    /// Subtype 4: Network address (IP address stored as string)
    #[schema(value_type = String)]
    #[schema(title = "NetworkAddress")]
    NetworkAddress(#[serde(with = "ip_addr_serde")] IpAddr),
    /// Subtype 5: Interface name (ifName from IF-MIB)
    #[schema(title = "InterfaceName")]
    InterfaceName(String),
    /// Subtype 6: Agent circuit ID (used by some providers)
    #[schema(title = "AgentCircuitId")]
    AgentCircuitId(String),
    /// Subtype 7: Locally assigned (device-specific identifier)
    #[schema(title = "LocallyAssigned")]
    LocallyAssigned(String),
}

/// Outcome of resolving an advertised LLDP identifier to a database entity.
///
/// "Didn't resolve" has two causes that call for opposite responses, so they are not collapsed
/// into a bare `None`: [`Self::NoStrategy`] means the neighbor advertised an identity this system
/// has no way to look up (a code-side gap, or a subtype that genuinely carries no usable
/// identity), while [`Self::NotFound`] means the lookup ran correctly and the device simply isn't
/// in this network's inventory (an operator-side gap — scan it and the link appears).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityResolution {
    /// Matched exactly one entity.
    Resolved(Uuid),
    /// No lookup strategy applies to the advertised identifier.
    NoStrategy,
    /// A strategy ran and matched nothing, or matched ambiguously.
    NotFound,
}

impl IdentityResolution {
    /// `Resolved(id)` when `found` is `Some`, otherwise [`Self::NotFound`].
    pub fn found(found: Option<Uuid>) -> Self {
        match found {
            Some(id) => Self::Resolved(id),
            None => Self::NotFound,
        }
    }
}

/// Serde helper for IpAddr as string
mod ip_addr_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::net::IpAddr;

    pub fn serialize<S>(ip: &IpAddr, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&ip.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<IpAddr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Decode a textual LLDP TLV payload.
///
/// Lossy, so a single bad byte degrades one character rather than discarding the whole neighbour,
/// and NUL-stripped, because these strings land in the `lldp_chassis_id` / `lldp_port_id` JSONB
/// columns and PostgreSQL rejects the escape outright (SQLSTATE 22P05). D-Link's DGS series
/// NUL-terminates its port identifiers — `31 00` for port `"1"` — which used to fail the write of
/// the whole host (GH #668). Stripping here also makes the value comparable: `"1\0"` matches no
/// interface named `"1"`.
fn decode_tlv_text(value: &[u8]) -> String {
    strip_nuls(&String::from_utf8_lossy(value)).into_owned()
}

impl LldpChassisId {
    /// Parse from SNMP raw values (subtype byte + value bytes).
    ///
    /// LLDP chassis ID TLV format: subtype (1 byte) + value (variable)
    pub fn from_snmp(subtype: u8, value: &[u8]) -> Option<Self> {
        match subtype {
            1 => Some(Self::ChassisComponent(decode_tlv_text(value))),
            2 => Some(Self::InterfaceAlias(decode_tlv_text(value))),
            3 => Some(Self::PortComponent(decode_tlv_text(value))),
            4 => parse_mac_id(value).map(Self::MacAddress),
            5 => parse_network_address(value).map(Self::NetworkAddress),
            6 => Some(Self::InterfaceName(decode_tlv_text(value))),
            7 => Some(Self::LocallyAssigned(decode_tlv_text(value))),
            _ => None,
        }
    }

    /// Build a chassis ID from an identifier string, for sources that report LLDP neighbor data
    /// without the IEEE subtype byte.
    ///
    /// The UniFi controller API is the motivating case: `lldp_table[].chassis_id` is already a
    /// decoded string, so there is no subtype to dispatch on. MAC-shaped values canonicalize to
    /// the same lowercase colon form [`Self::from_snmp`] produces — which is what lets the
    /// raw-string `hosts.chassis_id` fallback in [`Self::resolve_host_id`] match a device
    /// regardless of whether SNMP or a controller recorded it. Anything else is
    /// [`Self::LocallyAssigned`], the honest label for "device-specific identifier, subtype
    /// unknown", and the one whose resolution assumes least about the value.
    pub fn from_identifier_str(value: &str) -> Self {
        match canonical_mac(value) {
            Some(mac) => Self::MacAddress(mac),
            None => Self::LocallyAssigned(value.to_string()),
        }
    }

    /// The identifier as the remote device advertised it, in the same canonical form the daemon
    /// stores on `hosts.chassis_id` for a device's own LLDP local identity. Having one definition
    /// is what lets a neighbor's chassis ID be matched against a scanned host's own chassis ID.
    pub fn identifier(&self) -> String {
        match self {
            Self::NetworkAddress(addr) => addr.to_string(),
            Self::ChassisComponent(s)
            | Self::InterfaceAlias(s)
            | Self::PortComponent(s)
            | Self::MacAddress(s)
            | Self::InterfaceName(s)
            | Self::LocallyAssigned(s) => s.clone(),
        }
    }

    /// Resolve this chassis ID to a host_id, trying each applicable strategy in turn.
    ///
    /// Subtype-specific strategy first — it matches the identifier against the kind of column it
    /// actually is:
    /// - MacAddress: `ip_addresses.mac_address`, then `interfaces.mac_address`
    /// - NetworkAddress: `ip_addresses.ip_address`
    /// - InterfaceName: `interfaces.if_descr`
    /// - ChassisComponent/LocallyAssigned: `hosts.chassis_id`
    /// - InterfaceAlias/PortComponent: nothing reliable
    ///
    /// Then two subtype-independent fallbacks, both needed on real hardware:
    ///
    /// 1. `hosts.chassis_id`, for *every* subtype. A switch's chassis MAC is not required to be
    ///    any of its port MACs, and on several vendors it isn't: the Netgear GS724Tv3 advertises a
    ///    chassis MAC one octet below its port MACs and carries it on no interface and no IP, so
    ///    the MacAddress strategy above finds nothing (GH #664). The daemon already records each
    ///    scanned device's own LLDP chassis ID on `hosts.chassis_id` in this same canonical form,
    ///    which is the only place that MAC exists server-side.
    /// 2. The neighbor's advertised `sysName` against `hosts.sys_name` — the same last resort CDP
    ///    has always used, for devices whose chassis identity is unrecoverable but whose name was
    ///    captured by an SNMP scan. Only accepted when exactly one host matches.
    pub async fn resolve_host_id<R: LldpResolver>(
        &self,
        resolver: &R,
        network_id: Uuid,
        sys_name: Option<&str>,
    ) -> IdentityResolution {
        let mut strategy_ran = false;

        let by_subtype = match self {
            Self::MacAddress(mac) => resolver.find_host_by_mac(mac, network_id).await,
            Self::NetworkAddress(ip) => resolver.find_host_by_ip(ip, network_id).await,
            Self::InterfaceName(name) => resolver.find_host_by_if_name(name, network_id).await,
            Self::ChassisComponent(id) | Self::LocallyAssigned(id) => {
                resolver.find_host_by_chassis_id(id, network_id).await
            }
            // These subtypes don't have reliable resolution strategies
            Self::InterfaceAlias(_) | Self::PortComponent(_) => None,
        };
        strategy_ran |= !matches!(self, Self::InterfaceAlias(_) | Self::PortComponent(_));
        if by_subtype.is_some() {
            return IdentityResolution::found(by_subtype);
        }

        let identifier = self.identifier();
        if !identifier.is_empty() {
            strategy_ran = true;
            if let Some(host_id) = resolver
                .find_host_by_chassis_id(&identifier, network_id)
                .await
            {
                return IdentityResolution::Resolved(host_id);
            }
        }

        if let Some(sys_name) = sys_name.map(str::trim).filter(|s| !s.is_empty()) {
            strategy_ran = true;
            if let Some(host_id) = resolver.find_host_by_sys_name(sys_name, network_id).await {
                return IdentityResolution::Resolved(host_id);
            }
        }

        if strategy_ran {
            IdentityResolution::NotFound
        } else {
            IdentityResolution::NoStrategy
        }
    }
}

impl LldpPortId {
    /// Parse from SNMP raw values (subtype byte + value bytes).
    ///
    /// LLDP port ID TLV format: subtype (1 byte) + value (variable)
    pub fn from_snmp(subtype: u8, value: &[u8]) -> Option<Self> {
        match subtype {
            1 => Some(Self::InterfaceAlias(decode_tlv_text(value))),
            2 => Some(Self::PortComponent(decode_tlv_text(value))),
            3 => parse_mac_id(value).map(Self::MacAddress),
            4 => parse_network_address(value).map(Self::NetworkAddress),
            5 => Some(Self::InterfaceName(decode_tlv_text(value))),
            6 => Some(Self::AgentCircuitId(decode_tlv_text(value))),
            7 => Some(Self::LocallyAssigned(decode_tlv_text(value))),
            _ => None,
        }
    }

    /// Build a port ID from an identifier string, for sources that report LLDP neighbor data
    /// without the IEEE subtype byte. See [`LldpChassisId::from_identifier_str`] — same rationale.
    pub fn from_identifier_str(value: &str) -> Self {
        match canonical_mac(value) {
            Some(mac) => Self::MacAddress(mac),
            None => Self::LocallyAssigned(value.to_string()),
        }
    }

    /// Resolve this port ID to an interface_id using the appropriate lookup strategy.
    ///
    /// Requires the host_id to be already known (from chassis ID resolution), so every lookup is
    /// scoped to one device's own interfaces.
    ///
    /// The resolution strategy depends on the port ID subtype:
    /// - MacAddress: Look up via interfaces.mac_address
    /// - NetworkAddress: Look up via ip_address_id FK on interfaces
    /// - InterfaceName/PortComponent/AgentCircuitId/LocallyAssigned: device-local port identifier
    ///   — see [`Self::resolve_device_local_port`]
    /// - InterfaceAlias: user-configurable and non-unique, no reliable resolution
    pub async fn resolve_if_entry_id<R: LldpResolver>(
        &self,
        resolver: &R,
        host_id: Uuid,
    ) -> IdentityResolution {
        match self {
            Self::MacAddress(mac) => {
                IdentityResolution::found(resolver.find_if_entry_by_mac(mac, host_id).await)
            }
            Self::InterfaceAlias(_) => IdentityResolution::NoStrategy, // user-configurable, non-unique
            Self::NetworkAddress(ip) => {
                IdentityResolution::found(resolver.find_if_entry_by_ip(ip, host_id).await)
            }
            Self::InterfaceName(id)
            | Self::PortComponent(id)
            | Self::AgentCircuitId(id)
            | Self::LocallyAssigned(id) => {
                Self::resolve_device_local_port(resolver, id, host_id).await
            }
        }
    }

    /// Resolve a device-local port identifier (subtypes 2, 5, 6 and 7) against one host's
    /// interfaces.
    ///
    /// These subtypes are "whatever the device calls this port", which sounds unusable but in
    /// practice is one of two things, both of which the remote host's own ifTable already carries:
    /// the port's `ifDescr`/`ifName` (Aruba/HP ProCurve advertise bare port numbers such as `"41"`,
    /// which is exactly that switch's `ifDescr`), or its `ifIndex` as a decimal string.
    ///
    /// Treating the whole family as unresolvable is what produced the reported symptom: the
    /// neighbor resolved as far as the host and stopped, and a host-only neighbor renders no edge,
    /// so switches were absent from L2 Physical entirely (GH #649). Names are tried before indexes
    /// because a bare port number is ambiguous between the two and the name is the device's own
    /// label; both are scoped to the single already-resolved host, so a wrong match cannot reach
    /// another device.
    ///
    /// Subtype 5 (`interfaceName`) is routed here too, despite the name promising a real ifName.
    /// A D-Link DGS-1210-48 advertises subtype 5 with the bare port number `"2"` while its own
    /// interfaces are `Slot0/1..Slot0/48` — the value is an ifIndex wearing the wrong label, and
    /// name-only lookup could never match it (GH #668). This ladder is a strict superset of what
    /// subtype 5 did before: names are still tried first, so a device that means what it says is
    /// unaffected, and the index fallback is bounded to the one host already resolved.
    async fn resolve_device_local_port<R: LldpResolver>(
        resolver: &R,
        id: &str,
        host_id: Uuid,
    ) -> IdentityResolution {
        let id = id.trim();
        if id.is_empty() {
            return IdentityResolution::NoStrategy;
        }

        if let Some(interface_id) = resolver.find_if_entry_by_name(id, host_id).await {
            return IdentityResolution::Resolved(interface_id);
        }

        match id.parse::<i32>() {
            Ok(if_index) => IdentityResolution::found(
                resolver.find_if_entry_by_if_index(if_index, host_id).await,
            ),
            Err(_) => IdentityResolution::NotFound,
        }
    }
}

/// Parse an LLDP MAC-address identifier (chassis subtype 4 / port subtype 3).
///
/// Per IEEE 802.1AB a macAddress value is 6 raw octets, but some vendors
/// (MikroTik RouterOS, Extreme EXOS) instead send it as an ASCII string such as
/// `"48:A9:8A:BD:B4:7D"`. Accept both shapes and normalize to the same canonical
/// lowercase colon-separated form (`format_mac`) so downstream MAC matching is
/// independent of the wire encoding. Returns `None` for values that are neither.
///
/// The ASCII form is accepted with or without zero-padded octets: firmware that renders a MAC as
/// a string rather than emitting octets is formatting it itself, and `%x` ("0:1a:2b:0:10:0" —
/// also net-snmp's own display format) is as likely as `%02x`. Being strict here is not a
/// partial loss but a total one: an unparseable chassis ID makes `from_snmp` return `None`, the
/// neighbor record is never stored, and the device silently contributes nothing to L2 topology
/// at all — indistinguishable from a switch that advertises no neighbors.
fn parse_mac_id(value: &[u8]) -> Option<String> {
    if value.len() == 6 {
        return Some(format_mac(value));
    }

    // Vendor quirk: MAC encoded as an ASCII string instead of 6 raw octets.
    canonical_mac(std::str::from_utf8(value).ok()?)
}

/// Canonicalize a MAC rendered as text into the same lowercase colon-separated form
/// [`format_mac`] produces, or `None` if the value is not a MAC.
///
/// This is the text-only half of [`parse_mac_id`], split out because sources that report
/// identifiers as strings rather than raw TLV octets must not go through the 6-raw-octet
/// branch: a six-character name such as `"Switch"` is six bytes, and would otherwise be
/// silently reinterpreted as the MAC `53:77:69:74:63:68`.
///
/// Accepts the form with or without zero-padded octets: firmware that renders a MAC as a
/// string is formatting it itself, and `%x` ("0:1a:2b:0:10:0" — also net-snmp's own display
/// format) is as likely as `%02x`.
pub fn canonical_mac(value: &str) -> Option<String> {
    let s = value.trim();
    if let Ok(mac) = s.parse::<mac_address::MacAddress>() {
        return Some(format_mac(&mac.bytes()));
    }

    // Same string form, octets not zero-padded. Six colon-separated groups of one or two hex
    // digits is a MAC under any reading, and this is only reached once the padded parse has
    // failed, so there is nothing else it could be mistaken for.
    let octets: Vec<u8> = s
        .split(':')
        .map(|group| match group.len() {
            1 | 2 => u8::from_str_radix(group, 16).ok(),
            _ => None,
        })
        .collect::<Option<Vec<u8>>>()?;
    (octets.len() == 6).then(|| format_mac(&octets))
}

/// Format MAC address bytes as colon-separated hex string.
fn format_mac(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(":")
}

/// Parse LLDP network address format.
///
/// LLDP network address format: address family (1 byte) + address bytes
/// - Family 1: IPv4 (4 bytes)
/// - Family 2: IPv6 (16 bytes)
fn parse_network_address(value: &[u8]) -> Option<IpAddr> {
    if value.is_empty() {
        return None;
    }
    let addr_family = value[0];
    let addr_bytes = &value[1..];
    match addr_family {
        1 if addr_bytes.len() == 4 => Some(IpAddr::V4(std::net::Ipv4Addr::new(
            addr_bytes[0],
            addr_bytes[1],
            addr_bytes[2],
            addr_bytes[3],
        ))),
        2 if addr_bytes.len() == 16 => {
            let arr: [u8; 16] = addr_bytes.try_into().ok()?;
            Some(IpAddr::V6(std::net::Ipv6Addr::from(arr)))
        }
        _ => None,
    }
}

// Re-export LldpResolver trait from resolver module for backward compatibility
pub use super::resolver::LldpResolver;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chassis_id_from_snmp_mac() {
        let mac_bytes = [0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e];
        let chassis_id = LldpChassisId::from_snmp(4, &mac_bytes);
        assert_eq!(
            chassis_id,
            Some(LldpChassisId::MacAddress("00:1a:2b:3c:4d:5e".to_string()))
        );
    }

    #[test]
    fn test_chassis_id_from_snmp_mac_ascii_string() {
        // MikroTik/Extreme quirk: subtype 4 sent as 17-byte ASCII "48:A9:8A:BD:B4:7D"
        let ascii = b"48:A9:8A:BD:B4:7D";
        let chassis_id = LldpChassisId::from_snmp(4, ascii);
        assert_eq!(
            chassis_id,
            Some(LldpChassisId::MacAddress("48:a9:8a:bd:b4:7d".to_string()))
        );
    }

    /// Firmware that renders a MAC as a string rather than emitting octets formats it itself, and
    /// abbreviated octets are as likely as padded ones. Rejecting the abbreviated form does not
    /// degrade the record — it discards the whole neighbor, so the device contributes nothing to
    /// L2 topology and looks identical to one advertising no neighbors at all.
    #[test]
    fn test_chassis_id_from_snmp_mac_ascii_unpadded() {
        let ascii = b"0:1a:2b:0:10:0";
        assert_eq!(
            LldpChassisId::from_snmp(4, ascii),
            Some(LldpChassisId::MacAddress("00:1a:2b:00:10:00".to_string())),
            "an unpadded ASCII MAC must normalize to the same canonical form as a padded one"
        );
    }

    /// Both wire encodings of one address must land on the same string, or a neighbor advertising
    /// the abbreviated form would never match the host that reported the padded form.
    #[test]
    fn test_mac_encodings_agree_on_one_canonical_form() {
        let raw = LldpChassisId::from_snmp(4, &[0x00, 0x1a, 0x2b, 0x00, 0x10, 0x00]);
        let padded = LldpChassisId::from_snmp(4, b"00:1a:2b:00:10:00");
        let unpadded = LldpChassisId::from_snmp(4, b"0:1a:2b:0:10:0");
        assert_eq!(raw, padded);
        assert_eq!(raw, unpadded);
    }

    /// The whole point of `from_identifier_str`: a controller-sourced neighbor and an
    /// SNMP-sourced one must produce byte-identical chassis IDs, because `resolve_host_id`
    /// falls back to raw string equality against `hosts.chassis_id`.
    #[test]
    fn test_identifier_str_agrees_with_snmp_canonical_form() {
        let from_snmp = LldpChassisId::from_snmp(4, &[0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]);
        assert_eq!(
            LldpChassisId::from_identifier_str("0:1A:2b:3C:4d:5E"),
            from_snmp.unwrap()
        );
        assert_eq!(
            LldpPortId::from_identifier_str("00:1A:2B:3C:4D:5E"),
            LldpPortId::from_snmp(3, &[0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e]).unwrap()
        );
    }

    /// A string source has no subtype byte, so it must never take the six-raw-octets branch:
    /// plenty of device names are exactly six characters, and reading one as a MAC would
    /// invent a neighbor that does not exist.
    #[test]
    fn test_identifier_str_does_not_read_short_names_as_macs() {
        for name in ["Switch", "ap-101", "core01"] {
            assert_eq!(
                LldpChassisId::from_identifier_str(name),
                LldpChassisId::LocallyAssigned(name.to_string()),
                "expected {name:?} to stay an opaque identifier"
            );
        }
    }

    #[test]
    fn test_identifier_str_non_mac_is_locally_assigned() {
        assert_eq!(
            LldpChassisId::from_identifier_str("edge-switch.lan"),
            LldpChassisId::LocallyAssigned("edge-switch.lan".to_string())
        );
        assert_eq!(
            LldpPortId::from_identifier_str("Port 7"),
            LldpPortId::LocallyAssigned("Port 7".to_string())
        );
    }

    #[test]
    fn test_port_id_from_snmp_mac_ascii_unpadded() {
        assert_eq!(
            LldpPortId::from_snmp(3, b"0:c:29:aa:bb:c0"),
            Some(LldpPortId::MacAddress("00:0c:29:aa:bb:c0".to_string()))
        );
    }

    /// The tolerance must not swallow things that merely look colon-separated. (A value of exactly
    /// six bytes is deliberately absent: the spec defines subtype 4 as six binary octets, so six
    /// bytes are octets by definition and no heuristic can second-guess that.)
    #[test]
    fn test_chassis_id_from_snmp_mac_rejects_non_macs() {
        for not_a_mac in [
            &b"0:1a:2b:0:10"[..],     // five groups
            &b"0:1a:2b:0:10:0:5"[..], // seven groups
            &b"0:1a:2b:0:10:zz"[..],  // non-hex group
            &b"0:1a:2b:0:10:000"[..], // over-long group
            &b"not-a-mac-at-all"[..],
        ] {
            assert_eq!(
                LldpChassisId::from_snmp(4, not_a_mac),
                None,
                "expected {:?} to be rejected",
                String::from_utf8_lossy(not_a_mac)
            );
        }
    }

    #[test]
    fn test_chassis_id_from_snmp_mac_invalid() {
        // A non-MAC, non-6-byte value for subtype 4 is rejected.
        let chassis_id = LldpChassisId::from_snmp(4, b"not-a-mac");
        assert_eq!(chassis_id, None);
    }

    #[test]
    fn test_port_id_from_snmp_mac_raw_octets() {
        let mac_bytes = [0x00, 0x1a, 0x2b, 0x3c, 0x4d, 0x5e];
        let port_id = LldpPortId::from_snmp(3, &mac_bytes);
        assert_eq!(
            port_id,
            Some(LldpPortId::MacAddress("00:1a:2b:3c:4d:5e".to_string()))
        );
    }

    #[test]
    fn test_port_id_from_snmp_mac_ascii_string() {
        let ascii = b"48:A9:8A:BD:B4:7D";
        let port_id = LldpPortId::from_snmp(3, ascii);
        assert_eq!(
            port_id,
            Some(LldpPortId::MacAddress("48:a9:8a:bd:b4:7d".to_string()))
        );
    }

    #[test]
    fn test_chassis_id_from_snmp_locally_assigned() {
        let id_bytes = b"switch-1";
        let chassis_id = LldpChassisId::from_snmp(7, id_bytes);
        assert_eq!(
            chassis_id,
            Some(LldpChassisId::LocallyAssigned("switch-1".to_string()))
        );
    }

    #[test]
    fn test_chassis_id_from_snmp_ipv4() {
        // Family 1 (IPv4) + 192.168.1.1
        let addr_bytes = [1, 192, 168, 1, 1];
        let chassis_id = LldpChassisId::from_snmp(5, &addr_bytes);
        assert_eq!(
            chassis_id,
            Some(LldpChassisId::NetworkAddress(IpAddr::V4(
                std::net::Ipv4Addr::new(192, 168, 1, 1)
            )))
        );
    }

    #[test]
    fn test_port_id_from_snmp_interface_name() {
        let name_bytes = b"GigabitEthernet0/1";
        let port_id = LldpPortId::from_snmp(5, name_bytes);
        assert_eq!(
            port_id,
            Some(LldpPortId::InterfaceName("GigabitEthernet0/1".to_string()))
        );
    }

    #[test]
    fn test_chassis_id_serialization() {
        let chassis_id = LldpChassisId::MacAddress("00:1a:2b:3c:4d:5e".to_string());
        let json = serde_json::to_string(&chassis_id).unwrap();
        assert_eq!(
            json,
            r#"{"subtype":"MacAddress","value":"00:1a:2b:3c:4d:5e"}"#
        );

        let deserialized: LldpChassisId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, chassis_id);
    }
}

/// Resolution-strategy tests against an in-memory inventory.
///
/// These exercise which identity a neighbor is joined on, which is where the reported "L2
/// Physical is empty" failures live — a neighbor that resolves only as far as the remote host
/// draws no edge at all, so a strategy gap and a genuinely unknown device look identical from
/// the outside.
#[cfg(test)]
mod resolution_tests {
    use super::*;
    use async_trait::async_trait;
    use std::net::IpAddr;

    #[derive(Default)]
    struct FakeHost {
        id: Uuid,
        chassis_id: Option<String>,
        sys_name: Option<String>,
    }

    #[derive(Default)]
    struct FakeInterface {
        id: Uuid,
        host_id: Uuid,
        if_descr: Option<String>,
        if_name: Option<String>,
        if_index: i32,
        mac: Option<String>,
    }

    /// Stands in for the database: the same inventory the production resolver queries, matched
    /// with the same column semantics (exact match, and "exactly one" for the non-unique columns).
    #[derive(Default)]
    struct FakeInventory {
        hosts: Vec<FakeHost>,
        interfaces: Vec<FakeInterface>,
    }

    impl FakeInventory {
        fn only<T>(mut matches: Vec<T>) -> Option<T> {
            match matches.len() {
                1 => matches.pop(),
                _ => None,
            }
        }
    }

    #[async_trait]
    impl LldpResolver for FakeInventory {
        async fn find_host_by_mac(&self, mac: &str, _network_id: Uuid) -> Option<Uuid> {
            self.interfaces
                .iter()
                .find(|i| i.mac.as_deref() == Some(mac))
                .map(|i| i.host_id)
        }

        async fn find_host_by_ip(&self, _ip: &IpAddr, _network_id: Uuid) -> Option<Uuid> {
            None
        }

        async fn find_host_by_if_name(&self, name: &str, _network_id: Uuid) -> Option<Uuid> {
            self.interfaces
                .iter()
                .find(|i| i.if_descr.as_deref() == Some(name))
                .map(|i| i.host_id)
        }

        async fn find_host_by_chassis_id(
            &self,
            chassis_id: &str,
            _network_id: Uuid,
        ) -> Option<Uuid> {
            Self::only(
                self.hosts
                    .iter()
                    .filter(|h| h.chassis_id.as_deref() == Some(chassis_id))
                    .collect(),
            )
            .map(|h| h.id)
        }

        async fn find_host_by_sys_name(&self, sys_name: &str, _network_id: Uuid) -> Option<Uuid> {
            Self::only(
                self.hosts
                    .iter()
                    .filter(|h| h.sys_name.as_deref() == Some(sys_name))
                    .collect(),
            )
            .map(|h| h.id)
        }

        async fn find_if_entry_by_mac(&self, mac: &str, host_id: Uuid) -> Option<Uuid> {
            self.interfaces
                .iter()
                .find(|i| i.host_id == host_id && i.mac.as_deref() == Some(mac))
                .map(|i| i.id)
        }

        async fn find_if_entry_by_name(&self, name: &str, host_id: Uuid) -> Option<Uuid> {
            self.interfaces
                .iter()
                .find(|i| {
                    i.host_id == host_id
                        && (i.if_descr.as_deref() == Some(name)
                            || i.if_name.as_deref() == Some(name))
                })
                .map(|i| i.id)
        }

        async fn find_if_entry_by_if_index(&self, if_index: i32, host_id: Uuid) -> Option<Uuid> {
            self.interfaces
                .iter()
                .find(|i| i.host_id == host_id && i.if_index == if_index)
                .map(|i| i.id)
        }

        async fn find_if_entry_by_ip(&self, _ip: &IpAddr, _host_id: Uuid) -> Option<Uuid> {
            None
        }
    }

    /// GH #664: the Netgear GS724Tv3 advertises a chassis MAC that is on none of its ports and
    /// on no IP — the only server-side record of it is the host's own `chassis_id`, captured
    /// from its LLDP local identity. Matching MACs against interfaces/IPs alone leaves every
    /// neighbour of such a switch unresolved and L2 Physical empty.
    #[tokio::test]
    async fn chassis_mac_absent_from_ports_resolves_via_the_hosts_own_chassis_id() {
        let switch = Uuid::new_v4();
        let inventory = FakeInventory {
            hosts: vec![FakeHost {
                id: switch,
                chassis_id: Some("00:1a:2b:3c:4d:63".to_string()),
                ..Default::default()
            }],
            // Ports carry a different MAC than the chassis — the whole point of the bug.
            interfaces: vec![FakeInterface {
                id: Uuid::new_v4(),
                host_id: switch,
                mac: Some("00:1a:2b:3c:4d:65".to_string()),
                ..Default::default()
            }],
        };

        let chassis = LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".to_string());
        assert_eq!(
            chassis
                .resolve_host_id(&inventory, Uuid::new_v4(), None)
                .await,
            IdentityResolution::Resolved(switch)
        );
    }

    #[tokio::test]
    async fn neighbour_on_a_device_this_network_never_scanned_is_not_found() {
        let inventory = FakeInventory::default();
        let chassis = LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".to_string());

        assert_eq!(
            chassis
                .resolve_host_id(&inventory, Uuid::new_v4(), Some("some-ap"))
                .await,
            IdentityResolution::NotFound
        );
    }

    /// sysName is operator-assigned and often left at a vendor default, so two devices sharing
    /// one is a real configuration. Attaching a physical link to an arbitrary one of them is
    /// worse than reporting the neighbour unresolved.
    #[tokio::test]
    async fn a_sys_name_shared_by_two_hosts_resolves_to_neither() {
        let inventory = FakeInventory {
            hosts: vec![
                FakeHost {
                    id: Uuid::new_v4(),
                    sys_name: Some("switch".to_string()),
                    ..Default::default()
                },
                FakeHost {
                    id: Uuid::new_v4(),
                    sys_name: Some("switch".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let chassis = LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".to_string());
        assert_eq!(
            chassis
                .resolve_host_id(&inventory, Uuid::new_v4(), Some("switch"))
                .await,
            IdentityResolution::NotFound
        );
    }

    /// GH #649: Aruba/HP switches advertise the remote port as subtype 7 (locally assigned)
    /// carrying the port's own label — which is exactly that device's ifDescr. Treating the
    /// subtype as unresolvable stopped resolution at the host, and a host-only neighbour renders
    /// no edge, so whole switches were missing from L2 Physical.
    #[tokio::test]
    async fn locally_assigned_port_id_resolves_against_the_remote_ports_own_label() {
        let switch = Uuid::new_v4();
        let port_41 = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![
                FakeInterface {
                    id: port_41,
                    host_id: switch,
                    if_descr: Some("41".to_string()),
                    if_index: 41,
                    ..Default::default()
                },
                FakeInterface {
                    id: Uuid::new_v4(),
                    host_id: switch,
                    if_descr: Some("42".to_string()),
                    if_index: 42,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let port = LldpPortId::LocallyAssigned("41".to_string());
        assert_eq!(
            port.resolve_if_entry_id(&inventory, switch).await,
            IdentityResolution::Resolved(port_41)
        );
    }

    /// GH #668: a D-Link DGS-1210-48 sets port-id subtype 5 (`interfaceName`) but sends the bare
    /// port number, while its own interfaces are `Slot0/1..Slot0/48` with ifIndex == port number.
    /// Name-only lookup — which is all subtype 5 used to get — can never match that, so the
    /// neighbour stopped at the host and the switch drew no L2 edge.
    #[tokio::test]
    async fn interface_name_port_id_falls_back_to_the_remote_if_index() {
        let switch = Uuid::new_v4();
        let port_9 = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: port_9,
                host_id: switch,
                if_name: Some("Slot0/9".to_string()),
                if_descr: Some("D-Link DGS-1210-48 Rev.GX/7.20.003 Port 9".to_string()),
                if_index: 9,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::InterfaceName("9".to_string());
        assert_eq!(
            port.resolve_if_entry_id(&inventory, switch).await,
            IdentityResolution::Resolved(port_9)
        );
    }

    /// The fallback must not cost subtype 5 its literal reading: a device that means `interfaceName`
    /// still resolves by name, and the name is tried before the index.
    #[tokio::test]
    async fn interface_name_port_id_still_prefers_a_real_name() {
        let switch = Uuid::new_v4();
        let named = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![
                FakeInterface {
                    id: named,
                    host_id: switch,
                    if_name: Some("2".to_string()),
                    if_index: 40,
                    ..Default::default()
                },
                // Would win if the index were consulted first.
                FakeInterface {
                    id: Uuid::new_v4(),
                    host_id: switch,
                    if_name: Some("Gi0/2".to_string()),
                    if_index: 2,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let port = LldpPortId::InterfaceName("2".to_string());
        assert_eq!(
            port.resolve_if_entry_id(&inventory, switch).await,
            IdentityResolution::Resolved(named)
        );
    }

    /// GH #668: D-Link NUL-terminates its port identifiers, so `lldpRemPortId` arrives as
    /// `31 00`. The byte is valid UTF-8, so nothing rejected it — it reached `jsonb`, which
    /// cannot store the escape, and would never have matched an interface named "1" anyway.
    #[test]
    fn tlv_text_decoding_strips_nul_padding() {
        assert_eq!(
            LldpPortId::from_snmp(5, b"1\0"),
            Some(LldpPortId::InterfaceName("1".to_string()))
        );
        assert_eq!(
            LldpPortId::from_snmp(7, b"eth1/0/51\0\0"),
            Some(LldpPortId::LocallyAssigned("eth1/0/51".to_string()))
        );
        assert_eq!(
            LldpChassisId::from_snmp(7, b"switch4\0"),
            Some(LldpChassisId::LocallyAssigned("switch4".to_string()))
        );
    }

    /// The other shape of a locally-assigned port id: the remote device's ifIndex, on a switch
    /// whose port labels are something else entirely (HP A-series reports "A1".."A24" as ifDescr
    /// while advertising the index).
    #[tokio::test]
    async fn locally_assigned_port_id_falls_back_to_the_remote_if_index() {
        let switch = Uuid::new_v4();
        let uplink = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: uplink,
                host_id: switch,
                if_descr: Some("A5".to_string()),
                if_index: 197,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::LocallyAssigned("197".to_string());
        assert_eq!(
            port.resolve_if_entry_id(&inventory, switch).await,
            IdentityResolution::Resolved(uplink)
        );
    }

    #[tokio::test]
    async fn a_locally_assigned_port_id_matching_nothing_stays_unresolved() {
        let switch = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: Uuid::new_v4(),
                host_id: switch,
                if_descr: Some("1".to_string()),
                if_index: 1,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::LocallyAssigned("WAN PORT".to_string());
        assert_eq!(
            port.resolve_if_entry_id(&inventory, switch).await,
            IdentityResolution::NotFound
        );
    }

    /// A port id is only ever resolved against the host it was already attributed to, so an
    /// identifier that happens to collide with another device's port cannot cross the boundary.
    #[tokio::test]
    async fn port_resolution_cannot_reach_an_interface_on_another_host() {
        let switch = Uuid::new_v4();
        let other_switch = Uuid::new_v4();
        let inventory = FakeInventory {
            interfaces: vec![FakeInterface {
                id: Uuid::new_v4(),
                host_id: other_switch,
                if_descr: Some("41".to_string()),
                if_index: 41,
                ..Default::default()
            }],
            ..Default::default()
        };

        let port = LldpPortId::LocallyAssigned("41".to_string());
        assert_eq!(
            port.resolve_if_entry_id(&inventory, switch).await,
            IdentityResolution::NotFound
        );
    }

    /// An ifAlias is an operator-typed description, non-unique and frequently empty — there is
    /// nothing to look it up against, which is a different situation from "looked and found
    /// nothing" and is reported as such.
    #[tokio::test]
    async fn an_alias_port_id_reports_that_no_strategy_applies() {
        let inventory = FakeInventory::default();
        let port = LldpPortId::InterfaceAlias("uplink to core".to_string());

        assert_eq!(
            port.resolve_if_entry_id(&inventory, Uuid::new_v4()).await,
            IdentityResolution::NoStrategy
        );
    }
}
