//! Generic integration dispatch — probe and execute integrations for any host.
//!
//! Used by both network scanning (deep_scan_host) and localhost phase.
//! Given credential mappings + a target IP, probes each integration, then
//! executes successful ones against HostData.

use std::any::Any;
use std::collections::HashMap;
use std::net::IpAddr;

use anyhow::Error;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::daemon::discovery::credentials::resolve_credentials_for_ip;
use crate::daemon::discovery::service::ops::{DiscoveryOps, HostData};
use crate::daemon::utils::base::PlatformDaemonUtils;
use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::credentials::r#impl::types::CredentialAssignment;
use crate::server::discovery::r#impl::types::HostNamingFallback;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::subnets::r#impl::base::Subnet;

use super::{
    IntegrationContext, IntegrationRegistry, ProbeContext, execute_with_progress_reporting,
};

/// Results from probing all integrations for a single host IP.
pub struct IntegrationProbeResults {
    pub client_responses: HashMap<ClientProbe, Vec<PortType>>,
    pub probe_handles: HashMap<CredentialQueryPayloadDiscriminants, Box<dyn Any + Send + Sync>>,
    /// The credential that successfully probed per integration — `cred_id` is
    /// `Some` for user-configured (host-assigned) credentials and `None` for
    /// network-default fallbacks. Execute reads from this to run against the
    /// credential that actually worked; only `Some` entries participate in
    /// credential_assignments (defaults are network-wide, not host-scoped).
    pub working_credential_ids:
        HashMap<CredentialQueryPayloadDiscriminants, (Option<Uuid>, CredentialQueryPayload)>,
    /// Ports discovered by integration probes (added to open_ports).
    pub additional_ports: Vec<PortType>,
}

/// Probe all integrations for a host IP against credential mappings.
///
/// For each credential mapping, resolves the credential for this IP,
/// checks probe gate ports, then tries probe until one succeeds.
/// Returns aggregated probe results for subsequent service matching and execution.
/// `skip_gate` bypasses `probe_gate_ports` — used for the daemon's own host
/// (localhost) phase, which does no port scan and lets integrations self-probe.
/// The network-scan phase passes `false` so the gate keeps the broad scan cheap.
pub async fn probe_integrations(
    ip: IpAddr,
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
    open_ports: &[PortType],
    skip_gate: bool,
    cancel: &CancellationToken,
    utils: &PlatformDaemonUtils,
) -> Result<IntegrationProbeResults, Error> {
    let mut results = IntegrationProbeResults {
        client_responses: HashMap::new(),
        probe_handles: HashMap::new(),
        working_credential_ids: HashMap::new(),
        additional_ports: Vec::new(),
    };

    // Combine caller's open ports with probe-discovered ports for gate checks
    let mut all_open_ports: Vec<PortType> = open_ports.to_vec();

    for mapping in credential_mappings {
        let discriminant: Option<CredentialQueryPayloadDiscriminants> = mapping
            .default_credential
            .as_ref()
            .map(|c| c.into())
            .or_else(|| mapping.ip_overrides.first().map(|o| (&o.credential).into()));

        let Some(discriminant) = discriminant else {
            continue;
        };

        if cancel.is_cancelled() {
            return Err(Error::msg("Discovery was cancelled"));
        }

        let Some(integration) = IntegrationRegistry::get(discriminant) else {
            tracing::warn!(integration = ?discriminant, "Skipping unrecognized credential type from newer server");
            continue;
        };

        let credentials = resolve_credentials_for_ip(mapping, ip);
        if credentials.is_empty() {
            tracing::debug!(ip = %ip, integration = ?discriminant, "No credentials for this IP, skipping");
            continue;
        }

        tracing::debug!(ip = %ip, integration = ?discriminant, credentials = credentials.len(), "Probing integration");

        // Check probe gate ports (skipped on the daemon's own host, where there's
        // no port scan and integrations probe directly).
        if !skip_gate {
            let gate_ports = integration.probe_gate_ports(credentials[0].0);
            if !gate_ports.is_empty() && !gate_ports.iter().all(|gp| all_open_ports.contains(gp)) {
                continue;
            }
        }

        // Try each credential until probe succeeds
        for (credential, cred_id) in &credentials {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            let probe_ctx = ProbeContext {
                ip,
                credential,
                credential_id: *cred_id,
                cancel,
                utils,
            };

            match integration.probe(&probe_ctx).await {
                Ok(success) => {
                    tracing::info!(
                        ip = %ip,
                        integration = ?discriminant,
                        ports = ?success.ports,
                        "Integration probe succeeded"
                    );
                    // Track probe-discovered ports
                    for port in &success.ports {
                        if !all_open_ports.contains(port) {
                            all_open_ports.push(*port);
                            results.additional_ports.push(*port);
                        }
                    }
                    results
                        .client_responses
                        .insert(success.client_probe, success.ports);
                    if let Some(handle) = success.handle {
                        results.probe_handles.insert(discriminant, handle);
                    }
                    // Record the winning credential for this integration.
                    // `cred_id` is Some for user-configured creds (host assignments)
                    // and None for network-default fallbacks; execute needs the
                    // payload either way, so we insert unconditionally.
                    results
                        .working_credential_ids
                        .insert(discriminant, (*cred_id, (*credential).clone()));
                    break;
                }
                Err(failure) => {
                    tracing::debug!(
                        ip = %ip,
                        integration = ?discriminant,
                        error = %failure,
                        "Integration probe failed, trying next credential"
                    );
                }
            }
        }
    }

    Ok(results)
}

/// Parameters for integration execution dispatch.
pub struct ExecuteParams<'a> {
    pub ip: IpAddr,
    pub cancel: &'a CancellationToken,
    pub ops: &'a DiscoveryOps,
    pub utils: &'a PlatformDaemonUtils,
    pub open_ports: &'a [PortType],
    pub endpoint_responses: &'a [crate::server::services::r#impl::endpoints::EndpointResponse],
    pub host_id: Uuid,
    pub host_naming_fallback: HostNamingFallback,
    pub created_subnets: &'a [Subnet],
    pub scanning_subnet: Option<&'a Subnet>,
    pub ip_address_id: Option<Uuid>,
}

/// Execute integrations whose probe succeeded and whose associated service was matched.
///
/// Derive the integration discriminant a mapping resolves to (from its default
/// credential, else its first ip-override).
fn mapping_discriminant(
    mapping: &CredentialMapping<CredentialQueryPayload>,
) -> Option<CredentialQueryPayloadDiscriminants> {
    mapping
        .default_credential
        .as_ref()
        .map(|c| c.into())
        .or_else(|| mapping.ip_overrides.first().map(|o| (&o.credential).into()))
}

/// Collapse credential mappings to the distinct `(integration, winning credential id)`
/// collections `execute_integrations` should run, preserving first-seen order and
/// dropping mappings with no probe winner. Deduping by the winning credential (not the
/// mapping) means N mappings that share one integration + winner run once, while a
/// distinct winning credential still runs.
fn dedup_execution_keys(
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
    working_credential_ids: &HashMap<
        CredentialQueryPayloadDiscriminants,
        (Option<Uuid>, CredentialQueryPayload),
    >,
) -> Vec<(CredentialQueryPayloadDiscriminants, Option<Uuid>)> {
    let mut seen = std::collections::HashSet::new();
    let mut keys = Vec::new();
    for mapping in credential_mappings {
        let Some(discriminant) = mapping_discriminant(mapping) else {
            continue;
        };
        let Some((cred_id, _)) = working_credential_ids.get(&discriminant) else {
            continue;
        };
        let key = (discriminant, *cred_id);
        if seen.insert(key) {
            keys.push(key);
        }
    }
    keys
}

/// Enriches host_data with integration-discovered services, ports, ip_addresses.
/// Also populates credential_assignments for successful integrations.
pub async fn execute_integrations(
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
    probe_results: &IntegrationProbeResults,
    host_data: &mut HostData,
    params: &ExecuteParams<'_>,
) -> Result<(), Error> {
    // Multiple credential mappings can resolve to the same integration + winning
    // credential (e.g. SnmpV1/V2c/V3 credentials plus the injected public default all
    // collapse to the single Snmp discriminant, which has one probe winner). Running
    // execute() once per mapping re-does the full collection against the same host
    // with the same credential — pure repetition. dedup_execution_keys() collapses
    // the mappings to the distinct (integration, winning credential) collections that
    // actually need to run; a genuinely different winning credential still runs.
    for (discriminant, _cred_id) in
        dedup_execution_keys(credential_mappings, &probe_results.working_credential_ids)
    {
        let Some(integration) = IntegrationRegistry::get(discriminant) else {
            continue;
        };

        // Use the credential that actually succeeded during probe. If no probe
        // winner was recorded for this integration, there's nothing to execute.
        let Some((cred_id, credential)) = probe_results.working_credential_ids.get(&discriminant)
        else {
            continue;
        };

        // Check if integration's associated service was matched
        let cred_type_discriminant: crate::server::credentials::r#impl::types::CredentialTypeDiscriminants = discriminant.into();
        let associated_service = cred_type_discriminant
            .to_credential_type()
            .associated_service();
        let service_matched = host_data
            .services
            .iter()
            .any(|s| s.base.service_definition.id() == associated_service.id());

        if !service_matched {
            continue;
        }

        let accept_invalid_certs = params
            .ops
            .config_store
            .get_accept_invalid_scan_certs()
            .await
            .unwrap_or(false);

        let matched_services_snapshot = host_data.services.clone();

        let probe_handle_ref = probe_results
            .probe_handles
            .get(&discriminant)
            .map(|h| h.as_ref() as &(dyn std::any::Any + Send + Sync));

        let ctx = IntegrationContext {
            ip: params.ip,
            credential,
            credential_id: *cred_id,
            cancel: params.cancel,
            ops: params.ops,
            utils: params.utils,
            probe_handle: probe_handle_ref,
            matched_services: &matched_services_snapshot,
            open_ports: params.open_ports,
            endpoint_responses: params.endpoint_responses,
            host_id: params.host_id,
            host_naming_fallback: params.host_naming_fallback,
            created_subnets: params.created_subnets,
            accept_invalid_certs,
            scanning_subnet: params.scanning_subnet,
        };

        if let Err(e) =
            execute_with_progress_reporting(integration.as_ref(), &ctx, host_data, || async {
                let pct = params
                    .ops
                    .get_session()
                    .await
                    .map(|s| s.last_progress.load(std::sync::atomic::Ordering::Relaxed))
                    .unwrap_or(0);
                let _ = params.ops.report_progress(pct).await;
            })
            .await
        {
            // A failed integration execute means a matched service (e.g. a Docker/Podman
            // daemon) produced no child services — the user-visible "unclaimed open ports,
            // no services" symptom. Surface it at warn so the underlying error (often a
            // bollard/serde response mismatch) is diagnosable rather than swallowed.
            tracing::warn!(
                ip = %params.ip,
                integration = ?discriminant,
                error = %e,
                "Integration execute failed"
            );
        }
    }

    // Populate credential_assignments from successful integration probes
    // whose execute() doesn't handle credential assignments itself.
    // SNMP is handled by SnmpIntegration.execute().
    // Only user-configured credentials (Some id) are auto-assigned; network
    // defaults (None id) are network-wide and not host-scoped.
    for (discriminant, (cred_id, _credential)) in &probe_results.working_credential_ids {
        if *discriminant == CredentialQueryPayloadDiscriminants::Snmp {
            continue;
        }
        let Some(cred_id) = cred_id else {
            continue;
        };
        host_data
            .host
            .base
            .credential_assignments
            .push(CredentialAssignment {
                credential_id: *cred_id,
                ip_address_ids: params.ip_address_id.map(|id| vec![id]),
            });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::ContainerSocketQueryCredential;

    fn snmp_mapping() -> CredentialMapping<CredentialQueryPayload> {
        CredentialMapping {
            default_credential: Some(CredentialQueryPayload::default()), // Snmp
            ip_overrides: Vec::new(),
        }
    }

    fn docker_socket_mapping() -> CredentialMapping<CredentialQueryPayload> {
        CredentialMapping {
            default_credential: Some(CredentialQueryPayload::DockerSocket(
                ContainerSocketQueryCredential { socket_path: None },
            )),
            ip_overrides: Vec::new(),
        }
    }

    fn winners(
        entries: Vec<(CredentialQueryPayloadDiscriminants, Option<Uuid>, CredentialQueryPayload)>,
    ) -> HashMap<CredentialQueryPayloadDiscriminants, (Option<Uuid>, CredentialQueryPayload)> {
        entries
            .into_iter()
            .map(|(d, id, payload)| (d, (id, payload)))
            .collect()
    }

    #[test]
    fn dedup_collapses_duplicate_snmp_mappings_to_one() {
        // SnmpV1/V2c/V3 + injected public default all resolve to the single Snmp
        // discriminant with one probe winner: three mappings, one collection.
        let mappings = vec![snmp_mapping(), snmp_mapping(), snmp_mapping()];
        let cred_id = Some(Uuid::new_v4());
        let w = winners(vec![(
            CredentialQueryPayloadDiscriminants::Snmp,
            cred_id,
            CredentialQueryPayload::default(),
        )]);

        let keys = dedup_execution_keys(&mappings, &w);
        assert_eq!(keys, vec![(CredentialQueryPayloadDiscriminants::Snmp, cred_id)]);
    }

    #[test]
    fn dedup_drops_mappings_without_probe_winner() {
        // No probe winner for the mapping's integration => nothing to execute.
        let mappings = vec![snmp_mapping()];
        let w = winners(vec![]);
        assert!(dedup_execution_keys(&mappings, &w).is_empty());
    }

    #[test]
    fn dedup_preserves_distinct_integrations_in_order() {
        // Different integrations each keep their own collection; first-seen order.
        let mappings = vec![snmp_mapping(), docker_socket_mapping(), snmp_mapping()];
        let snmp_id = Some(Uuid::new_v4());
        let docker_id = Some(Uuid::new_v4());
        let w = winners(vec![
            (
                CredentialQueryPayloadDiscriminants::Snmp,
                snmp_id,
                CredentialQueryPayload::default(),
            ),
            (
                CredentialQueryPayloadDiscriminants::DockerSocket,
                docker_id,
                CredentialQueryPayload::DockerSocket(ContainerSocketQueryCredential {
                    socket_path: None,
                }),
            ),
        ]);

        let keys = dedup_execution_keys(&mappings, &w);
        assert_eq!(
            keys,
            vec![
                (CredentialQueryPayloadDiscriminants::Snmp, snmp_id),
                (CredentialQueryPayloadDiscriminants::DockerSocket, docker_id),
            ]
        );
    }
}
