use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, RemoteNeighbour,
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
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-access-01",
        ip: Ipv4Addr::new(192, 168, 7, 231),
        purpose: Purpose::Control {
            role: "a far end named by switch-core-01's Gi0/1",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some(
                "Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11"
                    .into(),
            ),
            sys_object_id: Some("1.3.6.1.4.1.9.1.516".into()),
            sys_name: Some("switch-access-01".into()),
            sys_location: Some("Floor 2, IDF B".into()),
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
        IfRow::port(
            1,
            "GigabitEthernet0/1",
            Some("00:1a:2b:00:11:01".parse().unwrap()),
        )
        .name("Gi0/1")
        .high_speed()
        .alias("Uplink to switch-core-01"),
        IfRow::port(
            2,
            "GigabitEthernet0/2",
            Some("00:1a:2b:00:11:02".parse().unwrap()),
        )
        .name("Gi0/2")
        .high_speed()
        .alias("Access port - Floor 2"),
        IfRow::port(
            3,
            "GigabitEthernet0/3",
            Some("00:1a:2b:00:11:03".parse().unwrap()),
        )
        .name("Gi0/3")
        .high_speed()
        .alias("Downlink to ap-wireless-01"),
    ])
}

pub fn lldp_table() -> LldpTable {
    LldpTable::new(
        Advertised::text(
            LldpChassisId::MacAddress("00:1a:2b:00:11:00".into()),
            MacEncoding::AsciiLower,
        ),
        "switch-access-01",
    )
    .sys_desc("Cisco IOS Software, C3750 Software (C3750-IPSERVICESK9-M), Version 15.0(2)SE11")
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:10:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("Gi0/1".into())),
        )
        .port_desc("GigabitEthernet0/1")
        .sys_name("switch-core-01")
        .sys_desc("Cisco IOS Software, C2960 Software (C2960-LANBASEK9-M), Version 15.2(7)E3"),
        RemoteNeighbour::new(
            3,
            Advertised::text(
                LldpChassisId::MacAddress("00:1a:2b:00:15:00".into()),
                MacEncoding::AsciiLower,
            ),
            Advertised::octets(LldpPortId::InterfaceName("eth0".into())),
        )
        .port_desc("eth0")
        .sys_name("ap-wireless-01")
        .sys_desc("Ubiquiti UniFi AP AC Pro, firmware 6.5.28"),
    ])
}

pub fn bridge_table() -> BridgeTable {
    BridgeTable::derived()
}

#[cfg(test)]
mod tests {
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// A control device: it exists to be a far end. `switch-core-01` advertises it on `Gi0/1` by
    /// the chassis id below, so this is the assertion that keeps that test honest — if this
    /// device stops reporting these values, the neighbour over there resolves through a fallback
    /// tier and looks identical while proving nothing.
    #[tokio::test]
    async fn it_reports_the_identity_switch_core_01_names_it_by() {
        let scan = harness::scan("switch-access-01").await;

        assert_eq!(scan.if_table.entries.len(), 3);
        assert!(scan.if_table.set_complete);
        assert_eq!(scan.interface(1).if_name.as_deref(), Some("Gi0/1"));
        assert_eq!(
            scan.interface(1)
                .if_phys_address
                .map(|m| m.to_string().to_lowercase()),
            Some("00:1a:2b:00:11:01".to_string())
        );
    }
}
