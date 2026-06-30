use std::fmt::Display;
use std::net::Ipv4Addr;

use crate::server::shared::entities::ChangeTriggersTopologyStaleness;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::api::deserialize_empty_string_as_none;
use crate::server::shared::types::entities::EntitySource;
use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::subnets::r#impl::virtualization::SubnetVirtualization;
use chrono::{DateTime, Utc};
use cidr::{IpCidr, Ipv4Cidr};
use pnet::ipnetwork::IpNetwork;
use serde::de::Error as DeError;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::{ip_addresses::r#impl::base::IPAddress, services::r#impl::base::Service};

fn deserialize_cidr<'de, D>(deserializer: D) -> Result<IpCidr, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<IpCidr>().map_err(|e| {
        let msg = e.to_string();
        if msg.contains("host part of address was not zero") {
            DeError::custom(format!(
                "Invalid CIDR '{}': address doesn't align with the subnet mask. Use a network address (e.g., for /24, the last octet should be 0).",
                s
            ))
        } else {
            DeError::custom(format!("Invalid CIDR '{}': {}", s, msg))
        }
    })
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize, Eq, PartialEq, Hash, ToSchema)]
pub struct SubnetBase {
    #[schema(value_type = String)]
    #[serde(deserialize_with = "deserialize_cidr")]
    pub cidr: IpCidr,
    pub network_id: Uuid,
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    #[serde(deserialize_with = "deserialize_empty_string_as_none")]
    #[validate(length(min = 0, max = 500))]
    pub description: Option<String>,
    pub subnet_type: SubnetType,
    /// Virtualization provider that owns this subnet.
    /// Docker bridge subnets use this for per-host dedup (same CIDR on different hosts = distinct).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::server::shared::types::api::deserialize_lenient_option"
    )]
    pub virtualization: Option<SubnetVirtualization>,
    #[serde(default)]
    #[schema(required)]
    /// Will be automatically set to Manual for creation through API
    pub source: EntitySource,
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
}

impl Default for SubnetBase {
    fn default() -> Self {
        Self {
            cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(192, 168, 4, 0), 24).unwrap()),
            name: "New Subnet".to_string(),
            network_id: Uuid::new_v4(),
            description: None,
            subnet_type: SubnetType::Unknown,
            virtualization: None,
            source: EntitySource::Manual,
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, ToSchema, Validate)]
#[schema(example = crate::server::shared::types::examples::subnet)]
pub struct Subnet {
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only)]
    pub valid_from: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only)]
    pub valid_to: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(read_only)]
    pub lineage_id: Option<Uuid>,
    #[serde(default)]
    #[schema(read_only)]
    pub last_seen_at: DateTime<Utc>,
    #[serde(default)]
    #[schema(read_only)]
    pub last_discovery_id: Option<Uuid>,
    #[serde(default)]
    #[schema(read_only)]
    pub first_discovery_id: Option<Uuid>,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: SubnetBase,
}

impl Subnet {
    /// Whether this subnet is a container-runtime bridge network (Docker or Podman).
    pub fn is_container_bridge_subnet(&self) -> bool {
        self.base.subnet_type.is_container_bridge()
    }

    pub fn is_vpn_subnet(&self) -> bool {
        self.base.subnet_type == SubnetType::VpnTunnel
    }

    pub fn from_discovery(
        interface_name: String,
        ip_network: &IpNetwork,
        network_id: Uuid,
    ) -> Option<Self> {
        let mut subnet_type = SubnetType::from_interface_name(&interface_name);

        match ip_network {
            IpNetwork::V6(_) => None,
            IpNetwork::V4(ipv4_network) => {
                // Non-loopback CIDRs on loopback ip_addresses (e.g. 10.99.0.0/24 aliased
                // on lo0) are real networks, not loopback
                if subnet_type.is_loopback() && ipv4_network.ip().octets()[0] != 127 {
                    subnet_type = SubnetType::Unknown;
                }

                let (network_addr, prefix_len) = match (&subnet_type, ipv4_network.prefix()) {
                    // VPN tunnels with /32 -> expand to /24
                    (SubnetType::VpnTunnel, 32) => {
                        let ip_octets = ipv4_network.ip().octets();
                        let network_addr =
                            std::net::Ipv4Addr::new(ip_octets[0], ip_octets[1], ip_octets[2], 0);
                        (network_addr, 24)
                    }
                    // Skip other /32 single IPs
                    (_, 32) => return None,
                    // Normal case - use the network's actual network address and prefix
                    _ => (ipv4_network.network(), ipv4_network.prefix()),
                };

                let cidr = IpCidr::V4(Ipv4Cidr::new(network_addr, prefix_len).ok()?);

                Some(Subnet::new(SubnetBase {
                    cidr,
                    network_id,
                    description: None,
                    tags: Vec::new(),
                    name: cidr.to_string(),
                    subnet_type,
                    virtualization: None,
                    source: EntitySource::Discovery,
                }))
            }
        }
    }

    pub fn has_interface_with_service(
        &self,
        host_interfaces: &[&IPAddress],
        service: &Service,
    ) -> bool {
        service.base.bindings.iter().any(|binding| {
            host_interfaces.iter().any(|ip_address| {
                let interface_match = match binding.ip_address_id() {
                    Some(id) => ip_address.id == id,
                    None => true, // Listens on all ip_addresses
                };

                interface_match && ip_address.base.subnet_id == self.id
            })
        })
    }

    pub fn is_organizational_subnet(&self) -> bool {
        let organizational_cidr = IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 0).unwrap());
        self.base.cidr == organizational_cidr
    }
}

impl PartialEq for Subnet {
    fn eq(&self, other: &Self) -> bool {
        let network_match =
            self.base.cidr == other.base.cidr && self.base.network_id == other.base.network_id;

        network_match || self.id == other.id
    }
}

impl Hash for Subnet {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.base.cidr.hash(state);
    }
}

impl Display for Subnet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Subnet {}: {}", self.base.name, self.id)
    }
}

impl ChangeTriggersTopologyStaleness<Subnet> for Subnet {
    fn triggers_staleness(&self, _other: Option<Subnet>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::ipnetwork::IpNetwork;
    use std::str::FromStr;

    #[test]
    fn from_discovery_accepts_valid_prefix() {
        let ip = IpNetwork::from_str("192.168.1.0/24").unwrap();
        let result = Subnet::from_discovery("eth0".to_string(), &ip, Uuid::nil());
        assert!(result.is_some(), "/24 prefix should be accepted");
    }

    #[test]
    fn from_discovery_accepts_prefix_2() {
        let ip = IpNetwork::from_str("10.0.0.0/2").unwrap();
        let result = Subnet::from_discovery("eth0".to_string(), &ip, Uuid::nil());
        assert!(result.is_some(), "/2 prefix should be accepted");
    }

    /// Guards the invariants `HostService::seed_loopback` depends on: the daemon-host
    /// loopback seed must produce a `127.0.0.0/8`, `Discovery`-sourced, non-bridge subnet.
    /// Exact CIDR is required for subnet dedup (`Subnet::eq`), and the `Discovery` source is
    /// required for `SubnetService::create` to reuse it when self-report re-reports loopback
    /// (a `System` source would not dedup).
    #[test]
    fn from_discovery_loopback_seed_shape() {
        let ip = IpNetwork::V4(
            pnet::ipnetwork::Ipv4Network::new(std::net::Ipv4Addr::LOCALHOST, 8).unwrap(),
        );
        let subnet = Subnet::from_discovery("lo".to_string(), &ip, Uuid::nil())
            .expect("loopback /8 should produce a subnet");
        assert_eq!(subnet.base.cidr.to_string(), "127.0.0.0/8");
        assert_eq!(subnet.base.source, EntitySource::Discovery);
        assert!(subnet.base.subnet_type.is_loopback());
        assert!(!subnet.is_container_bridge_subnet());
    }
}
