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
        name: "switch-exos-01",
        ip: Ipv4Addr::new(192, 168, 7, 238),
        purpose: Purpose::Regression {
            issue: "Issue 2, July 2026",
            defect: "ExtremeXOS numbers lldpRemTable local ports in a namespace distinct from ifIndex, so without the lldpLocPortTable remap this switch yields zero neighbours",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("ExtremeXOS version 31.7 X435-24P".into()),
            sys_object_id: Some("1.3.6.1.4.1.1916.2.219".into()),
            sys_name: Some("switch-exos-01".into()),
            sys_location: Some("Floor 3, IDF C".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(6),
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
        IfRow::port(1001, "1:1", Some("00:04:96:01:e0:01".parse().unwrap())).name("1:1"),
        IfRow::port(1002, "1:2", Some("00:04:96:01:e0:02".parse().unwrap())).name("1:2"),
        IfRow::port(1003, "1:3", Some("00:04:96:01:e0:03".parse().unwrap())).name("1:3"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("0:4:96:1:e0:0".into()),
            MacEncoding::AsciiAbbreviated,
        ),
        "switch-exos-01",
    )
    .sys_desc("ExtremeXOS version 31.7 X435-24P")
    .local_ports(vec![
        LocalPort::new(1, Advertised::octets(LldpPortId::InterfaceName("1".into()))),
        LocalPort::new(2, Advertised::octets(LldpPortId::InterfaceName("2".into()))),
        LocalPort::new(3, Advertised::octets(LldpPortId::InterfaceName("3".into()))),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("1".into())),
        )
        .port_desc("1:1")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            3,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:12:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("3".into())),
        )
        .port_desc("1:3")
        .sys_name("router-gw-01")
        .sys_desc("Juniper Networks JunOS MX204"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}
