use std::net::Ipv4Addr;

use crate::daemon::discovery::integration::snmp::sim::lldp::{
    Advertised, LldpTable, LocalPort, RemoteNeighbour,
};
use crate::daemon::discovery::integration::snmp::sim::tables::{IfRow, IfTable};
use crate::daemon::discovery::integration::snmp::sim::transport::Handler;
use crate::daemon::discovery::integration::snmp::sim::{Purpose, SimDevice, Tables};
use crate::daemon::discovery::integration::snmp::types::SystemInfo;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::lldp::{LldpChassisId, LldpPortId};

use super::inline;

/// The uplink neighbours' management addresses, on a range no device in the lab holds.
///
/// Deliberately outside `192.168.7.0/24`: these far ends are reachable from this switch and from
/// nowhere the daemon can see, which is the case the whole management-address tier exists for.
const OFFSITE_CORE: &str = "10.20.30.11";
const OFFSITE_EDGE: &str = "10.20.30.24";

pub fn device() -> SimDevice {
    SimDevice {
        name: "switch-offsite-01",
        ip: Ipv4Addr::new(192, 168, 7, 252),
        purpose: Purpose::Regression {
            issue: "GH #668",
            defect: "its neighbours publish a management address and nothing else this network \
                     holds, so before the address tier every one of them resolved to nothing and \
                     the ports drew as unconnected",
        },
        credential: CredentialType::SnmpV2c {
            community: inline("netdefault"),
        },
        system: SystemInfo {
            sys_descr: Some("Offsite aggregation switch, 8-port".into()),
            sys_object_id: Some("1.3.6.1.4.1.99999.3.1".into()),
            sys_name: Some("switch-offsite-01".into()),
            sys_location: Some("Rack 2, offsite".into()),
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
        lldp: Some(lldp()),
        ..Default::default()
    }
}

fn if_table() -> IfTable {
    IfTable::new(vec![
        IfRow::port(
            1,
            "GigabitEthernet0/1",
            Some("00:1a:2b:00:fc:01".parse().unwrap()),
        )
        .speed(1000000000)
        .name("Gi0/1")
        .high_speed(),
        IfRow::port(
            2,
            "GigabitEthernet0/2",
            Some("00:1a:2b:00:fc:02".parse().unwrap()),
        )
        .speed(1000000000)
        .name("Gi0/2")
        .high_speed(),
        IfRow::port(
            3,
            "GigabitEthernet0/3",
            Some("00:1a:2b:00:fc:03".parse().unwrap()),
        )
        .speed(1000000000)
        .name("Gi0/3")
        .high_speed(),
    ])
}

/// Three uplinks, and the three shapes the address tier has to tell apart.
///
/// Ports 1 and 2 name far ends that publish a management address, both in the same `10.20.30.0/24`
/// — that pair is what the server-side bucketing folds into one inferred subnet. Port 3 names a far
/// end that publishes none, which stays unplaceable no matter how much of the network is scanned
/// and is the control the other two are read against.
fn lldp() -> LldpTable {
    LldpTable::new(
        Advertised::octets(LldpChassisId::MacAddress("00:1a:2b:00:fc:00".to_string())),
        "switch-offsite-01",
    )
    .sys_desc("Offsite aggregation switch, 8-port")
    .local_ports(vec![
        LocalPort::new(
            1,
            Advertised::octets(LldpPortId::InterfaceName("GigabitEthernet0/1".to_string())),
        ),
        LocalPort::new(
            2,
            Advertised::octets(LldpPortId::InterfaceName("GigabitEthernet0/2".to_string())),
        ),
        LocalPort::new(
            3,
            Advertised::octets(LldpPortId::InterfaceName("GigabitEthernet0/3".to_string())),
        ),
    ])
    .neighbours(vec![
        RemoteNeighbour::new(
            1,
            Advertised::octets(LldpChassisId::MacAddress("00:ad:24:89:cc:f0".to_string())),
            Advertised::octets(LldpPortId::InterfaceName(
                "Ten-GigabitEthernet1/0/1".to_string(),
            )),
        )
        .sys_name("offsite-core-01")
        .mgmt_addr(OFFSITE_CORE.parse().unwrap()),
        RemoteNeighbour::new(
            2,
            Advertised::octets(LldpChassisId::MacAddress("00:ad:24:89:cc:f1".to_string())),
            Advertised::octets(LldpPortId::InterfaceName(
                "GigabitEthernet1/0/8".to_string(),
            )),
        )
        .sys_name("offsite-edge-01")
        .mgmt_addr(OFFSITE_EDGE.parse().unwrap()),
        RemoteNeighbour::new(
            3,
            Advertised::octets(LldpChassisId::MacAddress("00:ad:24:89:cc:f2".to_string())),
            Advertised::octets(LldpPortId::InterfaceName("eth0".to_string())),
        )
        .sys_name("offsite-ap-01"),
    ])
}

#[cfg(test)]
mod tests {
    use super::{OFFSITE_CORE, OFFSITE_EDGE};
    use crate::daemon::discovery::integration::snmp::sim::harness;

    /// The management address arrives in the *index* of `lldpRemManAddrTable`, not in a column
    /// value, so a fixture that served it as a value would look correct and be invisible to the
    /// collector. This is the only end-to-end coverage that the index is composed the way
    /// `split_lldp_man_addr_index` reads it.
    #[tokio::test]
    async fn a_published_management_address_reaches_the_neighbour_record() {
        let scan = harness::scan("switch-offsite-01").await;

        assert!(
            scan.neighbours.complete,
            "the management-address walk must not report the neighbour walk as short"
        );

        let core = scan
            .neighbours_named("offsite-core-01")
            .into_iter()
            .next()
            .expect("the neighbour on Gi0/1");
        assert_eq!(
            core.remote_mgmt_addr,
            Some(OFFSITE_CORE.parse().unwrap()),
            "the address must survive the trip through the OID index"
        );

        let edge = scan
            .neighbours_named("offsite-edge-01")
            .into_iter()
            .next()
            .expect("the neighbour on Gi0/2");
        assert_eq!(edge.remote_mgmt_addr, Some(OFFSITE_EDGE.parse().unwrap()));
    }

    /// A far end that publishes no management address must come back with `None` rather than
    /// inheriting a neighbouring row's — the man-addr table is indexed per neighbour, and reading
    /// its index wrongly would smear one address across every row on the device.
    #[tokio::test]
    async fn a_neighbour_that_publishes_no_address_carries_none() {
        let scan = harness::scan("switch-offsite-01").await;

        let ap = scan
            .neighbours_named("offsite-ap-01")
            .into_iter()
            .next()
            .expect("the neighbour on Gi0/3");
        assert_eq!(ap.remote_mgmt_addr, None);
    }
}
