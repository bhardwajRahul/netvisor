use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{
    ArpTable, BridgeTable, CdpTable, EntityTable, IpAddrTable,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-aruba-01",
        ip: Ipv4Addr::new(192, 168, 7, 241),
        purpose: Purpose::Regression {
            issue: "#649",
            defect: "neighbours advertised with locally-assigned port ids (subtype 7); treating those as unresolvable stops at the host and draws no edge",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("ProCurve J9145A 2910al-24G, revision W.15.16.0007".into()),
            sys_object_id: Some("1.3.6.1.4.1.11.2.3.7.11.79".into()),
            sys_name: Some("switch-aruba-01".into()),
            sys_location: Some("Floor 1, IDF B".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Normal,
        suppresses: Vec::new(),
    }
}

fn tables() -> Tables {
    Tables {
        if_table: Some(if_table()),
        lldp: Some(lldp_table()),
        bridge: bridge_table(),
        arp: ArpTable::default(),
        ip_addr: IpAddrTable::default(),
        entity: EntityTable::default(),
        cdp: CdpTable::default(),
        lldp_variants: Vec::new(),
        context_bridge: None,
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(41, "41", Some("00:0c:29:aa:bb:29".parse().unwrap())).name("41"),
        IfRow::port(42, "42", Some("00:0c:29:aa:bb:2a".parse().unwrap()))
            .name("42")
            .oper_down(),
        IfRow::port(197, "A5", Some("00:0c:29:aa:bb:c5".parse().unwrap())).name("A5"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:0c:29:aa:bb:c0".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-aruba-01",
    )
    .sys_desc("ProCurve J9145A 2910al-24G, revision W.15.16.0007")
    .local_ports(vec![
        LocalPort::new(
            41,
            Advertised::octets(LldpPortId::InterfaceName("41".into())),
        ),
        LocalPort::new(
            42,
            Advertised::octets(LldpPortId::InterfaceName("42".into())),
        ),
        LocalPort::new(
            197,
            Advertised::octets(LldpPortId::InterfaceName("A5".into())),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            41,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::text(
                LldpPortId::MacAddress("00:1a:2b:3c:4d:65".into()),
                MacEncoding::AsciiLower,
            ),
        )
        .port_desc("g1")
        .sys_name("switch-netgear-01")
        .sys_desc("NETGEAR GS724Tv3 ProSAFE 24-port Gigabit Smart Switch"),
        RemoteNeighbour::new(
            197,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::text(
                LldpPortId::MacAddress("00:1a:2b:3c:4d:66".into()),
                MacEncoding::AsciiLower,
            ),
        )
        .port_desc("g2")
        .sys_name("switch-netgear-01")
        .sys_desc("NETGEAR GS724Tv3 ProSAFE 24-port Gigabit Smart Switch"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}
