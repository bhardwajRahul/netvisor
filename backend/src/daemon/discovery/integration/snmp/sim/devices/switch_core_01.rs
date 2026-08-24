use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{
    ArpTable, BridgeTable, CdpTable, EntityTable, FdbEntry, FdbStatus, IpAddrTable,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::wire::MacEncoding;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::{
    CdpNeighbor, DeviceInventory, SystemInfo, VlanInfo,
};
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;
use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-core-01",
        ip: Ipv4Addr::new(192, 168, 7, 230),
        purpose: Purpose::Control {
            role: "the lab's baseline switch and the far end most other devices resolve against; also the first forwarding database and the only CDP cache",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some(
                "Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3".into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.9.1.1208".into()),
            sys_name: Some("switch-core-01".into()),
            sys_location: Some("Server Room A, Rack 1".into()),
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
        entity: entity_table(),
        cdp: cdp_table(),
        lldp_variants: Vec::new(),
        context_bridge: None,
    }
}

pub fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "GigabitEthernet0/1",
            Some("00:1a:2b:00:10:01".parse().unwrap()),
        )
        .name("Gi0/1")
        .high_speed()
        .alias("Uplink to switch-access-01"),
        IfRow::port(
            2,
            "GigabitEthernet0/2",
            Some("00:1a:2b:00:10:02".parse().unwrap()),
        )
        .name("Gi0/2")
        .high_speed()
        .alias("Uplink to router-gw-01"),
        IfRow::port(
            3,
            "GigabitEthernet0/3",
            Some("00:1a:2b:00:10:03".parse().unwrap()),
        )
        .name("Gi0/3")
        .high_speed()
        .alias("Server port"),
        IfRow::virtual_if(4, "Vlan10", if_type::PROP_VIRTUAL)
            .mac("00:1a:2b:00:10:00".parse().unwrap())
            .name("Vl10")
            .high_speed()
            .alias("Management VLAN"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-core-01",
    )
    .sys_desc("Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/1".into())),
        )
        .port_desc("GigabitEthernet0/1")
        .sys_name("switch-access-01")
        .sys_desc("Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11"),
        RemoteNeighbour::new(
            2,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:12:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("ge-0/0/0".into())),
        )
        .port_desc("ge-0/0/0")
        .sys_name("router-gw-01")
        .sys_desc("Juniper Networks, Inc. JunOS 21.4R3-S5, MX204"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
        .fdb(vec![
            FdbEntry::learned("00:1a:2b:00:10:00".parse().unwrap(), 0).status(FdbStatus::Self_),
            FdbEntry::learned("00:1a:2b:00:10:01".parse().unwrap(), 1).status(FdbStatus::Mgmt),
            FdbEntry::learned("00:1a:2b:00:11:00".parse().unwrap(), 1),
            FdbEntry::learned("00:1a:2b:00:11:01".parse().unwrap(), 1),
            FdbEntry::learned("00:1a:2b:00:12:01".parse().unwrap(), 2),
            FdbEntry::learned("00:1a:2b:00:13:01".parse().unwrap(), 3),
            FdbEntry::learned("00:1a:2b:00:14:01".parse().unwrap(), 3),
            FdbEntry::learned("00:1a:2b:00:15:01".parse().unwrap(), 3),
            FdbEntry::learned("00:1a:2b:00:11:01".parse().unwrap(), 1).in_vlan(10),
            FdbEntry::learned("00:1a:2b:00:12:01".parse().unwrap(), 2).in_vlan(10),
            FdbEntry::learned("00:1a:2b:00:14:01".parse().unwrap(), 3).in_vlan(20),
        ])
        .vlans(vec![
            VlanInfo {
                vlan_id: 10,
                name: "DATA".into(),
            },
            VlanInfo {
                vlan_id: 20,
                name: "VOICE".into(),
            },
        ])
        .port_vlans(vec![(1, 10), (2, 10), (3, 20)])
}

pub fn entity_table() -> EntityTable {
    EntityTable::chassis(
        DeviceInventory {
            description: Some("Cisco Catalyst 2960-24TC-L".into()),
            manufacturer: Some("Cisco".into()),
            model: Some("WS-C2960-24TC-L".into()),
            serial_number: Some("FOC1234X5YZ".into()),
        },
        "Chassis",
    )
}

pub fn cdp_table() -> CdpTable {
    CdpTable::new(vec![CdpNeighbor {
        local_port_index: 2,
        remote_device_id: Some("router-gw-01".into()),
        remote_port_id: Some("ge-0/0/0".into()),
        remote_platform: Some("Juniper MX204".into()),
        remote_address: None,
    }])
}
