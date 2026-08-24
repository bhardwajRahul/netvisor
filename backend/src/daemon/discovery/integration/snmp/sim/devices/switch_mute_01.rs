use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::oids::{arp, ip_mib};
use crate::daemon::discovery::integration::snmp::sim::mibs::{
    ArpTable, BridgeTable, CdpTable, EntityTable, IpAddrTable,
};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;

use super::inline;

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-mute-01",
        ip: Ipv4Addr::new(192, 168, 7, 248),
        purpose: Purpose::Regression {
            issue: "the partial-failure reporting",
            defect: "answers the credential and serves nothing, which used to read to an operator as a clean scan",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Mute agent, system MIB only".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.2.1".into()),
            sys_name: Some("switch-mute-01".into()),
            sys_location: Some("Rack 9, top".into()),
            sys_contact: Some("netops@example.com".into()),
            sys_services: Some(2),
            sys_uptime: None,
            // Published from the ifTable at emission, never stored.
            if_number: None,
        },
        tables: tables(),
        arp_handler: Handler::Normal,
        suppresses: vec![
            ip_mib::ip_addr_entry::IP_AD_ENT_ADDR,
            ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
            ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
            arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS,
            arp::entry::IP_NET_TO_MEDIA_TYPE,
        ],
    }
}

fn tables() -> Tables {
    Tables {
        if_table: None,
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
