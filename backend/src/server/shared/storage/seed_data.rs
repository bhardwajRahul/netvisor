use cidr::Ipv4Cidr;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::Binding,
    hosts::r#impl::base::{Host, HostBase},
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    networks::r#impl::{Network, NetworkBase},
    ports::r#impl::base::{Port, PortType},
    services::{
        definitions::{client::Client, dns_server::DnsServer, web_service::WebService},
        r#impl::base::{Service, ServiceBase},
    },
    shared::{storage::traits::Storable, types::entities::EntitySource},
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
    users::r#impl::base::{User, UserBase},
};

pub fn create_user() -> User {
    User::new(UserBase::default())
}

pub fn create_network(organization_id: Uuid) -> Network {
    Network::new(NetworkBase::new(organization_id))
}

pub fn create_wan_subnet(network_id: Uuid) -> Subnet {
    let base = SubnetBase {
        name: "Internet".to_string(),
        network_id,
        tags: Vec::new(),
        cidr: cidr::IpCidr::V4(
            Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 0).expect("Cidr for internet subnet"),
        ),
        description: Some(
            "For representing services on the public internet, like DNS servers, \
cloud providers, or SaaS applications, that your network depends on but wouldn't \
be found by scanning. Create ip_addresses on this subnet to include them in your topology."
                .to_string(),
        ),
        subnet_type: SubnetType::Internet,
        virtualization_service_id: None,
        source: EntitySource::System,
    };

    Subnet::new(base)
}

pub fn create_remote_subnet(network_id: Uuid) -> Subnet {
    let base = SubnetBase {
        name: "Remote Network".to_string(),
        network_id,
        tags: Vec::new(),
        cidr: cidr::IpCidr::V4(
            Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 0).expect("Cidr for internet subnet"),
        ),
        description: Some(
            "For representing services on networks outside your scan range, like \
branch offices, VPN endpoints, or remote infrastructure. Create ip_addresses on this \
subnet to include them in your topology."
                .to_string(),
        ),
        subnet_type: SubnetType::Remote,
        virtualization_service_id: None,
        source: EntitySource::System,
    };

    Subnet::new(base)
}

/// Returns (Host, Vec<IPAddress>, Vec<Port>, Service) - children are passed separately to discover_host
pub fn create_remote_host(
    remote_subnet: &Subnet,
    network_id: Uuid,
) -> (Host, Vec<IPAddress>, Vec<Port>, Service) {
    // Create interface with placeholder host_id - server will set the correct one
    let ip_address = IPAddress::new(IPAddressBase::new_conceptual(Uuid::nil(), remote_subnet));

    let dynamic_port = Port::new_hostless(PortType::new_tcp(0)); // Ephemeral port
    let binding = Binding::new_port_serviceless(dynamic_port.id, Some(ip_address.id));

    let base = HostBase {
        name: "Mobile Device".to_string(),
        hostname: None,
        network_id,
        tags: Vec::new(),
        description: Some("A mobile device connecting from a remote network".to_string()),
        source: EntitySource::System,
        virtualization_metadata: None,
        virtualization_service_id: None,
        hidden: false,
        ..Default::default()
    };

    let host = Host::new(base);

    let client_service = Service::new(ServiceBase {
        host_id: host.id,
        network_id,
        tags: Vec::new(),
        name: "Mobile Device".to_string(),
        service_definition: Box::new(Client),
        bindings: vec![binding],
        virtualization_metadata: None,
        virtualization_service_id: None,
        source: EntitySource::System,
        position: 0,
    });

    (host, vec![ip_address], vec![dynamic_port], client_service)
}

/// Returns (Host, Vec<IPAddress>, Vec<Port>, Service) - children are passed separately to discover_host
pub fn create_internet_connectivity_host(
    internet_subnet: &Subnet,
    network_id: Uuid,
) -> (Host, Vec<IPAddress>, Vec<Port>, Service) {
    // Create interface with placeholder host_id - server will set the correct one
    let ip_address = IPAddress::new(IPAddressBase::new_conceptual(Uuid::nil(), internet_subnet));

    let https_port = Port::new_hostless(PortType::Https);
    let binding = Binding::new_port_serviceless(https_port.id, Some(ip_address.id));

    let base = HostBase {
        name: "Google.com".to_string(),
        network_id,
        tags: Vec::new(),
        hostname: None,
        description: None,
        source: EntitySource::System,
        virtualization_metadata: None,
        virtualization_service_id: None,
        hidden: false,
        ..Default::default()
    };

    let host = Host::new(base);

    let web_service = Service::new(ServiceBase {
        host_id: host.id,
        name: "Google.com".to_string(),
        network_id,
        tags: Vec::new(),
        service_definition: Box::new(WebService),
        bindings: vec![binding],
        virtualization_metadata: None,
        virtualization_service_id: None,
        source: EntitySource::System,
        position: 0,
    });

    (host, vec![ip_address], vec![https_port], web_service)
}

/// Returns (Host, Vec<IPAddress>, Vec<Port>, Service) - children are passed separately to discover_host
pub fn create_public_dns_host(
    internet_subnet: &Subnet,
    network_id: Uuid,
) -> (Host, Vec<IPAddress>, Vec<Port>, Service) {
    // Create interface with placeholder host_id - server will set the correct one
    let mut ip_address =
        IPAddress::new(IPAddressBase::new_conceptual(Uuid::nil(), internet_subnet));
    ip_address.base.ip_address = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
    let dns_udp_port = Port::new_hostless(PortType::DnsUdp);
    let binding = Binding::new_port_serviceless(dns_udp_port.id, Some(ip_address.id));

    let base = HostBase {
        name: "Cloudflare DNS".to_string(),
        hostname: None,
        network_id,
        description: None,
        tags: Vec::new(),
        source: EntitySource::System,
        virtualization_metadata: None,
        virtualization_service_id: None,
        hidden: false,
        ..Default::default()
    };

    let host = Host::new(base);

    let dns_service = Service::new(ServiceBase {
        host_id: host.id,
        network_id,
        tags: Vec::new(),
        name: "Cloudflare DNS".to_string(),
        service_definition: Box::new(DnsServer),
        bindings: vec![binding],
        virtualization_metadata: None,
        virtualization_service_id: None,
        source: EntitySource::System,
        position: 0,
    });

    (host, vec![ip_address], vec![dns_udp_port], dns_service)
}
