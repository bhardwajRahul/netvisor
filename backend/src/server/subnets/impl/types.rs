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
    MacVlan,
    IpVlan,
    Management,
    Storage,

    Unknown,
    #[default]
    None,
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
            "MacVlan" => Ok(SubnetType::MacVlan),
            "IpVlan" => Ok(SubnetType::IpVlan),
            "Management" => Ok(SubnetType::Management),
            "Storage" => Ok(SubnetType::Storage),
            "Unknown" => Ok(SubnetType::Unknown),
            "None" => Ok(SubnetType::None),
            _ => Err(anyhow::anyhow!("Unknown SubnetType: {}", s)),
        }
    }
}

impl SubnetType {
    pub fn from_interface_name(interface_name: &str) -> Self {
        // Docker containers
        if Self::match_interface_names(&["docker", "br-", "docker"], interface_name) {
            return SubnetType::DockerBridge;
        }

        // VPN tunnels
        if Self::match_interface_names(&["tun", "utun", "wg", "tap", "ppp", "vpn"], interface_name)
        {
            return SubnetType::VpnTunnel;
        }

        // WiFi interfaces
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

        // Management interfaces
        if Self::match_interface_names(&["mgmt", "ipmi", "bmc"], interface_name) {
            return SubnetType::Management;
        }

        // Storage networks
        if Self::match_interface_names(&["iscsi", "san", "storage"], interface_name) {
            return SubnetType::Storage;
        }

        // MacVLAN interfaces
        if Self::match_interface_names(&["macvlan", "mvlan"], interface_name) {
            return SubnetType::MacVlan;
        }

        // ipvlan interfaces
        if Self::match_interface_names(&["ipvlan"], interface_name) {
            return SubnetType::IpVlan;
        }

        // Standard LAN interfaces (catch-all for ethernet and Linux bridges)
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

    pub fn is_docker_bridge(&self) -> bool {
        matches!(self, SubnetType::DockerBridge)
    }

    pub fn is_vlan_network(&self) -> bool {
        matches!(self, SubnetType::MacVlan | SubnetType::IpVlan)
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
            SubnetType::DockerBridge => Concept::Virtualization.color(),
            SubnetType::MacVlan => Concept::Virtualization.color(),
            SubnetType::IpVlan => Concept::Virtualization.color(),
            SubnetType::Storage => Concept::Storage.color(),

            SubnetType::Unknown => Color::Gray,
            SubnetType::None => Color::Gray,
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
            SubnetType::MacVlan => Icon::Network,
            SubnetType::IpVlan => Icon::Network,
            SubnetType::Storage => Concept::Storage.icon(),

            SubnetType::Unknown => EntityDiscriminants::Subnet.icon(),
            SubnetType::None => EntityDiscriminants::Subnet.icon(),
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
            SubnetType::MacVlan => "MacVLAN",
            SubnetType::IpVlan => "IpVLAN",
            SubnetType::Storage => "Storage",

            SubnetType::Unknown => "Unknown",
            SubnetType::None => "No Subnet",
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
            SubnetType::MacVlan => "MacVLAN network",
            SubnetType::IpVlan => "IpVLAN network",
            SubnetType::Storage => "Storage network",

            SubnetType::Unknown => "Unknown network type",
            SubnetType::None => "No Subnet",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let network_scan_discovery_eligible = !matches!(
            &self,
            SubnetType::Remote | SubnetType::Internet | SubnetType::DockerBridge
        );

        let is_for_containers = matches!(
            self,
            SubnetType::DockerBridge | SubnetType::MacVlan | SubnetType::IpVlan
        );

        serde_json::json!({
            "network_scan_discovery_eligible": network_scan_discovery_eligible,
            "is_for_containers": is_for_containers
        })
    }
}
