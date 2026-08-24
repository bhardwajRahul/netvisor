use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, ChassisDefect, LldpTable, RemoteNeighbour,
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
        name: "switch-flaky-01",
        ip: Ipv4Addr::new(192, 168, 7, 243),
        purpose: Purpose::Regression {
            issue: "#668",
            defect: "malformed neighbour records — each variant drives a different discard counter and a different piece of operator advice",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Scanopy SNMP simulator, flaky-LLDP profile".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.1".into()),
            sys_name: Some("switch-flaky-01".into()),
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
        lldp: Some(lldp_complete()),
        bridge: bridge_table(),
        arp: ArpTable::default(),
        ip_addr: IpAddrTable::default(),
        entity: EntityTable::default(),
        cdp: CdpTable::default(),
        lldp_variants: vec![
            ("badsubtype", lldp_badsubtype()),
            ("complete", lldp_complete()),
            ("ghost", lldp_ghost()),
            ("nochassis", lldp_nochassis()),
            ("nosubtype", lldp_nosubtype()),
        ],
        context_bridge: None,
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(1, "uplink0", Some("00:1a:2b:00:1f:01".parse().unwrap())).name("uplink0"),
        IfRow::port(2, "uplink1", Some("00:1a:2b:00:1f:02".parse().unwrap())).name("uplink1"),
    ])
}

pub fn lldp_complete() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
    ])
}

pub fn lldp_nochassis() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::octets(LldpChassisId::MacAddress("00:00:00:00:00:00".into())),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960")
        .defect(ChassisDefect::NoChassisColumns),
    ])
}

pub fn lldp_nosubtype() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960")
        .defect(ChassisDefect::NoSubtype),
    ])
}

pub fn lldp_badsubtype() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960")
        .defect(ChassisDefect::SubtypeWrongType("macAddress")),
    ])
}

pub fn lldp_ghost() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:1f:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-flaky-01",
    )
    .sys_desc("Scanopy SNMP simulator, flaky-LLDP profile")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/3".into())),
        )
        .port_desc("GigabitEthernet0/3")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960"),
        RemoteNeighbour::new(
            2,
            Advertised::octets(LldpChassisId::MacAddress("00:00:00:00:00:00".into())),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/4".into())),
        )
        .port_desc("GigabitEthernet0/4")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960")
        .defect(ChassisDefect::NoChassisColumns),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}
