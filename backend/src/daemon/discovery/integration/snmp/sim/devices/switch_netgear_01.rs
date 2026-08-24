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
        name: "switch-netgear-01",
        ip: Ipv4Addr::new(192, 168, 7, 240),
        purpose: Purpose::Regression {
            issue: "#664",
            defect: "its LLDP chassis id is on no port and no IP, so the far end is identifiable only through the chassis_id recorded from its own LLDP local identity",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("NETGEAR GS724Tv3 ProSAFE 24-port Gigabit Smart Switch".into()),
            sys_object_id: Some("1.3.6.1.4.1.4526.100.4.15".into()),
            sys_name: Some("switch-netgear-01".into()),
            sys_location: Some("Floor 1, IDF A".into()),
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
        IfRow::port(1, "g1", Some("00:1a:2b:3c:4d:65".parse().unwrap())).name("g1"),
        IfRow::port(2, "g2", Some("00:1a:2b:3c:4d:66".parse().unwrap())).name("g2"),
        IfRow::port(3, "g3", Some("00:1a:2b:3c:4d:67".parse().unwrap()))
            .name("g3")
            .oper_down(),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:3c:4d:63".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-netgear-01",
    )
    .sys_desc("NETGEAR GS724Tv3 ProSAFE 24-port Gigabit Smart Switch")
    .local_ports(vec![
        LocalPort::new(
            1,
            Advertised::octets(LldpPortId::InterfaceName("g1".into())),
        ),
        LocalPort::new(
            2,
            Advertised::octets(LldpPortId::InterfaceName("g2".into())),
        ),
        LocalPort::new(
            3,
            Advertised::octets(LldpPortId::InterfaceName("g3".into())),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:0c:29:aa:bb:c0".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::LocallyAssigned("41".into())),
        )
        .port_desc("41")
        .sys_name("switch-aruba-01")
        .sys_desc("ProCurve J9145A 2910al-24G, revision W.15.16.0007"),
        RemoteNeighbour::new(
            2,
            Advertised::text(
                LldpChassisId::MacAddress("00:0c:29:aa:bb:c0".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::LocallyAssigned("197".into())),
        )
        .port_desc("A5")
        .sys_name("switch-aruba-01")
        .sys_desc("ProCurve J9145A 2910al-24G, revision W.15.16.0007"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}
