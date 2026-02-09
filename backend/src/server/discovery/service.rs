use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::daemon::runtime::service::LOG_TARGET;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::discovery::r#impl::base::Discovery;
use crate::server::discovery::r#impl::types::{DiscoveryType, RunType};
use crate::server::networks::service::NetworkService;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants};
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::events::types::{EntityEvent, EntityOperation};
use crate::server::shared::events::types::{TelemetryEvent, TelemetryOperation};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::generic::GenericPostgresStorage;
use crate::server::shared::storage::traits::{Storable, Storage};
use crate::server::snmp_credentials::service::SnmpCredentialService;
use crate::server::tags::entity_tags::EntityTagService;
use anyhow::anyhow;
use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::Utc;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{RwLock, broadcast};
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

/// Server-side session management for discovery
pub struct DiscoveryService {
    discovery_storage: Arc<GenericPostgresStorage<Discovery>>,
    sessions: RwLock<HashMap<Uuid, DiscoveryUpdatePayload>>, // session_id -> session state mapping
    daemon_sessions: RwLock<HashMap<Uuid, Vec<Uuid>>>,       // daemon_id -> session_id mapping
    daemon_pull_cancellations: RwLock<HashMap<Uuid, (bool, Uuid)>>, // daemon_id -> (boolean, session_id) mapping for pull mode cancellations of current session on daemon
    session_last_updated: RwLock<HashMap<Uuid, chrono::DateTime<Utc>>>,
    update_tx: broadcast::Sender<DiscoveryUpdatePayload>,
    scheduler: Option<Arc<RwLock<JobScheduler>>>,
    job_ids: RwLock<HashMap<Uuid, Uuid>>, // discovery_id -> scheduler job_id mapping
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    snmp_credential_service: Arc<SnmpCredentialService>,
    network_service: Arc<NetworkService>,
    organization_service: Arc<OrganizationService>,
}

impl EventBusService<Discovery> for DiscoveryService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Discovery) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Discovery) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Discovery> for DiscoveryService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Discovery>> {
        &self.discovery_storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }
}

impl DiscoveryService {
    pub async fn new(
        discovery_storage: Arc<GenericPostgresStorage<Discovery>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
        snmp_credential_service: Arc<SnmpCredentialService>,
        network_service: Arc<NetworkService>,
        organization_service: Arc<OrganizationService>,
    ) -> Result<Arc<Self>> {
        let (tx, _rx) = broadcast::channel(100); // Buffer 100 messages
        let scheduler = JobScheduler::new().await?;

        Ok(Arc::new(Self {
            discovery_storage,
            sessions: RwLock::new(HashMap::new()),
            daemon_sessions: RwLock::new(HashMap::new()),
            daemon_pull_cancellations: RwLock::new(HashMap::new()),
            session_last_updated: RwLock::new(HashMap::new()),
            update_tx: tx,
            scheduler: Some(Arc::new(RwLock::new(scheduler))),
            job_ids: RwLock::new(HashMap::new()),
            event_bus,
            entity_tag_service,
            snmp_credential_service,
            network_service,
            organization_service,
        }))
    }

    /// Expose stream to handler
    pub fn subscribe(&self) -> broadcast::Receiver<DiscoveryUpdatePayload> {
        self.update_tx.subscribe()
    }

    /// Get session state
    pub async fn get_session(&self, session_id: &Uuid) -> Option<DiscoveryUpdatePayload> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Get session state
    pub async fn get_all_sessions(&self, network_ids: &[Uuid]) -> Vec<DiscoveryUpdatePayload> {
        let all_sessions = self.sessions.read().await;
        all_sessions
            .values()
            .filter(|v| network_ids.contains(&v.network_id))
            .cloned()
            .collect()
    }

    pub async fn get_sessions_for_daemon(&self, daemon_id: &Uuid) -> Vec<DiscoveryUpdatePayload> {
        let daemon_session_ids = self.daemon_sessions.read().await;
        let session_ids = daemon_session_ids
            .get(daemon_id)
            .cloned()
            .unwrap_or_default();

        let all_sessions = self.sessions.read().await;

        // Preserve order from daemon_sessions Vec (not HashMap iteration order)
        // Only return Pending sessions - once dispatched, they transition to Starting
        session_ids
            .iter()
            .filter_map(|session_id| all_sessions.get(session_id).cloned())
            .filter(|session| session.phase == DiscoveryPhase::Pending)
            .collect()
    }

    /// Clear all sessions for a daemon from in-memory state.
    /// Used by tests to ensure clean state between phases.
    pub async fn clear_sessions_for_daemon(&self, daemon_id: &Uuid) {
        let mut sessions = self.sessions.write().await;
        let mut daemon_sessions = self.daemon_sessions.write().await;
        let mut session_last_updated = self.session_last_updated.write().await;
        let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;

        if let Some(session_ids) = daemon_sessions.remove(daemon_id) {
            for session_id in &session_ids {
                sessions.remove(session_id);
                session_last_updated.remove(session_id);
            }
            tracing::debug!(
                daemon_id = %daemon_id,
                session_count = session_ids.len(),
                "Cleared all sessions for daemon"
            );
        }

        daemon_pull_cancellations.remove(daemon_id);
    }

    /// Check if daemon has an active (non-terminal, non-pending) discovery session.
    /// This is used to prevent dispatching new work while a session is in progress.
    pub async fn has_active_session_for_daemon(&self, daemon_id: &Uuid) -> bool {
        let daemon_session_ids = self.daemon_sessions.read().await;
        let session_ids = daemon_session_ids
            .get(daemon_id)
            .cloned()
            .unwrap_or_default();

        let all_sessions = self.sessions.read().await;

        session_ids.iter().any(|session_id| {
            all_sessions
                .get(session_id)
                .map(|s| !s.phase.is_terminal() && s.phase != DiscoveryPhase::Pending)
                .unwrap_or(false)
        })
    }

    /// Transition a session from Pending to Starting phase.
    /// Called when the session is dispatched to the daemon.
    pub async fn transition_session_to_starting(&self, session_id: Uuid) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id)
            && session.phase == DiscoveryPhase::Pending
        {
            session.phase = DiscoveryPhase::Starting;
            tracing::debug!(
                session_id = %session_id,
                "Transitioned session to Starting phase"
            );
        }
    }

    pub async fn pull_cancellation_for_daemon(&self, daemon_id: &Uuid) -> (bool, Uuid) {
        let mut daemon_cancellation_ids = self.daemon_pull_cancellations.write().await;
        daemon_cancellation_ids
            .remove(daemon_id)
            .unwrap_or((false, Uuid::nil()))
    }

    /// Create a new scheduled discovery
    pub async fn create_discovery(
        self: &Arc<Self>,
        discovery: Discovery,
        authentication: AuthenticatedEntity,
    ) -> Result<Discovery> {
        let mut created_discovery = if discovery.id == Uuid::nil() {
            self.discovery_storage
                .create(&Discovery::new(discovery.base))
                .await?
        } else {
            self.discovery_storage.create(&discovery).await?
        };

        // Save tags to junction table
        if let Some(entity_tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
        {
            entity_tag_service
                .set_tags(
                    created_discovery.id,
                    EntityDiscriminants::Discovery,
                    created_discovery.base.tags.clone(),
                    org_id,
                )
                .await?;
        }

        // If it's a scheduled discovery, add it to the scheduler
        if matches!(created_discovery.base.run_type, RunType::Scheduled { .. })
            && let Err(e) = Self::schedule_discovery(self, &created_discovery).await
        {
            // Disable and save to DB
            created_discovery.disable();
            let disabled_discovery = self
                .discovery_storage
                .update(&mut created_discovery)
                .await?;

            tracing::error!(
                "Failed to schedule discovery {}. Discovery created but disabled. Error: {}",
                disabled_discovery.id,
                e
            );

            return Ok(disabled_discovery);
        }

        let trigger_stale = created_discovery.triggers_staleness(None);

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: created_discovery.id(),
                network_id: self.get_network_id(&created_discovery),
                organization_id: self.get_organization_id(&created_discovery),
                entity_type: created_discovery.clone().into(),
                operation: EntityOperation::Created,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale
                }),

                authentication,
            })
            .await?;

        Ok(created_discovery)
    }

    /// Update discovery
    pub async fn update_discovery(
        self: &Arc<Self>,
        mut discovery: Discovery,
        authentication: AuthenticatedEntity,
    ) -> Result<Discovery, Error> {
        let current = self
            .get_by_id(&discovery.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not find discovery {}", discovery))?;

        // If it's a scheduled discovery and schedule has changed, need to reschedule
        let updated = if let RunType::Scheduled {
            cron_schedule: new_cron,
            ..
        } = &discovery.base.run_type
            && let RunType::Scheduled {
                cron_schedule: current_cron,
                ..
            } = &current.base.run_type
            && current_cron != new_cron
        {
            // Remove old schedule first using the actual job_id
            if let Some(scheduler) = &self.scheduler
                && let Some(job_id) = self.job_ids.read().await.get(&discovery.id).copied()
            {
                if let Err(e) = scheduler.write().await.remove(&job_id).await {
                    tracing::warn!(
                        discovery_id = %discovery.id,
                        job_id = %job_id,
                        error = ?e,
                        "Failed to remove old scheduled job"
                    );
                } else {
                    self.job_ids.write().await.remove(&discovery.id);
                }
            }

            // Update in DB first
            let mut updated = self.discovery_storage.update(&mut discovery).await?;

            // Try to reschedule with new cron expression
            if let Err(e) = Self::schedule_discovery(self, &updated).await {
                // Disable and save again
                updated.disable();
                let disabled_discovery = self.discovery_storage.update(&mut updated).await?;

                tracing::error!(
                    "Failed to reschedule discovery {}. Discovery updated but disabled. Error: {}",
                    disabled_discovery.id,
                    e
                );
            }

            updated
        } else {
            // For non-scheduled, just update
            self.discovery_storage.update(&mut discovery).await?
        };

        // Update tags in junction table
        if let Some(entity_tag_service) = self.entity_tag_service()
            && let Some(org_id) = authentication.organization_id()
        {
            entity_tag_service
                .set_tags(
                    updated.id,
                    EntityDiscriminants::Discovery,
                    discovery.base.tags,
                    org_id,
                )
                .await?;
        }

        let trigger_stale = updated.triggers_staleness(Some(current));

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: updated.id(),
                network_id: self.get_network_id(&updated),
                organization_id: self.get_organization_id(&updated),
                entity_type: updated.clone().into(),
                operation: EntityOperation::Updated,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "trigger_stale": trigger_stale
                }),

                authentication,
            })
            .await?;

        Ok(updated)
    }

    /// Delete group
    pub async fn delete_discovery(
        self: &Arc<Self>,
        id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        let discovery = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Discovery not found"))?;

        // If it's scheduled, remove from scheduler first using the actual job_id
        if matches!(discovery.base.run_type, RunType::Scheduled { .. })
            && let Some(scheduler) = &self.scheduler
        {
            if let Some(job_id) = self.job_ids.read().await.get(id).copied() {
                if let Err(e) = scheduler.write().await.remove(&job_id).await {
                    tracing::warn!(
                        discovery_id = %id,
                        job_id = %job_id,
                        error = ?e,
                        "Failed to remove scheduled job during deletion"
                    );
                } else {
                    self.job_ids.write().await.remove(id);
                }
            }
            tracing::debug!("Removed scheduled job for discovery {}", id);
        }

        // Remove tags from junction table
        if let Some(tag_service) = self.entity_tag_service() {
            tag_service
                .remove_all_for_entity(*id, EntityDiscriminants::Discovery)
                .await?;
        }

        self.discovery_storage.delete(id).await?;

        let trigger_stale = discovery.triggers_staleness(None);

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: discovery.id(),
                network_id: self.get_network_id(&discovery),
                organization_id: self.get_organization_id(&discovery),
                entity_type: discovery.into(),
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

    /// Initialize scheduler with all scheduled discoveries
    pub async fn start_scheduler(self: &Arc<Self>) -> Result<()> {
        let scheduler = self
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))?;

        // Clear any stale job_id mappings from previous runs
        self.job_ids.write().await.clear();

        let filter = StorableFilter::<Discovery>::new_for_scheduled_discoveries();

        let discoveries = self.discovery_storage.get_all(filter).await?;
        let count = discoveries.len();

        let mut failed_count = 0;
        for mut discovery in discoveries {
            if let Err(e) = Self::schedule_discovery(self, &discovery).await {
                tracing::error!(
                    "Failed to schedule discovery {}: {}. Disabling.",
                    discovery.id,
                    e
                );

                // Disable and save
                discovery.disable();
                let _ = self.discovery_storage.update(&mut discovery).await;
                failed_count += 1;
            }
        }

        scheduler.write().await.start().await?;

        if failed_count == 0 {
            tracing::info!(target: LOG_TARGET, "Discovery scheduler started with {} jobs", count);
        } else {
            tracing::warn!(
                target: LOG_TARGET,
                "Discovery scheduler started with {}/{} jobs. {} failed and were disabled.",
                count - failed_count,
                count,
                failed_count
            );
        }

        Ok(())
    }

    /// Schedule a single discovery
    async fn schedule_discovery(
        service: &Arc<DiscoveryService>,
        discovery: &Discovery,
    ) -> Result<Uuid> {
        let _ = service
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))?;

        let RunType::Scheduled {
            cron_schedule,
            enabled,
            ..
        } = &discovery.base.run_type
        else {
            return Err(anyhow::anyhow!("Discovery is not scheduled"));
        };

        if !enabled {
            return Err(anyhow::anyhow!("Discovery is not enabled"));
        }

        let scheduler = service
            .scheduler
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Scheduler not initialized"))?;

        let discovery = discovery.clone();
        let discovery_id = discovery.id;
        let storage = service.discovery_storage.clone();

        // Clone self to use start_session
        let service_clone = Arc::clone(service);

        let job = Job::new_async(cron_schedule.as_str(), move |_uuid, _lock| {
            let mut discovery = discovery.clone();
            let storage = storage.clone();
            let service = service_clone.clone();

            Box::pin(async move {
                tracing::info!("Running scheduled discovery {}", &discovery.id);

                match service
                    .start_session(discovery.clone(), AuthenticatedEntity::System)
                    .await
                {
                    Ok(_) => {
                        // Update last_run
                        if let RunType::Scheduled {
                            last_run: mut _last_run,
                            ..
                        } = discovery.base.run_type
                        {
                            _last_run = Some(Utc::now());
                            if let Err(e) = storage.update(&mut discovery).await {
                                tracing::error!("Failed to update schedule times: {}", e);
                            }
                        };
                    }
                    Err(e) => {
                        tracing::error!("Scheduled discovery {} failed: {}", discovery_id, e);
                    }
                }
            })
        })?;

        let job_id = scheduler.write().await.add(job).await?;

        // Store the mapping so we can remove the job later when the schedule is updated
        service.job_ids.write().await.insert(discovery_id, job_id);

        tracing::debug!(
            "Scheduled discovery {} with job_id {} and cron: {}",
            discovery_id,
            job_id,
            cron_schedule
        );
        Ok(job_id)
    }

    /// Create a new discovery session
    pub async fn start_session(
        &self,
        discovery: Discovery,
        authentication: AuthenticatedEntity,
    ) -> Result<DiscoveryUpdatePayload, anyhow::Error> {
        let session_id = Uuid::new_v4();

        // Hydrate SNMP credentials
        let discovery_type = if let DiscoveryType::Network {
            host_naming_fallback,
            subnet_ids,
            probe_raw_socket_ports,
            ..
        } = discovery.base.discovery_type
        {
            DiscoveryType::Network {
                subnet_ids,
                host_naming_fallback,
                snmp_credentials: self
                    .snmp_credential_service
                    .build_credentials_for_discovery(discovery.base.network_id)
                    .await?,
                probe_raw_socket_ports,
            }
        } else {
            discovery.base.discovery_type
        };

        let session_payload = DiscoveryUpdatePayload::new(
            session_id,
            discovery.base.daemon_id,
            discovery.base.network_id,
            discovery_type,
        );

        // Add to session map
        self.sessions
            .write()
            .await
            .insert(session_id, session_payload.clone());

        // Check if daemon has any sessions running
        let daemon_is_running_discovery = if let Some(daemon_sessions) = self
            .daemon_sessions
            .read()
            .await
            .get(&discovery.base.daemon_id)
        {
            !daemon_sessions.is_empty()
        } else {
            false
        };

        // Add session to queue
        self.daemon_sessions
            .write()
            .await
            .entry(discovery.base.daemon_id)
            .or_default()
            .push(session_id);

        // Publish Started event if no other sessions are running for daemon
        // DaemonService subscribes to this event and sends the request to the daemon.
        if !daemon_is_running_discovery {
            self.event_bus()
                .publish_discovery(session_payload.into_discovery_event_with_auth(authentication))
                .await?;
        }

        let _ = self.update_tx.send(session_payload.clone());

        Ok(session_payload)
    }

    /// Update progress for a session
    /// If the session doesn't exist (e.g., server restarted during discovery),
    /// auto-creates it from the payload context to maintain resilience.
    pub async fn update_session(&self, update: DiscoveryUpdatePayload) -> Result<(), Error> {
        tracing::debug!("Updated session {:?}", update);

        let mut sessions = self.sessions.write().await;

        let mut last_updated = self.session_last_updated.write().await;
        // Check if we've seen this session before (used as tombstone for completed sessions)
        let already_seen = last_updated.contains_key(&update.session_id);
        // Track last update time
        last_updated.insert(update.session_id, Utc::now());

        // Auto-create session if it doesn't exist (handles server restarts during discovery)
        if let std::collections::hash_map::Entry::Vacant(e) = sessions.entry(update.session_id) {
            // If we already tracked this session but it's no longer in the sessions map,
            // it was already processed and removed. Skip redundant terminal updates from
            // old daemons that don't clear their terminal payload after serving it.
            if update.phase.is_terminal() && already_seen {
                tracing::debug!(
                    session_id = %update.session_id,
                    phase = %update.phase,
                    "Ignoring redundant terminal update (already processed)"
                );
                return Ok(());
            }

            tracing::info!(
                session_id = %update.session_id,
                daemon_id = %update.daemon_id,
                network_id = %update.network_id,
                "Auto-creating session from daemon update"
            );

            // Track in daemon_sessions map
            let mut daemon_sessions = self.daemon_sessions.write().await;
            daemon_sessions
                .entry(update.daemon_id)
                .or_default()
                .push(update.session_id);
            drop(daemon_sessions);

            // Insert the session
            e.insert(update.clone());
        }

        let session = sessions.get_mut(&update.session_id).unwrap();

        let daemon_id = session.daemon_id;
        let network_id = session.network_id;

        tracing::debug!(
            session_id = %update.session_id,
            phase = %update.phase,
            progrsss = %update.progress,
            "Updated session",
        );

        let _ = self.update_tx.send(update.clone());

        *session = update.clone();

        if session.phase.is_terminal() {
            self.event_bus()
                .publish_discovery(session.into_discovery_event())
                .await?;

            // Emit FirstDiscoveryCompleted telemetry if this is the org's first completed discovery
            if session.phase == DiscoveryPhase::Complete
                && let Ok(Some(network)) = self.network_service.get_by_id(&network_id).await
                && let Ok(Some(org)) = self
                    .organization_service
                    .get_by_id(&network.base.organization_id)
                    .await
                && org.not_onboarded(&TelemetryOperation::FirstDiscoveryCompleted)
            {
                let _ = self
                    .event_bus
                    .publish_telemetry(TelemetryEvent::new(
                        Uuid::new_v4(),
                        org.id,
                        TelemetryOperation::FirstDiscoveryCompleted,
                        Utc::now(),
                        AuthenticatedEntity::System,
                        serde_json::json!({}),
                    ))
                    .await;
            }

            // If user cancelled session, but it finished before we could send cancellation, remove key so it doesn't cancel upcoming sessions
            self.pull_cancellation_for_daemon(&session.daemon_id).await;

            // Create historical discovery record
            let historical_discovery = Discovery {
                id: Uuid::new_v4(),
                created_at: session.started_at.unwrap_or(Utc::now()),
                updated_at: Utc::now(),
                base: crate::server::discovery::r#impl::base::DiscoveryBase {
                    daemon_id: session.daemon_id,
                    network_id: session.network_id,
                    name: session.discovery_type.to_string(),
                    tags: Vec::new(),
                    discovery_type: session.discovery_type.clone(),
                    run_type: RunType::Historical {
                        results: session.clone(),
                    },
                },
            };

            // Save to database
            if let Err(e) = self.discovery_storage.create(&historical_discovery).await {
                tracing::error!(
                    "Failed to create historical discovery record for session {}: {}",
                    session.session_id,
                    e
                );
            } else {
                self.event_bus()
                    .publish_entity(EntityEvent {
                        id: Uuid::new_v4(),
                        entity_id: historical_discovery.id(),
                        network_id: self.get_network_id(&historical_discovery),
                        organization_id: self.get_organization_id(&historical_discovery),
                        entity_type: historical_discovery.into(),
                        operation: EntityOperation::Created,
                        timestamp: Utc::now(),
                        metadata: serde_json::json!({}),
                        authentication: AuthenticatedEntity::System,
                    })
                    .await?;
            }

            // Get next session info BEFORE trying to send request
            let next_session_info = if let Some(daemon_sessions) = self
                .daemon_sessions
                .write()
                .await
                .get_mut(&session.daemon_id)
            {
                daemon_sessions.retain(|s| *s != session.session_id);

                // Get info about next session if it exists
                daemon_sessions
                    .first()
                    .and_then(|next_session_id| sessions.get_mut(next_session_id))
                    .map(|next_session| {
                        next_session.phase = DiscoveryPhase::Pending;
                        (next_session.discovery_type.clone(), next_session.session_id)
                    })
            } else {
                None
            };

            // Remove the completed session
            sessions.remove(&update.session_id);

            // Drop the sessions lock before sending the request
            drop(sessions);

            // Publish event which will trigger notifying any daemons in ServerPoll to start session
            // If daemon is daemon_poll mode, it will request next session on its next poll
            if let Some((discovery_type, session_id)) = next_session_info {
                let started_payload =
                    DiscoveryUpdatePayload::new(session_id, daemon_id, network_id, discovery_type);

                self.event_bus()
                    .publish_discovery(started_payload.into_discovery_event())
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn cancel_session(
        &self,
        session_id: Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), Error> {
        // Get the session
        let session = match self.get_session(&session_id).await {
            Some(session) => session,
            None => {
                return Err(anyhow!("Session '{}' not found", session_id));
            }
        };

        let network_id = session.network_id;
        let daemon_id = session.daemon_id;
        let phase = session.phase;

        let cancelled_update = DiscoveryUpdatePayload {
            session_id,
            network_id,
            daemon_id,
            phase: DiscoveryPhase::Cancelled,
            progress: 0,
            error: None,
            started_at: session.started_at,
            finished_at: Some(Utc::now()),
            discovery_type: session.discovery_type,
        };

        // Handle based on current phase
        match phase {
            // Pending sessions: just remove from queue
            DiscoveryPhase::Pending => {
                let mut sessions = self.sessions.write().await;
                let mut daemon_sessions = self.daemon_sessions.write().await;

                // Remove from sessions map
                sessions.remove(&session_id);

                // Remove from daemon queue
                if let Some(queue) = daemon_sessions.get_mut(&daemon_id) {
                    queue.retain(|id| *id != session_id);
                }

                drop(sessions);
                drop(daemon_sessions);

                // Broadcast cancellation update so frontend knows
                let _ = self.update_tx.send(cancelled_update);

                tracing::info!("Cancelled pending session {} from queue", session_id);
                Ok(())
            }

            // Starting phase: wait briefly then retry
            DiscoveryPhase::Starting => Err(anyhow!(
                "Session is starting on daemon. Please try again in a moment."
            )),

            // Active phases: send cancellation to daemon
            // We do BOTH actions to support both daemon modes:
            // 1. Publish DiscoveryCancelled event - DaemonService subscriber handles ServerPoll mode
            // 2. Set cancellation flag - DaemonPoll mode checks on next poll via request_work
            DiscoveryPhase::Started | DiscoveryPhase::Scanning => {
                self.event_bus()
                    .publish_discovery(
                        cancelled_update.into_discovery_event_with_auth(authentication),
                    )
                    .await?;

                // Set cancellation flag for DaemonPoll mode (checked on next poll)
                self.daemon_pull_cancellations
                    .write()
                    .await
                    .insert(daemon_id, (true, session_id));

                tracing::info!(
                    daemon_id = %daemon_id,
                    session_id = %session_id,
                    "Discovery cancellation requested",
                );

                Ok(())
            }

            // Terminal phases: already done
            DiscoveryPhase::Complete | DiscoveryPhase::Failed | DiscoveryPhase::Cancelled => {
                tracing::info!(
                    "Session {} is already in terminal state: {}, nothing to cancel",
                    session_id,
                    phase
                );
                Ok(())
            }
        }
    }

    pub async fn cleanup_old_sessions(&self, max_age_hours: i64) {
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);
        let mut sessions = self.sessions.write().await;
        let mut daemon_sessions = self.daemon_sessions.write().await;
        let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;

        let mut to_remove = Vec::new();
        for (session_id, session) in sessions.iter() {
            if let Some(finished_at) = session.finished_at
                && finished_at < cutoff
            {
                to_remove.push(*session_id);
            }
        }

        for session_id in to_remove {
            if let Some(session) = sessions.remove(&session_id) {
                daemon_pull_cancellations.remove(&session.daemon_id);

                if let Some(daemon_sessions) = daemon_sessions.get_mut(&session.daemon_id) {
                    daemon_sessions.retain(|s| *s != session.session_id);
                }

                tracing::debug!("Cleaned up old discovery session {}", session_id);
            }
        }
    }

    /// Cleanup stalled sessions (called periodically from background task)
    pub async fn cleanup_stalled_sessions(&self) {
        let now = Utc::now();
        let stall_threshold = chrono::Duration::minutes(5);

        // First pass: identify stalled sessions (read locks only)
        let stalled_sessions: Vec<DiscoveryUpdatePayload> = {
            let sessions = self.sessions.read().await;
            let last_updated = self.session_last_updated.read().await;

            sessions
                .iter()
                .filter_map(|(session_id, session)| {
                    // Skip terminal states
                    if session.phase.is_terminal() {
                        return None;
                    }

                    // Check last update time
                    let is_stalled = if let Some(last_update_time) = last_updated.get(session_id) {
                        now.signed_duration_since(*last_update_time) > stall_threshold
                    } else if let Some(started_at) = session.started_at {
                        now.signed_duration_since(started_at) > stall_threshold
                    } else {
                        false
                    };

                    if is_stalled {
                        Some(session.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        if stalled_sessions.is_empty() {
            return;
        }

        // Second pass: request cancellation for stalled sessions (no locks held)
        // We do BOTH actions to support both daemon modes:
        // 1. Publish DiscoveryCancelled event - DaemonService subscriber handles ServerPoll mode
        // 2. Set cancellation flag - DaemonPoll mode checks on next poll via request_work
        for session in &stalled_sessions {
            let daemon_id = session.daemon_id;
            let session_id = session.session_id;

            tracing::warn!(
                session_id = %session_id,
                daemon_id = %daemon_id,
                "Requesting cancellation for stalled session"
            );

            let cancelled_update = DiscoveryUpdatePayload {
                session_id,
                network_id: session.network_id,
                daemon_id,
                phase: DiscoveryPhase::Cancelled,
                progress: session.progress,
                error: None,
                started_at: session.started_at,
                finished_at: Some(Utc::now()),
                discovery_type: session.discovery_type.clone(),
            };

            if let Err(e) = self
                .event_bus()
                .publish_discovery(cancelled_update.into_discovery_event())
                .await
            {
                tracing::warn!(
                    daemon_id = %session.daemon_id,
                    session_id = %session.session_id,
                    error = %e,
                    "Failed to publish cancellation event for stalled session"
                );
            }

            // Set cancellation flag for DaemonPoll mode (checked on next poll)
            self.daemon_pull_cancellations
                .write()
                .await
                .insert(daemon_id, (true, session_id));

            tracing::info!(
                daemon_id = %daemon_id,
                session_id = %session_id,
                "Cancellation requested for stalled session"
            );
        }

        // Third pass: cleanup session state (write locks)
        let mut sessions = self.sessions.write().await;
        let mut last_updated = self.session_last_updated.write().await;
        let mut daemon_sessions = self.daemon_sessions.write().await;
        let mut daemon_pull_cancellations = self.daemon_pull_cancellations.write().await;

        let mut stalled_count = 0;

        for session in stalled_sessions {
            if let Some(mut session) = sessions.remove(&session.session_id) {
                let daemon_id = session.daemon_id;
                let session_id = session.session_id;

                tracing::warn!(
                    session_id = %session_id,
                    daemon_id = %daemon_id,
                    phase = ?session.phase,
                    "Cleaning up stalled discovery session (no updates for 5+ minutes)"
                );

                // Update to failed state
                session.phase = DiscoveryPhase::Failed;
                session.error = Some(
                    "Session stalled - no updates received from daemon for more than 5 minutes"
                        .to_string(),
                );
                session.finished_at = Some(now);

                // Remove from daemon sessions queue
                if let Some(queue) = daemon_sessions.get_mut(&daemon_id) {
                    queue.retain(|id| *id != session_id);
                }

                // Remove from last_updated tracking
                last_updated.remove(&session_id);

                // Broadcast the failed state update
                let _ = self.update_tx.send(session.clone());

                // Clean up any pending cancellation for this daemon/session
                if let Some((_, cancel_session_id)) = daemon_pull_cancellations.get(&daemon_id)
                    && *cancel_session_id == session_id
                {
                    daemon_pull_cancellations.remove(&daemon_id);
                    tracing::debug!(
                        "Removed stale cancellation flag for daemon {} session {}",
                        daemon_id,
                        session_id
                    );
                }

                // Create historical discovery record for the stalled session
                let historical_discovery = Discovery {
                    id: Uuid::new_v4(),
                    created_at: session.started_at.unwrap_or(now),
                    updated_at: now,
                    base: crate::server::discovery::r#impl::base::DiscoveryBase {
                        daemon_id: session.daemon_id,
                        network_id: session.network_id,
                        tags: Vec::new(),
                        name: "Discovery Run (Stalled)".to_string(),
                        discovery_type: session.discovery_type.clone(),
                        run_type: RunType::Historical { results: session },
                    },
                };

                if let Err(e) = self.discovery_storage.create(&historical_discovery).await {
                    tracing::error!(
                        "Failed to create historical discovery record for stalled session {}: {}",
                        session_id,
                        e
                    );
                }

                stalled_count += 1;
            }
        }

        // Evict tombstones: last_updated entries for sessions that no longer exist
        // in the sessions map and are older than the stall threshold. These are left
        // behind after terminal processing to guard against redundant polls from old
        // daemons (see update_session). Safe to clean up once enough time has passed.
        last_updated.retain(|id, ts| {
            sessions.contains_key(id) || now.signed_duration_since(*ts) < stall_threshold
        });

        if stalled_count > 0 {
            tracing::info!("Cleaned up {} stalled discovery sessions", stalled_count);
        }
    }
}
