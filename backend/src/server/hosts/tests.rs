//! Host API-shape tests.

use crate::server::hosts::r#impl::api::HostResponse;
use crate::server::hosts::r#impl::attributes::{
    HostChassisIdValue, HostFirmwareRevisionValue, HostManagementUrlValue, HostManufacturerValue,
    HostModelValue, HostSerialNumberValue, HostSoftwareRevisionValue, HostSysContactValue,
    HostSysDescrValue, HostSysLocationValue, HostSysNameValue, HostSysObjectIdValue,
};
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, Attributed};
use crate::server::shared::types::examples;

/// `HostResponse` is the only shape a host is ever read back through, and `to_host()` claims to be
/// its inverse. Every field dropped between the two is a column a customer can write and never
/// read — which is exactly how `sys_name`, `manufacturer`, `model` and `serial_number` stayed
/// invisible while being collected and stored. Assert the whole `HostBase` survives the round trip
/// rather than naming fields, so a field added later is covered without editing this test.
///
/// Now that every attribute carries its source, the round trip has to preserve both halves — the
/// response splits each pair into two flat fields and `to_host` reassembles it, and a source
/// dropped on the way through would silently demote every attribute to `Unspecified` the next
/// time a daemon read a host back and re-submitted it. The two sources below are deliberately
/// different so a conversion that hard-codes one would fail.
#[test]
fn host_response_round_trip_preserves_every_base_field() {
    let probe = AttributeSource::Probe(ClientProbe::Snmp);
    let authored = AttributeSource::Authored(ClientProbe::Snmp);

    let mut host = examples::host();
    host.base.sys_descr = Some(Attributed::new(
        HostSysDescrValue("Cisco IOS Software, C2960X".to_string()),
        probe,
    ));
    host.base.sys_object_id = Some(Attributed::new(
        HostSysObjectIdValue("1.3.6.1.4.1.9.1.2494".to_string()),
        probe,
    ));
    host.base.sys_location = Some(Attributed::new(
        HostSysLocationValue("Rack 4, DC1".to_string()),
        authored,
    ));
    host.base.sys_contact = Some(Attributed::new(
        HostSysContactValue("noc@example.com".to_string()),
        authored,
    ));
    host.base.management_url = Some(Attributed::new(
        HostManagementUrlValue("https://10.0.0.2".to_string()),
        probe,
    ));
    host.base.chassis_id = Some(Attributed::new(
        HostChassisIdValue("00:1a:2b:3c:4d:5e".to_string()),
        probe,
    ));
    host.base.sys_name = Some(Attributed::new(
        HostSysNameValue("core-sw-01".to_string()),
        probe,
    ));
    host.base.manufacturer = Some(Attributed::new(
        HostManufacturerValue("Cisco".to_string()),
        probe,
    ));
    host.base.model = Some(Attributed::new(
        HostModelValue("WS-C2960X-48FPD-L".to_string()),
        probe,
    ));
    host.base.serial_number = Some(Attributed::new(
        HostSerialNumberValue("FOC1234X5YZ".to_string()),
        probe,
    ));
    host.base.firmware_revision = Some(Attributed::new(
        HostFirmwareRevisionValue("15.0(4)".to_string()),
        probe,
    ));
    host.base.software_revision = Some(Attributed::new(
        HostSoftwareRevisionValue("15.0(2)SE11".to_string()),
        probe,
    ));

    let response =
        HostResponse::from_host_with_children(host.clone(), vec![], vec![], vec![], vec![]);

    assert_eq!(response.to_host().base, host.base);
}
