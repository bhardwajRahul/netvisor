//! ServerPoll polling loop and per-daemon poll state machine.
use super::*;

impl DaemonService {
    // ========================================================================
    // Polling loop methods (moved from poller.rs)
    // ========================================================================

    /// Start the ServerPoll polling loop. Should be called once from main.
    pub async fn start_polling_loop(self: Arc<Self>, email_service: Option<Arc<EmailService>>) {
        let poll_interval = Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS);
        let mut interval = tokio::time::interval(poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;

            if let Err(e) = self.poll_all_daemons(email_service.as_deref()).await {
                tracing::warn!("Daemon poller cycle failed: {}", e);
            }
        }
    }

    /// Poll all ServerPoll-mode daemons in parallel with semaphore-limited concurrency.
    /// Uses backon for per-daemon retries - daemon is marked unreachable after exhausting retries.
    async fn poll_all_daemons(&self, email_service: Option<&EmailService>) -> Result<()> {
        let daemons = self.get_server_poll_daemons().await?;

        if daemons.is_empty() {
            tracing::trace!("No ServerPoll daemons to poll");
            return Ok(());
        }

        tracing::debug!(
            "Polling {} ServerPoll daemons in parallel (max concurrent: {})",
            daemons.len(),
            MAX_CONCURRENT_POLLS
        );

        // Create parallel poll futures with semaphore-limited concurrency
        // Each daemon poll uses backon internally for retries
        let poll_futures: Vec<_> = daemons
            .into_iter()
            .map(|daemon| {
                let sem = self.poll_semaphore.clone();
                let daemon_id = daemon.id;
                let daemon_name = daemon.base.name.clone();
                async move {
                    let _permit = sem.acquire().await.expect("Semaphore closed");
                    // poll_daemon handles retries internally via backon
                    // Errors are logged inside poll_daemon, but log unexpected ones here too
                    if let Err(e) = self.poll_daemon(&daemon, email_service).await {
                        tracing::debug!(
                            daemon_id = %daemon_id,
                            daemon_name = %daemon_name,
                            error = %e,
                            "Poll cycle failed for daemon"
                        );
                    }
                }
            })
            .collect();

        // Execute all polls in parallel
        join_all(poll_futures).await;

        Ok(())
    }

    /// Get all daemons in ServerPoll mode that are reachable
    async fn get_server_poll_daemons(&self) -> Result<Vec<Daemon>> {
        let filter = StorableFilter::<Daemon>::new_for_daemon_poller_system_job();

        let reachable_server_poll_daemons = self.get_all(filter).await?;

        Ok(reachable_server_poll_daemons)
    }

    /// Mark a daemon as unreachable in the database and send notification
    async fn mark_daemon_unreachable(
        &self,
        daemon_id: Uuid,
        email_service: Option<&EmailService>,
    ) -> Result<()> {
        let mut daemon = self
            .get_by_id(&daemon_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Daemon {} not found", daemon_id))?;

        // Only notify if daemon was previously connected (skip if never seen = still setting up)
        let should_notify = daemon.base.last_seen.is_some();

        daemon.base.is_unreachable = true;

        self.update(&mut daemon, AuthenticatedEntity::System)
            .await?;

        if should_notify
            && let Some(email_service) = email_service
            && let Err(e) = self
                .send_unreachable_notification(&daemon, email_service)
                .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = %e,
                "Failed to send daemon unreachable notification email"
            );
        }

        Ok(())
    }

    /// Poll a single daemon for status and discovery data.
    /// Uses backon for retry with exponential backoff.
    /// Marks daemon unreachable after UNREACHABLE_THRESHOLD failures.
    ///
    /// Legacy daemons (< v0.14.0) are skipped entirely - they don't support
    /// the polling endpoints (/api/status, /api/poll, /api/first-contact,
    /// /api/discovery/entities-created). Legacy daemons stay alive via their
    /// own heartbeat calls to the server's backward-compat endpoint.
    async fn poll_daemon(
        &self,
        daemon: &Daemon,
        email_service: Option<&EmailService>,
    ) -> Result<()> {
        Self::warn_if_insecure_daemon_url(&daemon.base.url);

        // Skip polling for legacy daemons - they don't have the new endpoints
        if !daemon.supports_full_server_poll() {
            tracing::debug!(
                daemon_id = %daemon.id,
                daemon_name = %daemon.base.name,
                version = ?daemon.base.version,
                "Skipping poll for legacy daemon (< v0.14.0) - polling endpoints not supported"
            );
            return Ok(());
        }

        tracing::debug!(
            daemon_id = %daemon.id,
            daemon_name = %daemon.base.name,
            daemon_url = %daemon.base.url,
            api_key_id = ?daemon.base.api_key_id,
            "Starting poll for daemon"
        );

        // Get the API key for this daemon
        let api_key = match self.get_daemon_api_key(daemon).await {
            Ok(key) => key,
            Err(e) => {
                // API key lookup failure is a configuration error, not a network error.
                // Log it clearly so the user can fix it.
                tracing::error!(
                    daemon_id = %daemon.id,
                    daemon_name = %daemon.base.name,
                    error = %e,
                    "Failed to get API key for daemon - check that daemon has api_key_id set and the key has plaintext stored"
                );
                return Err(e);
            }
        };

        // Check if this is first contact (last_seen was None)
        let is_first_contact = daemon.base.last_seen.is_none();

        // Get status - either via first contact (which assigns daemon ID) or regular poll
        let status = if is_first_contact {
            tracing::info!(
                daemon_id = %daemon.id,
                daemon_name = %daemon.base.name,
                "First contact with ServerPoll daemon - assigning ID"
            );

            // Send first contact to assign daemon its server-side ID
            // This must succeed before we can proceed - without the correct ID,
            // discovery updates from the daemon won't be recognized by the server
            match self.send_first_contact(daemon, &api_key).await {
                Ok(status) => status,
                Err(e) => {
                    // First contact failed - abort poll entirely
                    // No point continuing since discovery updates won't work without correct ID
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        daemon_name = %daemon.base.name,
                        error = %e,
                        "First contact failed - aborting poll (will retry next cycle)"
                    );
                    // Mark unreachable after threshold failures
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        daemon_name = %daemon.base.name,
                        "Marking daemon unreachable after {} failures",
                        UNREACHABLE_THRESHOLD
                    );
                    if let Err(mark_err) =
                        self.mark_daemon_unreachable(daemon.id, email_service).await
                    {
                        tracing::error!(
                            daemon_id = %daemon.id,
                            "Failed to mark daemon as unreachable: {}",
                            mark_err
                        );
                    }
                    return Err(e);
                }
            }
        } else {
            // Regular status poll (retry is built into the helper)
            match self.poll_status(daemon, &api_key).await {
                Ok(status) => status,
                Err(e) => {
                    // Backon exhausted retries - mark daemon unreachable
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        daemon_name = %daemon.base.name,
                        "Marking daemon unreachable after {} failures",
                        UNREACHABLE_THRESHOLD
                    );
                    if let Err(mark_err) =
                        self.mark_daemon_unreachable(daemon.id, email_service).await
                    {
                        tracing::error!(
                            daemon_id = %daemon.id,
                            "Failed to mark daemon as unreachable: {}",
                            mark_err
                        );
                    }
                    return Err(e);
                }
            }
        };

        let auth = AuthenticatedEntity::System;

        tracing::debug!(
            daemon_id = %daemon.id,
            daemon_name = %daemon.base.name,
            ready_for_work = status.ready_for_work,
            interfaced_subnet_count = status.interfaced_subnets.len(),
            "ServerPoll status received"
        );

        // Process status data
        if let Err(e) = self
            .process_status(daemon.id, status.clone(), auth.clone())
            .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = ?e,
                "Failed to process daemon status"
            );
        }

        // If daemon has a version and it's different from what we have, process startup
        // (process_startup handles migration from legacy to unified discovery internally)
        if let Some(version) = status.version.clone()
            && daemon.base.version.as_ref() != Some(&version)
            && let Err(e) = self
                .process_startup(daemon.id, version.clone(), auth.clone())
                .await
        {
            tracing::warn!(
                daemon_id = %daemon.id,
                error = ?e,
                "Failed to process daemon startup"
            );
        }

        // Capabilities are now resolved inside process_status() — no separate call needed.

        // First contact - create default discovery jobs and emit telemetry
        if is_first_contact {
            tracing::info!(
                daemon_id = %daemon.id,
                daemon_name = %daemon.base.name,
                "First contact with ServerPoll daemon"
            );

            // Determine if org is on Free plan for discovery defaults
            let is_free_plan = if let Ok(Some(network)) = self
                .network_service
                .get_by_id(&daemon.base.network_id)
                .await
            {
                self.organization_service
                    .get_by_id(&network.base.organization_id)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|o| o.base.plan)
                    .map(|p| p.is_free())
                    .unwrap_or(true)
            } else {
                false
            };

            // Create default discovery jobs. ServerPoll first-contact carries no init-command
            // targeting (admin-provisioned via status, not the register endpoint), so targeting
            // is empty here — a future server-held daemon config would populate it.
            if let Err(e) = self
                .create_default_discovery_jobs(
                    daemon.id,
                    daemon.base.network_id,
                    daemon.base.host_id,
                    is_free_plan,
                    &[],
                )
                .await
            {
                tracing::warn!(
                    daemon_id = %daemon.id,
                    error = ?e,
                    "Failed to create default discovery jobs"
                );
            }

            // Emit telemetry
            if let Err(e) = self
                .emit_first_daemon_telemetry(daemon.id, daemon.base.network_id)
                .await
            {
                tracing::warn!(
                    daemon_id = %daemon.id,
                    error = ?e,
                    "Failed to emit first daemon telemetry"
                );
            }
        }

        // Poll discovery data
        match self.poll_discovery(daemon, &api_key).await {
            Ok(poll_response) => {
                let auth = AuthenticatedEntity::System;

                // Process progress update if available
                if let Some(progress) = poll_response.progress
                    && let Err(e) = self.process_discovery_progress(progress).await
                {
                    tracing::warn!(
                        daemon_id = %daemon.id,
                        error = ?e,
                        "Failed to process discovery progress"
                    );
                }

                // Process entities if any
                if !poll_response.entities.is_empty() {
                    match self
                        .process_discovery_entities(poll_response.entities, auth.clone())
                        .await
                    {
                        Ok(created_entities) => {
                            // Send created entities back to daemon
                            if let Err(e) = self
                                .send_created_entities(daemon, &api_key, created_entities)
                                .await
                            {
                                tracing::warn!(
                                    daemon_id = %daemon.id,
                                    "Failed to send created entities to daemon: {}",
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                daemon_id = %daemon.id,
                                error = ?e,
                                "Failed to process discovery entities"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::debug!(
                    daemon_id = %daemon.id,
                    "Failed to poll daemon discovery: {}",
                    e
                );
            }
        }

        // Check for pending work and initiate if daemon reports ready
        if status.ready_for_work
            && let Some(work) = self.get_pending_work(daemon.id).await
        {
            let integration_targets = self
                .discovery_service
                .get_integration_targets_for_session(&work.session_id)
                .await;
            let request = self
                .discovery_service
                .build_daemon_request(&work, work.network_id, &integration_targets)
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to build daemon request: {}", e);
                    DaemonDiscoveryRequest {
                        session_id: work.session_id,
                        discovery_id: work.discovery_id.unwrap_or_default(),
                        discovery_type: work.discovery_type,
                        credential_mappings: vec![],
                    }
                });
            if let Err(e) = self
                .send_discovery_request_to_daemon(daemon, Some(&api_key), request)
                .await
            {
                tracing::warn!(
                    daemon_id = %daemon.id,
                    "Failed to initiate discovery: {}",
                    e
                );
            }
        }

        Ok(())
    }

    /// Get the API key for a daemon (from the linked api_key_id)
    pub async fn get_daemon_api_key(&self, daemon: &Daemon) -> Result<String> {
        let api_key_id = daemon
            .base
            .api_key_id
            .ok_or_else(|| anyhow::anyhow!("Daemon {} has no linked API key", daemon.id))?;

        let api_key = self
            .daemon_api_key_service
            .get_by_id(&api_key_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("API key {} not found", api_key_id))?;

        // Get the plaintext key (stored for ServerPoll daemons)
        api_key
            .base
            .plaintext
            .as_ref()
            .map(|s| s.expose_secret().to_string())
            .ok_or_else(|| anyhow::anyhow!("API key {} has no stored plaintext", api_key_id))
    }
}
