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
            CredentialTypeDiscriminants, SnmpVersion,
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

/// The set of things a credential targets, normalized across the three storage
/// representations (network junction, host junction, bootstrap target IPs) so two
/// credentials can be tested for overlap. Used to enforce the single-endpoint
/// invariant — see [`CredentialService::find_single_endpoint_conflict`].
#[derive(Debug, Default)]
struct CredentialTargets {
    networks: HashSet<Uuid>,
    /// host_id → the IPs it is scoped to (`None` = the whole host).
    hosts: Vec<(Uuid, Option<HashSet<Uuid>>)>,
    ips: HashSet<IpAddr>,
}

impl CredentialTargets {
    fn build(
        networks: &[Uuid],
        host_assignments: &[CredentialHostAssignment],
        target_ips: Option<&[IpAddr]>,
    ) -> Self {
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
            ips: target_ips
                .map(|ips| ips.iter().copied().collect())
                .unwrap_or_default(),
        }
    }

    /// Two target sets overlap if they share a network, a target IP, or a host —
    /// where a shared host overlaps when either side covers the whole host or
    /// their IP scopes intersect.
    fn overlaps(&self, other: &Self) -> bool {
        if !self.networks.is_disjoint(&other.networks) {
            return true;
        }
        if !self.ips.is_disjoint(&other.ips) {
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
    /// assignments, target_ips) — call this before persisting. Try-many
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

        // Hydrate the others' junction-backed assignments (target_ips is already
        // loaded as a column on the credential rows).
        let other_ids: Vec<Uuid> = others.iter().map(|c| c.id).collect();
        let net_map = self.get_network_ids_for_credentials(&other_ids).await?;
        let host_map = self
            .get_host_assignments_for_credentials(&other_ids)
            .await?;

        let cand = CredentialTargets::build(
            &candidate.base.assigned_network_ids,
            &candidate.base.host_assignments,
            candidate.base.target_ips.as_deref(),
        );
        for other in &others {
            let empty_nets = Vec::new();
            let empty_hosts = Vec::new();
            let other_targets = CredentialTargets::build(
                net_map.get(&other.id).unwrap_or(&empty_nets),
                host_map.get(&other.id).unwrap_or(&empty_hosts),
                other.base.target_ips.as_deref(),
            );
            if cand.overlaps(&other_targets) {
                return Ok(Some(other.base.name.clone()));
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

        // Fetch network-level credentials
        let network_cred_ids = self.get_credential_ids_for_network(&network_id).await?;

        // Group network credentials by discriminant — one mapping per type
        let mut mappings_by_type: std::collections::HashMap<
            CredentialTypeDiscriminants,
            CredentialMapping<CredentialQueryPayload>,
        > = std::collections::HashMap::new();

        for cred_id in &network_cred_ids {
            if let Some(cred) = self.get_by_id(cred_id).await? {
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
                    if let Some(cred) = self.get_by_id(&assignment.credential_id).await? {
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
        // This is the single home for cred↔IP and credential-less local-socket targeting —
        // it replaces the old org-wide target_ips bootstrap and the modal's pending_credential_ids.
        // Resolving a credential is the only I/O here; the actual override-building is delegated to
        // the pure `apply_integration_target` so it can be unit-tested without a database.
        for target in integration_targets {
            let resolved = match target {
                IntegrationTarget::Credentialed { credential_id, .. } => {
                    match self.get_by_id(credential_id).await? {
                        Some(cred) => Some(cred.base.credential_type),
                        None => {
                            tracing::warn!(
                                credential_id = %credential_id,
                                "Integration target references unknown credential; skipping"
                            );
                            continue;
                        }
                    }
                }
                IntegrationTarget::Local { .. } => None,
            };
            apply_integration_target(&mut mappings_by_type, target, resolved.as_ref());
        }

        Ok(mappings_by_type.into_values().collect())
    }
}

/// Apply one [`IntegrationTarget`] to the per-credential-type mapping accumulator.
///
/// Pure (no I/O): the caller resolves the credential. `resolved_credential` is `Some` for a
/// `Credentialed` target whose credential was found, `None` for a `Local` target (credential-less)
/// or a `Credentialed` target whose credential is missing (no-op).
///
/// Idempotent — applying the same target twice (e.g. across scans) does not duplicate overrides,
/// which is core to the #637 fix: targeting lives per-daemon on the `Discovery` and is re-applied
/// every scan rather than consumed once.
pub(crate) fn apply_integration_target(
    mappings_by_type: &mut std::collections::HashMap<
        CredentialTypeDiscriminants,
        CredentialMapping<CredentialQueryPayload>,
    >,
    target: &IntegrationTarget,
    resolved_credential: Option<&CredentialType>,
) {
    match target {
        IntegrationTarget::Credentialed { credential_id, ips } => {
            let Some(cred_type) = resolved_credential else {
                return;
            };
            let discriminant = cred_type.discriminant();
            let payload = cred_type.to_query_payload();
            let mapping =
                mappings_by_type
                    .entry(discriminant)
                    .or_insert_with(|| CredentialMapping {
                        default_credential: None,
                        ip_overrides: vec![],
                    });

            if ips.is_empty() {
                // No explicit IP → network-level default (back-compat for bare-uuid tokens).
                if mapping.default_credential.is_none() {
                    mapping.default_credential = Some(payload);
                }
            } else {
                for ip in ips {
                    // De-dup against host-assignment overrides for the same (ip, cred).
                    if mapping
                        .ip_overrides
                        .iter()
                        .any(|o| o.ip == *ip && o.credential_id == *credential_id)
                    {
                        continue;
                    }
                    mapping.ip_overrides.push(IpOverride {
                        ip: *ip,
                        credential: payload.clone(),
                        credential_id: *credential_id,
                    });
                }
            }
        }
        IntegrationTarget::Local { integration } => {
            // Credential-less local integration: runs on the daemon host (127.0.0.1),
            // no stored credential (credential_id nil).
            let payload = integration.to_credential_type().to_query_payload();
            let mapping =
                mappings_by_type
                    .entry(*integration)
                    .or_insert_with(|| CredentialMapping {
                        default_credential: None,
                        ip_overrides: vec![],
                    });
            let localhost = IpAddr::V4(std::net::Ipv4Addr::LOCALHOST);
            if !mapping.ip_overrides.iter().any(|o| o.ip == localhost) {
                mapping.ip_overrides.push(IpOverride {
                    ip: localhost,
                    credential: payload,
                    credential_id: Uuid::nil(),
                });
            }
        }
    }
}

#[cfg(test)]
mod single_endpoint_tests {
    use super::*;
    use crate::server::credentials::r#impl::types::CredentialHostAssignment;
    use std::net::{IpAddr, Ipv4Addr};

    fn ip(n: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, n))
    }

    fn host(id: Uuid, ips: Option<Vec<Uuid>>) -> CredentialHostAssignment {
        CredentialHostAssignment {
            host_id: id,
            ip_address_ids: ips,
        }
    }

    #[test]
    fn disjoint_targets_do_not_overlap() {
        let a = CredentialTargets::build(&[Uuid::nil()], &[], Some(&[ip(1)]));
        let b = CredentialTargets::build(&[Uuid::from_u128(9)], &[], Some(&[ip(2)]));
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn shared_network_overlaps() {
        let net = Uuid::from_u128(1);
        let a = CredentialTargets::build(&[net], &[], None);
        let b = CredentialTargets::build(&[net], &[], None);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn shared_target_ip_overlaps_including_daemon_host() {
        // Two Docker credentials both pinned to the daemon host (127.0.0.1).
        let a = CredentialTargets::build(&[], &[], Some(&[ip(1)]));
        let b = CredentialTargets::build(&[], &[], Some(&[ip(1)]));
        assert!(a.overlaps(&b));
    }

    #[test]
    fn same_host_whole_host_overlaps() {
        let h = Uuid::from_u128(5);
        let a = CredentialTargets::build(&[], &[host(h, None)], None);
        let b = CredentialTargets::build(&[], &[host(h, Some(vec![Uuid::from_u128(2)]))], None);
        // One side covers the whole host → overlaps regardless of the other's scope.
        assert!(a.overlaps(&b));
    }

    #[test]
    fn same_host_disjoint_ip_scopes_do_not_overlap() {
        let h = Uuid::from_u128(5);
        let a = CredentialTargets::build(&[], &[host(h, Some(vec![Uuid::from_u128(1)]))], None);
        let b = CredentialTargets::build(&[], &[host(h, Some(vec![Uuid::from_u128(2)]))], None);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn same_host_intersecting_ip_scopes_overlap() {
        let h = Uuid::from_u128(5);
        let shared = Uuid::from_u128(7);
        let a = CredentialTargets::build(
            &[],
            &[host(h, Some(vec![shared, Uuid::from_u128(1)]))],
            None,
        );
        let b = CredentialTargets::build(&[], &[host(h, Some(vec![shared]))], None);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn different_hosts_do_not_overlap() {
        let a = CredentialTargets::build(&[], &[host(Uuid::from_u128(1), None)], None);
        let b = CredentialTargets::build(&[], &[host(Uuid::from_u128(2), None)], None);
        assert!(!a.overlaps(&b));
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

    /// #637 core: two daemons reusing ONE credential each target their own daemon host
    /// (127.0.0.1) independently. With targeting living per-daemon on the `Discovery` (not on a
    /// shared, consumed credential field), building daemon A's mappings neither mutates the shared
    /// target nor affects daemon B — so both daemons get the credential, on first scan and every
    /// scan after.
    #[test]
    fn two_daemons_share_one_credential_independently() {
        let cred_id = Uuid::new_v4();
        let cred_type = CredentialTypeDiscriminants::DockerProxy.to_credential_type();
        let target = IntegrationTarget::Credentialed {
            credential_id: cred_id,
            ips: vec![localhost()],
        };

        // Two separate Discovery rows → two separate accumulators.
        let mut daemon_a: Mappings = HashMap::new();
        let mut daemon_b: Mappings = HashMap::new();
        apply_integration_target(&mut daemon_a, &target, Some(&cred_type));
        apply_integration_target(&mut daemon_b, &target, Some(&cred_type));

        for map in [&daemon_a, &daemon_b] {
            let mapping = map
                .get(&CredentialTypeDiscriminants::DockerProxy)
                .expect("each daemon gets the docker mapping");
            assert_eq!(mapping.ip_overrides.len(), 1);
            assert_eq!(mapping.ip_overrides[0].ip, localhost());
            assert_eq!(mapping.ip_overrides[0].credential_id, cred_id);
        }

        // No consumption/clear: the shared target is untouched and still usable.
        assert_eq!(
            target,
            IntegrationTarget::Credentialed {
                credential_id: cred_id,
                ips: vec![localhost()],
            }
        );
    }

    /// Re-applying the same target (i.e. a subsequent scan re-reading the persistent
    /// `integration_targets`) must not duplicate overrides.
    #[test]
    fn reapplying_same_target_is_idempotent() {
        let cred_id = Uuid::new_v4();
        let cred_type = CredentialTypeDiscriminants::DockerProxy.to_credential_type();
        let target = IntegrationTarget::Credentialed {
            credential_id: cred_id,
            ips: vec![localhost()],
        };
        let mut map: Mappings = HashMap::new();
        apply_integration_target(&mut map, &target, Some(&cred_type));
        apply_integration_target(&mut map, &target, Some(&cred_type));
        let mapping = map.get(&CredentialTypeDiscriminants::DockerProxy).unwrap();
        assert_eq!(
            mapping.ip_overrides.len(),
            1,
            "subsequent scans must not duplicate the override"
        );
    }

    /// A credential-less `Local` target maps to a 127.0.0.1 override with a nil credential id.
    #[test]
    fn local_socket_target_maps_to_localhost_nil_credential() {
        let target = IntegrationTarget::Local {
            integration: CredentialTypeDiscriminants::DockerSocket,
        };
        let mut map: Mappings = HashMap::new();
        apply_integration_target(&mut map, &target, None);
        let mapping = map.get(&CredentialTypeDiscriminants::DockerSocket).unwrap();
        assert_eq!(mapping.ip_overrides.len(), 1);
        assert_eq!(mapping.ip_overrides[0].ip, localhost());
        assert_eq!(mapping.ip_overrides[0].credential_id, Uuid::nil());
        assert!(matches!(
            mapping.ip_overrides[0].credential,
            CredentialQueryPayload::DockerSocket(_)
        ));
    }

    /// A credentialed target whose credential can't be resolved is skipped (no panic, no entry).
    #[test]
    fn missing_credential_is_skipped() {
        let target = IntegrationTarget::Credentialed {
            credential_id: Uuid::new_v4(),
            ips: vec![localhost()],
        };
        let mut map: Mappings = HashMap::new();
        apply_integration_target(&mut map, &target, None);
        assert!(map.is_empty());
    }

    /// A credentialed target with no explicit IP becomes a network-level default, not an override
    /// (back-compat for bare-uuid tokens).
    #[test]
    fn empty_ips_sets_network_default_not_override() {
        let cred_type = CredentialTypeDiscriminants::SnmpV2c.to_credential_type();
        let target = IntegrationTarget::Credentialed {
            credential_id: Uuid::new_v4(),
            ips: vec![],
        };
        let mut map: Mappings = HashMap::new();
        apply_integration_target(&mut map, &target, Some(&cred_type));
        let mapping = map.get(&CredentialTypeDiscriminants::SnmpV2c).unwrap();
        assert!(mapping.ip_overrides.is_empty());
        assert!(mapping.default_credential.is_some());
    }

    /// The wire/storage format of `IntegrationTarget` is an internally-tagged enum; lock it so the
    /// JSONB column and registration request stay stable across daemon/server versions.
    #[test]
    fn integration_target_serde_is_tagged() {
        let cred_id = Uuid::nil();
        let credentialed = IntegrationTarget::Credentialed {
            credential_id: cred_id,
            ips: vec![localhost()],
        };
        let json = serde_json::to_value(&credentialed).unwrap();
        assert_eq!(json["type"], "Credentialed");
        assert_eq!(json["credential_id"], cred_id.to_string());

        let local = IntegrationTarget::Local {
            integration: CredentialTypeDiscriminants::PodmanSocket,
        };
        let json = serde_json::to_value(&local).unwrap();
        assert_eq!(json["type"], "Local");
        assert_eq!(json["integration"], "PodmanSocket");

        // Round-trip.
        assert_eq!(
            credentialed,
            serde_json::from_value(serde_json::to_value(&credentialed).unwrap()).unwrap()
        );
        assert_eq!(
            local,
            serde_json::from_value(serde_json::to_value(&local).unwrap()).unwrap()
        );
    }
}
