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

// The hard maximum duration for a single discovery run is now server-configurable
// via ScanSettings::max_discovery_duration (default 21600s = 6h); see scan.rs.

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

/// Cost (centiseconds) of a non-interfaced IP's TCP responsiveness check (an ~2s
/// multi-port connect scan a dead IP still has to be probed with). Counted in the cost
/// model — seeded into total_cost up front and accrued into completed_cost as each
/// check finishes — so progress/ETA reflect draining a large non-interfaced range
/// instead of pinning at ~95%/"<1 min" while those IPs are still being checked.
const RESPONSIVENESS_COST_CS: usize = 200; // ~2 seconds

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
    /// Specific addresses to scan (a rescan). `None` sweeps the subnets.
    target_ips: Option<HashSet<std::net::IpAddr>>,
    /// Precomputed TCP port set: discovery ports, credential-required ports, and
    /// for a rescan the ports already known on the target.
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
        target_ips: Option<HashSet<std::net::IpAddr>>,
        extra_ports: Vec<u16>,
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

        // A rescan verifies the ports already recorded on its target, so fold
        // them in the same way credentials widen the set. Cost and batching are
        // derived from this set's size, so they stay correct for free.
        light_scan_ports.extend(extra_ports);

        Self {
            subnet_ids,
            host_naming_fallback,
            scan_settings,
            credential_mappings,
            target_ips,
            light_scan_ports,
        }
    }

    /// Compute the total integration cost (centiseconds) for a specific IP.
    /// Thin delegator to [`integration_cost_for_ip`].
    fn compute_integration_cost_for_ip(&self, ip: IpAddr) -> usize {
        integration_cost_for_ip(&self.credential_mappings, ip)
    }
}

/// Total integration cost (centiseconds) the daemon attributes to `ip`, counting each
/// integration **once** regardless of how many credentials of that type cover the IP.
///
/// SNMP now runs a single collection per host (see `execute_integrations`' dedup), so
/// N SNMP credentials (v1/v2c/v3 + the injected "public" default) must cost the same as
/// one. This is used symmetrically for both the total-cost estimate and the completed-
/// cost accrual (`scan.rs`) so `completed_cost` converges to `total_cost` and the scan
/// ETA/progress stay accurate instead of over-counting per-credential.
pub(super) fn integration_cost_for_ip(
    credential_mappings: &[crate::server::credentials::r#impl::mapping::CredentialMapping<
        crate::server::credentials::r#impl::mapping::CredentialQueryPayload,
    >],
    ip: IpAddr,
) -> usize {
    let mut seen: HashSet<CredentialQueryPayloadDiscriminants> = HashSet::new();
    credential_mappings
        .iter()
        .filter_map(|m| {
            let discriminant: CredentialQueryPayloadDiscriminants = m
                .default_credential
                .as_ref()
                .map(|c| c.into())
                .or_else(|| m.ip_overrides.first().map(|o| (&o.credential).into()))?;
            let has_cred =
                m.ip_overrides.iter().any(|o| o.ip == ip) || m.default_credential.is_some();
            // Count each integration discriminant at most once per IP.
            if !has_cred || !seen.insert(discriminant) {
                return None;
            }
            let integration = IntegrationRegistry::get(discriminant)?;
            Some(integration.estimated_seconds() as usize * 100)
        })
        .sum()
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

#[cfg(test)]
mod tests {
    use super::integration_cost_for_ip;
    use crate::server::credentials::r#impl::mapping::{
        ContainerSocketQueryCredential, CredentialMapping, CredentialQueryPayload,
    };
    use std::net::{IpAddr, Ipv4Addr};

    fn snmp_mapping() -> CredentialMapping<CredentialQueryPayload> {
        CredentialMapping {
            default_credential: Some(CredentialQueryPayload::default()), // Snmp
            ip_overrides: Vec::new(),
        }
    }

    fn docker_mapping() -> CredentialMapping<CredentialQueryPayload> {
        CredentialMapping {
            default_credential: Some(CredentialQueryPayload::DockerSocket(
                ContainerSocketQueryCredential { socket_path: None },
            )),
            ip_overrides: Vec::new(),
        }
    }

    // The estimate must count each integration ONCE per host regardless of how many
    // credentials of that type are configured — SNMP now runs one collection per host,
    // so v1+v2c+v3 (+ the injected public default) must cost the same as a single SNMP
    // credential. This is the ETA-inflation regression guard.
    #[test]
    fn snmp_counted_once_regardless_of_credential_count() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let one = integration_cost_for_ip(&[snmp_mapping()], ip);
        let four = integration_cost_for_ip(
            &[
                snmp_mapping(),
                snmp_mapping(),
                snmp_mapping(),
                snmp_mapping(),
            ],
            ip,
        );
        assert!(
            one > 0,
            "SNMP integration should have a non-zero estimated cost"
        );
        assert_eq!(
            one, four,
            "N SNMP credentials must cost the same as one (deduped by integration)"
        );
    }

    #[test]
    fn distinct_integrations_each_count_once() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let snmp = integration_cost_for_ip(&[snmp_mapping()], ip);
        let docker = integration_cost_for_ip(&[docker_mapping()], ip);
        let both = integration_cost_for_ip(&[snmp_mapping(), docker_mapping()], ip);
        assert_eq!(
            both,
            snmp + docker,
            "distinct integrations should each be counted once and summed"
        );
    }
}
