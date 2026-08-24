use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour, TimeMark,
};
use crate::daemon::discovery::integration::snmp::sim::mibs::{
    ArpTable, BridgeTable, CdpTable, EntityTable, FdbEntry, IpAddrTable,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfNumber, IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::interfaces::r#impl::base::if_type;
use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-dell-01",
        ip: Ipv4Addr::new(192, 168, 7, 250),
        purpose: Purpose::Regression {
            issue: "#685",
            defect: "OS10 breakout port names carry both anchor characters, and lldpLocPortNum is a separate namespace running 4 and 555-570",
        },
        credential: CredentialType::SnmpV2c { community: inline("netdefault") },
        system: SystemInfo {
            sys_descr: Some("Dell EMC Networking OS10 Enterprise. Dell EMC Networking S4112T-ON. OS Version 10.4.3.4".into()),
            sys_object_id: Some("1.3.6.1.4.1.674.11000.5000.100.2.1".into()),
            sys_name: Some("switch-dell-01".into()),
            sys_location: Some("Rack 4, breakout panel".into()),
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
        IfRow::virtual_if(1, "lo", if_type::SOFTWARE_LOOPBACK)
            .mtu(65535)
            .name("lo")
            .high_speed(),
        IfRow::port(
            17301505,
            "ethernet1/1/1",
            Some("14:18:77:aa:bb:11".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/1")
        .high_speed(),
        IfRow::port(
            17301506,
            "ethernet1/1/2",
            Some("14:18:77:aa:bb:12".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/2")
        .high_speed(),
        IfRow::port(
            17301507,
            "ethernet1/1/3",
            Some("14:18:77:aa:bb:13".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/3")
        .high_speed(),
        IfRow::port(
            17301508,
            "ethernet1/1/4",
            Some("14:18:77:aa:bb:14".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/4")
        .high_speed(),
        IfRow::port(
            17301509,
            "ethernet1/1/5",
            Some("14:18:77:aa:bb:15".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/5")
        .high_speed()
        .oper_down(),
        IfRow::port(
            17301510,
            "ethernet1/1/6",
            Some("14:18:77:aa:bb:16".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/6")
        .high_speed()
        .oper_down(),
        IfRow::port(
            17301511,
            "ethernet1/1/7",
            Some("14:18:77:aa:bb:17".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/7")
        .high_speed()
        .oper_down(),
        IfRow::port(
            17301512,
            "ethernet1/1/8",
            Some("14:18:77:aa:bb:18".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/8")
        .high_speed()
        .oper_down(),
        IfRow::port(
            17301513,
            "ethernet1/1/9",
            Some("14:18:77:aa:bb:19".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/9")
        .high_speed()
        .oper_down(),
        IfRow::port(
            17301514,
            "ethernet1/1/10",
            Some("14:18:77:aa:bb:1a".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/10")
        .high_speed()
        .oper_down(),
        IfRow::port(
            17301515,
            "ethernet1/1/11",
            Some("14:18:77:aa:bb:1b".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/11")
        .high_speed()
        .oper_down(),
        IfRow::port(
            17301516,
            "ethernet1/1/12",
            Some("14:18:77:aa:bb:1c".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/12")
        .high_speed()
        .oper_down(),
        IfRow::port(
            17301517,
            "ethernet1/1/13",
            Some("14:18:77:aa:bb:1d".parse().unwrap()),
        )
        .mtu(1532)
        .speed(10000000000)
        .name("ethernet1/1/13")
        .high_speed()
        .oper_down(),
        IfRow::port(
            17301518,
            "ethernet1/1/14:1",
            Some("14:18:77:aa:bb:21".parse().unwrap()),
        )
        .mtu(1532)
        .speed(25000000000)
        .name("ethernet1/1/14:1")
        .high_speed()
        .alias("breakout lane 1"),
        IfRow::port(
            17301519,
            "ethernet1/1/14:2",
            Some("14:18:77:aa:bb:22".parse().unwrap()),
        )
        .mtu(1532)
        .speed(25000000000)
        .name("ethernet1/1/14:2")
        .high_speed()
        .alias("breakout lane 2"),
        IfRow::port(
            17301520,
            "ethernet1/1/14:3",
            Some("14:18:77:aa:bb:23".parse().unwrap()),
        )
        .mtu(1532)
        .speed(25000000000)
        .name("ethernet1/1/14:3")
        .high_speed()
        .alias("breakout lane 3"),
        IfRow::virtual_if(22020097, "port-channel1", if_type::IEEE8023AD_LAG)
            .mtu(1532)
            .speed(20000000000)
            .name("port-channel1")
            .high_speed()
            .alias("uplink lag")
            .oper_down(),
        IfRow::virtual_if(22020106, "port-channel10", if_type::IEEE8023AD_LAG)
            .mtu(1532)
            .name("port-channel10")
            .high_speed()
            .down(),
        IfRow::port(
            35127296,
            "mgmt1/1/1",
            Some("14:18:77:aa:bb:01".parse().unwrap()),
        )
        .mtu(1532)
        .name("mgmt1/1/1")
        .high_speed()
        .alias("out of band"),
        IfRow::virtual_if(1107787777, "vlan1", if_type::PROP_VIRTUAL)
            .mtu(1532)
            .name("vlan1")
            .high_speed(),
        IfRow::virtual_if(1107787876, "vlan100", if_type::PROP_VIRTUAL)
            .mtu(1532)
            .name("vlan100")
            .high_speed(),
        IfRow::virtual_if(1107787976, "vlan200", if_type::PROP_VIRTUAL)
            .mtu(1532)
            .name("vlan200")
            .high_speed(),
    ])
    .declaring(IfNumber::Declares(52))
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress("14:18:77:aa:bb:00".into())),
        "switch-dell-01",
    )
    .sys_desc(
        "Dell EMC Networking OS10 Enterprise. Dell EMC Networking S4112T-ON. OS Version 10.4.3.4",
    )
    .local_ports(vec![
        LocalPort::new(
            4,
            Advertised::octets(LldpPortId::InterfaceName("mgmt1/1/1".into())),
        )
        .desc("mgmt1/1/1"),
        LocalPort::new(
            555,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/1".into())),
        )
        .desc("ethernet1/1/1"),
        LocalPort::new(
            556,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/2".into())),
        )
        .desc("ethernet1/1/2"),
        LocalPort::new(
            557,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/3".into())),
        )
        .desc("ethernet1/1/3"),
        LocalPort::new(
            558,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/4".into())),
        )
        .desc("ethernet1/1/4"),
        LocalPort::new(
            559,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/5".into())),
        )
        .desc("ethernet1/1/5"),
        LocalPort::new(
            560,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/6".into())),
        )
        .desc("ethernet1/1/6"),
        LocalPort::new(
            561,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/7".into())),
        )
        .desc("ethernet1/1/7"),
        LocalPort::new(
            562,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/8".into())),
        )
        .desc("ethernet1/1/8"),
        LocalPort::new(
            563,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/9".into())),
        )
        .desc("ethernet1/1/9"),
        LocalPort::new(
            564,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/10".into())),
        )
        .desc("ethernet1/1/10"),
        LocalPort::new(
            565,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/11".into())),
        )
        .desc("ethernet1/1/11"),
        LocalPort::new(
            566,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/12".into())),
        )
        .desc("ethernet1/1/12"),
        LocalPort::new(
            567,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/13".into())),
        )
        .desc("ethernet1/1/13"),
        LocalPort::new(
            568,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/14:1".into())),
        )
        .desc("ethernet1/1/14:1"),
        LocalPort::new(
            569,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/14:2".into())),
        )
        .desc("ethernet1/1/14:2"),
        LocalPort::new(
            570,
            Advertised::octets(LldpPortId::InterfaceName("ethernet1/1/14:3".into())),
        )
        .desc("ethernet1/1/14:3"),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            570,
            Advertised::octets(LldpChassisId::LocallyAssigned("TAMMIERENEW".into())),
            Advertised::octets(LldpPortId::MacAddress("9c:6b:00:41:8d:21".into())),
        )
        .time_mark(TimeMark::At(31577700))
        .index(55)
        .port_desc("Realtek PCIe GbE Family Controller")
        .sys_name("TAMMIERENEW")
        .sys_desc("Windows 11 Pro 10.0.26100 x64"),
        RemoteNeighbour::new(
            4,
            Advertised::octets(LldpChassisId::MacAddress("f6:6b:d4:b4:b9:df".into())),
            Advertised::octets(LldpPortId::MacAddress("f6:6b:d4:b4:b9:df".into())),
        )
        .time_mark(TimeMark::At(93300700))
        .index(78),
        RemoteNeighbour::new(
            568,
            Advertised::octets(LldpChassisId::LocallyAssigned("EVILCORP".into())),
            Advertised::octets(LldpPortId::MacAddress("3c:ec:ef:40:12:aa".into())),
        )
        .time_mark(TimeMark::At(123380800))
        .index(85)
        .port_desc("Intel(R) Ethernet Controller X550")
        .sys_name("EVILCORP")
        .sys_desc("Ubuntu 24.04.1 LTS Linux 6.8.0-51-generic x86_64"),
        RemoteNeighbour::new(
            569,
            Advertised::octets(LldpChassisId::LocallyAssigned("VIRTUALPC".into())),
            Advertised::octets(LldpPortId::MacAddress("00:15:5d:01:64:0c".into())),
        )
        .time_mark(TimeMark::At(127153800))
        .index(87)
        .port_desc("Hyper-V Virtual Ethernet Adapter")
        .sys_name("VIRTUALPC")
        .sys_desc("Windows Server 2022 Datacenter 10.0.20348 x64"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::with_ports(vec![
        (1, 17301505),
        (2, 17301506),
        (3, 17301507),
        (4, 17301508),
        (10, 17301514),
        (11, 17301515),
        (12, 17301516),
    ])
    .fdb(vec![
        FdbEntry::learned("00:1a:2b:00:10:01".parse().unwrap(), 1),
        FdbEntry::learned("00:1a:2b:00:11:01".parse().unwrap(), 1),
        FdbEntry::learned("00:1a:2b:00:12:01".parse().unwrap(), 2),
        FdbEntry::learned("00:1a:2b:00:13:01".parse().unwrap(), 3),
        FdbEntry::learned("00:1a:2b:00:14:01".parse().unwrap(), 4),
        FdbEntry::learned("00:1a:2b:00:15:01".parse().unwrap(), 4),
        FdbEntry::learned("14:18:77:aa:bb:11".parse().unwrap(), 10),
        FdbEntry::learned("14:18:77:aa:bb:12".parse().unwrap(), 11),
        FdbEntry::learned("14:18:77:aa:bb:13".parse().unwrap(), 12),
    ])
}
