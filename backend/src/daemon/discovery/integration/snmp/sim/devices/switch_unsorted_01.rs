use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::{
    ArpTable, BridgeTable, CdpTable, EntityTable, IpAddrTable,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::{ArpEntry, SystemInfo};
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-unsorted-01",
        ip: Ipv4Addr::new(192, 168, 7, 246),
        purpose: Purpose::Regression {
            issue: "#674",
            defect: "an ARP table served out of ascending OID order; a strictly ascending walk gives up and every multi-column row is discarded",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("PoE switch, firmware V3.3.3".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.1.1".into()),
            sys_name: Some("switch-unsorted-01".into()),
            sys_location: Some("Floor 1, camera room".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Positional,
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
        IfRow::port(
            1,
            "GigabitEthernet0/1",
            Some("00:1f:c6:aa:00:01".parse().unwrap()),
        )
        .name("Gi0/1"),
        IfRow::port(
            2,
            "GigabitEthernet0/2",
            Some("00:1f:c6:aa:00:02".parse().unwrap()),
        )
        .name("Gi0/2"),
        IfRow::virtual_if(3, "Vlan1", if_type::PROP_VIRTUAL)
            .mac("00:1f:c6:aa:00:03".parse().unwrap())
            .name("Vlan1"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

pub fn arp_table() -> ArpTable {
    ArpTable::new(vec![
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:02".parse().unwrap(),
            ip_address: "10.20.30.2".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:04".parse().unwrap(),
            ip_address: "10.20.30.4".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:06".parse().unwrap(),
            ip_address: "10.20.30.6".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:08".parse().unwrap(),
            ip_address: "10.20.30.8".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:10".parse().unwrap(),
            ip_address: "10.20.30.10".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:12".parse().unwrap(),
            ip_address: "10.20.30.12".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:14".parse().unwrap(),
            ip_address: "10.20.30.14".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:16".parse().unwrap(),
            ip_address: "10.20.30.16".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:18".parse().unwrap(),
            ip_address: "10.20.30.18".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:20".parse().unwrap(),
            ip_address: "10.20.30.20".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:22".parse().unwrap(),
            ip_address: "10.20.30.22".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:24".parse().unwrap(),
            ip_address: "10.20.30.24".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:26".parse().unwrap(),
            ip_address: "10.20.30.26".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:28".parse().unwrap(),
            ip_address: "10.20.30.28".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:30".parse().unwrap(),
            ip_address: "10.20.30.30".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:32".parse().unwrap(),
            ip_address: "10.20.30.32".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:34".parse().unwrap(),
            ip_address: "10.20.30.34".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:36".parse().unwrap(),
            ip_address: "10.20.30.36".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:38".parse().unwrap(),
            ip_address: "10.20.30.38".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:40".parse().unwrap(),
            ip_address: "10.20.30.40".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:42".parse().unwrap(),
            ip_address: "10.20.30.42".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:44".parse().unwrap(),
            ip_address: "10.20.30.44".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:01".parse().unwrap(),
            ip_address: "10.20.30.1".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:03".parse().unwrap(),
            ip_address: "10.20.30.3".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:05".parse().unwrap(),
            ip_address: "10.20.30.5".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:07".parse().unwrap(),
            ip_address: "10.20.30.7".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:09".parse().unwrap(),
            ip_address: "10.20.30.9".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:11".parse().unwrap(),
            ip_address: "10.20.30.11".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:13".parse().unwrap(),
            ip_address: "10.20.30.13".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:15".parse().unwrap(),
            ip_address: "10.20.30.15".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:17".parse().unwrap(),
            ip_address: "10.20.30.17".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:19".parse().unwrap(),
            ip_address: "10.20.30.19".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:21".parse().unwrap(),
            ip_address: "10.20.30.21".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:23".parse().unwrap(),
            ip_address: "10.20.30.23".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:25".parse().unwrap(),
            ip_address: "10.20.30.25".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:27".parse().unwrap(),
            ip_address: "10.20.30.27".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:29".parse().unwrap(),
            ip_address: "10.20.30.29".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:31".parse().unwrap(),
            ip_address: "10.20.30.31".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:33".parse().unwrap(),
            ip_address: "10.20.30.33".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:35".parse().unwrap(),
            ip_address: "10.20.30.35".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:37".parse().unwrap(),
            ip_address: "10.20.30.37".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:39".parse().unwrap(),
            ip_address: "10.20.30.39".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:41".parse().unwrap(),
            ip_address: "10.20.30.41".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:43".parse().unwrap(),
            ip_address: "10.20.30.43".parse().unwrap(),
        },
        ArpEntry {
            if_index: 2,
            mac_address: "00:25:90:f0:00:45".parse().unwrap(),
            ip_address: "10.20.30.45".parse().unwrap(),
        },
    ])
}
