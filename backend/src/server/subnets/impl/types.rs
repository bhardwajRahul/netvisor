use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::shared::{
    concepts::Concept,
    entities::EntityDiscriminants,
    types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumDiscriminants,
    EnumIter,
    IntoStaticStr,
    Default,
    ToSchema,
)]
#[strum_discriminants(derive(Display, Hash, Serialize, Deserialize, EnumIter))]
pub enum SubnetType {
    Internet,
    Remote,

    Gateway,
    VpnTunnel,
    Dmz,

    Lan,
    WiFi,
    IoT,
    Guest,

    DockerBridge,
    PodmanBridge,
    MacVlan,
    IpVlan,
    Management,
    Storage,
    Loopback,

    // `other` makes any variant a newer server emits that this build doesn't
    // know (the production `unknown variant 'Loopback'` failure) degrade to
    // `Unknown` instead of hard-erroring. Subsumes the former `alias = "None"`.
    #[default]
    #[serde(other)]
    Unknown,
}

impl FromStr for SubnetType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Internet" => Ok(SubnetType::Internet),
            "Remote" => Ok(SubnetType::Remote),
            "Gateway" => Ok(SubnetType::Gateway),
            "VpnTunnel" => Ok(SubnetType::VpnTunnel),
            "Dmz" => Ok(SubnetType::Dmz),
            "Lan" => Ok(SubnetType::Lan),
            "WiFi" => Ok(SubnetType::WiFi),
            "IoT" => Ok(SubnetType::IoT),
            "Guest" => Ok(SubnetType::Guest),
            "DockerBridge" => Ok(SubnetType::DockerBridge),
            "PodmanBridge" => Ok(SubnetType::PodmanBridge),
            "MacVlan" => Ok(SubnetType::MacVlan),
            "IpVlan" => Ok(SubnetType::IpVlan),
            "Management" => Ok(SubnetType::Management),
            "Storage" => Ok(SubnetType::Storage),
            "Loopback" => Ok(SubnetType::Loopback),
            "Unknown" | "None" => Ok(SubnetType::Unknown),
            _ => Err(anyhow::anyhow!("Unknown SubnetType: {}", s)),
        }
    }
}

impl SubnetType {
    /// Whether this subnet type represents a container-runtime network
    /// (Docker/Podman bridge, MacVLAN, IpVLAN).
    pub fn is_container_network(&self) -> bool {
        matches!(
            self,
            SubnetType::DockerBridge
                | SubnetType::PodmanBridge
                | SubnetType::MacVlan
                | SubnetType::IpVlan
        )
    }

    pub fn from_interface_name(interface_name: &str) -> Self {
        // Loopback ip_addresses (lo on Linux, lo0 on macOS)
        if Self::match_interface_names(&["lo"], interface_name) {
            return SubnetType::Loopback;
        }

        // Docker containers
        if Self::match_interface_names(&["docker", "br-", "docker"], interface_name) {
            return SubnetType::DockerBridge;
        }

        // Podman containers (default bridge is `podman0`, CNI uses `cni-podman0`)
        if Self::match_interface_names(&["podman", "cni-podman"], interface_name) {
            return SubnetType::PodmanBridge;
        }

        // VPN tunnels
        if Self::match_interface_names(&["tun", "utun", "wg", "tap", "ppp", "vpn"], interface_name)
        {
            return SubnetType::VpnTunnel;
        }

        // WiFi ip_addresses
        if Self::match_interface_names(&["wlan", "wifi", "wl"], interface_name) {
            return SubnetType::WiFi;
        }

        // Guest network (often labeled explicitly)
        if Self::match_interface_names(&["guest"], interface_name) {
            return SubnetType::Guest;
        }

        // IoT network (some routers use this naming)
        if Self::match_interface_names(&["iot"], interface_name) {
            return SubnetType::IoT;
        }

        // DMZ (often labeled explicitly)
        if Self::match_interface_names(&["dmz"], interface_name) {
            return SubnetType::Dmz;
        }

        // Management ip_addresses
        if Self::match_interface_names(&["mgmt", "ipmi", "bmc"], interface_name) {
            return SubnetType::Management;
        }

        // Storage networks
        if Self::match_interface_names(&["iscsi", "san", "storage"], interface_name) {
            return SubnetType::Storage;
        }

        // MacVLAN ip_addresses
        if Self::match_interface_names(&["macvlan", "mvlan"], interface_name) {
            return SubnetType::MacVlan;
        }

        // ipvlan ip_addresses
        if Self::match_interface_names(&["ipvlan"], interface_name) {
            return SubnetType::IpVlan;
        }

        // Standard LAN ip_addresses (catch-all for ethernet and Linux bridges)
        // Note: "br" (e.g., br0) is a Linux bridge, commonly used on Unraid/Proxmox for LAN
        // This is distinct from "br-" which is Docker's bridge naming convention
        if Self::match_interface_names(&["eth", "en", "eno", "enp", "ens", "br"], interface_name) {
            return SubnetType::Lan;
        }

        SubnetType::Unknown
    }

    fn match_interface_names(patterns: &[&str], interface_name: &str) -> bool {
        let name_lower = interface_name.to_lowercase();
        patterns.iter().any(|pattern| {
            if *pattern == "br-" || *pattern == "docker-" {
                // Special case for Docker bridges: br- or docker- followed by hex chars
                name_lower.starts_with(pattern)
                    && name_lower
                        .get(pattern.len()..)
                        .map(|rest| {
                            !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric())
                        })
                        .unwrap_or(false)
            } else {
                // Original logic for other patterns
                name_lower.starts_with(pattern)
                    && name_lower
                        .get(pattern.len()..)
                        .map(|rest| {
                            rest.is_empty()
                                || rest.chars().next().unwrap_or_default().is_ascii_digit()
                        })
                        .unwrap_or(false)
            }
        })
    }

    /// Whether this subnet is a container-runtime bridge network
    /// (Docker or Podman). MacVLAN/IpVLAN are container networks but not bridges.
    pub fn is_container_bridge(&self) -> bool {
        matches!(self, SubnetType::DockerBridge | SubnetType::PodmanBridge)
    }

    /// Human-facing runtime label for a container bridge network
    /// (`"Docker"` / `"Podman"`), used to derive runtime-neutral container headers.
    /// `None` for non-bridge subnet types.
    pub fn container_runtime_label(&self) -> Option<&'static str> {
        match self {
            SubnetType::DockerBridge => Some("Docker"),
            SubnetType::PodmanBridge => Some("Podman"),
            _ => None,
        }
    }

    pub fn is_loopback(&self) -> bool {
        matches!(self, SubnetType::Loopback)
    }

    pub fn is_vlan_network(&self) -> bool {
        matches!(self, SubnetType::MacVlan | SubnetType::IpVlan)
    }

    pub fn exclude_from_topology(&self) -> bool {
        matches!(self, SubnetType::Loopback)
    }

    pub fn hide_from_subnet_list(&self) -> bool {
        matches!(
            self,
            SubnetType::Loopback | SubnetType::Internet | SubnetType::Remote
        )
    }

    pub fn show_label(&self) -> bool {
        !matches!(self, SubnetType::Unknown | SubnetType::Loopback)
    }
}

impl HasId for SubnetType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SubnetType {
    fn color(&self) -> Color {
        match self {
            SubnetType::Internet => Color::Blue,
            SubnetType::Remote => EntityDiscriminants::Subnet.color(),

            SubnetType::Gateway => Concept::Gateway.color(),
            SubnetType::VpnTunnel => Concept::Vpn.color(),
            SubnetType::Dmz => Color::Rose,

            SubnetType::Lan => EntityDiscriminants::Subnet.color(),
            SubnetType::IoT => Concept::IoT.color(),
            SubnetType::Guest => Color::Green,
            SubnetType::WiFi => Color::Teal,

            SubnetType::Management => Color::Gray,
            SubnetType::DockerBridge => Concept::Containerization.color(),
            SubnetType::PodmanBridge => Concept::Containerization.color(),
            SubnetType::MacVlan => Concept::Containerization.color(),
            SubnetType::IpVlan => Concept::Containerization.color(),
            SubnetType::Storage => Concept::Storage.color(),
            SubnetType::Loopback => Color::Gray,

            SubnetType::Unknown => Color::Gray,
        }
    }
    fn icon(&self) -> Icon {
        match self {
            SubnetType::Internet => Icon::Globe,
            SubnetType::Remote => EntityDiscriminants::Subnet.icon(),

            SubnetType::Gateway => Concept::Gateway.icon(),
            SubnetType::VpnTunnel => Concept::Vpn.icon(),
            SubnetType::Dmz => EntityDiscriminants::Subnet.icon(),

            SubnetType::Lan => EntityDiscriminants::Subnet.icon(),
            SubnetType::IoT => Concept::IoT.icon(),
            SubnetType::Guest => Icon::User,
            SubnetType::WiFi => Icon::Wifi,

            SubnetType::Management => Icon::ServerCog,
            SubnetType::DockerBridge => Icon::Box,
            SubnetType::PodmanBridge => Icon::Box,
            SubnetType::MacVlan => Icon::Network,
            SubnetType::IpVlan => Icon::Network,
            SubnetType::Storage => Concept::Storage.icon(),
            SubnetType::Loopback => Icon::Network,

            SubnetType::Unknown => EntityDiscriminants::Subnet.icon(),
        }
    }
}

impl TypeMetadataProvider for SubnetType {
    fn name(&self) -> &'static str {
        match self {
            SubnetType::Internet => "Internet",
            SubnetType::Remote => "Remote",

            SubnetType::Gateway => "Gateway",
            SubnetType::VpnTunnel => "VPN",
            SubnetType::Dmz => "DMZ",

            SubnetType::Lan => "LAN",
            SubnetType::IoT => "IoT",
            SubnetType::Guest => "Guest",
            SubnetType::WiFi => "WiFi",

            SubnetType::Management => "Management",
            SubnetType::DockerBridge => "Docker Bridge",
            SubnetType::PodmanBridge => "Podman Bridge",
            SubnetType::MacVlan => "MacVLAN",
            SubnetType::IpVlan => "IpVLAN",
            SubnetType::Storage => "Storage",
            SubnetType::Loopback => "Loopback",

            SubnetType::Unknown => "Unknown",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            SubnetType::Internet => "Internet",
            SubnetType::Remote => "Remote network",

            SubnetType::Gateway => "Gateway subnet",
            SubnetType::VpnTunnel => "Virtual private network",
            SubnetType::Dmz => "Demilitarized zone",

            SubnetType::Lan => "Local area network",
            SubnetType::IoT => "Internet of things",
            SubnetType::Guest => "Guest network",
            SubnetType::WiFi => "WiFi network",

            SubnetType::Management => "Management network",
            SubnetType::DockerBridge => "Docker bridge network",
            SubnetType::PodmanBridge => "Podman bridge network",
            SubnetType::MacVlan => "MacVLAN network",
            SubnetType::IpVlan => "IpVLAN network",
            SubnetType::Storage => "Storage network",
            SubnetType::Loopback => "Host-local loopback, excluded from topology and scans",

            SubnetType::Unknown => "Unknown network type",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let network_scan_discovery_eligible = !matches!(
            &self,
            SubnetType::Remote
                | SubnetType::Internet
                | SubnetType::DockerBridge
                | SubnetType::PodmanBridge
                | SubnetType::Loopback
        );

        let is_for_containers = matches!(
            self,
            SubnetType::DockerBridge
                | SubnetType::PodmanBridge
                | SubnetType::MacVlan
                | SubnetType::IpVlan
        );

        let show_label = !matches!(self, SubnetType::Unknown | SubnetType::Loopback);

        serde_json::json!({
            "network_scan_discovery_eligible": network_scan_discovery_eligible,
            "is_for_containers": is_for_containers,
            "is_container_bridge": self.is_container_bridge(),
            "show_label": show_label,
            "hide_from_subnet_list": self.hide_from_subnet_list()
        })
    }
}

#[cfg(test)]
mod forward_compat_tests {
    use super::*;

    #[test]
    fn unknown_variant_degrades_to_unknown() {
        // Reproduces the production `unknown variant 'Loopback'` failure class:
        // a subnet type a newer server emits that this build doesn't know now
        // degrades to `Unknown` instead of failing the whole subnets response.
        let parsed: SubnetType = serde_json::from_str("\"SomeFutureType\"").unwrap();
        assert_eq!(parsed, SubnetType::Unknown);
    }

    #[test]
    fn none_still_parses_to_unknown() {
        // The former `#[serde(alias = "None")]` is subsumed by `#[serde(other)]`.
        let parsed: SubnetType = serde_json::from_str("\"None\"").unwrap();
        assert_eq!(parsed, SubnetType::Unknown);
    }

    #[test]
    fn known_variants_round_trip() {
        for variant in [
            SubnetType::Loopback,
            SubnetType::Lan,
            SubnetType::DockerBridge,
            SubnetType::Unknown,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: SubnetType = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn container_runtime_label_present_iff_container_bridge() {
        // Invariant: a runtime label exists exactly for the container-bridge types.
        // Locks the two predicates together without restating each mapping, so it
        // fails only if a new bridge type forgets its label (or vice versa).
        use strum::IntoEnumIterator;
        for st in SubnetType::iter() {
            assert_eq!(
                st.container_runtime_label().is_some(),
                st.is_container_bridge(),
                "{st:?}: runtime label presence must match is_container_bridge()"
            );
        }
    }
}
