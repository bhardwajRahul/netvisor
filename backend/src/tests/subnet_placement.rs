//! Placing an address against a network's real subnet list.
//!
//! Against a database rather than a hand-built list, because the list is the whole point. Every
//! network is seeded with an `Internet` and a `Remote Network` subnet, both `0.0.0.0/0`, by
//! `NetworkService::create_organizational_subnets` — and a guard tested against a list somebody
//! assembled by hand never sees them. That is exactly how `place_address` shipped returning
//! `Unplaceable` for every address in no real subnet: `0.0.0.0/0` contains everything, so the
//! "is this already held?" check answered yes for every address on earth and no range was ever
//! inferred.
//!
//! These seed the organizational subnets the way a real network gets them, so the list under test
//! is the list production has.

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{services::traits::CrudService, storage::traits::Storage},
    subnets::{
        r#impl::{base::Subnet, types::SubnetCidrSource},
        service::Placement,
    },
};

use super::{network, organization, test_services};

/// Seed a network with the two organizational catch-alls a real one is created with, plus whatever
/// else the test wants it to hold.
async fn network_holding(
    cidrs: &[&str],
) -> (
    crate::server::shared::services::factory::ServiceFactory,
    uuid::Uuid,
    testcontainers::ContainerAsync<testcontainers::GenericImage>,
) {
    let (storage, services, container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = network(&org.id);
    storage.networks.create(&net).await.unwrap();

    services
        .network_service
        .create_organizational_subnets(net.id, AuthenticatedEntity::System)
        .await
        .unwrap();

    for cidr in cidrs {
        let mut subnet = super::subnet(&net.id);
        subnet.base.cidr = cidr.parse().expect("valid test CIDR");
        subnet.base.name = (*cidr).to_string();
        services
            .subnet_service
            .create(subnet, AuthenticatedEntity::System)
            .await
            .unwrap();
    }

    (services, net.id, container)
}

/// The defect Maya hit: an address in a range nothing holds must produce one, not come back
/// unplaceable because two `0.0.0.0/0` rows technically contain it.
#[tokio::test]
async fn an_address_in_no_held_range_infers_one_despite_the_catch_alls() {
    let (services, network_id, _container) = network_holding(&["192.168.4.0/22"]).await;

    let placement = services
        .subnet_service
        .place_address(network_id, "10.20.30.24".parse().unwrap())
        .await
        .unwrap();

    let Placement::Inferred(subnet_id) = placement else {
        panic!("expected a range to be inferred, got {placement:?}");
    };

    let created = services
        .subnet_service
        .get_by_id(&subnet_id)
        .await
        .unwrap()
        .expect("the inferred subnet");
    assert_eq!(created.base.cidr.to_string(), "10.20.30.0/24");
    assert_eq!(created.base.cidr_source, SubnetCidrSource::Inferred);
}

/// A real subnet still wins, and is never displaced by a catch-all that also contains the address.
#[tokio::test]
async fn an_address_a_real_subnet_holds_is_placed_there() {
    let (services, network_id, _container) = network_holding(&["192.168.4.0/22"]).await;

    let placement = services
        .subnet_service
        .place_address(network_id, "192.168.7.252".parse().unwrap())
        .await
        .unwrap();

    let Placement::Existing(subnet_id) = placement else {
        panic!("expected the existing subnet, got {placement:?}");
    };

    let held = services
        .subnet_service
        .get_by_id(&subnet_id)
        .await
        .unwrap()
        .expect("the holding subnet");
    assert_eq!(held.base.cidr.to_string(), "192.168.4.0/22");
}

/// Two addresses in one unknown range converge on a single subnet without being pooled — the
/// property that makes placing one address at a time equivalent to the pooled pass.
#[tokio::test]
async fn two_addresses_in_one_unknown_range_converge_on_one_subnet() {
    let (services, network_id, _container) = network_holding(&[]).await;

    let first = services
        .subnet_service
        .place_address(network_id, "10.20.30.11".parse().unwrap())
        .await
        .unwrap();
    let second = services
        .subnet_service
        .place_address(network_id, "10.20.30.240".parse().unwrap())
        .await
        .unwrap();

    let (Placement::Inferred(a), Placement::Inferred(b) | Placement::Existing(b)) = (first, second)
    else {
        panic!("expected both to land in an inferred range, got {first:?} then {second:?}");
    };
    assert_eq!(
        a, b,
        "the second address must reuse the range the first created"
    );

    let inferred: Vec<Subnet> = services
        .subnet_service
        .get_all(
            crate::server::shared::storage::filter::StorableFilter::<Subnet>::new_from_network_ids(
                &[network_id],
            )
            .live(),
        )
        .await
        .unwrap()
        .into_iter()
        .filter(|s| s.base.cidr_source == SubnetCidrSource::Inferred)
        .collect();
    assert_eq!(inferred.len(), 1, "one range, not one per address");
}

/// A public address is not a segment of this network to invent, whatever the catch-alls say.
#[tokio::test]
async fn a_public_address_is_unplaceable_rather_than_invented() {
    let (services, network_id, _container) = network_holding(&["192.168.4.0/22"]).await;

    let placement = services
        .subnet_service
        .place_address(network_id, "8.8.8.8".parse().unwrap())
        .await
        .unwrap();

    assert!(
        matches!(placement, Placement::Unplaceable),
        "got {placement:?}"
    );
}
