use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    credentials::r#impl::{
        base::Credential,
        junction::{HostCredentialStorage, NetworkCredentialStorage},
        mapping::{
            CredentialMapping, CredentialQueryPayload, IntegrationTarget, IpOverride,
            SnmpCredentialMapping, SnmpQueryCredential,
        },
        types::{
            CredentialAssignment, CredentialHostAssignment, CredentialType,
            CredentialTypeDiscriminants, SnmpVersion, Target,
        },
    },
    hosts::{r#impl::base::Host, service::HostService},
    ip_addresses::{r#impl::base::IPAddress, service::IPAddressService},
    networks::service::NetworkService,
    organizations::service::OrganizationService,
    shared::{
        events::{
            bus::EventBus,
            traits::{Event, OrgScope},
            types::{OnboardingOperation, OnboardingOperationDiscriminants},
        },
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, generic::GenericPostgresStorage},
    },
    tags::entity_tags::EntityTagService,
};
use anyhow::Error;
use async_trait::async_trait;
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::{Arc, OnceLock};
use strum::IntoDiscriminant;
use uuid::Uuid;

/// The set of things a credential targets, normalized across the network and host
/// junction tables so two credentials can be tested for overlap. Used to enforce
/// the single-endpoint invariant — see
/// [`CredentialService::find_single_endpoint_conflict`].
#[derive(Debug, Default)]
struct CredentialTargets {
    networks: HashSet<Uuid>,
    /// host_id → the IPs it is scoped to (`None` = the whole host).
    hosts: Vec<(Uuid, Option<HashSet<Uuid>>)>,
}

impl CredentialTargets {
    fn build(networks: &[Uuid], host_assignments: &[CredentialHostAssignment]) -> Self {
        Self {
            networks: networks.iter().copied().collect(),
            hosts: host_assignments
                .iter()
                .map(|h| {
                    (
                        h.host_id,
                        h.ip_address_ids
                            .as_ref()
                            .map(|ids| ids.iter().copied().collect()),
                    )
                })
                .collect(),
        }
    }

    /// A credential targeting exactly one whole host (e.g. a daemon-host target).
    fn for_host(host_id: Uuid) -> Self {
        Self {
            hosts: vec![(host_id, None)],
            ..Default::default()
        }
    }

    /// Two target sets overlap if they share a network, a target IP, or a host —
    /// where a shared host overlaps when either side covers the whole host or
    /// their IP scopes intersect.
    fn overlaps(&self, other: &Self) -> bool {
        if !self.networks.is_disjoint(&other.networks) {
            return true;
        }
        self.hosts.iter().any(|(h1, s1)| {
            other.hosts.iter().any(|(h2, s2)| {
                h1 == h2
                    && match (s1, s2) {
                        // One side covers the whole host → always overlaps.
                        (None, _) | (_, None) => true,
                        (Some(a), Some(b)) => !a.is_disjoint(b),
                    }
            })
        })
    }
}

/// The single-endpoint rule between two credentials: they conflict when both
/// belong to the same single-endpoint integration (e.g. both Docker, or both
/// Podman) and their targets overlap. This is the one rule shared by the
/// credential-write check ([`CredentialService::find_single_endpoint_conflict`])
/// and the discovery-update check ([`CredentialService::find_daemon_host_target_conflict`]);
/// the two callers differ only in the targets they feed in.
fn single_endpoint_targets_conflict(
    a_type: &CredentialType,
    a_targets: &CredentialTargets,
    b_type: &CredentialType,
    b_targets: &CredentialTargets,
) -> bool {
    a_type.single_endpoint_per_host()
        && b_type.single_endpoint_per_host()
        && ServiceDefinition::name(&*a_type.associated_service())
            == ServiceDefinition::name(&*b_type.associated_service())
        && a_targets.overlaps(b_targets)
}

pub struct CredentialService {
    storage: Arc<GenericPostgresStorage<Credential>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    #[allow(dead_code)]
    network_service: Arc<NetworkService>,
    ip_address_service: Arc<IPAddressService>,
    organization_service: Arc<OrganizationService>,
    host_service: OnceLock<Arc<HostService>>,
    network_credential_storage: NetworkCredentialStorage,
    host_credential_storage: HostCredentialStorage,
}

impl EventBusService<Credential> for CredentialService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &Credential) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, entity: &Credential) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

#[async_trait]
impl CrudService<Credential> for CredentialService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Credential>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn create(
        &self,
        entity: Credential,
        authentication: AuthenticatedEntity,
    ) -> Result<Credential, Error> {
        entity.base.credential_type.validate()?;

        let created = self.create_base(entity, authentication.clone()).await?;

        // Emit onboarding events for credential creation
        let organization_id = created.base.organization_id;
        if let Some(organization) = self
            .organization_service
            .get_by_id(&organization_id)
            .await?
        {
            // Generic event for any credential type
            if organization.not_onboarded(&OnboardingOperationDiscriminants::FirstCredentialCreated)
            {
                self.event_bus
                    .publish(Event::new(
                        OrgScope { organization_id },
                        OnboardingOperation::FirstCredentialCreated,
                        authentication.clone(),
                    ))
                    .await?;
            }

            // SNMP-specific event (preserves existing Brevo tracking) — any SNMP version counts.
            if matches!(
                created.base.credential_type,
                CredentialType::SnmpV1 { .. }
                    | CredentialType::SnmpV2c { .. }
                    | CredentialType::SnmpV3 { .. }
            ) && organization
                .not_onboarded(&OnboardingOperationDiscriminants::FirstSnmpCredentialCreated)
            {
                self.event_bus
                    .publish(Event::new(
                        OrgScope { organization_id },
                        OnboardingOperation::FirstSnmpCredentialCreated,
                        authentication,
                    ))
                    .await?;
            }
        }

        Ok(created)
    }
}

impl CredentialService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<GenericPostgresStorage<Credential>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        network_service: Arc<NetworkService>,
        ip_address_service: Arc<IPAddressService>,
        organization_service: Arc<OrganizationService>,
        pool: sqlx::PgPool,
    ) -> Self {
        Self {
            storage,
            event_bus,
            entity_tag_service,
            network_service,
            ip_address_service,
            organization_service,
            host_service: OnceLock::new(),
            network_credential_storage: NetworkCredentialStorage::new(pool.clone()),
            host_credential_storage: HostCredentialStorage::new(pool),
        }
    }

    /// Set the host service dependency after construction (breaks circular dep).
    pub fn set_host_service(&self, service: Arc<HostService>) -> Result<(), Arc<HostService>> {
        self.host_service.set(service)
    }

    // ========================================================================
    // Junction table methods — delegates to typed storage
    // ========================================================================

    /// Get credential IDs for a network from the junction table.
    pub async fn get_credential_ids_for_network(
        &self,
        network_id: &Uuid,
    ) -> Result<Vec<Uuid>, Error> {
        self.network_credential_storage
            .get_credential_ids_for_network(network_id)
            .await
    }

    /// Get credential IDs for multiple networks (batch).
    pub async fn get_credential_ids_for_networks(
        &self,
        network_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<Uuid>>, Error> {
        self.network_credential_storage
            .get_credential_ids_for_networks(network_ids)
            .await
    }

    /// Get credential assignments for a host from the junction table.
    pub async fn get_credential_assignments_for_host(
        &self,
        host_id: &Uuid,
    ) -> Result<Vec<CredentialAssignment>, Error> {
        self.host_credential_storage
            .get_assignments_for_host(host_id)
            .await
    }

    /// Get credential assignments for multiple hosts (batch).
    pub async fn get_credential_assignments_for_hosts(
        &self,
        host_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<CredentialAssignment>>, Error> {
        self.host_credential_storage
            .get_assignments_for_hosts(host_ids)
            .await
    }

    /// Replace all credentials for a network (atomic).
    pub async fn set_network_credentials(
        &self,
        network_id: &Uuid,
        credential_ids: &[Uuid],
    ) -> Result<(), Error> {
        self.network_credential_storage
            .save_for_network(network_id, credential_ids)
            .await
    }

    /// Replace all credential assignments for a host (atomic).
    pub async fn set_host_credentials(
        &self,
        host_id: &Uuid,
        assignments: &[CredentialAssignment],
    ) -> Result<(), Error> {
        self.host_credential_storage
            .save_for_host(host_id, assignments)
            .await
    }

    /// Merge `incoming` credential assignments into a host's junction (additive — never prunes).
    ///
    /// Discovery self-reports only the credentials that probed successfully on a host. Using the
    /// replace path (`set_host_credentials`) there would delete user/init-assigned daemon-host
    /// credentials that didn't probe this scan (e.g. a Docker socket on a Podman-only host),
    /// regressing the junction-as-source-of-truth model. Merge keeps existing assignments and
    /// overlays `incoming` (incoming wins per `credential_id`, refreshing `ip_address_ids`), so
    /// discovery only adds. Explicit user edits still go through the replace path.
    pub async fn merge_host_credentials(
        &self,
        host_id: &Uuid,
        incoming: &[CredentialAssignment],
    ) -> Result<(), Error> {
        let mut by_host = self
            .get_credential_assignments_for_hosts(&[*host_id])
            .await?;
        let existing = by_host.remove(host_id).unwrap_or_default();

        let incoming_ids: std::collections::HashSet<Uuid> =
            incoming.iter().map(|a| a.credential_id).collect();
        let mut merged: Vec<CredentialAssignment> = existing
            .into_iter()
            .filter(|a| !incoming_ids.contains(&a.credential_id))
            .collect();
        merged.extend(incoming.iter().cloned());

        self.set_host_credentials(host_id, &merged).await
    }

    /// Whether an integration target addresses the daemon's own host — a `DaemonHost`-scoped
    /// target, or a `Hosts` target naming a loopback IP. Such targets are junction-managed.
    fn targets_daemon_host(target: &IntegrationTarget) -> bool {
        match target {
            IntegrationTarget::DaemonHost { .. } => true,
            IntegrationTarget::Hosts { ips, .. } => ips.iter().any(|ip| ip.is_loopback()),
            IntegrationTarget::Network { .. } => false,
        }
    }

    /// Apply a discovery's integration targets, returning the targets that remain on the Discovery.
    ///
    /// Daemon-host targets (see [`Self::targets_daemon_host`]) are MERGED into the daemon host's
    /// `host_credentials` junction (additive — a no-op update can't wipe an existing assignment),
    /// because the scan-time mapping builder (`apply_integration_target`) is junction-sourced for
    /// them. The remaining Network / non-loopback Hosts targets are returned to persist on the
    /// Discovery row. Shared by daemon registration and the discovery-update handler so both apply
    /// credential targeting through identical logic.
    pub async fn apply_integration_targets(
        &self,
        daemon_host_id: Uuid,
        targets: Vec<IntegrationTarget>,
    ) -> Result<Vec<IntegrationTarget>, Error> {
        if targets.iter().any(Self::targets_daemon_host) {
            // Daemon-host credentials (local sockets, localhost proxies) connect over the
            // loopback, so scope them to the daemon host's loopback IP — not `None`, which
            // fans the credential out to every interface on the host (LAN, docker/podman
            // bridges, …). `seed_loopback` guarantees the loopback IP exists by this point.
            let loopback_ids = self.host_loopback_ip_ids(&daemon_host_id).await;
            if loopback_ids.is_empty() {
                tracing::warn!(
                    daemon_host_id = %daemon_host_id,
                    "No loopback IP on daemon host; daemon-host credentials will not be IP-scoped",
                );
            }
            let ip_address_ids = (!loopback_ids.is_empty()).then_some(loopback_ids);

            let assignments: Vec<CredentialAssignment> = targets
                .iter()
                .filter(|t| Self::targets_daemon_host(t))
                .map(|t| CredentialAssignment {
                    credential_id: t.credential_id(),
                    ip_address_ids: ip_address_ids.clone(),
                })
                .collect();
            self.merge_host_credentials(&daemon_host_id, &assignments)
                .await?;
        }

        Ok(targets
            .into_iter()
            .filter(|t| !Self::targets_daemon_host(t))
            .collect())
    }

    /// Loopback IP-address ids for a host (empty if none / unresolved). Daemon-host
    /// credentials are scoped to these so a local socket/proxy only targets 127.0.0.1
    /// instead of every interface on the daemon host.
    async fn host_loopback_ip_ids(&self, host_id: &Uuid) -> Vec<Uuid> {
        let Some(host_service) = self.host_service.get() else {
            return Vec::new();
        };
        match host_service.get_ip_addresses_for_host(host_id).await {
            Ok(ips) => ips
                .into_iter()
                .filter(|ip| ip.base.ip_address.is_loopback())
                .map(|ip| ip.id)
                .collect(),
            Err(e) => {
                tracing::warn!(host_id = %host_id, error = ?e, "Failed to resolve daemon host loopback IP");
                Vec::new()
            }
        }
    }

    /// Get the network IDs a credential is assigned to (reverse lookup).
    pub async fn get_network_ids_for_credential(
        &self,
        credential_id: &Uuid,
    ) -> Result<Vec<Uuid>, Error> {
        self.network_credential_storage
            .get_network_ids_for_credential(credential_id)
            .await
    }

    /// Get the network IDs for multiple credentials (batch, reverse lookup).
    pub async fn get_network_ids_for_credentials(
        &self,
        credential_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<Uuid>>, Error> {
        self.network_credential_storage
            .get_network_ids_for_credentials(credential_ids)
            .await
    }

    /// Get the host assignments for a credential (reverse lookup).
    pub async fn get_host_assignments_for_credential(
        &self,
        credential_id: &Uuid,
    ) -> Result<Vec<CredentialHostAssignment>, Error> {
        self.host_credential_storage
            .get_host_assignments_for_credential(credential_id)
            .await
    }

    /// Get the host assignments for multiple credentials (batch, reverse lookup).
    pub async fn get_host_assignments_for_credentials(
        &self,
        credential_ids: &[Uuid],
    ) -> Result<std::collections::HashMap<Uuid, Vec<CredentialHostAssignment>>, Error> {
        self.host_credential_storage
            .get_host_assignments_for_credentials(credential_ids)
            .await
    }

    /// Replace the full set of networks a credential is assigned to (atomic).
    pub async fn set_credential_networks(
        &self,
        credential_id: &Uuid,
        network_ids: &[Uuid],
    ) -> Result<(), Error> {
        self.network_credential_storage
            .save_networks_for_credential(credential_id, network_ids)
            .await
    }

    /// Replace the full set of host assignments for a credential (atomic).
    pub async fn set_credential_host_assignments(
        &self,
        credential_id: &Uuid,
        assignments: &[CredentialHostAssignment],
    ) -> Result<(), Error> {
        self.host_credential_storage
            .save_host_assignments_for_credential(credential_id, assignments)
            .await
    }

    /// Enforce the single-endpoint-per-host invariant: integrations whose
    /// credential type returns `single_endpoint_per_host()` (e.g. Docker) resolve
    /// to exactly one endpoint per host, so two credentials of that integration
    /// must not target an overlapping network, host, or IP (incl. the daemon host
    /// at 127.0.0.1). Returns the name of the first conflicting credential, if any.
    ///
    /// `candidate` carries its intended assignments in `base` (network ids, host
    /// assignments) — call this before persisting. Try-many
    /// integrations (e.g. SNMP, multiple communities per network) are unconstrained.
    pub async fn find_single_endpoint_conflict(
        &self,
        candidate: &Credential,
    ) -> Result<Option<String>, Error> {
        let ct = &candidate.base.credential_type;
        if !ct.single_endpoint_per_host() {
            return Ok(None);
        }
        let integration = ServiceDefinition::name(&*ct.associated_service());
        let org_id = candidate.base.organization_id;

        // Other credentials of the same integration in this org.
        let filter = StorableFilter::<Credential>::new_from_org_id(&org_id);
        let others: Vec<Credential> = self
            .get_all(filter)
            .await?
            .into_iter()
            .filter(|c| c.id != candidate.id)
            .filter(|c| {
                ServiceDefinition::name(&*c.base.credential_type.associated_service())
                    == integration
            })
            .collect();
        if others.is_empty() {
            return Ok(None);
        }

        // Hydrate the others' junction-backed assignments.
        let other_ids: Vec<Uuid> = others.iter().map(|c| c.id).collect();
        let net_map = self.get_network_ids_for_credentials(&other_ids).await?;
        let host_map = self
            .get_host_assignments_for_credentials(&other_ids)
            .await?;

        let cand = CredentialTargets::build(
            &candidate.base.assigned_network_ids,
            &candidate.base.host_assignments,
        );
        for other in &others {
            let empty_nets = Vec::new();
            let empty_hosts = Vec::new();
            let other_targets = CredentialTargets::build(
                net_map.get(&other.id).unwrap_or(&empty_nets),
                host_map.get(&other.id).unwrap_or(&empty_hosts),
            );
            if single_endpoint_targets_conflict(
                ct,
                &cand,
                &other.base.credential_type,
                &other_targets,
            ) {
                return Ok(Some(other.base.name.clone()));
            }
        }
        Ok(None)
    }

    /// Discovery-update counterpart to [`Self::find_single_endpoint_conflict`]: a
    /// daemon's `Discovery.integration_targets` all target that one daemon host, so
    /// two single-endpoint credentials of the same integration both targeting the
    /// daemon host (e.g. a Docker socket and a Docker proxy, or the Podman pair)
    /// conflict. Returns the first conflicting pair of credential names.
    ///
    /// Feeds the same [`single_endpoint_targets_conflict`] rule the credential-write
    /// path uses — only the targets differ (here, each daemon-host credential is
    /// modeled as targeting `daemon_host_id`).
    pub async fn find_daemon_host_target_conflict(
        &self,
        daemon_host_id: Uuid,
        integration_targets: &[IntegrationTarget],
    ) -> Result<Option<(String, String)>, Error> {
        // Credentials that target the daemon's own host: an explicit DaemonHost
        // scope, or a Hosts scope whose IPs are all loopback.
        let host_cred_ids: Vec<Uuid> = integration_targets
            .iter()
            .filter_map(|t| match t {
                IntegrationTarget::DaemonHost { credential_id } => Some(*credential_id),
                IntegrationTarget::Hosts { credential_id, ips }
                    if !ips.is_empty() && ips.iter().all(|ip| ip.is_loopback()) =>
                {
                    Some(*credential_id)
                }
                _ => None,
            })
            .collect();

        // Resolve each to (name, type), modeled as targeting the daemon host.
        let mut items: Vec<(String, CredentialType, CredentialTargets)> = Vec::new();
        for cred_id in host_cred_ids {
            if let Some(cred) = self.get_by_id(&cred_id).await? {
                items.push((
                    cred.base.name,
                    cred.base.credential_type,
                    CredentialTargets::for_host(daemon_host_id),
                ));
            }
        }

        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                if single_endpoint_targets_conflict(
                    &items[i].1,
                    &items[i].2,
                    &items[j].1,
                    &items[j].2,
                ) {
                    return Ok(Some((items[i].0.clone(), items[j].0.clone())));
                }
            }
        }
        Ok(None)
    }

    // ========================================================================
    // Discovery credential building
    // ========================================================================

    // === Legacy Daemon Support (pre-v0.15.0) ===

    /// Legacy: Supports daemons < v0.15.0 using SnmpCredentialMapping in DiscoveryType::Network.
    /// Modern equivalent: `build_credential_mappings_for_discovery()` with CredentialQueryPayload.
    /// Remove when minimum daemon version >= 0.15.0.
    pub async fn build_snmp_credentials_for_discovery(
        &self,
        network_id: Uuid,
    ) -> Result<SnmpCredentialMapping, Error> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| anyhow::anyhow!("HostService not initialized"))?;
        let host_filter = StorableFilter::<Host>::new_from_network_ids(&[network_id]);
        let hosts = host_service.get_all(host_filter).await?;

        let interface_filter = StorableFilter::<IPAddress>::new_from_network_ids(&[network_id]);
        let ip_addresses = self.ip_address_service.get_all(interface_filter).await?;

        // Get network's SNMP credentials (from junction table)
        let network_cred_ids = self.get_credential_ids_for_network(&network_id).await?;
        tracing::debug!(
            network_id = %network_id,
            credential_count = network_cred_ids.len(),
            "Credential IDs found for network via junction table"
        );
        // Legacy mapping only carries SNMPv2c — pre-v0.15.0 daemons can't speak
        // v1 or v3. v1/v3 credentials reach modern daemons via
        // build_all_credential_mappings.
        let mut network_snmp_credential: Option<SnmpQueryCredential> = None;
        for cred_id in &network_cred_ids {
            if let Some(cred) = self.get_by_id(cred_id).await?
                && let CredentialQueryPayload::Snmp(snmp) =
                    cred.base.credential_type.to_query_payload()
                && snmp.version == SnmpVersion::V2c
            {
                network_snmp_credential = Some(snmp);
                break;
            }
        }
        tracing::debug!(
            network_id = %network_id,
            has_default = network_snmp_credential.is_some(),
            "Network default SNMP credential resolution"
        );

        // Get host-level SNMP credential overrides
        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let host_cred_map = self.get_credential_assignments_for_hosts(&host_ids).await?;

        let mut overrides: Vec<IpOverride<SnmpQueryCredential>> = Vec::new();

        for host in &hosts {
            if let Some(assignments) = host_cred_map.get(&host.id) {
                for assignment in assignments {
                    if let Some(cred) = self.get_by_id(&assignment.credential_id).await?
                        && let CredentialQueryPayload::Snmp(query_cred) =
                            cred.base.credential_type.to_query_payload()
                        && query_cred.version == SnmpVersion::V2c
                    {
                        // If ip_address_ids is set, only create overrides for those ip_addresses
                        let relevant_interfaces: Vec<_> = ip_addresses
                            .iter()
                            .filter(|i| {
                                i.base.host_id == host.id
                                    && match &assignment.ip_address_ids {
                                        Some(ids) => ids.contains(&i.id),
                                        None => true,
                                    }
                            })
                            .collect();
                        overrides.extend(relevant_interfaces.iter().map(|i| IpOverride {
                            ip: i.base.ip_address,
                            credential: query_cred.clone(),
                            credential_id: cred.id,
                        }));
                        break;
                    }
                }
            }
        }

        tracing::debug!(
            network_id = %network_id,
            ip_overrides = overrides.len(),
            has_default = network_snmp_credential.is_some(),
            "SNMP credential mapping built for discovery"
        );

        Ok(SnmpCredentialMapping {
            default_credential: network_snmp_credential,
            ip_overrides: overrides,
        })
    }

    // === End Legacy Daemon Support ===

    /// Build generic credential mappings for unified discovery dispatch.
    /// Returns one `CredentialMapping<CredentialQueryPayload>` per credential type discriminant.
    ///
    /// Combines: network-level credentials (broadcast defaults), host-level credential
    /// assignments (IP overrides on discovered hosts), and the per-daemon `integration_targets`
    /// from the daemon's `Discovery` (init-command targeting — both credentialed cred↔IP and
    /// credential-less local sockets). The `integration_targets` source replaces the old global
    /// `credential.target_ips` org-wide bootstrap (which was consumed/cleared once, racing across
    /// daemons — #637) and the discovery modal's one-shot `pending_credential_ids`.
    pub async fn build_all_credential_mappings(
        &self,
        network_id: Uuid,
        integration_targets: &[IntegrationTarget],
        daemon_version: Option<&semver::Version>,
    ) -> Result<Vec<CredentialMapping<CredentialQueryPayload>>, Error> {
        let host_service = self
            .host_service
            .get()
            .ok_or_else(|| anyhow::anyhow!("HostService not initialized"))?;

        // Fetch hosts + ip_addresses on network
        let host_filter = StorableFilter::<Host>::new_from_network_ids(&[network_id]);
        let hosts = host_service.get_all(host_filter).await?;

        let interface_filter = StorableFilter::<IPAddress>::new_from_network_ids(&[network_id]);
        let ip_addresses = self.ip_address_service.get_all(interface_filter).await?;

        // Defense-in-depth: only ever hand a daemon credentials owned by the
        // scanned network's own organization. Write-time validation already
        // rejects cross-org credential↔network/host assignments, but a stale or
        // otherwise-inconsistent junction row must never serialize a foreign
        // tenant's secret onto the wire. If the network's org can't be resolved
        // we fall back to the write-time guard rather than break a live scan.
        let network_org_id = self
            .network_service
            .get_by_id(&network_id)
            .await?
            .map(|n| n.base.organization_id);
        let owned_by_network_org = |cred: &Credential| -> bool {
            network_org_id
                .map(|org| cred.base.organization_id == org)
                .unwrap_or(true)
        };

        // Fetch network-level credentials
        let network_cred_ids = self.get_credential_ids_for_network(&network_id).await?;

        // Group network credentials by discriminant — one mapping per type
        let mut mappings_by_type: std::collections::HashMap<
            CredentialTypeDiscriminants,
            CredentialMapping<CredentialQueryPayload>,
        > = std::collections::HashMap::new();

        for cred_id in &network_cred_ids {
            if let Some(cred) = self.get_by_id(cred_id).await?
                && owned_by_network_org(&cred)
            {
                let cred_type = &cred.base.credential_type;
                let discriminant = cred_type.discriminant();
                let payload = cred_type.to_query_payload();
                let mapping =
                    mappings_by_type
                        .entry(discriminant)
                        .or_insert_with(|| CredentialMapping {
                            default_credential: None,
                            ip_overrides: vec![],
                        });
                if mapping.default_credential.is_none() {
                    mapping.default_credential = Some(payload);
                }
            }
        }

        // Fetch host-level credential assignments
        let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();
        let host_cred_map = self.get_credential_assignments_for_hosts(&host_ids).await?;

        for host in &hosts {
            if let Some(assignments) = host_cred_map.get(&host.id) {
                for assignment in assignments {
                    if let Some(cred) = self.get_by_id(&assignment.credential_id).await?
                        && owned_by_network_org(&cred)
                    {
                        let cred_type = &cred.base.credential_type;
                        let discriminant = cred_type.discriminant();
                        let payload = cred_type.to_query_payload();
                        let mapping = mappings_by_type.entry(discriminant).or_insert_with(|| {
                            CredentialMapping {
                                default_credential: None,
                                ip_overrides: vec![],
                            }
                        });

                        // Create IP overrides for relevant ip_addresses
                        let relevant_interfaces: Vec<_> = ip_addresses
                            .iter()
                            .filter(|i| {
                                i.base.host_id == host.id
                                    && match &assignment.ip_address_ids {
                                        Some(ids) => ids.contains(&i.id),
                                        None => true,
                                    }
                            })
                            .collect();

                        mapping
                            .ip_overrides
                            .extend(relevant_interfaces.iter().map(|i| IpOverride {
                                ip: i.base.ip_address,
                                credential: payload.clone(),
                                credential_id: cred.id,
                            }));
                    }
                }
            }
        }

        // Per-daemon integration targets from this daemon's Discovery (init-command targeting).
        // The single home for cred↔IP and local-socket targeting — it replaces the old org-wide
        // target_ips bootstrap and the modal's pending_credential_ids. Resolving the credential is
        // the only I/O; override-building is delegated to the pure `apply_integration_target`.
        for target in integration_targets {
            let Some(cred) = self.get_by_id(&target.credential_id()).await? else {
                tracing::warn!(
                    credential_id = %target.credential_id(),
                    "Integration target references unknown credential; skipping"
                );
                continue;
            };
            apply_integration_target(&mut mappings_by_type, target, &cred.base.credential_type);
        }

        // Version gate: never dispatch a credential mapping the target daemon can't
        // deserialize. Filter on the `CredentialType` discriminant (the HashMap key,
        // 7-way) BEFORE the lossy collapse to the 5-way `CredentialQueryPayload` wire
        // tag — SnmpV1/V3 have a higher floor than SnmpV2c. A missing version is
        // treated conservatively (keep only the 0.16.2 wire floor). This protects the
        // installed base of already-released daemons that predate `serde(other)`.
        mappings_by_type
            .retain(|discriminant, _| discriminant.compatible_with_daemon(daemon_version));

        Ok(mappings_by_type.into_values().collect())
    }
}

/// Apply one [`IntegrationTarget`] to the per-credential-type mapping accumulator.
///
/// Pure (no I/O): the caller resolves the credential by `target.credential_id()`. The target's
/// scope (its [`Target`] discriminant) must be permitted by the credential type's `targets()`,
/// otherwise it is skipped. Every target carries a real credential id — a local socket is just a
/// credential whose type targets only the daemon host, so there is no nil sentinel.
///
/// Idempotent — applying the same target twice (e.g. across scans) does not duplicate overrides,
/// core to the #637 fix: targeting lives per-daemon on the `Discovery` and is re-applied each scan.
pub(crate) fn apply_integration_target(
    mappings_by_type: &mut std::collections::HashMap<
        CredentialTypeDiscriminants,
        CredentialMapping<CredentialQueryPayload>,
    >,
    target: &IntegrationTarget,
    credential_type: &CredentialType,
) {
    let scope = Target::from(target);
    if !credential_type.targets().contains(&scope) {
        tracing::warn!(
            ?scope,
            credential_id = %target.credential_id(),
            "Integration target scope not permitted by its credential type; skipping"
        );
        return;
    }

    // Daemon-host targeting (DaemonHost scope, or only loopback Hosts IPs) is sourced
    // from the `host_credentials` junction — managed via the host/credential modals.
    // Skip it here (without even creating an empty mapping) so integration_targets
    // never double-sources a daemon-host credential or re-injects one the user removed
    // via those modals.
    let non_loopback_ips: Vec<IpAddr> = match target {
        IntegrationTarget::DaemonHost { .. } => return,
        IntegrationTarget::Hosts { ips, .. } => {
            ips.iter().copied().filter(|ip| !ip.is_loopback()).collect()
        }
        IntegrationTarget::Network { .. } => Vec::new(),
    };
    if matches!(target, IntegrationTarget::Hosts { .. }) && non_loopback_ips.is_empty() {
        return;
    }

    let discriminant = credential_type.discriminant();
    let payload = credential_type.to_query_payload();
    let credential_id = target.credential_id();
    let mapping = mappings_by_type
        .entry(discriminant)
        .or_insert_with(|| CredentialMapping {
            default_credential: None,
            ip_overrides: vec![],
        });

    match target {
        IntegrationTarget::Network { .. } => {
            if mapping.default_credential.is_none() {
                mapping.default_credential = Some(payload);
            }
        }
        IntegrationTarget::Hosts { .. } => {
            for ip in non_loopback_ips {
                push_unique_override(mapping, ip, payload.clone(), credential_id);
            }
        }
        IntegrationTarget::DaemonHost { .. } => unreachable!("returned above"),
    }
}

/// Push an IP-override unless an identical `(ip, credential_id)` override is already present
/// (de-dups against host-assignment overrides and repeated scans).
fn push_unique_override(
    mapping: &mut CredentialMapping<CredentialQueryPayload>,
    ip: IpAddr,
    payload: CredentialQueryPayload,
    credential_id: Uuid,
) {
    if !mapping
        .ip_overrides
        .iter()
        .any(|o| o.ip == ip && o.credential_id == credential_id)
    {
        mapping.ip_overrides.push(IpOverride {
            ip,
            credential: payload,
            credential_id,
        });
    }
}

#[cfg(test)]
mod single_endpoint_tests {
    use super::*;
    use crate::server::credentials::r#impl::types::CredentialHostAssignment;

    fn host(id: Uuid, ips: Option<Vec<Uuid>>) -> CredentialHostAssignment {
        CredentialHostAssignment {
            host_id: id,
            ip_address_ids: ips,
        }
    }

    #[test]
    fn disjoint_targets_do_not_overlap() {
        let a = CredentialTargets::build(&[Uuid::nil()], &[]);
        let b = CredentialTargets::build(&[Uuid::from_u128(9)], &[]);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn shared_network_overlaps() {
        let net = Uuid::from_u128(1);
        let a = CredentialTargets::build(&[net], &[]);
        let b = CredentialTargets::build(&[net], &[]);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn same_host_whole_host_overlaps() {
        let h = Uuid::from_u128(5);
        let a = CredentialTargets::build(&[], &[host(h, None)]);
        let b = CredentialTargets::build(&[], &[host(h, Some(vec![Uuid::from_u128(2)]))]);
        // One side covers the whole host → overlaps regardless of the other's scope.
        assert!(a.overlaps(&b));
    }

    #[test]
    fn same_host_disjoint_ip_scopes_do_not_overlap() {
        let h = Uuid::from_u128(5);
        let a = CredentialTargets::build(&[], &[host(h, Some(vec![Uuid::from_u128(1)]))]);
        let b = CredentialTargets::build(&[], &[host(h, Some(vec![Uuid::from_u128(2)]))]);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn same_host_intersecting_ip_scopes_overlap() {
        let h = Uuid::from_u128(5);
        let shared = Uuid::from_u128(7);
        let a = CredentialTargets::build(&[], &[host(h, Some(vec![shared, Uuid::from_u128(1)]))]);
        let b = CredentialTargets::build(&[], &[host(h, Some(vec![shared]))]);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn different_hosts_do_not_overlap() {
        let a = CredentialTargets::build(&[], &[host(Uuid::from_u128(1), None)]);
        let b = CredentialTargets::build(&[], &[host(Uuid::from_u128(2), None)]);
        assert!(!a.overlaps(&b));
    }

    // --- single_endpoint_targets_conflict (the shared rule both write paths use) ---

    use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;

    fn cred_type(d: CredentialTypeDiscriminants) -> CredentialType {
        d.to_credential_type()
    }

    #[test]
    fn socket_and_proxy_same_integration_same_host_conflict() {
        // A Docker socket and a Docker proxy both targeting the daemon host conflict
        // — bidirectionally (order doesn't matter).
        let h = Uuid::from_u128(42);
        let socket = cred_type(CredentialTypeDiscriminants::DockerSocket);
        let proxy = cred_type(CredentialTypeDiscriminants::DockerProxy);
        let t = CredentialTargets::for_host(h);
        assert!(single_endpoint_targets_conflict(&socket, &t, &proxy, &t));
        assert!(single_endpoint_targets_conflict(&proxy, &t, &socket, &t));
    }

    #[test]
    fn podman_socket_and_proxy_same_host_conflict() {
        let h = Uuid::from_u128(42);
        let socket = cred_type(CredentialTypeDiscriminants::PodmanSocket);
        let proxy = cred_type(CredentialTypeDiscriminants::PodmanProxy);
        let t = CredentialTargets::for_host(h);
        assert!(single_endpoint_targets_conflict(&socket, &t, &proxy, &t));
    }

    #[test]
    fn different_integrations_same_host_do_not_conflict() {
        // Docker and Podman are distinct integrations — both may target one host.
        let h = Uuid::from_u128(42);
        let docker = cred_type(CredentialTypeDiscriminants::DockerSocket);
        let podman = cred_type(CredentialTypeDiscriminants::PodmanSocket);
        let t = CredentialTargets::for_host(h);
        assert!(!single_endpoint_targets_conflict(&docker, &t, &podman, &t));
    }

    #[test]
    fn same_integration_different_hosts_do_not_conflict() {
        let socket = cred_type(CredentialTypeDiscriminants::DockerSocket);
        let proxy = cred_type(CredentialTypeDiscriminants::DockerProxy);
        let a = CredentialTargets::for_host(Uuid::from_u128(1));
        let b = CredentialTargets::for_host(Uuid::from_u128(2));
        assert!(!single_endpoint_targets_conflict(&socket, &a, &proxy, &b));
    }

    #[test]
    fn non_single_endpoint_integration_never_conflicts() {
        // SNMP is try-many, not single-endpoint per host.
        let h = Uuid::from_u128(42);
        let snmp = cred_type(CredentialTypeDiscriminants::SnmpV2c);
        let t = CredentialTargets::for_host(h);
        assert!(!single_endpoint_targets_conflict(&snmp, &t, &snmp, &t));
    }
}

/// Characterization tests for #637 (per-daemon credential↔IP targeting via `IntegrationTarget`).
/// These exercise the pure `apply_integration_target` transformation — the heart of the fix —
/// without a database.
#[cfg(test)]
mod integration_target_tests {
    use super::*;
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr};

    type Mappings = HashMap<CredentialTypeDiscriminants, CredentialMapping<CredentialQueryPayload>>;

    fn localhost() -> IpAddr {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    }

    /// Daemon-host targeting is sourced from the `host_credentials` junction (managed via the
    /// host/credential modals), so `apply_integration_target` produces NO override for a
    /// `DaemonHost` target — avoiding a double-source with the junction and letting modal
    /// removal fully clear a daemon-host credential. The shared target is left untouched.
    #[test]
    fn daemon_host_target_produces_no_override_junction_sourced() {
        let cred_id = Uuid::new_v4();
        let target = IntegrationTarget::DaemonHost {
            credential_id: cred_id,
        };
        for disc in [
            CredentialTypeDiscriminants::DockerProxy,
            CredentialTypeDiscriminants::DockerSocket,
        ] {
            let mut map: Mappings = HashMap::new();
            apply_integration_target(&mut map, &target, &disc.to_credential_type());
            assert!(
                map.is_empty(),
                "DaemonHost targets are junction-sourced, not integration_targets overrides"
            );
        }
        // No consumption/clear: the shared target is untouched and still usable.
        assert_eq!(
            target,
            IntegrationTarget::DaemonHost {
                credential_id: cred_id
            }
        );
    }

    /// A loopback `Hosts` IP is also daemon-host targeting → junction-sourced, so it's skipped;
    /// non-loopback IPs in the same target still produce overrides.
    #[test]
    fn loopback_hosts_ip_skipped_non_loopback_kept() {
        let cred_id = Uuid::new_v4();
        let cred_type = CredentialTypeDiscriminants::DockerProxy.to_credential_type();
        let target = IntegrationTarget::Hosts {
            credential_id: cred_id,
            ips: vec![localhost(), "10.0.0.7".parse::<IpAddr>().unwrap()],
        };
        let mut map: Mappings = HashMap::new();
        apply_integration_target(&mut map, &target, &cred_type);
        let mapping = map.get(&CredentialTypeDiscriminants::DockerProxy).unwrap();
        assert_eq!(mapping.ip_overrides.len(), 1, "loopback IP must be skipped");
        assert_eq!(
            mapping.ip_overrides[0].ip,
            "10.0.0.7".parse::<IpAddr>().unwrap()
        );
    }

    /// Re-applying the same target (i.e. a subsequent scan re-reading the persistent
    /// `integration_targets`) must not duplicate overrides.
    #[test]
    fn reapplying_same_target_is_idempotent() {
        let cred_id = Uuid::new_v4();
        let cred_type = CredentialTypeDiscriminants::DockerProxy.to_credential_type();
        let target = IntegrationTarget::Hosts {
            credential_id: cred_id,
            ips: vec!["10.0.0.5".parse::<IpAddr>().unwrap()],
        };
        let mut map: Mappings = HashMap::new();
        apply_integration_target(&mut map, &target, &cred_type);
        apply_integration_target(&mut map, &target, &cred_type);
        let mapping = map.get(&CredentialTypeDiscriminants::DockerProxy).unwrap();
        assert_eq!(
            mapping.ip_overrides.len(),
            1,
            "subsequent scans must not duplicate the override"
        );
    }

    /// A target whose scope isn't permitted by its credential type's `targets()` is skipped —
    /// e.g. a `DockerSocket` credential (daemon-host only) given a `Network` scope.
    #[test]
    fn scope_not_permitted_by_credential_type_is_skipped() {
        let cred_type = CredentialTypeDiscriminants::DockerSocket.to_credential_type();
        let target = IntegrationTarget::Network {
            credential_id: Uuid::new_v4(),
        };
        let mut map: Mappings = HashMap::new();
        apply_integration_target(&mut map, &target, &cred_type);
        assert!(map.is_empty(), "invalid scope must not produce a mapping");
    }

    /// A `Network`-scoped target becomes a network-level default, not an IP override.
    #[test]
    fn network_scope_sets_default_not_override() {
        let cred_type = CredentialTypeDiscriminants::SnmpV2c.to_credential_type();
        let target = IntegrationTarget::Network {
            credential_id: Uuid::new_v4(),
        };
        let mut map: Mappings = HashMap::new();
        apply_integration_target(&mut map, &target, &cred_type);
        let mapping = map.get(&CredentialTypeDiscriminants::SnmpV2c).unwrap();
        assert!(mapping.ip_overrides.is_empty());
        assert!(mapping.default_credential.is_some());
    }

    /// A `Hosts` target produces one override per IP.
    #[test]
    fn hosts_scope_overrides_each_ip() {
        let cred_id = Uuid::new_v4();
        let cred_type = CredentialTypeDiscriminants::SnmpV2c.to_credential_type();
        let ips = vec![
            "10.0.0.5".parse::<IpAddr>().unwrap(),
            "10.0.0.6".parse::<IpAddr>().unwrap(),
        ];
        let target = IntegrationTarget::Hosts {
            credential_id: cred_id,
            ips: ips.clone(),
        };
        let mut map: Mappings = HashMap::new();
        apply_integration_target(&mut map, &target, &cred_type);
        let mapping = map.get(&CredentialTypeDiscriminants::SnmpV2c).unwrap();
        assert_eq!(mapping.ip_overrides.len(), 2);
        assert_eq!(
            mapping
                .ip_overrides
                .iter()
                .map(|o| o.ip)
                .collect::<Vec<_>>(),
            ips
        );
    }

    /// The wire/storage format of `IntegrationTarget` is an internally-tagged enum keyed on
    /// `scope`; lock it so the JSONB column and registration request stay stable, and so `Target`
    /// (its discriminant) tracks the variant names.
    #[test]
    fn integration_target_serde_is_tagged() {
        let cred_id = Uuid::nil();
        let daemon_host = IntegrationTarget::DaemonHost {
            credential_id: cred_id,
        };
        let json = serde_json::to_value(&daemon_host).unwrap();
        assert_eq!(json["scope"], "DaemonHost");
        assert_eq!(json["credential_id"], cred_id.to_string());

        let hosts = IntegrationTarget::Hosts {
            credential_id: cred_id,
            ips: vec![localhost()],
        };
        let json = serde_json::to_value(&hosts).unwrap();
        assert_eq!(json["scope"], "Hosts");
        assert_eq!(json["ips"][0], "127.0.0.1");

        // Discriminant tracks variant names.
        assert_eq!(Target::from(&daemon_host), Target::DaemonHost);
        assert_eq!(Target::from(&hosts), Target::Hosts);

        // Round-trip.
        for t in [daemon_host, hosts] {
            assert_eq!(
                t,
                serde_json::from_value(serde_json::to_value(&t).unwrap()).unwrap()
            );
        }
    }
}
