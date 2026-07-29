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
    DiscoveryIntegration, IntegrationContext, IntegrationRegistry, ProbeContext, ProbeSuccess,
    execute_with_progress_reporting,
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
    accept_invalid_certs: bool,
) -> Result<IntegrationProbeResults, Error> {
    let mut results = IntegrationProbeResults {
        client_responses: HashMap::new(),
        probe_handles: HashMap::new(),
        working_credential_ids: HashMap::new(),
        additional_ports: Vec::new(),
    };

    // Combine caller's open ports with probe-discovered ports for gate checks
    let mut all_open_ports: Vec<PortType> = open_ports.to_vec();

    // First pass (synchronous, cheap): resolve each mapping to a probe task, applying
    // the discriminant / integration / credentials / gate checks. Gate checks use the
    // port-scan `open_ports`; probe-discovered ports don't feed later gates (negligible
    // in practice — probes surface their own service's ports — and it lets the probes
    // run concurrently below).
    struct ProbeTask<'a> {
        discriminant: CredentialQueryPayloadDiscriminants,
        integration: Box<dyn DiscoveryIntegration>,
        credentials: Vec<(&'a CredentialQueryPayload, Option<Uuid>)>,
    }
    let mut tasks: Vec<ProbeTask> = Vec::new();
    for mapping in credential_mappings {
        let Some(discriminant) = mapping
            .default_credential
            .as_ref()
            .map(|c| c.into())
            .or_else(|| mapping.ip_overrides.first().map(|o| (&o.credential).into()))
        else {
            continue;
        };
        let Some(integration) = IntegrationRegistry::get(discriminant) else {
            tracing::warn!(integration = ?discriminant, "Skipping unrecognized credential type from newer server");
            continue;
        };
        let credentials = resolve_credentials_for_ip(mapping, ip);
        if credentials.is_empty() {
            continue;
        }
        if !skip_gate {
            let gate_ports = integration.probe_gate_ports(credentials[0].0);
            if !gate_ports.is_empty() && !gate_ports.iter().all(|gp| all_open_ports.contains(gp)) {
                continue;
            }
        }
        tasks.push(ProbeTask {
            discriminant,
            integration,
            credentials,
        });
    }

    if cancel.is_cancelled() {
        return Err(Error::msg("Discovery was cancelled"));
    }

    // Probe all mappings concurrently. Each task tries its credentials in order and
    // returns the first success (or None). This collapses the previously-serial
    // per-credential probe latency (e.g. v1+v2c+v3 SNMP + the public default, each with
    // multi-second UDP timeouts on non-responders) into roughly one probe's wall-clock.
    let outcomes = futures::future::join_all(tasks.into_iter().map(|task| {
        let ProbeTask {
            discriminant,
            integration,
            credentials,
        } = task;
        async move {
            for (credential, cred_id) in &credentials {
                if cancel.is_cancelled() {
                    return None;
                }
                match integration
                    .probe(&ProbeContext {
                        ip,
                        credential,
                        credential_id: *cred_id,
                        cancel,
                        utils,
                        accept_invalid_certs,
                    })
                    .await
                {
                    Ok(success) => {
                        return Some((discriminant, *cred_id, (*credential).clone(), success));
                    }
                    Err(failure) => {
                        tracing::debug!(ip = %ip, integration = ?discriminant, error = %failure, "Integration probe failed, trying next credential");
                    }
                }
            }
            None
        }
    }))
    .await;

    if cancel.is_cancelled() {
        return Err(Error::msg("Discovery was cancelled"));
    }

    // Merge in original mapping order so winner-selection is unchanged from the serial
    // version: for a given integration the last successful mapping's credential wins
    // (overwrite), and probe-discovered ports are unioned.
    for (discriminant, cred_id, credential, success) in outcomes.into_iter().flatten() {
        let ProbeSuccess {
            client_probe,
            ports,
            handle,
        } = success;
        tracing::info!(ip = %ip, integration = ?discriminant, ports = ?ports, "Integration probe succeeded");
        for port in &ports {
            if !all_open_ports.contains(port) {
                all_open_ports.push(*port);
                results.additional_ports.push(*port);
            }
        }
        results.client_responses.insert(client_probe, ports);
        if let Some(handle) = handle {
            results.probe_handles.insert(discriminant, handle);
        }
        // `cred_id` is Some for user-configured creds and None for network-default
        // fallbacks; execute needs the payload either way, so we insert unconditionally.
        results
            .working_credential_ids
            .insert(discriminant, (cred_id, credential));
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

    host_data
        .host
        .base
        .credential_assignments
        .extend(credential_assignments_from_probes(
            &probe_results.working_credential_ids,
            params.ip_address_id,
        ));

    Ok(())
}

/// Turn the credentials that probed successfully into host credential assignments.
///
/// This is how a discovery-scoped credential earns its keep: the server drops a
/// discovery's one-shot `integration_targets` once a scan completes, and what
/// survives is exactly the assignments produced here (written to the
/// `host_credentials` junction by `discover_host`). A Docker/Podman socket
/// credential probed over the daemon's own loopback address earns an assignment
/// on the daemon host and so keeps scanning containers on every later scan.
///
/// Two kinds are deliberately excluded:
/// - **SNMP**, which records its own assignments in `SnmpIntegration::execute`.
/// - **Network defaults** (`None` id), which are network-wide by definition and
///   must not be pinned to whichever host happened to answer them.
///
/// Runs on probe success alone — a matched-service skip or a failed `execute()`
/// does not suppress it, because the credential is proven either way.
pub(crate) fn credential_assignments_from_probes(
    working_credential_ids: &HashMap<
        CredentialQueryPayloadDiscriminants,
        (Option<Uuid>, CredentialQueryPayload),
    >,
    ip_address_id: Option<Uuid>,
) -> Vec<CredentialAssignment> {
    working_credential_ids
        .iter()
        .filter(|(discriminant, _)| **discriminant != CredentialQueryPayloadDiscriminants::Snmp)
        .filter_map(|(_, (cred_id, _credential))| {
            Some(CredentialAssignment {
                credential_id: (*cred_id)?,
                ip_address_ids: ip_address_id.map(|id| vec![id]),
            })
        })
        .collect()
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
        entries: Vec<(
            CredentialQueryPayloadDiscriminants,
            Option<Uuid>,
            CredentialQueryPayload,
        )>,
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
        assert_eq!(
            keys,
            vec![(CredentialQueryPayloadDiscriminants::Snmp, cred_id)]
        );
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

    fn docker_socket_payload() -> CredentialQueryPayload {
        CredentialQueryPayload::DockerSocket(ContainerSocketQueryCredential { socket_path: None })
    }

    /// A local Docker/Podman socket credential arrives as a `DaemonHost` integration target,
    /// which the server drops from the discovery once the scan completes. What has to carry it
    /// into every later scan is the assignment produced here, on the daemon host's own loopback
    /// address — without it, container discovery would silently stop after the first scan.
    #[test]
    fn a_working_socket_credential_earns_an_assignment_on_the_address_it_probed() {
        let cred_id = Uuid::new_v4();
        let loopback_ip_id = Uuid::new_v4();
        let w = winners(vec![(
            CredentialQueryPayloadDiscriminants::DockerSocket,
            Some(cred_id),
            docker_socket_payload(),
        )]);

        let assignments = credential_assignments_from_probes(&w, Some(loopback_ip_id));

        assert_eq!(assignments.len(), 1);
        assert_eq!(assignments[0].credential_id, cred_id);
        assert_eq!(assignments[0].ip_address_ids, Some(vec![loopback_ip_id]));
    }

    /// Two kinds must never be promoted here: SNMP records its own assignments inside
    /// `SnmpIntegration::execute`, and a network default (`None` id) is network-wide by
    /// definition — pinning it to whichever host answered would turn a broadcast credential
    /// into a host-scoped one.
    #[test]
    fn snmp_and_network_defaults_are_not_promoted() {
        let w = winners(vec![
            (
                CredentialQueryPayloadDiscriminants::Snmp,
                Some(Uuid::new_v4()),
                CredentialQueryPayload::default(),
            ),
            (
                CredentialQueryPayloadDiscriminants::DockerSocket,
                None,
                docker_socket_payload(),
            ),
        ]);

        assert!(
            credential_assignments_from_probes(&w, Some(Uuid::new_v4())).is_empty(),
            "neither an SNMP winner nor an unidentified network default earns a host assignment"
        );
    }
}
