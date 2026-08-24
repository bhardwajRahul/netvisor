use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::mibs::{
    ArpTable, BridgeTable, CdpTable, EntityTable, FdbEntry, IpAddrTable,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::{
    CredentialType, SnmpV3AuthProtocol, SnmpV3PrivProtocol,
};
use crate::server::interfaces::r#impl::base::if_type;

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-cisco-01",
        ip: Ipv4Addr::new(192, 168, 7, 251),
        purpose: Purpose::Regression {
            issue: "#686",
            defect: "IOS-XE partitions its forwarding database per VLAN, so a scan that cannot name a context reads the wrong table and is told nothing is wrong",
        },
        credential: CredentialType::SnmpV3 {
            security_name: "scanopyctx".into(),
            auth_protocol: SnmpV3AuthProtocol::Sha256,
            auth_password: inline("ctxauthpass12345"),
            priv_protocol: SnmpV3PrivProtocol::Aes128,
            priv_password: inline("ctxprivpass12345"), context_name: Some("vlan-20".into()),
        },
        system: SystemInfo {
            sys_descr: Some("Cisco IOS Software [Fuji], Catalyst L3 Switch Software (CAT3K_CAA-UNIVERSALK9-M), Version 16.9.5".into()),
            sys_object_id: Some("1.3.6.1.4.1.9.1.1745".into()),
            sys_name: Some("switch-cisco-01".into()),
            sys_location: Some("Server Room B, Rack 2".into()),
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
        lldp: None,
        bridge: bridge_table(),
        arp: ArpTable::default(),
        ip_addr: IpAddrTable::default(),
        entity: EntityTable::default(),
        cdp: CdpTable::default(),
        lldp_variants: Vec::new(),
        context_bridge: Some(vlan20_bridge_table()),
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "GigabitEthernet1/0/1",
            Some("00:1e:4a:7c:3b:01".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/1")
        .high_speed(),
        IfRow::port(
            2,
            "GigabitEthernet1/0/2",
            Some("00:1e:4a:7c:3b:02".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/2")
        .high_speed(),
        IfRow::port(
            3,
            "GigabitEthernet1/0/3",
            Some("00:1e:4a:7c:3b:03".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/3")
        .high_speed(),
        IfRow::port(
            4,
            "GigabitEthernet1/0/4",
            Some("00:1e:4a:7c:3b:04".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/4")
        .high_speed(),
        IfRow::port(
            5,
            "GigabitEthernet1/0/5",
            Some("00:1e:4a:7c:3b:05".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/5")
        .high_speed(),
        IfRow::port(
            6,
            "GigabitEthernet1/0/6",
            Some("00:1e:4a:7c:3b:06".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/6")
        .high_speed(),
        IfRow::port(
            7,
            "GigabitEthernet1/0/7",
            Some("00:1e:4a:7c:3b:07".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/7")
        .high_speed(),
        IfRow::port(
            8,
            "GigabitEthernet1/0/8",
            Some("00:1e:4a:7c:3b:08".parse().unwrap()),
        )
        .mtu(1500)
        .name("GigabitEthernet1/0/8")
        .high_speed(),
        IfRow::virtual_if(101, "Vlan1", if_type::PROP_VIRTUAL)
            .mtu(1500)
            .name("Vlan1")
            .high_speed(),
        IfRow::virtual_if(120, "Vlan20", if_type::PROP_VIRTUAL)
            .mtu(1500)
            .name("Vlan20")
            .high_speed(),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived().fdb(vec![FdbEntry::learned(
        "00:50:56:9a:14:01".parse().unwrap(),
        1,
    )])
}

pub fn vlan20_bridge_table() -> BridgeTable {
    BridgeTable::derived().fdb(vec![
        FdbEntry::learned("00:50:56:9a:20:01".parse().unwrap(), 1),
        FdbEntry::learned("00:50:56:9a:20:02".parse().unwrap(), 2),
        FdbEntry::learned("00:50:56:9a:20:03".parse().unwrap(), 2),
        FdbEntry::learned("00:50:56:9a:20:04".parse().unwrap(), 3),
        FdbEntry::learned("00:50:56:9a:20:05".parse().unwrap(), 4),
        FdbEntry::learned("00:50:56:9a:20:06".parse().unwrap(), 5),
        FdbEntry::learned("00:50:56:9a:20:07".parse().unwrap(), 6),
        FdbEntry::learned("00:50:56:9a:20:08".parse().unwrap(), 7),
        FdbEntry::learned("00:50:56:9a:20:09".parse().unwrap(), 8),
    ])
}
