use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::{
    ArpTable, BridgeTable, CdpTable, EntityTable, IpAddrTable,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "printer-lobby",
        ip: Ipv4Addr::new(192, 168, 7, 234),
        purpose: Purpose::Control {
            role: "an endpoint with no bridge and no neighbours",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("public"),
        },
        system: SystemInfo {
            sys_descr: Some("HP LaserJet Pro MFP M428fdw, FW 2406334_042882".into()),
            sys_object_id: Some("1.3.6.1.4.1.11.2.3.9.1".into()),
            sys_name: Some("printer-lobby".into()),
            sys_location: Some("Lobby, Reception Desk".into()),
            sys_contact: Some("facilities@example.com".into()),
            sys_services: Some(72),
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
        lldp: None,
        bridge: BridgeTable::default(),
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
        IfRow::port(1, "Ethernet", Some("00:1a:2b:00:14:01".parse().unwrap()))
            .speed(100000000)
            .name("Ethernet")
            .high_speed()
            .alias("Network port"),
        IfRow::port(2, "USB", Some("00:1a:2b:00:14:02".parse().unwrap()))
            .speed(480000000)
            .name("USB")
            .high_speed()
            .alias("USB port"),
    ])
}
