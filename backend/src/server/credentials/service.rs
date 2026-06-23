use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    credentials::r#impl::{
        base::Credential,
        junction::{HostCredentialStorage, NetworkCredentialStorage},
        mapping::{
            CredentialMapping, CredentialQueryPayload, IpOverride, SnmpCredentialMapping,
            SnmpQueryCredential,
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
use std::sync::{Arc, OnceLock};
use strum::IntoDiscriminant;
use uuid::Uuid;

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
    /// Build all credential mappings for a discovery session.
    /// Combines: network-level credentials, host-level overrides, org-level target_ips,
    /// and pending credentials from the discovery edit modal.
    pub async fn build_all_credential_mappings(
        &self,
        network_id: Uuid,
        pending_credential_ids: &[Uuid],
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

                        // Add target IP overrides (bootstrap IPs for new daemon hosts without ip_addresses)
                        if let Some(target_ips) = &cred.base.target_ips {
                            for ip in target_ips {
                                mapping.ip_overrides.push(IpOverride {
                                    ip: *ip,
                                    credential: payload.clone(),
                                    credential_id: cred.id,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Fetch org credentials with target_ips that aren't already included via host/network
        // assignment. These are bootstrap credentials for hosts that don't exist yet.
        let org_id = self
            .network_service
            .get_by_id(&network_id)
            .await?
            .map(|n| n.base.organization_id);

        if let Some(org_id) = org_id {
            let target_ip_filter =
                StorableFilter::<Credential>::new_from_org_id(&org_id).with_target_ips();
            let target_ip_creds = self.get_all(target_ip_filter).await?;

            // Track which credential IDs are already included
            let existing_cred_ids: std::collections::HashSet<Uuid> = mappings_by_type
                .values()
                .flat_map(|m| m.ip_overrides.iter().map(|o| o.credential_id))
                .collect();

            let mut creds_to_clear = Vec::new();

            for cred in &target_ip_creds {
                // Always clear target_ips — even if this credential is already assigned
                // to a host. The host-level section above adds the credential to mappings
                // but doesn't clear target_ips, so we must do it here.
                creds_to_clear.push(cred.id);

                if existing_cred_ids.contains(&cred.id) {
                    continue; // Already in mappings via host assignment
                }

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

                if let Some(target_ips) = &cred.base.target_ips {
                    for ip in target_ips {
                        mapping.ip_overrides.push(IpOverride {
                            ip: *ip,
                            credential: payload.clone(),
                            credential_id: cred.id,
                        });
                    }
                }
            }

            // Clear target_ips immediately to prevent other daemons from picking them up
            for cred_id in &creds_to_clear {
                if let Err(e) = self
                    .clear_target_ips(cred_id, AuthenticatedEntity::System)
                    .await
                {
                    tracing::warn!(
                        credential_id = %cred_id,
                        error = ?e,
                        "Failed to clear target_ips after loading into credential mappings"
                    );
                }
            }
        }

        // Pending credentials from the discovery edit modal.
        // Skip any already included by network-level, host-level, or target_ips sections above.
        let already_included: std::collections::HashSet<Uuid> = mappings_by_type
            .values()
            .flat_map(|m| m.ip_overrides.iter().map(|o| o.credential_id))
            .collect();
        for cred_id in pending_credential_ids {
            if already_included.contains(cred_id) {
                continue;
            }
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

                if let Some(target_ips) = &cred.base.target_ips {
                    for ip in target_ips {
                        mapping.ip_overrides.push(IpOverride {
                            ip: *ip,
                            credential: payload.clone(),
                            credential_id: cred.id,
                        });
                    }
                } else if mapping.default_credential.is_none() {
                    mapping.default_credential = Some(payload);
                }
            }
        }

        Ok(mappings_by_type.into_values().collect())
    }

    /// Clear target_ips on a credential by loading and updating through CrudService.
    pub async fn clear_target_ips(
        &self,
        credential_id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        if let Some(mut cred) = self.get_by_id(credential_id).await? {
            cred.base.target_ips = None;
            self.update(&mut cred, authentication).await?;
        }
        Ok(())
    }
}
