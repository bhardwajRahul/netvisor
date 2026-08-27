//! Host API-shape tests.

use crate::server::hosts::r#impl::api::HostResponse;
use crate::server::shared::types::examples;

/// `HostResponse` is the only shape a host is ever read back through, and `to_host()` claims to be
/// its inverse. Every field dropped between the two is a column a customer can write and never
/// read — which is exactly how `sys_name`, `manufacturer`, `model` and `serial_number` stayed
/// invisible while being collected and stored. Assert the whole `HostBase` survives the round trip
/// rather than naming fields, so a field added later is covered without editing this test.
#[test]
fn host_response_round_trip_preserves_every_base_field() {
    let mut host = examples::host();
    host.base.sys_descr = Some("Cisco IOS Software, C2960X".to_string());
    host.base.sys_object_id = Some("1.3.6.1.4.1.9.1.2494".to_string());
    host.base.sys_location = Some("Rack 4, DC1".to_string());
    host.base.sys_contact = Some("noc@example.com".to_string());
    host.base.management_url = Some("https://10.0.0.2".to_string());
    host.base.chassis_id = Some("00:1a:2b:3c:4d:5e".to_string());
    host.base.sys_name = Some("core-sw-01".to_string());
    host.base.manufacturer = Some("Cisco".to_string());
    host.base.model = Some("WS-C2960X-48FPD-L".to_string());
    host.base.serial_number = Some("FOC1234X5YZ".to_string());
    host.base.firmware_revision = Some("15.0(2)SE11".to_string());

    let response =
        HostResponse::from_host_with_children(host.clone(), vec![], vec![], vec![], vec![]);

    assert_eq!(response.to_host().base, host.base);
}
