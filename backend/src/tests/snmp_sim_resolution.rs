//! The simulated devices, resolved against a real database.
//!
//! These run against Postgres rather than the in-memory inventory on purpose. Every defect below
//! was a *SQL semantics* defect — a lookup with no `ORDER BY` and no `LIMIT` returning an arbitrary
//! row, or a tier that declined when it should have matched — and the fake inventory in
//! `snmp::resolution::lldp`'s tests re-implements those semantics in Rust, so it cannot fail the
//! way the database did.
//!
//! What makes them device tests rather than resolver tests is where the inventory comes from: the
//! far ends are seeded from what a real collection of the *simulated device* produced, so a fixture
//! that stops reporting the identifiers a neighbour is matched on breaks the test that depends on
//! it. Collection-side behaviour is covered without a database in each device's own module.

use std::net::IpAddr;

use mac_address::MacAddress;
use uuid::Uuid;

use crate::daemon::discovery::integration::snmp::sim::harness::{self, Collected};
use crate::daemon::discovery::integration::snmp::types::{IfTableEntry, LldpNeighbor};
use crate::server::hosts::r#impl::name::HostName;
use crate::server::{
    hosts::r#impl::base::Host,
    interfaces::r#impl::base::{IfAdminStatus, IfOperStatus, Interface, InterfaceBase},
    shared::storage::traits::Storage,
    snmp::resolution::{
        lldp::{IdentityResolution, LldpChassisId, LldpPortId},
        resolver::{LldpResolver, LldpResolverImpl},
    },
};

use super::{host, network, organization, subnet, test_services};

/// A network holding simulated devices as scanned hosts, and a resolver pointed at the same
/// database.
struct Lab {
    resolver: LldpResolverImpl,
    storage: crate::server::shared::storage::factory::StorageFactory,
    network_id: Uuid,
    _subnet_id: Uuid,
    _container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
}

impl Lab {
    async fn new() -> Self {
        let (storage, services, _container) = test_services().await;

        let org = organization();
        storage.organizations.create(&org).await.unwrap();
        let network = network(&org.id);
        storage.networks.create(&network).await.unwrap();
        let subnet = subnet(&network.id);
        storage.subnets.create(&subnet).await.unwrap();

        let resolver = LldpResolverImpl::new(
            services.interface_service.clone(),
            services.ip_address_service.clone(),
            storage.hosts.clone(),
        );

        Self {
            resolver,
            network_id: network.id,
            _subnet_id: subnet.id,
            storage,
            _container,
        }
    }

    /// Scan a simulated device and persist what the collection read.
    ///
    /// The host's `chassis_id` and `sys_name` come from the device's own LLDP local identity, and
    /// its interfaces from its ifTable — the same two sources a real scan writes, which is what
    /// makes a neighbour pointing here resolve through the tier it is supposed to.
    async fn scan(&self, name: &str) -> Scanned {
        let device = crate::daemon::discovery::integration::snmp::sim::device(name);
        let collected = harness::collect(&device).await;

        let lldp = device.tables.lldp.as_ref();
        let (subtype, value) = lldp
            .map(|table| table.chassis.id.to_snmp(table.chassis.encoding))
            .expect("every device in these tests advertises LLDP");
        let chassis_id = LldpChassisId::from_snmp(subtype, &value).map(|id| id.identifier());

        let mut record = host(&self.network_id);
        record.base.name = HostName::Manual(name.to_string());
        record.base.chassis_id = chassis_id;
        record.base.sys_name = device.system.sys_name.clone();
        self.storage.hosts.create(&record).await.unwrap();

        for entry in &collected.if_table.entries {
            self.interface(record.id, entry).await;
        }

        Scanned {
            host: record,
            collected,
        }
    }

    async fn interface(&self, host_id: Uuid, entry: &IfTableEntry) -> Interface {
        let interface = Interface::new(InterfaceBase {
            host_id,
            network_id: self.network_id,
            if_index: entry.if_index,
            if_descr: entry.if_descr.clone().unwrap_or_default(),
            if_name: entry.if_name.clone(),
            if_alias: entry.if_alias.clone(),
            if_type: entry.if_type.unwrap_or_default(),
            mac_address: entry.if_phys_address,
            admin_status: IfAdminStatus::Up,
            oper_status: IfOperStatus::Up,
            ..Default::default()
        });
        self.storage.interfaces.create(&interface).await.unwrap();
        interface
    }
}

struct Scanned {
    host: Host,
    collected: Collected,
}

impl Scanned {
    /// The chassis id a neighbour of this device advertises, as the far end reads it.
    fn advertised_chassis(neighbour: &LldpNeighbor) -> LldpChassisId {
        LldpChassisId::from_snmp(
            neighbour.remote_chassis_id_subtype.expect("a subtype"),
            neighbour.remote_chassis_id_bytes.as_ref().expect("a value"),
        )
        .expect("a chassis id")
    }

    fn advertised_port(neighbour: &LldpNeighbor) -> LldpPortId {
        LldpPortId::from_snmp(
            neighbour.remote_port_id_subtype.expect("a subtype"),
            neighbour.remote_port_id_bytes.as_ref().expect("a value"),
        )
        .expect("a port id")
    }
}

/// GH #664: a chassis MAC that is on no port and no IP.
///
/// `switch-netgear-01`'s LLDP chassis id is `00:1a:2b:3c:4d:63` while its ports report `…:65/:66/
/// :67`, and it bears no address with that MAC. `switch-aruba-01`'s neighbour entries advertise
/// that chassis id, so the remote host is identifiable **only** through `hosts.chassis_id`,
/// recorded from switch-netgear-01's own LLDP local identity.
///
/// Matching against interfaces and IPs alone yields `hosts_resolved=0` and an empty L2 Physical
/// view. This runs against the database because that is where it failed.
#[tokio::test]
async fn a_chassis_id_on_no_port_still_finds_its_device() {
    let lab = Lab::new().await;
    let netgear = lab.scan("switch-netgear-01").await;
    let aruba = lab.scan("switch-aruba-01").await;

    // Cabled twice — `g1 ↔ port 41` and `g2 ↔ A5` — so both of its neighbours name the same far
    // end, and either will do for a chassis lookup.
    let neighbours = aruba.collected.neighbours_named("switch-netgear-01");
    assert_eq!(neighbours.len(), 2, "the pair is cabled twice on purpose");
    let advertised = Scanned::advertised_chassis(neighbours[0]);

    // The precondition: nothing but `hosts.chassis_id` can answer this.
    assert_eq!(
        lab.resolver
            .find_host_by_mac(&advertised.identifier(), lab.network_id)
            .await,
        IdentityResolution::NotFound,
        "the chassis MAC must be on no interface, or this test proves the wrong tier"
    );

    assert_eq!(
        advertised
            .resolve_host_id(&lab.resolver, lab.network_id, None)
            .await,
        IdentityResolution::Resolved(netgear.host.id),
        "the far end is findable only through the chassis id it recorded about itself"
    );
}

/// GH #649: locally-assigned port ids (subtype 7), reaching a port two different ways.
///
/// `switch-netgear-01` advertises `41` — which is `switch-aruba-01`'s `ifDescr` — and `197`, which
/// matches only its `ifIndex` because that port is labelled `A5`. Treating subtype 7 as
/// unresolvable stops at the host, and a host-only neighbour draws no edge at all.
#[tokio::test]
async fn locally_assigned_port_ids_reach_a_port_by_name_and_by_index() {
    let lab = Lab::new().await;
    let aruba = lab.scan("switch-aruba-01").await;
    let netgear = lab.scan("switch-netgear-01").await;

    let ports: Vec<LldpPortId> = netgear
        .collected
        .neighbours
        .records
        .iter()
        .map(Scanned::advertised_port)
        .collect();

    let by_descr = ports
        .iter()
        .find(|id| matches!(id, LldpPortId::LocallyAssigned(v) if v == "41"))
        .expect("the ifDescr-shaped id");
    let by_index = ports
        .iter()
        .find(|id| matches!(id, LldpPortId::LocallyAssigned(v) if v == "197"))
        .expect("the ifIndex-shaped id");

    assert!(
        matches!(
            by_descr
                .resolve_if_entry_id(&lab.resolver, aruba.host.id)
                .await,
            IdentityResolution::Resolved(_)
        ),
        "`41` is that switch's ifDescr and must reach the port"
    );
    assert!(
        matches!(
            by_index
                .resolve_if_entry_id(&lab.resolver, aruba.host.id)
                .await,
            IdentityResolution::Resolved(_)
        ),
        "`197` matches no name on that switch, so it must fall through to ifIndex"
    );
}

/// GH #668: a MAC port id that belongs to three ports identifies none of them.
///
/// `switch-tplink-01`'s `1/0/4` advertises `lldpRemPortIdSubtype` = 3 with the address
/// `switch-dlink-01` reports on all of its ports. The chassis id resolves the host; the port id
/// must then resolve *nothing*, because a MAC belonging to three ports identifies no port.
///
/// Expect an ambiguous verdict rather than whichever row the database returned first — which is
/// what it drew before #668, and what a lookup with no `ORDER BY` and no `LIMIT` will do again.
#[tokio::test]
async fn a_mac_on_every_port_of_the_far_end_resolves_to_no_port() {
    let lab = Lab::new().await;
    let dlink = lab.scan("switch-dlink-01").await;
    let tplink = lab.scan("switch-tplink-01").await;

    // Local port 4 is the shared-MAC case; it names switch-dlink-01 like local port 3 does.
    let ambiguous = tplink
        .collected
        .neighbours_on(4)
        .into_iter()
        .next()
        .expect("a neighbour on 1/0/4");
    let port_id = Scanned::advertised_port(ambiguous);
    assert!(matches!(port_id, LldpPortId::MacAddress(_)));

    // The host resolves; only the port must not.
    assert_eq!(
        Scanned::advertised_chassis(ambiguous)
            .resolve_host_id(&lab.resolver, lab.network_id, None)
            .await,
        IdentityResolution::Resolved(dlink.host.id)
    );
    assert_eq!(
        port_id
            .resolve_if_entry_id(&lab.resolver, dlink.host.id)
            .await,
        IdentityResolution::Ambiguous,
        "an address on three ports must be declined, not resolved to whichever row came back first"
    );
}

/// The positive half of the same pair, and the reason the guard above cannot simply reject every
/// MAC port id.
///
/// `switch-dlink-01`'s local port 3 advertises subtype 3 with `00:07:7c:20:01:e3`, which is
/// `switch-macport-01`'s `ifPhysAddress` for `eth3` and for nothing else — so it must resolve to
/// that port. A guard that rejected this too would look correct while quietly costing every vendor
/// that addresses its ports individually.
#[tokio::test]
async fn a_mac_that_identifies_exactly_one_port_still_resolves() {
    let lab = Lab::new().await;
    let macport = lab.scan("switch-macport-01").await;
    let dlink = lab.scan("switch-dlink-01").await;

    let neighbour = dlink
        .collected
        .neighbours_on(3)
        .into_iter()
        .next()
        .expect("a neighbour on local port 3");
    let port_id = Scanned::advertised_port(neighbour);

    let resolved = port_id
        .resolve_if_entry_id(&lab.resolver, macport.host.id)
        .await;
    let IdentityResolution::Resolved(interface_id) = resolved else {
        panic!("a unique per-port address must resolve, got {resolved:?}");
    };

    let interface = lab
        .storage
        .interfaces
        .get_by_id(&interface_id)
        .await
        .unwrap()
        .expect("the interface exists");
    assert_eq!(
        interface.base.mac_address,
        Some("00:07:7c:20:01:e3".parse::<MacAddress>().unwrap()),
        "it must land on the port that actually carries that address"
    );
}

/// GH #664's other half, and the one that decides whether the fixture is worth anything.
///
/// `switch-netgear-01`'s chassis id is on none of its *physical* ports — but its own scanned rows
/// must still carry addresses, or the MAC tier is untested rather than declining. This asserts the
/// tier runs and finds nothing, which is a different outcome from the tier never running.
#[tokio::test]
async fn the_mac_tier_runs_and_declines_rather_than_being_skipped() {
    let lab = Lab::new().await;
    let netgear = lab.scan("switch-netgear-01").await;

    let port_mac = netgear
        .collected
        .if_table
        .entries
        .iter()
        .find_map(|e| e.if_phys_address)
        .expect("its ports carry addresses");

    // A port address does find the device...
    assert_eq!(
        lab.resolver
            .find_host_by_mac(&port_mac.to_string().to_lowercase(), lab.network_id)
            .await,
        IdentityResolution::Resolved(netgear.host.id)
    );
    // ...while the chassis address, which is on no port, does not.
    assert_eq!(
        lab.resolver
            .find_host_by_mac("00:1a:2b:3c:4d:63", lab.network_id)
            .await,
        IdentityResolution::NotFound
    );
}

/// An endpoint no device in the lab knows.
///
/// `switch-tplink-01`'s `1/0/2` advertises a desk phone whose MAC and sysName belong to nothing
/// here, so every host tier fails. It is the environment's only source of a non-zero
/// `host_not_found`, and that counter is otherwise permanently 0 — which left the server-side
/// summary that names unmatched far ends with no way to fire.
#[tokio::test]
async fn a_far_end_nobody_scanned_resolves_to_nothing() {
    let lab = Lab::new().await;
    let tplink = lab.scan("switch-tplink-01").await;
    lab.scan("switch-dlink-01").await;

    let stranger = tplink
        .collected
        .neighbours_on(2)
        .into_iter()
        .next()
        .expect("a neighbour on 1/0/2");

    let resolved = Scanned::advertised_chassis(stranger)
        .resolve_host_id(
            &lab.resolver,
            lab.network_id,
            stranger.remote_sys_name.as_deref(),
        )
        .await;
    assert_eq!(
        resolved,
        IdentityResolution::NotFound,
        "an endpoint nobody scanned is not found, which is not the same as unresolvable"
    );
}

/// A sanity check on the seeding itself: the far ends really are in the database with the
/// identifiers the neighbours name them by.
///
/// Without this, every test above could pass against an empty inventory for the wrong reason.
#[tokio::test]
async fn the_seeded_far_ends_carry_the_identifiers_they_are_named_by() {
    let lab = Lab::new().await;
    let core = lab.scan("switch-core-01").await;

    assert_eq!(
        lab.resolver
            .find_host_by_chassis_id("00:1a:2b:00:10:00", lab.network_id)
            .await,
        IdentityResolution::Resolved(core.host.id)
    );
    let _: IpAddr = "192.168.7.230".parse().unwrap();
    assert_eq!(core.collected.if_table.entries.len(), 4);
}
