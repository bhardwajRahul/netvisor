use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::{
    ArpTable, BridgeTable, CdpTable, EntityTable, IpAddrTable,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::{ArpEntry, SystemInfo};
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-stuck-01",
        ip: Ipv4Addr::new(192, 168, 7, 249),
        purpose: Purpose::Regression {
            issue: "the walk's retry-then-stop guard",
            defect: "answers every request for its ARP table with the same row, whatever was asked",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Non-advancing agent, ARP table loops".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.3.1".into()),
            sys_name: Some("switch-stuck-01".into()),
            sys_location: Some("Rack 9, middle".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Stuck,
        suppresses: Vec::new(),
    }
}

fn tables() -> Tables {
    Tables {
        if_table: Some(if_table()),
        lldp: None,
        bridge: bridge_table(),
        arp: arp_table(),
        ip_addr: IpAddrTable::default(),
        entity: EntityTable::default(),
        cdp: CdpTable::default(),
        lldp_variants: Vec::new(),
        context_bridge: None,
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(1, "ether1", Some("00:0c:42:7a:00:01".parse().unwrap())).name("ether1"),
        IfRow::port(2, "ether2", Some("00:0c:42:7a:00:02".parse().unwrap())).name("ether2"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

pub fn arp_table() -> ArpTable {
    ArpTable::index_column(vec![ArpEntry {
        if_index: 1,
        mac_address: "00:00:00:00:00:00".parse().unwrap(),
        ip_address: "10.40.50.1".parse().unwrap(),
    }])
}
