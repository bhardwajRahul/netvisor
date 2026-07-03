pub mod arp;
mod dns;
mod scan;
mod subnets;

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use mac_address::MacAddress;
use uuid::Uuid;

use crate::daemon::discovery::integration::IntegrationRegistry;
use crate::daemon::utils::scanner::ScanConcurrencyController;
use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::HostNamingFallback;
use crate::server::ip_addresses::r#impl::base::IPAddress;
use crate::server::services::r#impl::base::Service;
use crate::server::subnets::r#impl::base::Subnet;

/// Per-host data discovered during deep_scan_host().
/// Used by subsequent discovery phases (e.g., Docker container scanning) to link
/// containers to the correct virtualizing service and provide host ip_addresses.
#[derive(Debug, Clone, Default)]
pub struct DiscoveredHostData {
    pub docker_service_id: Option<Uuid>,
    pub ip_addresses: Vec<IPAddress>,
}

/// Grace period to wait for late ARP arrivals after the last deep scan completes
const LATE_ARRIVAL_GRACE_PERIOD: Duration = Duration::from_secs(30);

/// Hard maximum duration for a single discovery run (safety net)
const MAX_DISCOVERY_DURATION: Duration = Duration::from_secs(21600); // 6 hours

/// Maximum interval between progress reports (heartbeat even if progress unchanged)
const MAX_PROGRESS_REPORT_INTERVAL: Duration = Duration::from_secs(30);

// Progress phase weights (must sum to 100)
const PROGRESS_ARP_PHASE: u8 = 30; // 0-30%: ARP discovery
const PROGRESS_DEEP_SCAN_PHASE: u8 = 65; // 30-95%: Deep scanning
const PROGRESS_GRACE_PHASE: u8 = 5; // 95-100%: Grace period

/// Cost of a full port scan per host in centiseconds
const FULL_SCAN_COST_CS: usize = 9000; // ~90 seconds
/// Cost of a light scan per host in centiseconds
const LIGHT_SCAN_COST_CS: usize = 800; // ~8 seconds

#[derive(Default)]
pub struct NetworkScan {
    subnet_ids: Option<Vec<Uuid>>,
    host_naming_fallback: HostNamingFallback,
    scan_settings: ScanSettings,
    /// All credential mappings for integration dispatch.
    credential_mappings: Vec<
        crate::server::credentials::r#impl::mapping::CredentialMapping<
            crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
        >,
    >,
    /// Precomputed set of ports for light scans (discovery + credential ports)
    light_scan_ports: HashSet<u16>,
}

impl NetworkScan {
    pub fn new(
        subnet_ids: Option<Vec<Uuid>>,
        host_naming_fallback: HostNamingFallback,
        scan_settings: ScanSettings,
        credential_mappings: Vec<
            crate::server::credentials::r#impl::mapping::CredentialMapping<
                crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
            >,
        >,
    ) -> Self {
        // Build light scan port set: discovery ports + credential-required ports
        let mut light_scan_ports: HashSet<u16> = Service::all_discovery_ports()
            .iter()
            .filter(|p| p.is_tcp())
            .map(|p| p.number())
            .collect();

        // Add ports from all credential types generically
        for mapping in &credential_mappings {
            if let Some(default) = &mapping.default_credential {
                light_scan_ports.extend(default.required_scan_ports());
            }
            for override_entry in &mapping.ip_overrides {
                light_scan_ports.extend(override_entry.credential.required_scan_ports());
            }
        }

        Self {
            subnet_ids,
            host_naming_fallback,
            scan_settings,
            credential_mappings,
            light_scan_ports,
        }
    }

    /// Compute the total integration cost (centiseconds) for a specific IP.
    /// Sums estimated_seconds for each integration that has a credential covering this IP.
    fn compute_integration_cost_for_ip(&self, ip: IpAddr) -> usize {
        self.credential_mappings
            .iter()
            .filter_map(|m| {
                let discriminant: CredentialQueryPayloadDiscriminants = m
                    .default_credential
                    .as_ref()
                    .map(|c| c.into())
                    .or_else(|| m.ip_overrides.first().map(|o| (&o.credential).into()))?;
                let has_cred =
                    m.ip_overrides.iter().any(|o| o.ip == ip) || m.default_credential.is_some();
                if has_cred {
                    let integration = IntegrationRegistry::get(discriminant)?;
                    Some(integration.estimated_seconds() as usize * 100)
                } else {
                    None
                }
            })
            .sum()
    }
}

pub(super) struct DeepScanParams<'a> {
    ip: IpAddr,
    subnet: &'a Subnet,
    mac: Option<MacAddress>,
    cancel: tokio_util::sync::CancellationToken,
    scan_rate_pps: u32,
    port_scan_batch_size: usize,
    gateway_ips: &'a [IpAddr],
    completed_cost: Option<&'a Arc<AtomicUsize>>,
    total_cost: Option<&'a Arc<AtomicUsize>>,
    hosts_discovered: Option<&'a Arc<AtomicUsize>>,
    batches_per_host: usize,
    scan_cost_cs: usize,
    scan_controller: Arc<ScanConcurrencyController>,
    probe_raw_socket_ports: bool,
    early_host_id: Uuid,
    is_full_scan: bool,
    light_scan_ports: &'a HashSet<u16>,
    credential_mappings: &'a [crate::server::credentials::r#impl::mapping::CredentialMapping<
        crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
    >],
    created_subnets: Vec<Subnet>,
}
