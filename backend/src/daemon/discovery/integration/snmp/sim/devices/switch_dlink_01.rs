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
        name: "switch-dlink-01",
        ip: Ipv4Addr::new(192, 168, 7, 244),
        purpose: Purpose::Regression {
            issue: "#668",
            defect: "four neighbour records each needing a different route to their far end, and one chassis MAC repeated across every port",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("D-Link DGS-1210-48 Rev.GX/7.20.003".into()),
            sys_object_id: Some("1.3.6.1.4.1.171.10.76.28".into()),
            sys_name: Some("switch-dlink-01".into()),
            sys_location: Some("Lab".into()),
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
        IfRow::port(
            1,
            "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 1",
            Some("00:ad:24:af:4e:00".parse().unwrap()),
        )
        .name("Slot0/1"),
        IfRow::port(
            2,
            "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 2",
            Some("00:ad:24:af:4e:00".parse().unwrap()),
        )
        .name("Slot0/2"),
        IfRow::port(
            3,
            "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 3",
            Some("00:ad:24:af:4e:00".parse().unwrap()),
        )
        .name("Slot0/3"),
        IfRow::port(
            4,
            "D-Link DGS-1210-48 Rev.GX/7.20.003 Port 4",
            Some("00:ad:24:af:4e:00".parse().unwrap()),
        )
        .name("Slot0/4"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:ad:24:af:4e:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-dlink-01",
    )
    .sys_desc("D-Link DGS-1210-48 Rev.GX/7.20.003")
    .local_ports(vec![
        LocalPort::new(
            1,
            Advertised::octets(LldpPortId::InterfaceName("Slot0/1".into())),
        ),
        LocalPort::new(
            2,
            Advertised::octets(LldpPortId::InterfaceName("Slot0/2".into())),
        ),
        LocalPort::new(
            3,
            Advertised::octets(LldpPortId::InterfaceName("Slot0/3".into())),
        ),
        LocalPort::new(
            4,
            Advertised::octets(LldpPortId::InterfaceName("Slot0/4".into())),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("2".into())),
        )
        .port_desc("GigabitEthernet0/2")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            2,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/0/44".into())),
        )
        .port_desc("GigabitEthernet0/1")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            3,
            Advertised::octets(LldpChassisId::MacAddress("00:07:7c:20:01:e0".into())),
            Advertised::octets(LldpPortId::MacAddress("00:07:7c:20:01:e3".into())),
        )
        .port_desc("Ring port to peer")
        .sys_name("switch-macport-01")
        .sys_desc("Westermo WeOS"),
        RemoteNeighbour::new(
            4,
            Advertised::octets(LldpChassisId::MacAddress("30:de:4b:30:f0:ac".into())),
            Advertised::octets(LldpPortId::MacAddress("30:de:4b:30:f0:ac".into())),
        )
        .port_desc("Uplink")
        .sys_name("switch")
        .sys_desc("TP-Link Omada TL-SG3216"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}
