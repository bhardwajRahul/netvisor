//! Demo data for populating demo organizations with realistic network infrastructure.
//!
//! This module provides a complete dataset representing "Acme Technologies", a mid-size
//! company with MSP operations. The data includes multiple networks, subnets, hosts,
//! services, daemons, API keys, tags, and groups.

use crate::server::{
    bindings::r#impl::base::Binding,
    daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase},
    daemons::r#impl::{
        api::DaemonCapabilities,
        base::{Daemon, DaemonBase, DaemonMode},
    },
    groups::r#impl::{
        base::{Group, GroupBase},
        types::GroupType,
    },
    hosts::r#impl::base::{Host, HostBase},
    if_entries::r#impl::base::{IfAdminStatus, IfEntry, IfEntryBase, IfOperStatus, Neighbor},
    interfaces::r#impl::base::{Interface, InterfaceBase},
    networks::r#impl::{Network, NetworkBase},
    ports::r#impl::base::{Port, PortType},
    services::{
        definitions::ServiceDefinitionRegistry,
        r#impl::base::{Service, ServiceBase},
    },
    shared::types::{Color, entities::EntitySource},
    snmp_credentials::{
        r#impl::base::{SnmpCredential, SnmpCredentialBase, SnmpVersion},
        resolution::lldp::{LldpChassisId, LldpPortId},
    },
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
    tags::r#impl::base::{Tag, TagBase},
    topology::types::{
        base::{Topology, TopologyBase},
        edges::EdgeStyle,
    },
};
use chrono::{DateTime, Utc};
use cidr::{IpCidr, Ipv4Cidr};
use secrecy::SecretString;
use semver::Version;
use std::net::{IpAddr, Ipv4Addr};
use uuid::Uuid;

// ============================================================================
// Demo Data Container
// ============================================================================

/// A host bundled with its interfaces, ports, and services for creation via discover_host
pub struct HostWithServices {
    pub host: Host,
    pub interfaces: Vec<Interface>,
    pub ports: Vec<Port>,
    pub services: Vec<Service>,
}

/// Deferred neighbor update to apply after all if_entries exist.
/// Uses host_name + if_index to identify entries (stable across creation)
/// instead of pre-generated UUIDs (which may not be preserved by storage).
pub struct NeighborUpdate {
    /// Source if_entry identifier
    pub source_host_name: String,
    pub source_if_index: i32,
    /// Target if_entry identifier (for IfEntry neighbors)
    pub target_host_name: String,
    pub target_if_index: i32,
}

/// Container for all demo data entities
pub struct DemoData {
    pub tags: Vec<Tag>,
    pub snmp_credentials: Vec<SnmpCredential>,
    pub networks: Vec<Network>,
    pub subnets: Vec<Subnet>,
    pub hosts_with_services: Vec<HostWithServices>,
    pub if_entries: Vec<IfEntry>,
    pub neighbor_updates: Vec<NeighborUpdate>,
    pub daemons: Vec<Daemon>,
    pub api_keys: Vec<DaemonApiKey>,
    pub groups: Vec<Group>,
    pub topologies: Vec<Topology>,
}

impl DemoData {
    /// Generate all demo data for the given organization
    /// Note: Groups are intentionally empty - they must be generated after services are created
    /// because group bindings reference actual service binding IDs from the database.
    pub fn generate(organization_id: Uuid, user_id: Uuid) -> Self {
        let now = Utc::now();

        // Generate all entities in dependency order
        let tags = generate_tags(organization_id, now);
        let snmp_credentials = generate_snmp_credentials(organization_id, now);
        let networks = generate_networks(organization_id, &tags, now);
        let subnets = generate_subnets(&networks, &tags, now);
        let hosts_with_services = generate_hosts_and_services(&networks, &subnets, &tags, now);

        // Collect hosts for daemon generation and if_entry generation
        let hosts: Vec<&Host> = hosts_with_services.iter().map(|h| &h.host).collect();
        let interfaces: Vec<&Interface> = hosts_with_services
            .iter()
            .flat_map(|h| h.interfaces.iter())
            .collect();

        let (if_entries, neighbor_updates) =
            generate_if_entries(&networks, &hosts, &interfaces, now);
        let daemons = generate_daemons(&networks, &hosts, &subnets, now, user_id);
        let api_keys = generate_api_keys(&networks, now);
        let topologies = generate_topologies(&networks, now);

        // Groups are empty - they'll be generated in the handler after services are created
        // This ensures group bindings reference actual service binding IDs
        let groups = vec![];

        Self {
            tags,
            snmp_credentials,
            networks,
            subnets,
            hosts_with_services,
            if_entries,
            neighbor_updates,
            daemons,
            api_keys,
            groups,
            topologies,
        }
    }
}

// ============================================================================
// Topologies
// ============================================================================

fn generate_topologies(networks: &[Network], now: DateTime<Utc>) -> Vec<Topology> {
    networks
        .iter()
        .map(|network| Topology {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: TopologyBase::new(format!("{} Topology", network.base.name), network.id),
        })
        .collect()
}

// ============================================================================
// Tags
// ============================================================================

fn generate_tags(organization_id: Uuid, now: DateTime<Utc>) -> Vec<Tag> {
    let tag_definitions: [(&str, &str, Color); 10] = [
        ("Production", "Systems running in production", Color::Red),
        ("Development", "Development and test systems", Color::Blue),
        ("Critical", "Business-critical services", Color::Orange),
        ("Backup Target", "Backup destinations", Color::Green),
        ("Monitoring", "Monitoring infrastructure", Color::Purple),
        ("Database", "Database servers", Color::Cyan),
        ("Web Tier", "Web and application servers", Color::Teal),
        ("IoT Device", "Smart devices", Color::Yellow),
        ("Needs Attention", "Requires admin review", Color::Rose),
        ("Managed Client", "Client-owned assets", Color::Indigo),
    ];

    tag_definitions
        .iter()
        .map(|(name, description, color)| Tag {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: TagBase {
                name: name.to_string(),
                description: Some(description.to_string()),
                color: *color,
                organization_id,
            },
        })
        .collect()
}

// ============================================================================
// SNMP Credentials
// ============================================================================

fn generate_snmp_credentials(organization_id: Uuid, now: DateTime<Utc>) -> Vec<SnmpCredential> {
    vec![
        SnmpCredential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SnmpCredentialBase {
                organization_id,
                name: "Default SNMPv2c".to_string(),
                version: SnmpVersion::V2c,
                community: SecretString::from("public".to_string()),
                tags: Vec::new(),
            },
        },
        SnmpCredential {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SnmpCredentialBase {
                organization_id,
                name: "Network Devices".to_string(),
                version: SnmpVersion::V2c,
                community: SecretString::from("acme-network".to_string()),
                tags: Vec::new(),
            },
        },
    ]
}

// ============================================================================
// Networks
// ============================================================================

fn generate_networks(organization_id: Uuid, tags: &[Tag], now: DateTime<Utc>) -> Vec<Network> {
    let production_tag = tags
        .iter()
        .find(|t| t.base.name == "Production")
        .map(|t| t.id);
    let managed_client_tag = tags
        .iter()
        .find(|t| t.base.name == "Managed Client")
        .map(|t| t.id);

    // Stagger timestamps so networks sort in predictable order (Headquarters first)
    vec![
        Network {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: NetworkBase {
                name: "Headquarters".to_string(),
                organization_id,
                tags: production_tag.into_iter().collect(),
                snmp_credential_id: None,
            },
        },
        Network {
            id: Uuid::new_v4(),
            created_at: now + chrono::Duration::seconds(1),
            updated_at: now + chrono::Duration::seconds(1),
            base: NetworkBase {
                name: "Cloud Infrastructure".to_string(),
                organization_id,
                tags: production_tag.into_iter().collect(),
                snmp_credential_id: None,
            },
        },
        Network {
            id: Uuid::new_v4(),
            created_at: now + chrono::Duration::seconds(2),
            updated_at: now + chrono::Duration::seconds(2),
            base: NetworkBase {
                name: "Remote Office - Denver".to_string(),
                organization_id,
                tags: vec![],
                snmp_credential_id: None,
            },
        },
        Network {
            id: Uuid::new_v4(),
            created_at: now + chrono::Duration::seconds(3),
            updated_at: now + chrono::Duration::seconds(3),
            base: NetworkBase {
                name: "Client: Riverside Medical".to_string(),
                organization_id,
                tags: managed_client_tag.into_iter().collect(),
                snmp_credential_id: None,
            },
        },
    ]
}

// ============================================================================
// Subnets
// ============================================================================

fn generate_subnets(networks: &[Network], tags: &[Tag], now: DateTime<Utc>) -> Vec<Subnet> {
    let hq_network = networks
        .iter()
        .find(|n| n.base.name == "Headquarters")
        .unwrap();
    let cloud_network = networks
        .iter()
        .find(|n| n.base.name == "Cloud Infrastructure")
        .unwrap();
    let denver_network = networks
        .iter()
        .find(|n| n.base.name.contains("Denver"))
        .unwrap();
    let riverside_network = networks
        .iter()
        .find(|n| n.base.name.contains("Riverside"))
        .unwrap();

    let monitoring_tag = tags
        .iter()
        .find(|t| t.base.name == "Monitoring")
        .map(|t| t.id);

    vec![
        // Headquarters subnets
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 1, 0), 24).unwrap()),
                network_id: hq_network.id,
                name: "HQ Management".to_string(),
                description: Some("Network management and monitoring".to_string()),
                subnet_type: SubnetType::Management,
                source: EntitySource::Manual,
                tags: monitoring_tag.into_iter().collect(),
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 10, 0), 24).unwrap()),
                network_id: hq_network.id,
                name: "HQ Office LAN".to_string(),
                description: Some("Office workstations".to_string()),
                subnet_type: SubnetType::Lan,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 20, 0), 24).unwrap()),
                network_id: hq_network.id,
                name: "HQ Servers".to_string(),
                description: Some("On-premises servers".to_string()),
                subnet_type: SubnetType::Lan,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 30, 0), 24).unwrap()),
                network_id: hq_network.id,
                name: "HQ IoT".to_string(),
                description: Some("Smart office devices".to_string()),
                subnet_type: SubnetType::IoT,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 0, 100, 0), 24).unwrap()),
                network_id: hq_network.id,
                name: "HQ Guest WiFi".to_string(),
                description: Some("Guest wireless network".to_string()),
                subnet_type: SubnetType::Guest,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 17, 0, 0), 16).unwrap()),
                network_id: hq_network.id,
                name: "HQ Docker Bridge".to_string(),
                description: Some("Docker container network".to_string()),
                subnet_type: SubnetType::DockerBridge,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        // Cloud subnets
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 16, 0, 0), 24).unwrap()),
                network_id: cloud_network.id,
                name: "Cloud Production".to_string(),
                description: Some("Production VPC".to_string()),
                subnet_type: SubnetType::Lan,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(172, 16, 1, 0), 24).unwrap()),
                network_id: cloud_network.id,
                name: "Cloud Database Tier".to_string(),
                description: Some("Database subnet".to_string()),
                subnet_type: SubnetType::Storage,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        // Denver subnets
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(192, 168, 50, 0), 24).unwrap()),
                network_id: denver_network.id,
                name: "Denver Office LAN".to_string(),
                description: Some("Branch office network".to_string()),
                subnet_type: SubnetType::Lan,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 8, 0, 0), 24).unwrap()),
                network_id: denver_network.id,
                name: "Denver VPN Tunnel".to_string(),
                description: Some("Site-to-site VPN to HQ".to_string()),
                subnet_type: SubnetType::VpnTunnel,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        // Riverside Medical subnets
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 100, 0, 0), 24).unwrap()),
                network_id: riverside_network.id,
                name: "Riverside LAN".to_string(),
                description: Some("Client main network".to_string()),
                subnet_type: SubnetType::Lan,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
        Subnet {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: SubnetBase {
                cidr: IpCidr::V4(Ipv4Cidr::new(Ipv4Addr::new(10, 100, 10, 0), 24).unwrap()),
                network_id: riverside_network.id,
                name: "Riverside Management".to_string(),
                description: Some("Client management network".to_string()),
                subnet_type: SubnetType::Management,
                source: EntitySource::Manual,
                tags: vec![],
            },
        },
    ]
}

// ============================================================================
// Hosts and Services
// ============================================================================

/// Helper to create a host with a single interface.
/// Returns (Host, Interface) - host has interface_ids: vec![] initially,
/// the server will populate it after creating the interface.
#[allow(clippy::too_many_arguments)]
fn create_host(
    name: &str,
    hostname: Option<&str>,
    description: Option<&str>,
    network: &Network,
    subnet: &Subnet,
    ip: Ipv4Addr,
    tags: Vec<Uuid>,
    now: DateTime<Utc>,
) -> (Host, Interface) {
    let host_id = Uuid::new_v4();
    let interface = Interface {
        id: Uuid::new_v4(),
        created_at: now,
        updated_at: now,
        base: InterfaceBase {
            network_id: network.id,
            host_id,
            subnet_id: subnet.id,
            ip_address: IpAddr::V4(ip),
            mac_address: None,
            name: Some("eth0".to_string()),
            position: 0,
        },
    };
    let host = Host {
        id: host_id,
        created_at: now,
        updated_at: now,
        base: HostBase {
            name: name.to_string(),
            network_id: network.id,
            hostname: hostname.map(String::from),
            description: description.map(String::from),
            source: EntitySource::Manual,
            virtualization: None,
            hidden: false,
            tags,
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            snmp_credential_id: None,
        },
    };
    (host, interface)
}

/// Helper to create a service for a host.
/// Returns (Service, Option<Port>) - the port must be added to the host's ports list.
fn create_service(
    service_def_id: &str,
    name: &str,
    host: &Host,
    interface: &Interface,
    port_type: Option<PortType>,
    tags: Vec<Uuid>,
    now: DateTime<Utc>,
) -> Option<(Service, Option<Port>)> {
    let service_definition = ServiceDefinitionRegistry::find_by_id(service_def_id)?;

    let (bindings, port) = if let Some(pt) = port_type {
        let port = Port::new_hostless(pt);
        let binding = Binding::new_port_serviceless(port.id, Some(interface.id));
        (vec![binding], Some(port))
    } else {
        let binding = Binding::new_interface_serviceless(interface.id);
        (vec![binding], None)
    };

    Some((
        Service {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: ServiceBase {
                host_id: host.id,
                network_id: host.base.network_id,
                service_definition,
                name: name.to_string(),
                bindings,
                virtualization: None,
                source: EntitySource::Manual,
                tags,
                position: 0,
            },
        },
        port,
    ))
}

/// Helper macro to create a host with its services bundled together.
/// Ports are collected separately and bundled with the host.
/// Takes a tuple of (Host, Interface) from create_host().
macro_rules! host_with_services {
    ($host_tuple:expr, $now:expr, $( ($svc_def:expr, $svc_name:expr, $port:expr, $tags:expr) ),* $(,)?) => {{
        let (host, interface) = $host_tuple;
        let interfaces = vec![interface];
        let mut ports = Vec::new();
        let mut services = Vec::new();
        $(
            if let Some((svc, port)) = create_service($svc_def, $svc_name, &host, &interfaces[0], $port, $tags, $now) {
                // Collect port separately if present
                if let Some(p) = port {
                    ports.push(p);
                }
                services.push(svc);
            }
        )*
        HostWithServices { host, interfaces, ports, services }
    }};
}

fn generate_hosts_and_services(
    networks: &[Network],
    subnets: &[Subnet],
    tags: &[Tag],
    now: DateTime<Utc>,
) -> Vec<HostWithServices> {
    let mut result = Vec::new();

    // Helper to find entities
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };
    let find_subnet = |name: &str| subnets.iter().find(|s| s.base.name.contains(name)).unwrap();
    let find_tag = |name: &str| tags.iter().find(|t| t.base.name == name).map(|t| t.id);

    let critical_tag = find_tag("Critical");
    let production_tag = find_tag("Production");
    let database_tag = find_tag("Database");
    let monitoring_tag = find_tag("Monitoring");
    let iot_tag = find_tag("IoT Device");
    let managed_tag = find_tag("Managed Client");
    let web_tier_tag = find_tag("Web Tier");
    let backup_tag = find_tag("Backup Target");

    // ========== HEADQUARTERS NETWORK ==========
    let hq = find_network("Headquarters");
    let hq_mgmt = find_subnet("HQ Management");
    let hq_servers = find_subnet("HQ Servers");
    let hq_lan = find_subnet("HQ Office LAN");
    let hq_iot = find_subnet("HQ IoT");

    // -- pfSense Firewall --
    result.push(host_with_services!(
        create_host(
            "pfsense-fw01",
            Some("pfsense-fw01.acme.local"),
            Some("Primary pfSense firewall"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 1),
            critical_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "pfSense",
            "pfSense",
            Some(PortType::Https),
            critical_tag.into_iter().collect()
        ),
    ));

    // -- UniFi Controller --
    result.push(host_with_services!(
        create_host(
            "unifi-controller",
            Some("unifi.acme.local"),
            Some("UniFi Network Controller"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 10),
            vec![],
            now
        ),
        now,
        (
            "UniFi Controller",
            "UniFi Controller",
            Some(PortType::Https8443),
            vec![]
        ),
    ));

    // -- UniFi Access Point --
    result.push(host_with_services!(
        create_host(
            "unifi-ap-lobby",
            Some("ap-lobby.acme.local"),
            Some("UniFi AP - Main Lobby"),
            hq,
            hq_iot,
            Ipv4Addr::new(10, 0, 30, 100),
            iot_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Unifi Access Point",
            "UniFi AP",
            None,
            iot_tag.into_iter().collect()
        ),
    ));

    // -- UniFi Switch (core switch with LLDP neighbors) --
    result.push(host_with_services!(
        create_host(
            "unifi-usw-24",
            Some("switch.acme.local"),
            Some("UniFi Switch 24 PoE"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 3),
            vec![],
            now
        ),
        now,
        ("SNMP", "SNMP", Some(PortType::Snmp), vec![]),
    ));

    // -- Proxmox Hypervisors --
    result.push(host_with_services!(
        create_host(
            "proxmox-hv01",
            Some("proxmox-hv01.acme.local"),
            Some("Proxmox hypervisor node 1"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 5),
            production_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Proxmox VE",
            "Proxmox VE",
            Some(PortType::Https8443),
            production_tag.into_iter().collect()
        ),
    ));

    result.push(host_with_services!(
        create_host(
            "proxmox-hv02",
            Some("proxmox-hv02.acme.local"),
            Some("Proxmox hypervisor node 2"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 6),
            production_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Proxmox VE",
            "Proxmox VE",
            Some(PortType::Https8443),
            production_tag.into_iter().collect()
        ),
    ));

    // -- TrueNAS Storage --
    result.push(host_with_services!(
        create_host(
            "truenas-primary",
            Some("truenas.acme.local"),
            Some("Primary NAS storage"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 10),
            critical_tag.into_iter().chain(backup_tag).collect(),
            now
        ),
        now,
        (
            "TrueNAS",
            "TrueNAS",
            Some(PortType::Https),
            backup_tag.into_iter().collect()
        ),
    ));

    // -- Docker Host with Portainer --
    result.push(host_with_services!(
        create_host(
            "docker-prod01",
            Some("docker-prod01.acme.local"),
            Some("Production Docker host"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 20),
            production_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Portainer",
            "Portainer",
            Some(PortType::Http9000),
            production_tag.into_iter().collect()
        ),
        ("Docker", "Docker Daemon", Some(PortType::Docker), vec![]),
    ));

    // -- GitLab --
    result.push(host_with_services!(
        create_host(
            "gitlab-server",
            Some("gitlab.acme.local"),
            Some("GitLab instance"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 25),
            production_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "GitLab",
            "GitLab",
            Some(PortType::Https),
            production_tag.into_iter().collect()
        ),
    ));

    // -- Jenkins CI --
    result.push(host_with_services!(
        create_host(
            "jenkins-ci",
            Some("jenkins.acme.local"),
            Some("Jenkins CI/CD server"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 30),
            production_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Jenkins",
            "Jenkins",
            Some(PortType::Http8080),
            production_tag.into_iter().collect()
        ),
    ));

    // -- Grafana Monitoring --
    result.push(host_with_services!(
        create_host(
            "grafana-mon",
            Some("grafana.acme.local"),
            Some("Grafana monitoring dashboard"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 50),
            monitoring_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Grafana",
            "Grafana",
            Some(PortType::Http3000),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // -- Prometheus --
    result.push(host_with_services!(
        create_host(
            "prometheus",
            Some("prometheus.acme.local"),
            Some("Prometheus metrics server"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 51),
            monitoring_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Prometheus",
            "Prometheus",
            Some(PortType::Http9000),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // -- Uptime Kuma --
    result.push(host_with_services!(
        create_host(
            "uptime-kuma",
            Some("status.acme.local"),
            Some("Uptime Kuma status page"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 52),
            monitoring_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "UptimeKuma",
            "Uptime Kuma",
            Some(PortType::Http3000),
            monitoring_tag.into_iter().collect()
        ),
    ));

    // -- Pi-hole DNS --
    result.push(host_with_services!(
        create_host(
            "pihole-dns",
            Some("pihole.acme.local"),
            Some("Pi-hole DNS ad blocker"),
            hq,
            hq_mgmt,
            Ipv4Addr::new(10, 0, 1, 5),
            vec![],
            now
        ),
        now,
        ("Pi-Hole", "Pi-hole", Some(PortType::Http), vec![]),
    ));

    // -- Vaultwarden --
    result.push(host_with_services!(
        create_host(
            "vaultwarden",
            Some("vault.acme.local"),
            Some("Vaultwarden password manager"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 35),
            critical_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Vaultwarden",
            "Vaultwarden",
            Some(PortType::Https),
            critical_tag.into_iter().collect()
        ),
    ));

    // -- Nextcloud --
    result.push(host_with_services!(
        create_host(
            "nextcloud",
            Some("cloud.acme.local"),
            Some("Nextcloud file sharing"),
            hq,
            hq_servers,
            Ipv4Addr::new(10, 0, 20, 40),
            production_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "NextCloud",
            "Nextcloud",
            Some(PortType::Https),
            production_tag.into_iter().collect()
        ),
    ));

    // -- Philips Hue Bridge --
    result.push(host_with_services!(
        create_host(
            "hue-bridge",
            None,
            Some("Philips Hue Bridge"),
            hq,
            hq_iot,
            Ipv4Addr::new(10, 0, 30, 10),
            iot_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Philips Hue Bridge",
            "Philips Hue",
            Some(PortType::Https),
            iot_tag.into_iter().collect()
        ),
    ));

    // -- HP Printer --
    result.push(host_with_services!(
        create_host(
            "printer-hp-main",
            None,
            Some("HP LaserJet Pro"),
            hq,
            hq_iot,
            Ipv4Addr::new(10, 0, 30, 50),
            iot_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Hp Printer",
            "HP Printer",
            Some(PortType::Ipp),
            iot_tag.into_iter().collect()
        ),
    ));

    // -- Security Camera --
    result.push(host_with_services!(
        create_host(
            "cam-entrance",
            None,
            Some("Entrance security camera"),
            hq,
            hq_iot,
            Ipv4Addr::new(10, 0, 30, 60),
            iot_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "RTSP Camera",
            "Security Camera",
            Some(PortType::Rtsp),
            iot_tag.into_iter().collect()
        ),
    ));

    // -- Workstations --
    result.push(host_with_services!(
        create_host(
            "ws-engineering-01",
            Some("ws-eng-01.acme.local"),
            Some("Engineering workstation"),
            hq,
            hq_lan,
            Ipv4Addr::new(10, 0, 10, 101),
            vec![],
            now
        ),
        now,
        ("Workstation", "Workstation", Some(PortType::Rdp), vec![]),
    ));

    result.push(host_with_services!(
        create_host(
            "ws-accounting-01",
            Some("ws-acct-01.acme.local"),
            Some("Accounting workstation"),
            hq,
            hq_lan,
            Ipv4Addr::new(10, 0, 10, 102),
            vec![],
            now
        ),
        now,
        ("Workstation", "Workstation", Some(PortType::Rdp), vec![]),
    ));

    // ========== CLOUD INFRASTRUCTURE ==========
    let cloud = find_network("Cloud");
    let cloud_prod = find_subnet("Cloud Production");
    let cloud_db = find_subnet("Cloud Database");

    // -- Traefik Load Balancer --
    result.push(host_with_services!(
        create_host(
            "traefik-lb01",
            Some("traefik-lb01.cloud.acme.io"),
            Some("Traefik load balancer"),
            cloud,
            cloud_prod,
            Ipv4Addr::new(172, 16, 0, 10),
            production_tag
                .into_iter()
                .chain(critical_tag)
                .chain(web_tier_tag)
                .collect(),
            now
        ),
        now,
        (
            "Traefik",
            "Traefik",
            Some(PortType::Https),
            web_tier_tag.into_iter().collect()
        ),
    ));

    // -- Application Servers --
    result.push(host_with_services!(
        create_host(
            "app-server-01",
            Some("app-01.cloud.acme.io"),
            Some("Application server 1"),
            cloud,
            cloud_prod,
            Ipv4Addr::new(172, 16, 0, 20),
            production_tag.into_iter().chain(web_tier_tag).collect(),
            now
        ),
        now,
        ("SSH", "SSH", Some(PortType::Ssh), vec![]),
        (
            "Web Service",
            "Web Application",
            Some(PortType::Http8080),
            web_tier_tag.into_iter().collect()
        ),
    ));

    result.push(host_with_services!(
        create_host(
            "app-server-02",
            Some("app-02.cloud.acme.io"),
            Some("Application server 2"),
            cloud,
            cloud_prod,
            Ipv4Addr::new(172, 16, 0, 21),
            production_tag.into_iter().chain(web_tier_tag).collect(),
            now
        ),
        now,
        ("SSH", "SSH", Some(PortType::Ssh), vec![]),
        (
            "Web Service",
            "Web Application",
            Some(PortType::Http8080),
            web_tier_tag.into_iter().collect()
        ),
    ));

    // -- PostgreSQL Primary --
    result.push(host_with_services!(
        create_host(
            "postgres-primary",
            Some("pg-primary.cloud.acme.io"),
            Some("PostgreSQL primary"),
            cloud,
            cloud_db,
            Ipv4Addr::new(172, 16, 1, 10),
            database_tag.into_iter().chain(critical_tag).collect(),
            now
        ),
        now,
        (
            "PostgreSQL",
            "PostgreSQL Primary",
            Some(PortType::PostgreSQL),
            database_tag.into_iter().collect()
        ),
    ));

    // -- PostgreSQL Replica --
    result.push(host_with_services!(
        create_host(
            "postgres-replica",
            Some("pg-replica.cloud.acme.io"),
            Some("PostgreSQL replica"),
            cloud,
            cloud_db,
            Ipv4Addr::new(172, 16, 1, 11),
            database_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "PostgreSQL",
            "PostgreSQL Replica",
            Some(PortType::PostgreSQL),
            database_tag.into_iter().collect()
        ),
    ));

    // -- Redis Cache --
    result.push(host_with_services!(
        create_host(
            "redis-cache",
            Some("redis.cloud.acme.io"),
            Some("Redis cache server"),
            cloud,
            cloud_db,
            Ipv4Addr::new(172, 16, 1, 20),
            database_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Redis",
            "Redis",
            Some(PortType::Redis),
            database_tag.into_iter().collect()
        ),
    ));

    // -- Elasticsearch --
    result.push(host_with_services!(
        create_host(
            "elasticsearch",
            Some("es.cloud.acme.io"),
            Some("Elasticsearch cluster"),
            cloud,
            cloud_db,
            Ipv4Addr::new(172, 16, 1, 30),
            database_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Elasticsearch",
            "Elasticsearch",
            Some(PortType::Elasticsearch),
            database_tag.into_iter().collect()
        ),
    ));

    // -- RabbitMQ --
    result.push(host_with_services!(
        create_host(
            "rabbitmq",
            Some("mq.cloud.acme.io"),
            Some("RabbitMQ message broker"),
            cloud,
            cloud_prod,
            Ipv4Addr::new(172, 16, 0, 30),
            production_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "RabbitMQ",
            "RabbitMQ",
            Some(PortType::AMQP),
            production_tag.into_iter().collect()
        ),
    ));

    // ========== DENVER REMOTE OFFICE ==========
    let denver = find_network("Denver");
    let denver_lan = find_subnet("Denver Office LAN");

    result.push(host_with_services!(
        create_host(
            "denver-fw",
            Some("fw.denver.acme.local"),
            Some("Denver branch firewall"),
            denver,
            denver_lan,
            Ipv4Addr::new(192, 168, 50, 1),
            vec![],
            now
        ),
        now,
        ("OPNsense", "OPNsense", Some(PortType::Https), vec![]),
    ));

    result.push(host_with_services!(
        create_host(
            "denver-nas",
            Some("nas.denver.acme.local"),
            Some("Denver local NAS"),
            denver,
            denver_lan,
            Ipv4Addr::new(192, 168, 50, 10),
            backup_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Synology DSM",
            "Synology NAS",
            Some(PortType::Https),
            backup_tag.into_iter().collect()
        ),
    ));

    result.push(host_with_services!(
        create_host(
            "denver-printer",
            None,
            Some("Denver office printer"),
            denver,
            denver_lan,
            Ipv4Addr::new(192, 168, 50, 50),
            iot_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Hp Printer",
            "HP Printer",
            Some(PortType::Ipp),
            iot_tag.into_iter().collect()
        ),
    ));

    result.push(host_with_services!(
        create_host(
            "denver-ap",
            Some("ap.denver.acme.local"),
            Some("Denver WiFi access point"),
            denver,
            denver_lan,
            Ipv4Addr::new(192, 168, 50, 2),
            iot_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Unifi Access Point",
            "UniFi AP",
            None,
            iot_tag.into_iter().collect()
        ),
    ));

    // -- Denver VPN Endpoint --
    let denver_vpn = find_subnet("Denver VPN");
    result.push(host_with_services!(
        create_host(
            "denver-vpn",
            Some("vpn.denver.acme.local"),
            Some("VPN endpoint to headquarters"),
            denver,
            denver_vpn,
            Ipv4Addr::new(10, 8, 0, 1),
            vec![],
            now
        ),
        now,
        (
            "WireGuard",
            "WireGuard VPN",
            Some(PortType::Wireguard),
            vec![]
        ),
    ));

    // ========== RIVERSIDE MEDICAL (CLIENT) ==========
    let riverside = find_network("Riverside");
    let riverside_lan = find_subnet("Riverside LAN");
    let riverside_mgmt = find_subnet("Riverside Management");

    result.push(host_with_services!(
        create_host(
            "rm-firewall",
            Some("fw.riverside-medical.local"),
            Some("Client firewall"),
            riverside,
            riverside_lan,
            Ipv4Addr::new(10, 100, 0, 1),
            managed_tag.into_iter().chain(critical_tag).collect(),
            now
        ),
        now,
        (
            "Fortinet",
            "FortiGate",
            Some(PortType::Https),
            managed_tag.into_iter().collect()
        ),
    ));

    result.push(host_with_services!(
        create_host(
            "rm-dc01",
            Some("dc01.riverside-medical.local"),
            Some("Domain controller"),
            riverside,
            riverside_lan,
            Ipv4Addr::new(10, 100, 0, 10),
            managed_tag.into_iter().chain(critical_tag).collect(),
            now
        ),
        now,
        (
            "Active Directory",
            "Active Directory",
            Some(PortType::Ldap),
            managed_tag.into_iter().collect()
        ),
    ));

    result.push(host_with_services!(
        create_host(
            "rm-fileserver",
            Some("files.riverside-medical.local"),
            Some("File server"),
            riverside,
            riverside_lan,
            Ipv4Addr::new(10, 100, 0, 20),
            managed_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Samba",
            "Samba File Share",
            Some(PortType::Samba),
            managed_tag.into_iter().collect()
        ),
    ));

    result.push(host_with_services!(
        create_host(
            "rm-backup",
            Some("backup.riverside-medical.local"),
            Some("Backup server"),
            riverside,
            riverside_mgmt,
            Ipv4Addr::new(10, 100, 10, 5),
            managed_tag.into_iter().chain(backup_tag).collect(),
            now
        ),
        now,
        (
            "Veeam",
            "Veeam Backup",
            Some(PortType::Https),
            managed_tag.into_iter().chain(backup_tag).collect()
        ),
    ));

    result.push(host_with_services!(
        create_host(
            "rm-reception-01",
            Some("ws-reception.riverside-medical.local"),
            Some("Reception workstation"),
            riverside,
            riverside_lan,
            Ipv4Addr::new(10, 100, 0, 101),
            managed_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Workstation",
            "Workstation",
            Some(PortType::Rdp),
            managed_tag.into_iter().collect()
        ),
    ));

    result.push(host_with_services!(
        create_host(
            "rm-nurse-station-01",
            Some("ws-nurse-01.riverside-medical.local"),
            Some("Nurse station workstation"),
            riverside,
            riverside_lan,
            Ipv4Addr::new(10, 100, 0, 102),
            managed_tag.into_iter().collect(),
            now
        ),
        now,
        (
            "Workstation",
            "Workstation",
            Some(PortType::Rdp),
            managed_tag.into_iter().collect()
        ),
    ));

    result
}

// ============================================================================
// IfEntries (SNMP Interface Data)
// ============================================================================

fn generate_if_entries(
    networks: &[Network],
    hosts: &[&Host],
    interfaces: &[&Interface],
    now: DateTime<Utc>,
) -> (Vec<IfEntry>, Vec<NeighborUpdate>) {
    let mut if_entries = Vec::new();
    let mut neighbor_updates = Vec::new();

    // Find network devices that would have SNMP data
    let find_host = |name: &str| hosts.iter().find(|h| h.base.name == name).copied();
    let find_interface = |host_id: Uuid| {
        interfaces
            .iter()
            .find(|i| i.base.host_id == host_id)
            .copied()
    };

    // Pre-generate IDs for bidirectional neighbor relationships
    let pfsense_lan_id = Uuid::new_v4();
    let truenas_lagg0_id = Uuid::new_v4();
    let proxmox_eno1_id = Uuid::new_v4();
    let switch_port1_id = Uuid::new_v4(); // connects to pfSense
    let switch_port2_id = Uuid::new_v4(); // connects to TrueNAS
    let switch_port3_id = Uuid::new_v4(); // connects to Proxmox
    let switch_port4_id = Uuid::new_v4(); // connects to UniFi AP (partial - Host only)

    // Switch MAC address (used as chassis ID)
    let switch_mac = "78:45:c4:ab:cd:01";

    // pfSense firewall - multiple interfaces
    if let Some(host) = find_host("pfsense-fw01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        // WAN interface
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "igb0".to_string(),
                if_alias: Some("WAN".to_string()),
                if_type: 6, // ethernet
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
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
            },
        });

        // LAN interface - connected to switch port 1
        // Defer the neighbor reference since switch ports are created later
        if_entries.push(IfEntry {
            id: pfsense_lan_id,
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 2,
                if_descr: "igb1".to_string(),
                if_alias: Some("LAN".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None, // Deferred - switch port doesn't exist yet
                lldp_chassis_id: Some(LldpChassisId::MacAddress(switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/1".to_string())),
                lldp_sys_name: Some("unifi-usw-24".to_string()),
                lldp_port_desc: Some("Port 1 - pfSense uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 3))),
                lldp_sys_desc: Some("UniFi USW-24-PoE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "pfsense-fw01".to_string(),
            source_if_index: 2, // igb1 LAN interface
            target_host_name: "unifi-usw-24".to_string(),
            target_if_index: 1, // Port 1/0/1
        });

        // OPT1 interface (disabled)
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 3,
                if_descr: "igb2".to_string(),
                if_alias: Some("OPT1".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Down,
                oper_status: IfOperStatus::Down,
                mac_address: None,
                interface_id: None,
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
            },
        });
    }

    // TrueNAS - bonded interfaces, connected to switch port 2
    if let Some(host) = find_host("truenas-primary") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        // Bond interface
        // Defer the neighbor reference since switch ports are created later
        if_entries.push(IfEntry {
            id: truenas_lagg0_id,
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "lagg0".to_string(),
                if_alias: Some("LACP Bond".to_string()),
                if_type: 161,                    // IEEE8023AD_LAG
                speed_bps: Some(10_000_000_000), // 10 Gbps bonded
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None, // Deferred - switch port doesn't exist yet
                lldp_chassis_id: Some(LldpChassisId::MacAddress(switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/2".to_string())),
                lldp_sys_name: Some("unifi-usw-24".to_string()),
                lldp_port_desc: Some("Port 2 - TrueNAS uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 3))),
                lldp_sys_desc: Some("UniFi USW-24-PoE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "truenas-primary".to_string(),
            source_if_index: 1, // lagg0
            target_host_name: "unifi-usw-24".to_string(),
            target_if_index: 2, // Port 1/0/2
        });
    }

    // Proxmox - with loopback, connected to switch port 3
    if let Some(host) = find_host("proxmox-hv01") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        // Primary interface
        // Defer the neighbor reference since switch ports are created later
        if_entries.push(IfEntry {
            id: proxmox_eno1_id,
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "eno1".to_string(),
                if_alias: Some("Primary NIC".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: interface.map(|i| i.id),
                neighbor: None, // Deferred - switch port doesn't exist yet
                lldp_chassis_id: Some(LldpChassisId::MacAddress(switch_mac.to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("Port 1/0/3".to_string())),
                lldp_sys_name: Some("unifi-usw-24".to_string()),
                lldp_port_desc: Some("Port 3 - Proxmox uplink".to_string()),
                lldp_mgmt_addr: Some(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 3))),
                lldp_sys_desc: Some("UniFi USW-24-PoE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "proxmox-hv01".to_string(),
            source_if_index: 1, // eno1
            target_host_name: "unifi-usw-24".to_string(),
            target_if_index: 3, // Port 1/0/3
        });

        // Loopback
        if_entries.push(IfEntry {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 2,
                if_descr: "lo".to_string(),
                if_alias: None,
                if_type: 24, // SOFTWARE_LOOPBACK
                speed_bps: None,
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
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
            },
        });
    }

    // UniFi Switch - 24 port switch with LLDP neighbors
    if let Some(host) = find_host("unifi-usw-24") {
        let network = networks
            .iter()
            .find(|n| n.id == host.base.network_id)
            .unwrap();
        let interface = find_interface(host.id);

        // Find hosts for neighbor references
        let pfsense_host = find_host("pfsense-fw01");
        let truenas_host = find_host("truenas-primary");
        let proxmox_host = find_host("proxmox-hv01");
        let ap_host = find_host("unifi-ap-lobby");

        // Port 1 - connected to pfSense
        // Defer neighbor reference - bidirectional link set up after all entries exist
        if_entries.push(IfEntry {
            id: switch_port1_id,
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 1,
                if_descr: "Port 1/0/1".to_string(),
                if_alias: Some("pfSense uplink".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: if interface.is_some() {
                    interface.map(|i| i.id)
                } else {
                    None
                },
                neighbor: None, // Deferred - bidirectional link
                lldp_chassis_id: Some(LldpChassisId::MacAddress("00:0d:b9:4a:f2:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("igb1".to_string())),
                lldp_sys_name: Some("pfsense-fw01".to_string()),
                lldp_port_desc: Some("LAN".to_string()),
                lldp_mgmt_addr: pfsense_host
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 1, 1))),
                lldp_sys_desc: Some("pfSense 2.7.0-RELEASE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-24".to_string(),
            source_if_index: 1, // Port 1/0/1
            target_host_name: "pfsense-fw01".to_string(),
            target_if_index: 2, // igb1 LAN interface
        });

        // Port 2 - connected to TrueNAS
        // Defer neighbor reference - TrueNAS host created after switch
        // interface_id is None - single-interface fallback will resolve it
        if_entries.push(IfEntry {
            id: switch_port2_id,
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 2,
                if_descr: "Port 1/0/2".to_string(),
                if_alias: Some("TrueNAS uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None, // Deferred - TrueNAS doesn't exist yet
                lldp_chassis_id: Some(LldpChassisId::MacAddress("3c:ec:ef:12:34:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("lagg0".to_string())),
                lldp_sys_name: Some("truenas-primary".to_string()),
                lldp_port_desc: Some("LACP Bond".to_string()),
                lldp_mgmt_addr: truenas_host
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 20, 10))),
                lldp_sys_desc: Some("TrueNAS SCALE".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-24".to_string(),
            source_if_index: 2, // Port 1/0/2
            target_host_name: "truenas-primary".to_string(),
            target_if_index: 1, // lagg0
        });

        // Port 3 - connected to Proxmox
        // Defer neighbor reference - Proxmox host created after switch
        // interface_id is None - single-interface fallback will resolve it
        if_entries.push(IfEntry {
            id: switch_port3_id,
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 3,
                if_descr: "Port 1/0/3".to_string(),
                if_alias: Some("Proxmox uplink".to_string()),
                if_type: 6,
                speed_bps: Some(10_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                neighbor: None, // Deferred - Proxmox doesn't exist yet
                lldp_chassis_id: Some(LldpChassisId::MacAddress("d4:be:d9:56:78:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("eno1".to_string())),
                lldp_sys_name: Some("proxmox-hv01".to_string()),
                lldp_port_desc: Some("Primary NIC".to_string()),
                lldp_mgmt_addr: proxmox_host
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 20, 5))),
                lldp_sys_desc: Some("Proxmox VE 8.1".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
            },
        });
        neighbor_updates.push(NeighborUpdate {
            source_host_name: "unifi-usw-24".to_string(),
            source_if_index: 3, // Port 1/0/3
            target_host_name: "proxmox-hv01".to_string(),
            target_if_index: 1, // eno1
        });

        // Port 4 - connected to UniFi AP (partial resolution - Host only since AP has no IfEntry)
        if_entries.push(IfEntry {
            id: switch_port4_id,
            created_at: now,
            updated_at: now,
            base: IfEntryBase {
                host_id: host.id,
                network_id: network.id,
                if_index: 4,
                if_descr: "Port 1/0/4".to_string(),
                if_alias: Some("UniFi AP".to_string()),
                if_type: 6,
                speed_bps: Some(1_000_000_000),
                admin_status: IfAdminStatus::Up,
                oper_status: IfOperStatus::Up,
                mac_address: None,
                interface_id: None,
                // Partial resolution - only host known, no IfEntry for the AP
                neighbor: ap_host.map(|h| Neighbor::Host(h.id)),
                lldp_chassis_id: Some(LldpChassisId::MacAddress("fc:ec:da:aa:bb:01".to_string())),
                lldp_port_id: Some(LldpPortId::InterfaceName("eth0".to_string())),
                lldp_sys_name: Some("unifi-ap-lobby".to_string()),
                lldp_port_desc: Some("Ethernet".to_string()),
                lldp_mgmt_addr: ap_host
                    .map(|_| std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 30, 100))),
                lldp_sys_desc: Some("UniFi AP U6-Pro".to_string()),
                cdp_device_id: None,
                cdp_port_id: None,
                cdp_platform: None,
                cdp_address: None,
            },
        });

        // Ports 5-24 - empty ports (no neighbors)
        for port_num in 5..=24 {
            if_entries.push(IfEntry {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                base: IfEntryBase {
                    host_id: host.id,
                    network_id: network.id,
                    if_index: port_num,
                    if_descr: format!("Port 1/0/{}", port_num),
                    if_alias: None,
                    if_type: 6,
                    speed_bps: Some(1_000_000_000),
                    admin_status: IfAdminStatus::Up,
                    oper_status: IfOperStatus::Down, // No link
                    mac_address: None,
                    interface_id: None,
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
                },
            });
        }
    }

    (if_entries, neighbor_updates)
}

// ============================================================================
// Daemons
// ============================================================================

fn generate_daemons(
    networks: &[Network],
    hosts: &[&Host],
    subnets: &[Subnet],
    now: DateTime<Utc>,
    user_id: Uuid,
) -> Vec<Daemon> {
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };
    let find_host = |name: &str| hosts.iter().find(|h| h.base.name == name).copied();
    let find_subnet = |name: &str| subnets.iter().find(|s| s.base.name.contains(name));

    let mut daemons = Vec::new();

    // HQ Daemon on docker-prod01
    if let (Some(host), Some(subnet)) = (find_host("docker-prod01"), find_subnet("HQ Servers")) {
        let network = find_network("Headquarters");
        daemons.push(Daemon {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonBase {
                host_id: host.id,
                network_id: network.id,
                url: "https://docker-prod01.acme.local:8443".to_string(),
                last_seen: Some(now),
                capabilities: DaemonCapabilities {
                    has_docker_socket: true,
                    interfaced_subnet_ids: vec![subnet.id],
                },
                mode: DaemonMode::DaemonPoll,
                name: "HQ Daemon".to_string(),
                tags: vec![],
                version: Version::parse(env!("CARGO_PKG_VERSION"))
                    .map(Some)
                    .unwrap_or_default(),
                user_id,
                api_key_id: None,
                is_unreachable: false,
            },
        });
    }

    // Cloud Daemon on app-server-01
    if let (Some(host), Some(subnet)) =
        (find_host("app-server-01"), find_subnet("Cloud Production"))
    {
        let network = find_network("Cloud");
        daemons.push(Daemon {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonBase {
                host_id: host.id,
                network_id: network.id,
                url: "https://app-01.cloud.acme.io:8443".to_string(),
                last_seen: Some(now),
                capabilities: DaemonCapabilities {
                    has_docker_socket: true,
                    interfaced_subnet_ids: vec![subnet.id],
                },
                mode: DaemonMode::DaemonPoll,
                name: "Cloud Daemon".to_string(),
                tags: vec![],
                version: Version::parse(env!("CARGO_PKG_VERSION"))
                    .map(Some)
                    .unwrap_or_default(),
                user_id,
                api_key_id: None,
                is_unreachable: false,
            },
        });
    }

    // Denver Daemon on denver-nas
    if let (Some(host), Some(subnet)) = (find_host("denver-nas"), find_subnet("Denver Office LAN"))
    {
        let network = find_network("Denver");
        daemons.push(Daemon {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonBase {
                host_id: host.id,
                network_id: network.id,
                url: "https://nas.denver.acme.local:8443".to_string(),
                last_seen: Some(now),
                capabilities: DaemonCapabilities {
                    has_docker_socket: false,
                    interfaced_subnet_ids: vec![subnet.id],
                },
                mode: DaemonMode::DaemonPoll,
                name: "Denver Daemon".to_string(),
                tags: vec![],
                version: Version::parse(env!("CARGO_PKG_VERSION"))
                    .map(Some)
                    .unwrap_or_default(),
                user_id,
                api_key_id: None,
                is_unreachable: false,
            },
        });
    }

    // Riverside Daemon on rm-dc01
    if let (Some(host), Some(subnet)) = (find_host("rm-dc01"), find_subnet("Riverside LAN")) {
        let network = find_network("Riverside");
        daemons.push(Daemon {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonBase {
                host_id: host.id,
                network_id: network.id,
                url: "https://dc01.riverside-medical.local:8443".to_string(),
                last_seen: Some(now),
                capabilities: DaemonCapabilities {
                    has_docker_socket: false,
                    interfaced_subnet_ids: vec![subnet.id],
                },
                mode: DaemonMode::DaemonPoll,
                name: "Riverside Daemon".to_string(),
                tags: vec![],
                version: Some(Version::new(0, 13, 5)),
                user_id,
                api_key_id: None,
                is_unreachable: false,
            },
        });
    }

    daemons
}

// ============================================================================
// API Keys
// ============================================================================

fn generate_api_keys(networks: &[Network], now: DateTime<Utc>) -> Vec<DaemonApiKey> {
    let find_network = |name: &str| {
        networks
            .iter()
            .find(|n| n.base.name.contains(name))
            .unwrap()
    };

    vec![
        DaemonApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonApiKeyBase {
                key: format!("demo_hq_{}", Uuid::new_v4().simple()),
                name: "HQ Daemon Key".to_string(),
                last_used: Some(now),
                expires_at: None,
                network_id: find_network("Headquarters").id,
                is_enabled: true,
                tags: vec![],
                plaintext: None,
            },
        },
        DaemonApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonApiKeyBase {
                key: format!("demo_cloud_{}", Uuid::new_v4().simple()),
                name: "Cloud Daemon Key".to_string(),
                last_used: Some(now),
                expires_at: None,
                network_id: find_network("Cloud").id,
                is_enabled: true,
                tags: vec![],
                plaintext: None,
            },
        },
        DaemonApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonApiKeyBase {
                key: format!("demo_denver_{}", Uuid::new_v4().simple()),
                name: "Denver Daemon Key".to_string(),
                last_used: Some(now),
                expires_at: None,
                network_id: find_network("Denver").id,
                is_enabled: true,
                tags: vec![],
                plaintext: None,
            },
        },
        DaemonApiKey {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: DaemonApiKeyBase {
                key: format!("demo_riverside_{}", Uuid::new_v4().simple()),
                name: "Riverside Daemon Key".to_string(),
                last_used: Some(now),
                expires_at: None,
                network_id: find_network("Riverside").id,
                is_enabled: true,
                tags: vec![],
                plaintext: None,
            },
        },
    ]
}

// ============================================================================
// Groups
// ============================================================================

/// Generate demo groups using actual created services.
/// This must be called AFTER services are created to ensure binding IDs are correct.
pub fn generate_groups(networks: &[Network], services: &[Service], tags: &[Tag]) -> Vec<Group> {
    let now = Utc::now();
    let cloud = networks
        .iter()
        .find(|n| n.base.name.contains("Cloud"))
        .unwrap();
    let hq = networks
        .iter()
        .find(|n| n.base.name == "Headquarters")
        .unwrap();
    let denver = networks
        .iter()
        .find(|n| n.base.name.contains("Denver"))
        .unwrap();
    let riverside = networks
        .iter()
        .find(|n| n.base.name.contains("Riverside"))
        .unwrap();

    let monitoring_tag = tags
        .iter()
        .find(|t| t.base.name == "Monitoring")
        .map(|t| t.id);
    let backup_tag = tags
        .iter()
        .find(|t| t.base.name == "Backup Target")
        .map(|t| t.id);

    // Find service bindings for groups
    let find_service_binding = |name: &str| -> Option<Uuid> {
        services
            .iter()
            .find(|s| s.base.name.contains(name))
            .and_then(|s| s.base.bindings.first())
            .map(|b| b.id())
    };

    let mut groups = Vec::new();

    // Web Traffic Flow: Traefik -> App Servers -> PostgreSQL
    let traefik_binding = find_service_binding("Traefik");
    let app_binding = find_service_binding("Web Application");
    let pg_binding = find_service_binding("PostgreSQL Primary");

    if let (Some(traefik), Some(app), Some(pg)) = (traefik_binding, app_binding, pg_binding) {
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Web Traffic Flow".to_string(),
                network_id: cloud.id,
                description: Some("Production web request path from load balancer through app servers to database".to_string()),
                group_type: GroupType::RequestPath,
                binding_ids: vec![traefik, app, pg],
                source: EntitySource::Manual,
                color: Color::Blue,
                edge_style: EdgeStyle::Bezier,
                tags: vec![],
            },
        });
    }

    // Monitoring Stack: Prometheus -> Grafana (Hub and Spoke)
    let prometheus_binding = find_service_binding("Prometheus");
    let grafana_binding = find_service_binding("Grafana");
    let uptime_binding = find_service_binding("Uptime Kuma");

    if let (Some(prometheus), Some(grafana)) = (prometheus_binding, grafana_binding) {
        let mut bindings = vec![prometheus, grafana];
        if let Some(uptime) = uptime_binding {
            bindings.push(uptime);
        }
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Monitoring Stack".to_string(),
                network_id: hq.id,
                description: Some(
                    "Prometheus metrics collection with Grafana visualization".to_string(),
                ),
                group_type: GroupType::HubAndSpoke,
                binding_ids: bindings,
                source: EntitySource::Manual,
                color: Color::Purple,
                edge_style: EdgeStyle::Straight,
                tags: monitoring_tag.into_iter().collect(),
            },
        });
    }

    // Backup Flow: Servers -> TrueNAS
    let truenas_binding = find_service_binding("TrueNAS");
    let proxmox_binding = find_service_binding("Proxmox");

    if let (Some(proxmox), Some(truenas)) = (proxmox_binding, truenas_binding) {
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Backup Flow".to_string(),
                network_id: hq.id,
                description: Some("Server backup targets to TrueNAS storage".to_string()),
                group_type: GroupType::RequestPath,
                binding_ids: vec![proxmox, truenas],
                source: EntitySource::Manual,
                color: Color::Green,
                edge_style: EdgeStyle::SmoothStep,
                tags: vec![],
            },
        });
    }

    // HQ Network Access Path: pfSense (Management) -> Portainer (Servers)
    // Cross-subnet group spanning Management and Servers subnets
    let pfsense_binding = find_service_binding("pfSense");
    let portainer_binding = find_service_binding("Portainer");

    if let (Some(pfsense), Some(portainer)) = (pfsense_binding, portainer_binding) {
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Network Access Path".to_string(),
                network_id: hq.id,
                description: Some("Traffic path from firewall to application servers".to_string()),
                group_type: GroupType::RequestPath,
                binding_ids: vec![pfsense, portainer],
                source: EntitySource::Manual,
                color: Color::Cyan,
                edge_style: EdgeStyle::Bezier,
                tags: vec![],
            },
        });
    }

    // Denver VPN Connection: OPNsense (LAN) -> WireGuard (VPN Tunnel)
    // Cross-subnet group spanning Office LAN and VPN Tunnel subnets
    let opnsense_binding = find_service_binding("OPNsense");
    let wireguard_binding = find_service_binding("WireGuard");

    if let (Some(opnsense), Some(wireguard)) = (opnsense_binding, wireguard_binding) {
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "VPN Connection".to_string(),
                network_id: denver.id,
                description: Some("VPN tunnel connection to headquarters".to_string()),
                group_type: GroupType::RequestPath,
                binding_ids: vec![opnsense, wireguard],
                source: EntitySource::Manual,
                color: Color::Teal,
                edge_style: EdgeStyle::Bezier,
                tags: vec![],
            },
        });
    }

    // Riverside Backup Path: Active Directory (LAN) -> Veeam (Management)
    // Cross-subnet group spanning LAN and Management subnets
    let ad_binding = find_service_binding("Active Directory");
    let veeam_binding = find_service_binding("Veeam");

    if let (Some(ad), Some(veeam)) = (ad_binding, veeam_binding) {
        groups.push(Group {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base: GroupBase {
                name: "Backup Path".to_string(),
                network_id: riverside.id,
                description: Some("Domain controller backup to Veeam server".to_string()),
                group_type: GroupType::RequestPath,
                binding_ids: vec![ad, veeam],
                source: EntitySource::Manual,
                color: Color::Green,
                edge_style: EdgeStyle::SmoothStep,
                tags: backup_tag.into_iter().collect(),
            },
        });
    }

    groups
}
