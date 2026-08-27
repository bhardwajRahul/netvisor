//! The application-probe stage: unicast an address on a known port, speak the protocol, and
//! report what answered.
//!
//! Before this module a non-credentialed probe was reached through a hardcoded `match port` inside
//! `scan_udp_ports`, every probe returned `Result<Option<u16>, Error>` — a port number — and the
//! response body was discarded. BACnet read an I-Am carrying a vendor ID and device instance,
//! checked that byte 0 was `0x81`, and dropped it. Adding a probe meant editing that function, and
//! DHCP's "only on the gateway" rule lived inside one of its arms as an `if is_gateway`.
//!
//! **A probe is a method on the [`ServiceDefinition`] it belongs to.** That gives one registry
//! rather than two and makes the invariants that matter structural: a probe cannot exist without a
//! service definition, because a definition is the only thing that can hand one out; and the port
//! that gets probed is the port that gets scanned, because
//! [`probe_pattern`](crate::server::services::r#impl::patterns::probe_pattern) derives the
//! definition's discovery pattern from [`AppProbe::port`]. What stays outside is HTTP endpoint
//! probing, which is already declarative, and the credentialed integrations, which have their own
//! registry keyed on credential type — a split that means something, unlike the one this replaces.
//!
//! The probe owns its transport, because they differ: DNS and NTP use library clients (hickory,
//! sntpc), BACnet and EtherNet/IP bind a UDP socket, and Modbus opens a TCP stream and keeps it for
//! a second exchange.

pub mod ethernet_ip;
pub mod modbus;
pub mod opcua;
pub mod udp;

use anyhow::Error;
use async_trait::async_trait;
use cidr::IpCidr;
use std::net::IpAddr;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::daemon::discovery::service::ops::HostData;
use crate::daemon::discovery::types::base::DiscoveryCriticalError;
use crate::daemon::utils::scanner::{ScanConcurrencyController, batch_scan};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::ServiceDefinitionRegistry;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::ClientProbe;

/// Everything a probe is allowed to know about its target.
///
/// Owned rather than borrowed: [`batch_scan`] requires the futures it drives to be `'static`, so a
/// context holding references could not be captured into one.
#[derive(Debug, Clone)]
pub struct ProbeContext {
    pub ip: IpAddr,
    /// The subnet the address was found on. DHCP needs it to compute a broadcast address.
    pub subnet_cidr: IpCidr,
    /// Whether this address is in the daemon's routing table.
    pub is_gateway: bool,
    pub cancel: CancellationToken,
    /// Reports FD exhaustion so the scan degrades instead of failing.
    ///
    /// `scan_endpoints` does not feed this and does not need to — reqwest pools its connections.
    /// A raw-byte probe opens its own socket per target, so it is in the same position as
    /// `scan_tcp_ports`, which is the one stage that does feed it today.
    pub scan_controller: Arc<ScanConcurrencyController>,
}

impl ProbeContext {
    /// Report a connect failure, so an FD-exhaustion error degrades the whole scan's concurrency
    /// rather than being read as "nothing is listening there".
    pub fn note_connect_error(&self, error: &std::io::Error) {
        self.scan_controller.check_and_handle_error(error);
    }
}

/// What a probe established, beyond "it did not work".
///
/// Two variants rather than more because a probe has exactly two things to say that the caller can
/// act on. A transport failure is an `Err` from [`AppProbe::run`] instead, so the stage can keep
/// applying the same critical-error classification the old dispatch did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppProbeOutcome {
    /// The protocol answered on this port.
    ///
    /// `identity` is `None` both for protocols that carry no identity at all (DNS, NTP, DHCP,
    /// BACnet, OPC UA) and for a device that answered detection but declined identification — a
    /// Modbus device without `0x2B` support. Those two cases need no distinction downstream: the
    /// service matches on the probe having answered, and the host simply has no `model`.
    Answered { identity: Option<DeviceIdentity> },
    /// Nothing answered, or what answered was not this protocol.
    NoAnswer,
}

/// The hardware identity a probe read off a device.
///
/// No `Default`, following [`ControllerIdentity`](crate::daemon::discovery::integration::controller::ControllerIdentity):
/// a new probe cannot `..Default::default()` past a field, so "we forgot to record the model" is a
/// compile error rather than a silently empty column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    /// Firmware or product revision, as the device words it — Modbus MajorMinorRevision,
    /// EtherNet/IP `<major>.<minor>`.
    pub firmware_revision: Option<String>,
}

impl DeviceIdentity {
    /// Fold this identity into the host currently being scanned.
    ///
    /// First-write-wins throughout, via the existing `with_*` setters, and applied *after* the
    /// credentialed integrations have run — so an SNMP or controller read of the same field keeps
    /// its value. That is the same rule and the same reasoning as
    /// [`ControllerIdentity::enrich`](crate::daemon::discovery::integration::controller::ControllerIdentity::enrich):
    /// a credentialed read reaches the device's own inventory, while a probe sees only what a
    /// discovery packet happens to carry.
    pub fn enrich(&self, host_data: &mut HostData) {
        let Self {
            manufacturer,
            model,
            serial_number,
            firmware_revision,
        } = self.clone().normalized();

        if let Some(manufacturer) = manufacturer {
            host_data.with_manufacturer(manufacturer);
        }
        if let Some(model) = model {
            host_data.with_model(model);
        }
        if let Some(serial_number) = serial_number {
            host_data.with_serial_number(serial_number);
        }
        if let Some(firmware_revision) = firmware_revision {
            host_data.with_firmware_revision(firmware_revision);
        }
    }

    /// Devices routinely pad a fixed-width identity field with spaces or return it empty. An empty
    /// string is absence, not a value, and must not displace something real.
    fn normalized(self) -> Self {
        fn blank_to_none(v: Option<String>) -> Option<String> {
            v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
        }
        Self {
            manufacturer: blank_to_none(self.manufacturer),
            model: blank_to_none(self.model),
            serial_number: blank_to_none(self.serial_number),
            firmware_revision: blank_to_none(self.firmware_revision),
        }
    }
}

/// A non-credentialed application probe.
#[async_trait]
pub trait AppProbe: Send + Sync {
    /// The port this probe speaks on. Also the port the definition's discovery pattern scans, so
    /// the two cannot disagree.
    fn port(&self) -> PortType;

    /// The evidence a successful probe contributes to service matching, for protocols whose
    /// definition matches on [`Pattern::ClientResponse`](crate::server::services::r#impl::patterns::Pattern::ClientResponse).
    ///
    /// `None` for the four probes migrated off the old dispatch: their definitions match on the
    /// port alone, exactly as they did before, and minting `ClientProbe` variants nothing consumes
    /// would manufacture the very problem `every_client_probe_variant_has_a_producer` exists to
    /// catch.
    fn client_probe(&self) -> Option<ClientProbe> {
        None
    }

    /// Whether this probe applies to this target at all. DHCP overrides it.
    fn applies(&self, _ctx: &ProbeContext) -> bool {
        true
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error>;
}

/// What one answered probe contributes back to the scan.
#[derive(Debug, Clone)]
pub struct AppProbeResult {
    pub port: PortType,
    pub client_probe: Option<ClientProbe>,
    pub identity: Option<DeviceIdentity>,
}

/// The probe registry *is* the service-definition registry.
pub fn all_app_probes() -> Vec<Box<dyn AppProbe>> {
    ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .filter_map(|d| d.app_probe())
        .collect()
}

/// Run every applicable probe against one address.
///
/// TCP probes gate on the connect scan having found the port open; UDP has no connect scan to gate
/// on, so its probes always run — which is what `scan_udp_ports` did for the same four probes.
pub async fn scan_app_probes(
    ctx: &ProbeContext,
    open_ports: &[PortType],
    batch_size: usize,
    scan_rate_pps: u32,
) -> Vec<AppProbeResult> {
    let probes: Vec<Box<dyn AppProbe>> = all_app_probes()
        .into_iter()
        .filter(|p| p.applies(ctx))
        .filter(|p| p.port().is_udp() || open_ports.contains(&p.port()))
        .collect();

    if probes.is_empty() {
        return Vec::new();
    }

    // UDP is slower and less reliable, capped the same way `scan_udp_ports` capped it.
    let probe_batch_size = std::cmp::min(batch_size, 10);
    let total = probes.len();
    let ip = ctx.ip;
    let ctx = ctx.clone();

    let results = batch_scan(
        probes,
        probe_batch_size,
        scan_rate_pps,
        ctx.cancel.clone(),
        move |probe| {
            let ctx = ctx.clone();
            async move {
                let port = probe.port();
                match probe.run(&ctx).await {
                    Ok(AppProbeOutcome::Answered { identity }) => {
                        tracing::trace!(ip = %ctx.ip, port = %port.number(), "Application probe answered");
                        Some(AppProbeResult {
                            port,
                            client_probe: probe.client_probe(),
                            identity,
                        })
                    }
                    Ok(AppProbeOutcome::NoAnswer) => None,
                    Err(e) => {
                        if DiscoveryCriticalError::is_critical_error(e.to_string()) {
                            tracing::error!(
                                "Critical error running application probe {}:{}: {}",
                                ctx.ip,
                                port.number(),
                                e
                            );
                        }
                        None
                    }
                }
            }
        },
    )
    .await;

    tracing::debug!(
        ip = %ip,
        probes_run = total,
        answered = results.len(),
        "Application probes complete"
    );

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every probe is reachable only through a service definition, so this also asserts that each
    /// one has a definition behind it.
    #[test]
    fn every_registered_probe_declares_a_port_and_is_uniquely_ported() {
        let probes = all_app_probes();
        assert!(
            !probes.is_empty(),
            "the registry should expose the migrated UDP probes at minimum"
        );

        let mut ports: Vec<_> = probes.iter().map(|p| p.port()).collect();
        let before = ports.len();
        ports.sort_by_key(|p| (p.number(), p.protocol()));
        ports.dedup();
        assert_eq!(
            before,
            ports.len(),
            "two probes claiming one port would race for the same evidence slot"
        );
    }

    #[test]
    fn blank_identity_fields_are_absence_not_values() {
        let identity = DeviceIdentity {
            manufacturer: Some("   ".to_string()),
            model: Some("".to_string()),
            serial_number: Some("  FOC1234X5YZ  ".to_string()),
            firmware_revision: None,
        }
        .normalized();

        assert_eq!(identity.manufacturer, None);
        assert_eq!(identity.model, None);
        assert_eq!(identity.serial_number, Some("FOC1234X5YZ".to_string()));
    }
}
