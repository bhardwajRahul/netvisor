//! UniFi Network Application (controller) discovery integration.
//!
//! Reads `GET /stat/device` from a UniFi controller and turns the controller's inventory into
//! Scanopy hosts, interfaces, LLDP neighbors and bridge-FDB entries.
//!
//! # Why this exists
//!
//! UniFi switches (e.g. the USW-Pro-Max) do not expose LLDP-MIB over SNMP — a walk of
//! `1.0.8802.1.1.2.1.4.1` returns `No Such Object` — so SNMP-based L2 topology comes up empty
//! for them. The firmware collects LLDP internally and the controller API exposes it, so this
//! integration recovers the same neighbor data from a different source.
//!
//! # Shape
//!
//! Unlike SNMP or Docker, one credential describes an endpoint that reports on *many* devices.
//! So `execute` enriches the scanned host only when the controller is itself a UniFi device
//! (a Dream Machine or Cloud Key), and creates the rest via `ops.create_host`. Everything it
//! produces flows into the same ingestion and neighbor-resolution path SNMP feeds; there is no
//! UniFi-specific topology code anywhere on the server.
//!
//! Layering: [`types`] holds the raw wire structs, [`client`] the HTTP transport and auth, and
//! [`mapping`] the wire → entity translation. A field-shape correction is a one-file change.

pub mod client;
pub mod mapping;
pub mod types;

use std::time::Duration;

use anyhow::{Error, Result};
use async_trait::async_trait;

use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::interfaces::r#impl::base::InterfaceDataComplete;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::base::ServiceMatchBaselineParams;
use crate::server::services::r#impl::patterns::{ClientProbe, ManagedDevice};

use super::{
    DiscoveryIntegration, IntegrationContext, IntegrationFailure, ProbeContext, ProbeFailure,
    ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
use client::UnifiClient;
use types::UnifiDevice;

/// Connected client carried from `probe` to `execute`.
struct UnifiProbeHandle {
    client: UnifiClient,
}

pub struct UnifiIntegration;

#[async_trait]
impl DiscoveryIntegration for UnifiIntegration {
    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::UnifiController
    }

    fn estimated_seconds(&self) -> u32 {
        10
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(180)
    }

    fn probe_gate_ports(&self, credential: &CredentialQueryPayload) -> Vec<PortType> {
        match credential {
            CredentialQueryPayload::UnifiController(c) => vec![PortType::new_tcp(c.port)],
            _ => vec![],
        }
    }

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let CredentialQueryPayload::UnifiController(credential) = ctx.credential else {
            return Err(ProbeFailure::malformed("Not a UniFi credential"));
        };

        let client =
            UnifiClient::connect(&ctx.ip.to_string(), credential, ctx.accept_invalid_certs)
                .await
                .map_err(|e| {
                    // `e` is already phrased for an operator and never contains the secret. The
                    // client distinguishes a refused login from an unknown site from a transport
                    // failure, so keep that distinction rather than flattening it — a wrong
                    // password and a self-signed certificate send an operator to opposite ends
                    // of the problem, and both used to read as "was rejected".
                    ProbeFailure::with_outcome(
                        UnifiClient::classify_connect_error(&e),
                        e.to_string(),
                    )
                })?;

        tracing::info!(
            ip = %ctx.ip,
            flavor = ?client.flavor(),
            site = %client.site(),
            "Authenticated to UniFi controller"
        );

        Ok(ProbeSuccess {
            client_probe: ClientProbe::UnifiController,
            ports: vec![PortType::new_tcp(credential.port)],
            handle: Some(Box::new(UnifiProbeHandle { client })),
        })
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
    ) -> Result<(), IntegrationFailure> {
        let handle = ctx
            .probe_handle
            .and_then(|h| h.downcast_ref::<UnifiProbeHandle>())
            .ok_or_else(|| Error::msg("UniFi execute ran without a probe handle"))?;

        ctx.ops.report_progress(10).await.ok();

        let devices: Vec<UnifiDevice> = handle.client.get_site("stat/device").await?.data;
        tracing::info!(ip = %ctx.ip, devices = devices.len(), "Fetched UniFi device inventory");
        ctx.ops.report_progress(40).await.ok();

        let network_id = host_data.host.base.network_id;
        let subnets = collect_subnets(ctx, host_data);
        let mapped = mapping::map_devices(&devices, network_id, &subnets);

        if mapped.len() < devices.len() {
            // Silent truncation would read as "the controller only manages these devices".
            tracing::info!(
                skipped = devices.len() - mapped.len(),
                "Skipped UniFi devices with no IP in a known subnet — host identity is \
                 IP-based, so they cannot be deduplicated across scans"
            );
        }

        let mut created = 0usize;
        for device in mapped {
            if ctx.cancel.is_cancelled() {
                return Err(IntegrationFailure::cancelled());
            }

            // A UDM / Cloud Key is itself a UniFi device, so when the controller we scanned
            // appears in its own inventory it enriches the scanned host rather than becoming
            // a second one.
            if device.ip == ctx.ip {
                enrich_scanned_host(host_data, &device);
                continue;
            }

            match create_device_host(ctx, &subnets, device).await {
                Ok(()) => created += 1,
                Err(e) => {
                    tracing::debug!(error = %e, "Failed to create UniFi-discovered host");
                }
            }
        }

        tracing::info!(created, "UniFi device sync complete");
        ctx.ops.report_progress(90).await.ok();
        Ok(())
    }
}

/// Subnets available to place each managed device's IP in.
///
/// `known_subnets` is the network's whole address space, not the scan's scope — which is what
/// makes a rescan of the controller useful. The controller reports every switch it manages, and
/// on a segmented network almost none of them sit in the subnet the rescan is sweeping; scoping
/// this to the sweep dropped all of them.
fn collect_subnets(
    ctx: &IntegrationContext<'_>,
    host_data: &HostData,
) -> Vec<crate::server::subnets::r#impl::base::Subnet> {
    merge_subnets(ctx.known_subnets, ctx.scanning_subnet, &host_data.subnets)
}

/// Union the three sources by id, preserving order: network-wide first, then the subnet being
/// swept, then anything this host's own collection turned up.
fn merge_subnets(
    known: &[crate::server::subnets::r#impl::base::Subnet],
    scanning: Option<&crate::server::subnets::r#impl::base::Subnet>,
    from_host: &[crate::server::subnets::r#impl::base::Subnet],
) -> Vec<crate::server::subnets::r#impl::base::Subnet> {
    let mut subnets = known.to_vec();
    for subnet in scanning.into_iter().chain(from_host) {
        if !subnets.iter().any(|s| s.id == subnet.id) {
            subnets.push(subnet.clone());
        }
    }
    subnets
}

/// Fold the controller's own device record into the host being scanned.
///
/// Every setter here is first-write-wins, so a prior SNMP pass in the same scan keeps its
/// values — SNMP reads the device directly and is the better source when both are present.
fn enrich_scanned_host(host_data: &mut HostData, device: &mapping::MappedDevice) {
    let base = &device.host.base;
    if let Some(name) = &base.sys_name {
        host_data.with_sys_name(name.clone());
    }
    if let Some(chassis_id) = &base.chassis_id {
        host_data.with_chassis_id(chassis_id.clone());
    }
    if let Some(manufacturer) = &base.manufacturer {
        host_data.with_manufacturer(manufacturer.clone());
    }
    if let Some(model) = &base.model {
        host_data.with_model(model.clone());
    }
    if let Some(serial) = &base.serial_number {
        host_data.with_serial_number(serial.clone());
    }

    // `HostData` is shared across every integration for this IP. If SNMP already walked the
    // ifTable, its view is strictly richer (VLAN, loopback and CPU interfaces UniFi omits),
    // and overwriting it would also discard SNMP's honest partial-collection flags.
    if !host_data.interfaces.is_empty() {
        tracing::debug!(
            "Interfaces already collected for this host; skipping UniFi interface mapping"
        );
        return;
    }
    host_data.replace_interfaces(device.interfaces.clone());
    host_data.set_interfaces_complete(interfaces_complete());
    host_data.set_interface_data_complete(interface_data_complete());
}

/// Create a host for a device the controller manages but we did not scan.
async fn create_device_host(
    ctx: &IntegrationContext<'_>,
    subnets: &[crate::server::subnets::r#impl::base::Subnet],
    device: mapping::MappedDevice,
) -> Result<(), Error> {
    let mapping::MappedDevice {
        host,
        ip_address,
        interfaces,
        device_type,
        ip,
    } = device;

    // Run the real service matcher rather than stamping a service on. The controller's
    // reported device class enters as `ManagedDevice` evidence and `Pattern::ManagedDeviceType`
    // consumes it, exactly as the container scanner feeds `ServiceVirtualization` — so these
    // hosts get an ordinary `DiscoveryWithMatch` service with a real confidence.
    let managed_device = device_type.map(|device_type| ManagedDevice { device_type });
    let subnet = subnets
        .iter()
        .find(|s| s.base.cidr.contains(&ip))
        .ok_or_else(|| Error::msg("device IP is not in any known subnet"))?;

    let all_ports: Vec<PortType> = vec![];
    let endpoint_responses = vec![];
    let client_responses = std::collections::HashMap::new();
    let daemon_id = ctx.ops.daemon_id().await?;
    let (services, ports) = ctx.ops.match_services(
        &host,
        &ServiceMatchBaselineParams {
            subnet,
            ip_address: &ip_address,
            all_ports: &all_ports,
            endpoint_responses: &endpoint_responses,
            virtualization: &None,
            client_responses: &client_responses,
            managed_device: &managed_device,
        },
        &[],
        &daemon_id,
        &host.base.network_id,
    )?;

    ctx.ops
        .create_host(
            host,
            vec![ip_address],
            ports,
            services,
            interfaces,
            vec![],
            interfaces_complete(),
            interface_data_complete(),
            ctx.cancel,
        )
        .await?;
    Ok(())
}

/// Always `false`, deliberately.
///
/// A UniFi `port_table` is a complete list of *physical ports*, not of the device's ifTable —
/// the same switch under SNMP also reports VLAN, loopback and CPU interfaces UniFi never
/// mentions. Claiming completeness would let `should_prune_interfaces` delete those rows and,
/// with them, their server-resolved `neighbor` links (the GH #649 failure mode). Stale
/// interfaces are the safe direction; deleted ones are not.
fn interfaces_complete() -> bool {
    false
}

/// UniFi reports LLDP and the bridge FDB, but never CDP or VLAN membership. Saying so keeps
/// `preserve_uncollected_data` from nulling CDP/VLAN columns an SNMP poll populated.
fn interface_data_complete() -> InterfaceDataComplete {
    InterfaceDataComplete {
        lldp: true,
        cdp: false,
        fdb: true,
        vlan_membership: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::types::entities::EntitySource;
    use crate::server::subnets::r#impl::base::{Subnet, SubnetBase};
    use crate::server::subnets::r#impl::types::SubnetType;
    use uuid::Uuid;

    fn subnet(cidr: &str) -> Subnet {
        Subnet {
            base: SubnetBase {
                name: cidr.to_string(),
                network_id: Uuid::nil(),
                cidr: cidr.parse().expect("valid CIDR"),
                subnet_type: SubnetType::Lan,
                source: EntitySource::System,
                ..Default::default()
            },
            id: Uuid::new_v4(),
            ..Default::default()
        }
    }

    /// The failure Motala hit: a rescan of the controller sweeps only the controller's own
    /// subnet, but the controller manages switches on a management VLAN it never touches. When
    /// the subnet list came from the sweep, `map_devices` could place none of them and dropped
    /// every switch — so the rescan appeared to do nothing at all.
    #[test]
    fn a_managed_device_outside_the_swept_subnet_is_still_placeable() {
        let management = subnet("192.168.210.0/24");
        let controller = subnet("172.16.8.0/24");

        let available = merge_subnets(
            // The network's whole address space, which is what `known_subnets` now carries.
            &[controller.clone(), management.clone()],
            // A rescan of the controller sweeps only this one.
            Some(&controller),
            &[],
        );

        let switch: std::net::IpAddr = "192.168.210.207".parse().expect("valid IP");
        assert!(
            available.iter().any(|s| s.base.cidr.contains(&switch)),
            "a switch on the management VLAN must be placeable from a rescan of the controller"
        );
    }

    /// The sweep's subnet and anything the host's own collection found still get folded in, and
    /// nothing appears twice — `map_devices` picks the first containing subnet, so a duplicate
    /// would be a silent coin-flip over which id an address is stamped with.
    #[test]
    fn merging_is_a_union_by_id() {
        let a = subnet("10.0.0.0/24");
        let b = subnet("10.0.1.0/24");

        let merged = merge_subnets(&[a.clone()], Some(&a), &[b.clone(), a.clone()]);

        let ids: Vec<Uuid> = merged.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![a.id, b.id]);
    }
}
