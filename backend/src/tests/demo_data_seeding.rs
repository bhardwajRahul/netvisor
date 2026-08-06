//! Coverage for the demo seeding write path — `organizations::demo_seed`.
//!
//! Demo seeding deliberately bypasses `discover_host` ("no collisions in fresh org") and writes
//! through bulk `create_many`, so none of the ordering the discovery path grew for GH #650
//! applies to it. When `virtualization_service_id` became a real foreign key on hosts, services
//! and subnets, that left the seeder writing subnets and hosts that name services it has not
//! created yet.
//!
//! These run against a testcontainers Postgres with the migrations applied, so the foreign keys
//! are the real ones. Same vehicle as `host_create_with_children.rs`.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::hosts::r#impl::base::Host;
use crate::server::organizations::demo_data::DemoData;
use crate::server::organizations::demo_seed::insert_demo_data;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Storage;
use crate::server::subnets::r#impl::base::Subnet;

use super::{organization, test_services, user};

/// Every `virtualization_service_id` the dataset declares, as `(row id, owning service id)`,
/// split by the table the row lands in.
#[derive(Default)]
struct DeclaredOwners {
    subnets: Vec<(Uuid, Uuid)>,
    hosts: Vec<(Uuid, Uuid)>,
    services: Vec<(Uuid, Uuid)>,
}

impl DeclaredOwners {
    fn from(demo_data: &DemoData) -> Self {
        let mut declared = Self::default();

        for subnet in &demo_data.subnets {
            if let Some(owner) = subnet.base.virtualization_service_id {
                declared.subnets.push((subnet.id, owner));
            }
        }

        let bundles = demo_data
            .hosts_with_services
            .iter()
            .chain(&demo_data.recent_hosts_with_services);
        for hws in bundles {
            if let Some(owner) = hws.host.base.virtualization_service_id {
                declared.hosts.push((hws.host.id, owner));
            }
            for svc in &hws.services {
                if let Some(owner) = svc.base.virtualization_service_id {
                    declared.services.push((svc.id, owner));
                }
            }
        }

        declared
    }

    fn is_empty(&self) -> bool {
        self.subnets.is_empty() && self.hosts.is_empty() && self.services.is_empty()
    }
}

/// The seeder's own guarantee, asserted against real foreign keys.
///
/// Red before the reorder: the DockerBridge subnets name Docker daemon services that step 5.5
/// has not written yet, so step 4 aborts on `subnets_virtualization_service_id_fkey`.
///
/// Green only if every declared relationship survives. A "fix" that satisfies the constraint by
/// setting these ids to `None` seeds cleanly and fails here, which is the point — a demo whose
/// containers are not under their runtime and whose VMs are not under their hypervisor is
/// worthless, and nothing else in the suite would notice.
#[tokio::test]
async fn demo_seed_persists_every_virtualization_relationship() {
    let (storage, services, _container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let owner = user(&org.id);
    storage.users.create(&owner).await.unwrap();

    // Held rather than regenerated: `DemoData::generate` mints fresh UUIDs every call.
    let demo_data = DemoData::generate(org.id, owner.id);
    let declared = DeclaredOwners::from(&demo_data);
    assert!(
        !declared.is_empty(),
        "demo data declares no virtualization at all — the fixture this test guards is gone"
    );

    insert_demo_data(
        &services,
        demo_data,
        org.id,
        owner.id,
        AuthenticatedEntity::System,
    )
    .await
    .expect("demo seeding must satisfy the virtualization_service_id foreign keys");

    // Live rows only: step 14 snapshots each network, and close-and-clone leaves a closed copy of
    // every entity behind. Counting those would double every figure below.
    let subnets: Vec<Subnet> = storage
        .subnets
        .get_all(StorableFilter::new_unfiltered().live())
        .await
        .unwrap();
    let hosts: Vec<Host> = storage
        .hosts
        .get_all(StorableFilter::new_unfiltered().live())
        .await
        .unwrap();
    let persisted_services: Vec<Service> = storage
        .services
        .get_all(StorableFilter::new_unfiltered().live())
        .await
        .unwrap();

    let live_service_ids: HashSet<Uuid> = persisted_services.iter().map(|s| s.id).collect();

    let subnet_owners: HashMap<Uuid, Option<Uuid>> = subnets
        .iter()
        .map(|s| (s.id, s.base.virtualization_service_id))
        .collect();
    let host_owners: HashMap<Uuid, Option<Uuid>> = hosts
        .iter()
        .map(|h| (h.id, h.base.virtualization_service_id))
        .collect();
    let service_owners: HashMap<Uuid, Option<Uuid>> = persisted_services
        .iter()
        .map(|s| (s.id, s.base.virtualization_service_id))
        .collect();

    let cases = [
        ("subnets", &declared.subnets, &subnet_owners),
        ("hosts", &declared.hosts, &host_owners),
        ("services", &declared.services, &service_owners),
    ];

    for (table, declared_rows, persisted) in cases {
        for (row_id, expected_owner) in declared_rows {
            let actual = persisted.get(row_id).copied().unwrap_or_else(|| {
                panic!("{table}: declared row {row_id} was never persisted");
            });
            // Identity, not merely non-NULL: a row repointed at some *other* service would
            // still resolve and would still be wrong.
            assert_eq!(
                actual,
                Some(*expected_owner),
                "{table}: row {row_id} should be virtualized by {expected_owner}"
            );
            assert!(
                live_service_ids.contains(expected_owner),
                "{table}: row {row_id} names service {expected_owner}, which does not exist"
            );
        }

        // The other direction: nothing acquired an owner the dataset never declared, and the
        // per-table totals match. Catches a partial seed as well as a silent downgrade.
        let declared_ids: HashSet<Uuid> = declared_rows.iter().map(|(id, _)| *id).collect();
        let persisted_ids: HashSet<Uuid> = persisted
            .iter()
            .filter(|(_, owner)| owner.is_some())
            .map(|(id, _)| *id)
            .collect();
        assert_eq!(
            persisted_ids, declared_ids,
            "{table}: rows carrying a virtualization_service_id differ from those declared"
        );
    }

    // Nothing anywhere points at a service that is gone — including the closed snapshot copies,
    // which the foreign key covers too.
    for (table, all_rows) in [
        (
            "subnets",
            storage
                .subnets
                .get_all(StorableFilter::<Subnet>::new_unfiltered())
                .await
                .unwrap()
                .iter()
                .filter_map(|s| s.base.virtualization_service_id)
                .collect::<Vec<_>>(),
        ),
        (
            "hosts",
            storage
                .hosts
                .get_all(StorableFilter::<Host>::new_unfiltered())
                .await
                .unwrap()
                .iter()
                .filter_map(|h| h.base.virtualization_service_id)
                .collect::<Vec<_>>(),
        ),
        (
            "services",
            storage
                .services
                .get_all(StorableFilter::<Service>::new_unfiltered())
                .await
                .unwrap()
                .iter()
                .filter_map(|s| s.base.virtualization_service_id)
                .collect::<Vec<_>>(),
        ),
    ] {
        let all_service_ids: HashSet<Uuid> = storage
            .services
            .get_all(StorableFilter::<Service>::new_unfiltered())
            .await
            .unwrap()
            .iter()
            .map(|s| s.id)
            .collect();
        for owner in all_rows {
            assert!(
                all_service_ids.contains(&owner),
                "{table}: a row names service {owner}, which does not exist"
            );
        }
    }

    println!(
        "virtualization relationships persisted — subnets: {}, hosts: {}, services: {}",
        declared.subnets.len(),
        declared.hosts.len(),
        declared.services.len()
    );
}
