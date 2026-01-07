use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::services::entity_tags::EntityTagService;
use crate::server::shared::storage::traits::StorableEntity;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::{
        r#impl::base::{Binding, BindingType},
        service::BindingService,
    },
    groups::{r#impl::base::Group, service::GroupService},
    hosts::{r#impl::base::Host, service::HostService},
    interfaces::r#impl::base::Interface,
    ports::r#impl::base::Port,
    services::r#impl::{base::Service, patterns::MatchDetails},
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{
            bus::EventBus,
            types::{EntityEvent, EntityOperation},
        },
        position::next_position,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{filter::EntityFilter, generic::GenericPostgresStorage, traits::Storage},
        types::{api::ValidationError, entities::EntitySource},
    },
};
use anyhow::anyhow;
use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::Utc;
use futures::lock::Mutex;
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};
use uuid::Uuid;

pub struct ServiceService {
    storage: Arc<GenericPostgresStorage<Service>>,
    binding_service: Arc<BindingService>,
    host_service: OnceLock<Arc<HostService>>,
    group_service: Arc<GroupService>,
    group_update_lock: Arc<Mutex<()>>,
    service_locks: Arc<Mutex<HashMap<Uuid, Arc<Mutex<()>>>>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<Service> for ServiceService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Service) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Service) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Service> for ServiceService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Service>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Service>, anyhow::Error> {
        let service = self.storage().get_by_id(id).await?;
        match service {
            Some(mut s) => {
                s.base.bindings = self.binding_service.get_for_parent(&s.id).await?;
                self.hydrate_tags(&mut s).await?;
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }

    async fn get_all(&self, filter: EntityFilter) -> Result<Vec<Service>, anyhow::Error> {
        let mut services = self.storage().get_all(filter).await?;
        if services.is_empty() {
            return Ok(services);
        }

        let service_ids: Vec<Uuid> = services.iter().map(|s| s.id).collect();
        let bindings_map = self.binding_service.get_for_parents(&service_ids).await?;

        for service in &mut services {
            if let Some(bindings) = bindings_map.get(&service.id) {
                service.base.bindings = bindings.clone();
            }
        }

        self.bulk_hydrate_tags(&mut services).await?;

        Ok(services)
    }

    async fn get_one(&self, filter: EntityFilter) -> Result<Option<Service>, anyhow::Error> {
        let service = self.storage().get_one(filter).await?;
        match service {
            Some(mut s) => {
                s.base.bindings = self.binding_service.get_for_parent(&s.id).await?;
                self.hydrate_tags(&mut s).await?;
                Ok(Some(s))
            }
            None => Ok(None),
        }
    }

    async fn create(
        &self,
        service: Service,
        authentication: AuthenticatedEntity,
    ) -> Result<Service> {
        let mut service = if service.id == Uuid::nil() {
            Service::new(service.base)
        } else {
            service
        };

        // Deduplicate bindings before validation
        service.base.bindings = Self::deduplicate_bindings(service.base.bindings);

        let lock = self.get_service_lock(&service.id).await;
        let _guard = lock.lock().await;

        let filter = EntityFilter::unfiltered().host_id(&service.base.host_id);
        let existing_services = self.get_all(filter).await?;

        // Auto-assign position for new services (next available position on host)
        let next_pos = next_position(&existing_services);

        let service_from_storage = match existing_services
            .into_iter()
            .find(|existing: &Service| *existing == service)
        {
            // If both are from discovery, or if they have the same ID but for some reason the create route is being used, upsert data
            Some(existing_service)
                if (service.base.source.is_from_discovery()
                    && existing_service.base.source.is_from_discovery())
                    || service.id == existing_service.id =>
            {
                tracing::warn!(
                    service = %service,
                    existing_service = %existing_service,
                    "Duplicate service found, upserting discovery data...",
                );
                self.upsert_service(existing_service, service, authentication)
                    .await?
            }
            _ => {
                // Auto-assign position (users cannot set position via /api/services)
                service.base.position = next_pos;

                // Validate bindings don't conflict with each other before creating
                Self::validate_bindings_no_conflicts(&service.base.bindings)?;

                // Validate bindings reference ports/interfaces on the service's host
                self.validate_bindings_belong_to_host(
                    &service.base.host_id,
                    &service.base.bindings,
                )
                .await?;

                // For non-discovery sources, validate bindings aren't already claimed by other services
                // Discovery sources handle conflicts via partition_conflicting_bindings in create_with_children
                if !service.base.source.is_from_discovery() {
                    self.validate_bindings_available(
                        &service.base.host_id,
                        &service.id,
                        &service.base.bindings,
                    )
                    .await?;
                }

                let mut created = self.storage.create(&service).await?;

                // Save bindings to separate table with correct service_id and network_id
                let bindings_with_ids: Vec<Binding> = service
                    .base
                    .bindings
                    .iter()
                    .cloned()
                    .map(|b| b.with_service(created.id, created.base.network_id))
                    .collect();
                let saved_bindings = self
                    .binding_service
                    .save_for_parent(&created.id, &bindings_with_ids, authentication.clone())
                    .await?;

                // Update service with the saved bindings (which have actual IDs)
                created.base.bindings = saved_bindings;

                // Save tags to junction table
                if let Some(tag_service) = self.entity_tag_service()
                    && let Some(org_id) = authentication.organization_id()
                {
                    tag_service
                        .set_tags(
                            created.id,
                            EntityDiscriminants::Service,
                            service.base.tags.clone(),
                            org_id,
                        )
                        .await?;
                    created.base.tags = service.base.tags;
                }

                let trigger_stale = created.triggers_staleness(None);

                self.event_bus()
                    .publish_entity(EntityEvent {
                        id: Uuid::new_v4(),
                        entity_id: created.id,
                        network_id: self.get_network_id(&created),
                        organization_id: self.get_organization_id(&created),
                        entity_type: created.clone().into(),
                        operation: EntityOperation::Created,
                        timestamp: Utc::now(),
                        metadata: serde_json::json!({
                            "trigger_stale": trigger_stale
                        }),
                        authentication,
                    })
                    .await?;

                created
            }
        };

        Ok(service_from_storage)
    }

    async fn update(
        &self,
        service: &mut Service,
        authentication: AuthenticatedEntity,
    ) -> Result<Service> {
        let lock = self.get_service_lock(&service.id).await;
        let _guard = lock.lock().await;

        tracing::trace!("Updating service: {:?}", service);

        let current_service = self
            .get_by_id(&service.id)
            .await?
            .ok_or_else(|| anyhow!("Could not find service"))?;

        // Deduplicate bindings before validation
        service.base.bindings =
            Self::deduplicate_bindings(std::mem::take(&mut service.base.bindings));

        // Validate bindings don't conflict with each other
        Self::validate_bindings_no_conflicts(&service.base.bindings)?;

        // Validate bindings reference ports/interfaces on the service's host
        self.validate_bindings_belong_to_host(&service.base.host_id, &service.base.bindings)
            .await?;

        // Validate bindings aren't already claimed by other services on this host
        self.validate_bindings_available(
            &service.base.host_id,
            &service.id,
            &service.base.bindings,
        )
        .await?;

        self.update_group_service_bindings(&current_service, Some(service), authentication.clone())
            .await?;

        let mut updated = self.storage.update(service).await?;

        // Save bindings to separate table with correct service_id and network_id
        let bindings_with_ids: Vec<Binding> = service
            .base
            .bindings
            .iter()
            .cloned()
            .map(|b| b.with_service(updated.id, updated.base.network_id))
            .collect();
        let saved_bindings = self
            .binding_service
            .save_for_parent(&updated.id, &bindings_with_ids, authentication.clone())
            .await?;

        // Update service with the saved bindings (which have actual IDs and preserved created_at)
        updated.base.bindings = saved_bindings;

        // Update tags in junction table
        if let Some(tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
        {
            tag_service
                .set_tags(
                    updated.id,
                    EntityDiscriminants::Service,
                    updated.base.tags,
                    org_id,
                )
                .await?;
            updated.base.tags = service.base.tags.clone();
        }

        let trigger_stale = updated.triggers_staleness(Some(current_service));

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: updated.id,
                network_id: self.get_network_id(&updated),
                organization_id: self.get_organization_id(&updated),
                entity_type: updated.clone().into(),
                operation: EntityOperation::Updated,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale
                }),
                authentication: authentication.clone(),
            })
            .await?;

        Ok(updated)
    }

    async fn delete(&self, id: &Uuid, authentication: AuthenticatedEntity) -> Result<()> {
        let lock = self.get_service_lock(id).await;
        let _guard = lock.lock().await;

        let service = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Service {} not found", id))?;

        self.update_group_service_bindings(&service, None, authentication.clone())
            .await?;

        // Remove tags from junction table
        if let Some(tag_service) = self.entity_tag_service() {
            tag_service
                .remove_all_for_entity(*id, EntityDiscriminants::Service)
                .await?;
        }

        self.storage.delete(id).await?;

        let trigger_stale = service.triggers_staleness(None);

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: service.id,
                network_id: self.get_network_id(&service),
                organization_id: self.get_organization_id(&service),
                entity_type: service.into(),
                operation: EntityOperation::Deleted,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale
                }),
                authentication,
            })
            .await?;
        Ok(())
    }
}

impl ChildCrudService<Service> for ServiceService {}

impl ServiceService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Service>>,
        binding_service: Arc<BindingService>,
        group_service: Arc<GroupService>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            binding_service,
            group_service,
            host_service: OnceLock::new(),
            group_update_lock: Arc::new(Mutex::new(())),
            service_locks: Arc::new(Mutex::new(HashMap::new())),
            event_bus,
            entity_tag_service,
        }
    }

    /// Get all services matching filter, ordered by the specified column.
    /// Also loads bindings and tags for each service.
    pub async fn get_all_ordered(
        &self,
        filter: EntityFilter,
        order_by: &str,
    ) -> Result<Vec<Service>> {
        let mut services = self.storage.get_all_ordered(filter, order_by).await?;
        if services.is_empty() {
            return Ok(services);
        }

        let service_ids: Vec<Uuid> = services.iter().map(|s| s.id).collect();
        let bindings_map = self.binding_service.get_for_parents(&service_ids).await?;

        for service in &mut services {
            if let Some(bindings) = bindings_map.get(&service.id) {
                service.base.bindings = bindings.clone();
            }
        }

        self.bulk_hydrate_tags(&mut services).await?;

        Ok(services)
    }

    async fn get_service_lock(&self, service_id: &Uuid) -> Arc<Mutex<()>> {
        let mut locks = self.service_locks.lock().await;
        locks
            .entry(*service_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub fn set_host_service(&self, host_service: Arc<HostService>) -> Result<(), Arc<HostService>> {
        self.host_service.set(host_service)
    }

    /// Validate that all bindings reference ports/interfaces that belong to the service's host.
    /// Returns Ok(()) if all bindings are valid, Err with ValidationError if any are invalid.
    async fn validate_bindings_belong_to_host(
        &self,
        host_id: &Uuid,
        bindings: &[Binding],
    ) -> Result<()> {
        if bindings.is_empty() {
            return Ok(());
        }

        let host_service = self
            .host_service
            .get()
            .expect("host_service not initialized");

        // Get all ports and interfaces for this host
        let host_ports = host_service.get_ports_for_host(host_id).await?;
        let host_interfaces = host_service.get_interfaces_for_host(host_id).await?;

        let valid_port_ids: std::collections::HashSet<Uuid> =
            host_ports.iter().map(|p| p.id).collect();
        let valid_interface_ids: std::collections::HashSet<Uuid> =
            host_interfaces.iter().map(|i| i.id).collect();

        for binding in bindings {
            match &binding.base.binding_type {
                BindingType::Interface { interface_id } => {
                    if !valid_interface_ids.contains(interface_id) {
                        return Err(ValidationError::new(format!(
                            "Interface binding references interface {} which does not belong to this host",
                            interface_id
                        )).into());
                    }
                }
                BindingType::Port {
                    port_id,
                    interface_id,
                } => {
                    if !valid_port_ids.contains(port_id) {
                        return Err(ValidationError::new(format!(
                            "Port binding references port {} which does not belong to this host",
                            port_id
                        ))
                        .into());
                    }
                    if let Some(iface_id) = interface_id
                        && !valid_interface_ids.contains(iface_id)
                    {
                        return Err(ValidationError::new(format!(
                                "Port binding references interface {} which does not belong to this host",
                                iface_id
                            )).into());
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a new binding is already covered by existing bindings.
    /// A Port binding with a specific interface is covered if there's already
    /// a Port binding for the same port with interface_id = None (all interfaces).
    fn is_binding_covered_by_existing(
        new_binding: &Binding,
        existing_bindings: &[Binding],
    ) -> bool {
        match &new_binding.base.binding_type {
            // A Port binding with a specific interface is covered by an "all interfaces" binding for the same port
            BindingType::Port {
                port_id,
                interface_id: Some(_),
            } => existing_bindings.iter().any(|existing| {
                matches!(
                    &existing.base.binding_type,
                    BindingType::Port {
                        port_id: existing_port_id,
                        interface_id: None,
                    } if existing_port_id == port_id
                )
            }),
            // Other binding types are not covered by anything else
            _ => false,
        }
    }

    /// Validates that a binding doesn't conflict with existing bindings.
    /// Rules:
    /// - Interface binding conflicts with port bindings on same interface OR port bindings on all interfaces
    /// - Port binding (specific interface) conflicts with interface binding on same interface
    /// - Port binding (all interfaces) conflicts with ANY interface binding
    ///
    /// Returns None if valid, Some(error_message) if conflict found.
    fn validate_binding_no_conflict(
        new_binding: &BindingType,
        existing_bindings: &[Binding],
    ) -> Option<&'static str> {
        match new_binding {
            BindingType::Interface { interface_id } => {
                // Check for conflicting port bindings: same interface OR all-interfaces
                for existing in existing_bindings {
                    if let BindingType::Port {
                        interface_id: existing_iface,
                        ..
                    } = &existing.base.binding_type
                        && (*existing_iface == Some(*interface_id) || existing_iface.is_none())
                    {
                        return Some(
                            "Cannot add interface binding: service already has a port binding on this interface (or on all interfaces).",
                        );
                    }
                }
            }
            BindingType::Port {
                interface_id: Some(interface_id),
                ..
            } => {
                // Check for conflicting interface binding on same interface
                for existing in existing_bindings {
                    if let BindingType::Interface {
                        interface_id: existing_iface,
                    } = &existing.base.binding_type
                        && existing_iface == interface_id
                    {
                        return Some(
                            "Cannot add port binding: service already has an interface binding on this interface.",
                        );
                    }
                }
            }
            BindingType::Port {
                interface_id: None, ..
            } => {
                // Port binding on all interfaces: conflicts with ANY interface binding
                for existing in existing_bindings {
                    if matches!(existing.base.binding_type, BindingType::Interface { .. }) {
                        return Some(
                            "Cannot add port binding on all interfaces: service already has interface bindings.",
                        );
                    }
                }
            }
        }
        None
    }

    /// Deduplicate bindings in a list.
    /// - Removes exact duplicates (same binding_type)
    /// - When an all-interfaces port binding is present, removes specific-interface bindings for the same port
    fn deduplicate_bindings(bindings: Vec<Binding>) -> Vec<Binding> {
        use std::collections::HashSet;

        // First, collect all port_ids that have all-interfaces bindings
        let all_interface_port_ids: HashSet<Uuid> = bindings
            .iter()
            .filter_map(|b| {
                if let BindingType::Port {
                    port_id,
                    interface_id: None,
                } = &b.base.binding_type
                {
                    Some(*port_id)
                } else {
                    None
                }
            })
            .collect();

        // Track seen binding types for deduplication
        let mut seen_binding_types: HashSet<String> = HashSet::new();
        let mut result = Vec::new();

        for binding in bindings {
            // Skip specific-interface port bindings when an all-interfaces binding exists for the same port
            if let BindingType::Port {
                port_id,
                interface_id: Some(_),
            } = &binding.base.binding_type
                && all_interface_port_ids.contains(port_id)
            {
                tracing::debug!(
                    port_id = %port_id,
                    "Deduplicating specific-interface binding superseded by all-interfaces binding"
                );
                continue;
            }

            // Create a key for deduplication based on binding type
            let key = format!("{:?}", binding.base.binding_type);
            if seen_binding_types.contains(&key) {
                tracing::debug!(
                    binding_type = %key,
                    "Deduplicating duplicate binding"
                );
                continue;
            }

            seen_binding_types.insert(key);
            result.push(binding);
        }

        result
    }

    /// Validate all bindings in a list don't conflict with each other.
    /// Returns Ok(()) if all bindings are valid, Err with message if any conflict.
    fn validate_bindings_no_conflicts(bindings: &[Binding]) -> Result<()> {
        for (i, binding) in bindings.iter().enumerate() {
            // Check against all bindings before this one (to avoid duplicate checks)
            let preceding_bindings = &bindings[..i];
            if let Some(error_msg) =
                Self::validate_binding_no_conflict(&binding.base.binding_type, preceding_bindings)
            {
                return Err(ValidationError::new(error_msg).into());
            }
        }
        Ok(())
    }

    /// Partition bindings into non-conflicting and conflicting sets.
    ///
    /// A binding conflicts if another service on the same host already has a port binding
    /// to the same port on the same interface (or either is "all interfaces").
    ///
    /// Also checks against `batch_claimed` for in-batch conflict detection during discovery.
    ///
    /// Returns: (valid_bindings, conflicting_bindings)
    pub async fn partition_conflicting_bindings(
        &self,
        host_id: &Uuid,
        service_id: &Uuid,
        bindings: Vec<Binding>,
        batch_claimed: &[(Uuid, Option<Uuid>)],
    ) -> Result<(Vec<Binding>, Vec<Binding>)> {
        if bindings.is_empty() {
            return Ok((vec![], vec![]));
        }

        // Get existing claimed bindings from database
        let filter = EntityFilter::unfiltered().host_id(host_id);
        let db_claimed: Vec<(Uuid, Option<Uuid>)> = self
            .get_all(filter)
            .await?
            .into_iter()
            .filter(|s| s.id != *service_id)
            .flat_map(|s| {
                s.base.bindings.into_iter().filter_map(|b| {
                    if let BindingType::Port {
                        port_id,
                        interface_id,
                    } = b.base.binding_type
                    {
                        Some((port_id, interface_id))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Combine DB claims with batch claims
        let all_claimed: Vec<_> = db_claimed
            .iter()
            .chain(batch_claimed.iter())
            .cloned()
            .collect();

        if all_claimed.is_empty() {
            return Ok((bindings, vec![]));
        }

        let mut valid = Vec::new();
        let mut conflicting = Vec::new();

        for binding in bindings {
            if let BindingType::Port {
                port_id,
                interface_id,
            } = &binding.base.binding_type
            {
                let has_conflict = all_claimed.iter().any(|(claimed_port, claimed_iface)| {
                    if claimed_port != port_id {
                        return false;
                    }
                    // Conflict if same port AND interfaces overlap:
                    // - Either is "all interfaces" (None) -> conflict
                    // - Both specific and same interface -> conflict
                    match (interface_id, claimed_iface) {
                        (None, _) | (_, None) => true,
                        (Some(a), Some(b)) => a == b,
                    }
                });

                if has_conflict {
                    conflicting.push(binding);
                } else {
                    valid.push(binding);
                }
            } else {
                // Interface bindings don't conflict cross-service
                valid.push(binding);
            }
        }

        Ok((valid, conflicting))
    }

    /// Validate that proposed bindings don't conflict with other services on the same host.
    /// Returns error with helpful message identifying the conflicting service.
    /// Used for manual service creation/update validation.
    async fn validate_bindings_available(
        &self,
        host_id: &Uuid,
        service_id: &Uuid,
        bindings: &[Binding],
    ) -> Result<()> {
        if bindings.is_empty() {
            return Ok(());
        }

        let filter = EntityFilter::unfiltered().host_id(host_id);
        let other_services: Vec<_> = self
            .get_all(filter)
            .await?
            .into_iter()
            .filter(|s| s.id != *service_id)
            .collect();

        for binding in bindings {
            if let BindingType::Port {
                port_id,
                interface_id,
            } = &binding.base.binding_type
            {
                let conflicting_service = other_services.iter().find(|s| {
                    s.base.bindings.iter().any(|b| {
                        if let BindingType::Port {
                            port_id: existing_port,
                            interface_id: existing_iface,
                        } = &b.base.binding_type
                        {
                            if existing_port != port_id {
                                return false;
                            }
                            match (interface_id, existing_iface) {
                                (None, _) | (_, None) => true,
                                (Some(a), Some(b)) => a == b,
                            }
                        } else {
                            false
                        }
                    })
                });

                if let Some(owner) = conflicting_service {
                    let host_service = self
                        .host_service
                        .get()
                        .expect("host_service not initialized");

                    let ports = host_service.get_ports_for_host(host_id).await?;
                    let port_display = ports
                        .iter()
                        .find(|p| p.id == *port_id)
                        .map(|p| p.to_string())
                        .unwrap_or_else(|| port_id.to_string());

                    return Err(ValidationError::new(format!(
                        "Port {} is already bound to '{}' on this host. \
                         Use 'Transfer Ports' to reassign it, or remove the binding from '{}' first.",
                        port_display, owner.base.name, owner.base.name,
                    ))
                    .into());
                }
            }
        }

        Ok(())
    }

    pub async fn upsert_service(
        &self,
        mut existing_service: Service,
        new_service_data: Service,
        authentication: AuthenticatedEntity,
    ) -> Result<Service> {
        // NOTE: This function assumes the caller already holds the service lock.
        // It's called from create() which acquires the lock before calling this.
        let mut binding_updates = 0;

        let service_before_updates = existing_service.clone();

        tracing::trace!(
            "Upserting new service data {:?} into {:?}",
            new_service_data,
            existing_service
        );

        for new_service_binding in &new_service_data.base.bindings {
            // Check if this binding is already covered by existing bindings
            // (e.g., a specific interface binding is covered by an "all interfaces" binding for the same port)
            let is_covered = Self::is_binding_covered_by_existing(
                new_service_binding,
                &existing_service.base.bindings,
            );

            if is_covered {
                tracing::trace!(
                    "Skipping binding {:?} - already covered by existing all-interfaces binding",
                    new_service_binding.base.binding_type
                );
                continue;
            }

            // Check for binding type conflicts (Interface vs Port on same interface)
            if let Some(conflict_msg) = Self::validate_binding_no_conflict(
                &new_service_binding.base.binding_type,
                &existing_service.base.bindings,
            ) {
                tracing::warn!(
                    "Skipping binding {:?} - conflicts with existing binding: {}",
                    new_service_binding.base.binding_type,
                    conflict_msg
                );
                continue;
            }

            // If new binding is "all interfaces" port binding, remove specific interface bindings for same port
            // (the all-interfaces binding supersedes them)
            if let BindingType::Port {
                port_id,
                interface_id: None,
            } = &new_service_binding.base.binding_type
            {
                let before_count = existing_service.base.bindings.len();
                existing_service.base.bindings.retain(|existing| {
                    // Log each comparison for debugging
                    if let BindingType::Port {
                        port_id: existing_port_id,
                        interface_id: existing_interface_id,
                    } = &existing.base.binding_type
                    {
                        let dominated =
                            existing_interface_id.is_some() && existing_port_id == port_id;

                        return !dominated;
                    }
                    true // Keep non-port bindings
                });
                let removed = before_count - existing_service.base.bindings.len();

                if removed > 0 {
                    binding_updates += removed;
                }
            }

            if !existing_service.base.bindings.contains(new_service_binding) {
                binding_updates += 1;
                existing_service.base.bindings.push(*new_service_binding);
            }
        }

        if let Some(virtualization) = &new_service_data.base.virtualization {
            existing_service.base.virtualization = Some(virtualization.clone())
        }

        existing_service.base.source = match (
            existing_service.base.source,
            new_service_data.base.source.clone(),
        ) {
            // Add latest discovery metadata to vec, update details to summarize what was discovered + highest confidence
            (
                EntitySource::DiscoveryWithMatch {
                    metadata: existing_service_metadata,
                    details: existing_service_details,
                },
                EntitySource::DiscoveryWithMatch {
                    metadata: new_service_metadata,
                    details: new_service_details,
                },
            ) => {
                let new_metadata = [
                    new_service_metadata.clone(),
                    existing_service_metadata.clone(),
                ]
                .concat();

                // Max confidence
                let confidence = existing_service_details
                    .confidence
                    .max(new_service_details.confidence);

                let reason = if new_service_details.confidence > existing_service_details.confidence
                {
                    new_service_details.reason // Use the better match reason
                } else {
                    existing_service_details.reason // Keep existing reason
                };

                EntitySource::DiscoveryWithMatch {
                    metadata: new_metadata,
                    details: MatchDetails { confidence, reason },
                }
            }

            // Less-likely scenario: new service data is upserted to a manually or system-created record
            (
                _,
                EntitySource::DiscoveryWithMatch {
                    metadata: new_service_metadata,
                    details: new_service_details,
                },
            ) => EntitySource::DiscoveryWithMatch {
                metadata: new_service_metadata,
                details: new_service_details,
            },

            // The following case shouldn't be possible since upsert only happens from discovered services, but cover with something reasonable just in case
            (existing_source, _) => existing_source,
        };

        self.storage.update(&mut existing_service).await?;

        // Save bindings to separate table with correct service_id and network_id
        let bindings_with_ids: Vec<Binding> = existing_service
            .base
            .bindings
            .iter()
            .cloned()
            .map(|b| b.with_service(existing_service.id, existing_service.base.network_id))
            .collect();

        let saved_bindings = self
            .binding_service
            .save_for_parent(
                &existing_service.id,
                &bindings_with_ids,
                authentication.clone(),
            )
            .await?;

        // Update service with the saved bindings (which have actual IDs and preserved created_at)
        existing_service.base.bindings = saved_bindings;

        let mut data = Vec::new();

        if binding_updates > 0 {
            data.push(format!("{} bindings", binding_updates))
        };

        if !data.is_empty() {
            let trigger_stale = existing_service.triggers_staleness(Some(service_before_updates));

            self.event_bus()
                .publish_entity(EntityEvent {
                    id: Uuid::new_v4(),
                    entity_id: existing_service.id,
                    network_id: self.get_network_id(&existing_service),
                    organization_id: self.get_organization_id(&existing_service),
                    entity_type: existing_service.clone().into(),
                    operation: EntityOperation::Updated,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({
                        "trigger_stale": trigger_stale
                    }),
                    authentication,
                })
                .await?;
        } else {
            tracing::debug!(
                service_id = %existing_service.id,
                "Service upsert - no binding changes needed"
            );
        }

        Ok(existing_service)
    }

    async fn update_group_service_bindings(
        &self,
        current_service: &Service,
        updates: Option<&Service>,
        authenticated: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let filter = EntityFilter::unfiltered().network_ids(&[current_service.base.network_id]);
        let groups = self.group_service.get_all(filter).await?;

        let _guard = self.group_update_lock.lock().await;

        let current_service_binding_ids: Vec<Uuid> = current_service
            .base
            .bindings
            .iter()
            .map(|b| b.id())
            .collect();
        let updated_service_binding_ids: Vec<Uuid> = match updates {
            Some(updated_service) => updated_service
                .base
                .bindings
                .iter()
                .map(|b| b.id())
                .collect(),
            None => Vec::new(),
        };

        let groups_to_update: Vec<Group> = groups
            .into_iter()
            .filter_map(|mut group| {
                let initial_bindings_length = group.base.binding_ids.len();

                group.base.binding_ids.retain(|sb| {
                    let in_current = current_service_binding_ids.contains(sb);
                    let in_updated = updated_service_binding_ids.contains(sb);
                    if in_current { in_updated } else { true }
                });

                if group.base.binding_ids.len() != initial_bindings_length {
                    Some(group)
                } else {
                    None
                }
            })
            .collect();

        if !groups_to_update.is_empty() {
            for mut group in groups_to_update {
                self.group_service
                    .update(&mut group, authenticated.clone())
                    .await?;
            }
        }

        Ok(())
    }

    /// Update bindings to match ports and interfaces available on new host
    /// `original_interfaces` and `updated_interfaces` are the interfaces for the respective hosts
    /// `original_ports` and `updated_ports` are the ports for the respective hosts
    #[allow(clippy::too_many_arguments)]
    pub async fn reassign_service_interface_bindings(
        &self,
        service: Service,
        original_host: &Host,
        original_interfaces: &[Interface],
        original_ports: &[Port],
        updated_host: &Host,
        updated_interfaces: &[Interface],
        updated_ports: &[Port],
    ) -> Service {
        let lock = self.get_service_lock(&service.id).await;
        let _guard = lock.lock().await;

        tracing::trace!(
            "Preparing service {:?} for transfer from host {:?} to host {:?}",
            service,
            original_host,
            updated_host
        );

        let mut mutable_service = service.clone();

        let service_name = service.base.name.clone();
        let service_id = service.id;

        mutable_service.base.bindings = mutable_service
            .base
            .bindings
            .iter_mut()
            .filter_map(|b| {
                // Look up original interface from the provided slice
                let original_interface = b
                    .interface_id()
                    .and_then(|id| original_interfaces.iter().find(|i| i.id == id));

                match &mut b.base.binding_type {
                    BindingType::Interface { interface_id } => {
                        if let Some(original_interface) = original_interface {
                            let new_interface: Option<&Interface> =
                                updated_interfaces.iter().find(|i| *i == original_interface);

                            if let Some(new_interface) = new_interface {
                                *interface_id = new_interface.id;
                                return Some(*b);
                            }
                        }
                        // Interface binding couldn't be matched - this can happen during consolidation
                        // when the source host's interface doesn't exist on the destination host.
                        // We drop the binding and warn.
                        tracing::warn!(
                            service_id = %service_id,
                            service_name = %service_name,
                            original_interface_id = ?b.interface_id(),
                            "Dropping interface binding during reassignment: \
                             no matching interface found on destination host"
                        );
                        None::<Binding>
                    }
                    BindingType::Port {
                        port_id,
                        interface_id,
                    } => {
                        if let Some(original_port) =
                            original_ports.iter().find(|p| p.id == *port_id)
                            && let Some(new_port) =
                                updated_ports.iter().find(|p| *p == original_port)
                        {
                            let new_interface: Option<Option<Interface>> = match original_interface
                            {
                                // None interface = listen on all interfaces, assume same for new host
                                None => Some(None),
                                Some(original_interface) => {
                                    match updated_interfaces
                                        .iter()
                                        .find(|i| *i == original_interface)
                                    {
                                        Some(found_interface) => {
                                            Some(Some(found_interface.clone()))
                                        }
                                        None => {
                                            // Interface not found on destination host - fall back to "all interfaces"
                                            // This is better than dropping the binding entirely
                                            tracing::warn!(
                                                service_id = %service_id,
                                                service_name = %service_name,
                                                port_number = %new_port.base.port_type.config().number,
                                                original_interface_ip = %original_interface.base.ip_address,
                                                "Port binding interface not found on destination host - \
                                                 falling back to 'all interfaces'"
                                            );
                                            Some(None)
                                        }
                                    }
                                }
                            };

                            match new_interface {
                                None => return None,
                                Some(new_interface) => {
                                    *port_id = new_port.id;
                                    *interface_id = match new_interface {
                                        Some(new_interface) => Some(new_interface.id),
                                        None => None,
                                    };
                                    return Some(*b);
                                }
                            }
                        }
                        // Port not found on destination host - drop the binding
                        tracing::warn!(
                            service_id = %service_id,
                            service_name = %service_name,
                            original_port_id = %port_id,
                            "Dropping port binding during reassignment: \
                             no matching port found on destination host"
                        );
                        None::<Binding>
                    }
                };

                None
            })
            .collect();

        mutable_service.base.host_id = updated_host.id;

        mutable_service.base.network_id = updated_host.base.network_id;

        tracing::trace!(
            "Reassigned service {:?} bindings for from host {:?} to host {:?}",
            mutable_service,
            original_host,
            updated_host
        );

        mutable_service
    }
}
