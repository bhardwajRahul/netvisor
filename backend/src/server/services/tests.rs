use serial_test::serial;

use crate::{
    server::{
        auth::middleware::auth::AuthenticatedEntity,
        bindings::r#impl::base::Binding,
        services::r#impl::patterns::MatchDetails,
        shared::{
            services::traits::CrudService,
            storage::{filter::EntityFilter, traits::Storable},
            types::entities::EntitySource,
        },
    },
    tests::*,
};

#[tokio::test]
#[serial]
async fn test_service_deduplication_on_create() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create first service + host
    let host_obj = host(&network.id);
    let iface = interface(&network.id, &subnet_obj.id);
    let port = port(&network.id, &host_obj.id);

    let mut svc1 = service(&network.id, &host_obj.id);
    // Add bindings so the deduplication logic can match them
    svc1.base.bindings = vec![Binding::new_port_serviceless(port.id, Some(iface.id))];
    // Set source to discovery so upsert route is used
    svc1.base.source = EntitySource::DiscoveryWithMatch {
        metadata: vec![],
        details: MatchDetails::new_certain("Test"),
    };

    let created1 = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port],
            vec![svc1.clone()],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Verify children were created
    assert!(
        !created1.interfaces.is_empty(),
        "Interface should have been created with host"
    );
    assert!(
        !created1.ports.is_empty(),
        "Port should have been created with host"
    );
    assert!(
        !created1.services.is_empty(),
        "Service should have been created with host"
    );

    // Get created interface & port from returned HostResponse
    let created_iface = &created1.interfaces[0];
    let created_port = &created1.ports[0];

    // Try to create duplicate (same definition + matching bindings)
    // Must use created_host's IDs since host deduplication may have changed them
    let mut svc2 = service(&network.id, &created1.id);
    svc2.base.service_definition = svc1.base.service_definition.clone();
    svc2.base.bindings = vec![Binding::new_port_serviceless(
        created_port.id,
        Some(created_iface.id),
    )];
    svc2.base.source = EntitySource::DiscoveryWithMatch {
        metadata: vec![],
        details: MatchDetails::new_certain("Test"),
    };

    let created2 = services
        .service_service
        .create(svc2.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Should return same service (upserted) - compare service IDs
    let created_svc1 = &created1.services[0];
    assert_eq!(
        created_svc1.id, created2.id,
        "Services should have been deduplicated"
    );

    let filter = EntityFilter::unfiltered().host_id(&created1.id);
    // Verify only one service in DB
    let all_services = services.service_service.get_all(filter).await.unwrap();
    assert_eq!(all_services.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_service_deletion_cleans_up_relationships() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    let created_subnet = services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    let host_obj = host(&network.id);
    let iface = interface(&network.id, &created_subnet.id);
    let port = port(&network.id, &host_obj.id);

    // Create service in a group
    let mut svc = service(&network.id, &host_obj.id);
    let binding = Binding::new_port_serviceless(port.id, Some(iface.id));
    svc.base.bindings = vec![binding];

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port],
            vec![svc.clone()],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Verify children were created
    assert!(
        !created_host.interfaces.is_empty(),
        "Interface should have been created"
    );
    assert!(
        !created_host.ports.is_empty(),
        "Port should have been created"
    );
    assert!(
        !created_host.services.is_empty(),
        "Service should have been created"
    );

    // Get the created service from the HostResponse
    let created_svc = &created_host.services[0];

    // Verify the service has bindings (they should be populated via reassign_service_interface_bindings)
    assert!(
        !created_svc.base.bindings.is_empty(),
        "Service should have bindings after creation"
    );

    let mut group_obj = group(&network.id);
    group_obj.base.binding_ids = vec![created_svc.base.bindings[0].id()];
    let created_group = services
        .group_service
        .create(group_obj, AuthenticatedEntity::System)
        .await
        .unwrap();

    // Delete service
    services
        .service_service
        .delete(&created_svc.id, AuthenticatedEntity::System)
        .await
        .unwrap();

    // Group should no longer have service binding
    let group_after = services
        .group_service
        .get_by_id(&created_group.id)
        .await
        .unwrap()
        .unwrap();

    assert!(group_after.base.binding_ids.is_empty());
}

// =============================================================================
// BINDING CONFLICT TESTS
// =============================================================================

/// Test that interface binding + port binding on same interface causes conflict
#[tokio::test]
#[serial]
async fn test_binding_conflict_interface_and_port_same_interface() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host with interface and port
    let host_obj = host(&network.id);
    let iface = interface(&network.id, &subnet_obj.id);
    let port_obj = port(&network.id, &host_obj.id);

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port_obj],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_iface = &created_host.interfaces[0];
    let created_port = &created_host.ports[0];

    // Try to create service with conflicting bindings:
    // Interface binding + Port binding on same interface
    let mut svc = service(&network.id, &created_host.id);
    svc.base.bindings = vec![
        Binding::new_interface_serviceless(created_iface.id),
        Binding::new_port_serviceless(created_port.id, Some(created_iface.id)),
    ];

    let result = services
        .service_service
        .create(svc, AuthenticatedEntity::System)
        .await;

    assert!(result.is_err(), "Should reject conflicting bindings");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("conflict") || err.contains("interface binding"),
        "Error should mention conflict: {}",
        err
    );
}

/// Test that interface binding + port binding on different interfaces is OK
#[tokio::test]
#[serial]
async fn test_binding_no_conflict_different_interfaces() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host with two interfaces and a port
    let host_obj = host(&network.id);
    let mut iface1 = interface(&network.id, &subnet_obj.id);
    iface1.base.ip_address = "192.168.1.100".parse().unwrap();
    let mut iface2 = interface(&network.id, &subnet_obj.id);
    iface2.base.ip_address = "192.168.1.101".parse().unwrap();
    let port_obj = port(&network.id, &host_obj.id);

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface1, iface2],
            vec![port_obj],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_iface1 = &created_host.interfaces[0];
    let created_iface2 = &created_host.interfaces[1];
    let created_port = &created_host.ports[0];

    // Interface binding on iface1 + Port binding on iface2 should be OK
    let mut svc = service(&network.id, &created_host.id);
    svc.base.bindings = vec![
        Binding::new_interface_serviceless(created_iface1.id),
        Binding::new_port_serviceless(created_port.id, Some(created_iface2.id)),
    ];

    let result = services
        .service_service
        .create(svc, AuthenticatedEntity::System)
        .await;

    assert!(
        result.is_ok(),
        "Should allow non-conflicting bindings: {:?}",
        result.err()
    );
}

/// Test that interface binding + all-interfaces port binding causes conflict
#[tokio::test]
#[serial]
async fn test_binding_conflict_interface_and_all_interfaces_port() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    let host_obj = host(&network.id);
    let iface = interface(&network.id, &subnet_obj.id);
    let port_obj = port(&network.id, &host_obj.id);

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port_obj],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_iface = &created_host.interfaces[0];
    let created_port = &created_host.ports[0];

    // Interface binding + Port binding on ALL interfaces (interface_id: None) should conflict
    let mut svc = service(&network.id, &created_host.id);
    svc.base.bindings = vec![
        Binding::new_interface_serviceless(created_iface.id),
        Binding::new_port_serviceless(created_port.id, None), // all interfaces
    ];

    let result = services
        .service_service
        .create(svc, AuthenticatedEntity::System)
        .await;

    assert!(result.is_err(), "Should reject conflicting bindings");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("conflict") || err.contains("interface"),
        "Error should mention conflict: {}",
        err
    );
}

// =============================================================================
// BINDING OWNERSHIP TESTS
// =============================================================================

/// Test that port binding referencing port from different host is rejected
#[tokio::test]
#[serial]
async fn test_binding_port_from_different_host_rejected() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host1 with a port
    let host1 = host(&network.id);
    let port1 = port(&network.id, &host1.id);
    let iface1 = interface(&network.id, &subnet_obj.id);

    let created_host1 = services
        .host_service
        .discover_host(
            host1.clone(),
            vec![iface1],
            vec![port1],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Create host2 (different host)
    let mut host2 = host(&network.id);
    host2.base.name = "Host 2".to_string();
    let mut iface2 = interface(&network.id, &subnet_obj.id);
    iface2.base.ip_address = "192.168.1.200".parse().unwrap();

    let created_host2 = services
        .host_service
        .discover_host(
            host2.clone(),
            vec![iface2],
            vec![],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Try to create service on host2 with binding to host1's port
    let mut svc = service(&network.id, &created_host2.id);
    svc.base.bindings = vec![Binding::new_port_serviceless(
        created_host1.ports[0].id,
        None,
    )];

    let result = services
        .service_service
        .create(svc, AuthenticatedEntity::System)
        .await;

    assert!(
        result.is_err(),
        "Should reject binding to port from different host"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("does not belong") || err.contains("host"),
        "Error should mention port doesn't belong to host: {}",
        err
    );
}

/// Test that interface binding referencing interface from different host is rejected
#[tokio::test]
#[serial]
async fn test_binding_interface_from_different_host_rejected() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host1 with an interface
    let host1 = host(&network.id);
    let iface1 = interface(&network.id, &subnet_obj.id);

    let created_host1 = services
        .host_service
        .discover_host(
            host1.clone(),
            vec![iface1],
            vec![],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Create host2 (different host)
    let mut host2 = host(&network.id);
    host2.base.name = "Host 2".to_string();
    let mut iface2 = interface(&network.id, &subnet_obj.id);
    iface2.base.ip_address = "192.168.1.200".parse().unwrap();

    let created_host2 = services
        .host_service
        .discover_host(
            host2.clone(),
            vec![iface2],
            vec![],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // Try to create service on host2 with binding to host1's interface
    let mut svc = service(&network.id, &created_host2.id);
    svc.base.bindings = vec![Binding::new_interface_serviceless(
        created_host1.interfaces[0].id,
    )];

    let result = services
        .service_service
        .create(svc, AuthenticatedEntity::System)
        .await;

    assert!(
        result.is_err(),
        "Should reject binding to interface from different host"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("does not belong") || err.contains("host"),
        "Error should mention interface doesn't belong to host: {}",
        err
    );
}

// =============================================================================
// CROSS-SERVICE BINDING CONFLICT TESTS
// =============================================================================

/// Test that manual service creation is blocked when port is already bound to another service
#[tokio::test]
#[serial]
async fn test_cross_service_manual_create_blocked() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host with interface and port
    let host_obj = host(&network.id);
    let iface = interface(&network.id, &subnet_obj.id);
    let port_obj = port(&network.id, &host_obj.id);

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port_obj],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_iface = &created_host.interfaces[0];
    let created_port = &created_host.ports[0];

    // Create first service with port binding (manual source)
    let mut svc1 = service(&network.id, &created_host.id);
    svc1.base.name = "Jellyfin".to_string();
    svc1.base.source = EntitySource::Manual;
    svc1.base.bindings = vec![Binding::new_port_serviceless(
        created_port.id,
        Some(created_iface.id),
    )];

    let created_svc1 = services
        .service_service
        .create(svc1, AuthenticatedEntity::System)
        .await
        .unwrap();

    assert_eq!(created_svc1.base.name, "Jellyfin");

    // Try to create second service with same port binding (should be rejected)
    let mut svc2 = service(&network.id, &created_host.id);
    svc2.base.name = "Plex".to_string();
    svc2.base.source = EntitySource::Manual;
    svc2.base.bindings = vec![Binding::new_port_serviceless(
        created_port.id,
        Some(created_iface.id),
    )];

    let result = services
        .service_service
        .create(svc2, AuthenticatedEntity::System)
        .await;

    assert!(
        result.is_err(),
        "Should reject creating service with port already bound to another service"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Jellyfin") && err.contains("already bound"),
        "Error should mention the conflicting service name: {}",
        err
    );
}

/// Test that service update is blocked when trying to add a port binding already claimed
#[tokio::test]
#[serial]
async fn test_cross_service_update_blocked() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host with interface and two ports
    let host_obj = host(&network.id);
    let iface = interface(&network.id, &subnet_obj.id);
    let port1 = port(&network.id, &host_obj.id);
    let mut port2 = port(&network.id, &host_obj.id);
    port2.base.port_type = crate::server::ports::r#impl::base::PortType::new_tcp(9090);

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port1, port2],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_iface = &created_host.interfaces[0];
    let created_port1 = &created_host.ports[0];
    let created_port2 = &created_host.ports[1];

    // Create first service with port1 binding
    let mut svc1 = service(&network.id, &created_host.id);
    svc1.base.name = "Service1".to_string();
    svc1.base.source = EntitySource::Manual;
    svc1.base.bindings = vec![Binding::new_port_serviceless(
        created_port1.id,
        Some(created_iface.id),
    )];

    services
        .service_service
        .create(svc1, AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create second service with port2 binding
    let mut svc2 = service(&network.id, &created_host.id);
    svc2.base.name = "Service2".to_string();
    svc2.base.source = EntitySource::Manual;
    svc2.base.bindings = vec![Binding::new_port_serviceless(
        created_port2.id,
        Some(created_iface.id),
    )];

    let mut created_svc2 = services
        .service_service
        .create(svc2, AuthenticatedEntity::System)
        .await
        .unwrap();

    // Try to update svc2 to also claim port1 (should be rejected)
    created_svc2
        .base
        .bindings
        .push(Binding::new_port_serviceless(
            created_port1.id,
            Some(created_iface.id),
        ));

    let result = services
        .service_service
        .update(&mut created_svc2, AuthenticatedEntity::System)
        .await;

    assert!(
        result.is_err(),
        "Should reject update that claims port already bound to another service"
    );
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Service1") && err.contains("already bound"),
        "Error should mention the conflicting service name: {}",
        err
    );
}

/// Test that "all interfaces" port binding conflicts with specific interface binding
#[tokio::test]
#[serial]
async fn test_cross_service_all_interfaces_conflict() {
    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host with interface and port
    let host_obj = host(&network.id);
    let iface = interface(&network.id, &subnet_obj.id);
    let port_obj = port(&network.id, &host_obj.id);

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port_obj],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_iface = &created_host.interfaces[0];
    let created_port = &created_host.ports[0];

    // Create first service with port binding on "all interfaces" (None)
    let mut svc1 = service(&network.id, &created_host.id);
    svc1.base.name = "AllInterfacesService".to_string();
    svc1.base.source = EntitySource::Manual;
    svc1.base.bindings = vec![Binding::new_port_serviceless(created_port.id, None)];

    services
        .service_service
        .create(svc1, AuthenticatedEntity::System)
        .await
        .unwrap();

    // Try to create second service with same port but specific interface (should conflict)
    let mut svc2 = service(&network.id, &created_host.id);
    svc2.base.name = "SpecificInterfaceService".to_string();
    svc2.base.source = EntitySource::Manual;
    svc2.base.bindings = vec![Binding::new_port_serviceless(
        created_port.id,
        Some(created_iface.id),
    )];

    let result = services
        .service_service
        .create(svc2, AuthenticatedEntity::System)
        .await;

    assert!(
        result.is_err(),
        "Should reject when 'all interfaces' binding exists for same port"
    );
}

/// Test that discovery drops conflicting service and orphans valid bindings to OpenPorts
#[tokio::test]
#[serial]
async fn test_discovery_conflict_drops_service_orphans_to_open_ports() {
    use crate::server::services::r#impl::definitions::ServiceDefinitionExt;
    use crate::server::services::r#impl::patterns::MatchDetails;

    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host with interface and two ports
    let host_obj = host(&network.id);
    let iface = interface(&network.id, &subnet_obj.id);
    let port1 = port(&network.id, &host_obj.id);
    let mut port2 = port(&network.id, &host_obj.id);
    port2.base.port_type = crate::server::ports::r#impl::base::PortType::new_tcp(9090);

    // First discovery - create host with a service bound to port1
    let mut existing_svc = service(&network.id, &host_obj.id);
    existing_svc.base.name = "ExistingService".to_string();
    existing_svc.base.source = EntitySource::DiscoveryWithMatch {
        metadata: vec![],
        details: MatchDetails::new_certain("Test"),
    };
    existing_svc.base.bindings = vec![Binding::new_port_serviceless(port1.id, Some(iface.id))];

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface.clone()],
            vec![port1.clone(), port2.clone()],
            vec![existing_svc],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_port1 = &created_host.ports[0];
    let created_port2 = &created_host.ports[1];
    let created_iface = &created_host.interfaces[0];

    // Verify first service was created
    assert_eq!(created_host.services.len(), 1);
    assert_eq!(created_host.services[0].base.name, "ExistingService");

    // Second discovery - create new service that conflicts on port1 but has port2
    let mut conflicting_svc = service(&network.id, &created_host.id);
    conflicting_svc.base.name = "ConflictingService".to_string();
    conflicting_svc.base.source = EntitySource::DiscoveryWithMatch {
        metadata: vec![],
        details: MatchDetails::new_certain("Test"),
    };
    conflicting_svc.base.bindings = vec![
        Binding::new_port_serviceless(created_port1.id, Some(created_iface.id)), // conflicts
        Binding::new_port_serviceless(created_port2.id, Some(created_iface.id)), // valid
    ];

    services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port1, port2],
            vec![conflicting_svc],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    // The conflicting service should have been dropped, but an OpenPorts service
    // should have been created with the valid port2 binding
    let filter = EntityFilter::unfiltered().host_id(&created_host.id);
    let all_services = services.service_service.get_all(filter).await.unwrap();

    // Should have: ExistingService + OpenPorts (with orphaned port2 binding)
    assert!(
        all_services.len() >= 2,
        "Should have at least ExistingService and OpenPorts, got {}",
        all_services.len()
    );

    // Find the OpenPorts service
    let open_ports_svc = all_services
        .iter()
        .find(|s| s.base.service_definition.is_open_ports());

    assert!(
        open_ports_svc.is_some(),
        "OpenPorts service should exist with orphaned bindings"
    );

    let open_ports_svc = open_ports_svc.unwrap();

    // OpenPorts should have the port2 binding (the valid one from the dropped service)
    let has_port2_binding = open_ports_svc.base.bindings.iter().any(|b| {
        if let crate::server::bindings::r#impl::base::BindingType::Port { port_id, .. } =
            &b.base.binding_type
        {
            *port_id == created_port2.id
        } else {
            false
        }
    });

    assert!(
        has_port2_binding,
        "OpenPorts should have the orphaned port2 binding"
    );

    // ConflictingService should NOT exist
    let conflicting_exists = all_services
        .iter()
        .any(|s| s.base.name == "ConflictingService");
    assert!(
        !conflicting_exists,
        "ConflictingService should have been dropped"
    );
}

/// Test that OpenPorts services are singletons per host (merged via upsert)
#[tokio::test]
#[serial]
async fn test_open_ports_singleton_per_host() {
    use crate::server::services::definitions::open_ports::OpenPorts as OpenPortsDef;
    use crate::server::services::r#impl::base::ServiceBase;
    use crate::server::services::r#impl::definitions::ServiceDefinitionExt;
    use crate::server::shared::storage::traits::Storable;

    let (_, services, _container) = test_services().await;

    let organization = services
        .organization_service
        .create(organization(), AuthenticatedEntity::System)
        .await
        .unwrap();
    let network = services
        .network_service
        .create(network(&organization.id), AuthenticatedEntity::System)
        .await
        .unwrap();

    let subnet_obj = subnet(&network.id);
    services
        .subnet_service
        .create(subnet_obj.clone(), AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create host with interface and two ports
    let host_obj = host(&network.id);
    let iface = interface(&network.id, &subnet_obj.id);
    let port1 = port(&network.id, &host_obj.id);
    let mut port2 = port(&network.id, &host_obj.id);
    port2.base.port_type = crate::server::ports::r#impl::base::PortType::new_tcp(9090);

    let created_host = services
        .host_service
        .discover_host(
            host_obj.clone(),
            vec![iface],
            vec![port1, port2],
            vec![],
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let created_port1 = &created_host.ports[0];
    let created_port2 = &created_host.ports[1];
    let created_iface = &created_host.interfaces[0];

    // Create first OpenPorts service with port1
    let open_ports1 = crate::server::services::r#impl::base::Service::new(ServiceBase {
        host_id: created_host.id,
        network_id: network.id,
        service_definition: Box::new(OpenPortsDef),
        name: "Unclaimed Open Ports".to_string(),
        bindings: vec![Binding::new_port_serviceless(
            created_port1.id,
            Some(created_iface.id),
        )],
        virtualization: None,
        source: EntitySource::Discovery { metadata: vec![] },
        tags: Vec::new(),
        position: 0,
    });

    let created_op1 = services
        .service_service
        .create(open_ports1, AuthenticatedEntity::System)
        .await
        .unwrap();

    // Create second OpenPorts service with port2 - should be merged into first
    let open_ports2 = crate::server::services::r#impl::base::Service::new(ServiceBase {
        host_id: created_host.id,
        network_id: network.id,
        service_definition: Box::new(OpenPortsDef),
        name: "Unclaimed Open Ports".to_string(),
        bindings: vec![Binding::new_port_serviceless(
            created_port2.id,
            Some(created_iface.id),
        )],
        virtualization: None,
        source: EntitySource::Discovery { metadata: vec![] },
        tags: Vec::new(),
        position: 0,
    });

    let created_op2 = services
        .service_service
        .create(open_ports2, AuthenticatedEntity::System)
        .await
        .unwrap();

    // Should be the same service (singleton behavior)
    assert_eq!(
        created_op1.id, created_op2.id,
        "OpenPorts services should be merged (same ID)"
    );

    // Should have both bindings now
    let filter = EntityFilter::unfiltered().host_id(&created_host.id);
    let all_services = services.service_service.get_all(filter).await.unwrap();

    let open_ports_services: Vec<_> = all_services
        .iter()
        .filter(|s| s.base.service_definition.is_open_ports())
        .collect();

    assert_eq!(
        open_ports_services.len(),
        1,
        "Should have exactly one OpenPorts service"
    );

    assert!(
        open_ports_services[0].base.bindings.len() >= 2,
        "OpenPorts should have both port bindings merged"
    );
}
