use crate::daemon::discovery::manager::DaemonDiscoverySessionManager;
use crate::daemon::runtime::state::DaemonStatus;
use crate::daemon::shared::api_client::DaemonApiClient;
use crate::daemon::shared::config::ConfigStore;
use crate::daemon::utils::base::DaemonUtils;
use crate::daemon::utils::base::{PlatformDaemonUtils, create_system_utils};
use crate::server::daemons::r#impl::api::{
    DaemonCapabilities, DaemonDiscoveryRequest, DaemonRegistrationRequest,
    DaemonRegistrationResponse, DaemonStartupRequest, ServerCapabilities,
};
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse};
use crate::server::shared::types::error_codes::ErrorCode;
use anyhow::Result;
use backon::{ExponentialBuilder, Retryable};

/// Outcome of daemon startup initialization
pub enum StartupOutcome {
    /// Successfully connected and announced/registered
    Ok,
    /// Connection failed (timeout, refused, DNS) — retryable
    ConnectionFailed(anyhow::Error),
    /// Auth failed (invalid API key) — fatal, don't poll
    AuthFailed(anyhow::Error),
}
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Number of heartbeats between health summary logs (at 30s interval = ~5 minutes)
const HEALTH_LOG_INTERVAL: u64 = 10;

/// Log target for consistent daemon logging output
pub const LOG_TARGET: &str = "daemon";

/// Format a duration as human-readable uptime (e.g., "1h 23m", "45m", "2d 5h")
fn format_uptime(duration: Duration) -> String {
    let secs = duration.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins.max(1)) // Show at least 1m
    }
}

pub struct DaemonRuntimeService {
    pub config: Arc<ConfigStore>,
    pub api_client: Arc<DaemonApiClient>,
    pub utils: PlatformDaemonUtils,
    pub discovery_manager: Arc<DaemonDiscoverySessionManager>,
}

impl DaemonRuntimeService {
    pub fn new(
        config_store: Arc<ConfigStore>,
        discovery_manager: Arc<DaemonDiscoverySessionManager>,
    ) -> Self {
        Self {
            config: config_store.clone(),
            api_client: Arc::new(DaemonApiClient::new(config_store)),
            utils: create_system_utils(),
            discovery_manager,
        }
    }

    /// Check if an error indicates the API key is no longer valid (rotated/revoked).
    /// Returns Some(error) if authorization failed and the daemon should stop, None otherwise.
    fn check_authorization_error(error: &anyhow::Error, daemon_id: &Uuid) -> Option<anyhow::Error> {
        if let Some(api_err) = error.downcast_ref::<ApiErrorResponse>()
            && (api_err.matches_error(&ApiError::daemon_api_key_expired())
                || api_err.matches_error(&ApiError::daemon_api_key_disabled()))
        {
            tracing::error!(
                daemon_id = %daemon_id,
                "API key is no longer valid. The key may have been rotated or revoked. \
                 Please reconfigure the daemon with a valid API key."
            );
            return Some(anyhow::anyhow!(
                "Daemon authorization failed: API key is no longer valid"
            ));
        }
        None
    }

    /// Check if an error indicates the daemon record doesn't exist on the server.
    /// This can happen if the server's database was reset or the daemon was deleted.
    fn is_daemon_not_found_error(error: &anyhow::Error, daemon_id: &Uuid) -> bool {
        error
            .downcast_ref::<ApiErrorResponse>()
            .is_some_and(|e| e.matches_error(&ApiError::entity_not_found::<Daemon>(daemon_id)))
    }

    /// Check if an error indicates an authorization failure where the daemon is registered
    /// but the API key is invalid/revoked. Should fail immediately with a clear message.
    fn is_registered_daemon_auth_error(error: &anyhow::Error) -> bool {
        error
            .downcast_ref::<ApiErrorResponse>()
            .is_some_and(|e| e.matches_error(&ApiError::not_authenticated()))
    }

    /// Check if an error indicates an authorization failure for an unregistered daemon.
    /// This happens during onboarding when the API key isn't active yet in the database.
    fn is_unregistered_auth_error(error: &anyhow::Error) -> bool {
        error
            .downcast_ref::<ApiErrorResponse>()
            .is_some_and(|e| e.matches_error(&ApiError::daemon_key_not_yet_active()))
    }

    /// Maximum consecutive poll failures before falling back to outer retry loop
    const MAX_POLL_RETRIES: usize = 5;

    pub async fn request_work(&self) -> Result<()> {
        let interval_secs = self.config.get_heartbeat_interval().await?;
        let interval = Duration::from_secs(interval_secs);
        let daemon_id = self.config.get_id().await?;
        let name = self.config.get_name().await?;
        let mode = self.config.get_mode().await?;

        let mut interval_timer = tokio::time::interval(interval);
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut poll_count: u64 = 0;
        let start_time = std::time::Instant::now();

        loop {
            interval_timer.tick().await;

            if self.config.get_network_id().await?.is_none() {
                tracing::warn!(target: LOG_TARGET, "Work request skipped - network_id not configured");
                continue;
            }

            poll_count += 1;
            tracing::debug!(target: LOG_TARGET, daemon_id = %daemon_id, "Polling server for work");

            let path = format!("/api/daemons/{}/request-work", daemon_id);
            // Detect ip_addresses fresh — cheap NIC enumeration
            let interfaced_subnets = self.detect_interfaced_subnets().await.unwrap_or_default();

            let status_payload = DaemonStatus {
                // URL not sent - server manages this via provisioning
                url: None,
                name: name.clone(),
                mode,
                version: Some(semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()),
                capabilities: DaemonCapabilities::default(),
                interfaced_subnets,
                ready_for_work: !self.discovery_manager.is_discovery_running().await,
            };

            // Use backon for retry with exponential backoff
            let result = (|| async {
                self.api_client
                    .post::<_, (Option<DaemonDiscoveryRequest>, bool)>(
                        &path,
                        &status_payload,
                        "Failed to request work",
                    )
                    .await
            })
            .retry(
                ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(30))
                    .with_max_times(Self::MAX_POLL_RETRIES),
            )
            .when(|e| {
                // Don't retry auth errors - exit immediately
                Self::check_authorization_error(e, &daemon_id).is_none()
                    // Don't retry API errors (structured responses from server)
                    && e.downcast_ref::<ApiErrorResponse>().is_none()
            })
            .notify(|e, dur| {
                tracing::warn!(
                    target: LOG_TARGET,
                    "Server unreachable, retrying in {:.0?}... ({})",
                    dur,
                    e
                );
            })
            .await;

            match result {
                Ok((request, cancel_current_session)) => {
                    if cancel_current_session {
                        tracing::info!(target: LOG_TARGET, "Received cancellation request from server");
                        self.discovery_manager.cancel_current_session().await;
                    }

                    if let Some(request) = request {
                        tracing::info!(
                            target: LOG_TARGET,
                            "Discovery session received: {} ({:?})",
                            request.session_id,
                            request.discovery_type
                        );
                        self.discovery_manager.initiate_session(request).await;
                    }
                }
                Err(e) => {
                    // Check if daemon record was deleted or DB was reset
                    if let Some(api_err) = e.downcast_ref::<ApiErrorResponse>()
                        && api_err.matches_error(&ApiError::coded(
                            axum::http::StatusCode::NOT_FOUND,
                            ErrorCode::DaemonNotRegistered,
                        ))
                    {
                        tracing::error!(
                            target: LOG_TARGET,
                            daemon_id = %daemon_id,
                            "Daemon not found on server — deleted or database was reset. \
                             Entering standby. Reinstall or reconfigure the daemon to resume. \
                             Waiting for shutdown signal (Ctrl+C)..."
                        );
                        tokio::signal::ctrl_c().await.ok();
                        return Err(anyhow::anyhow!("Daemon not registered — shutting down"));
                    }

                    // Check if daemon has been put on standby (inactivity)
                    if let Some(api_err) = e.downcast_ref::<ApiErrorResponse>()
                        && api_err.matches_error(&ApiError::coded(
                            axum::http::StatusCode::FORBIDDEN,
                            ErrorCode::DaemonStandby,
                        ))
                    {
                        tracing::warn!(
                            target: LOG_TARGET,
                            "Daemon is on standby due to inactivity (no completed discovery in 30 days). \
                             To end standby, restart the daemon or queue a discovery from the UI. \
                             Waiting for shutdown signal (Ctrl+C)..."
                        );
                        tokio::signal::ctrl_c().await.ok();
                        return Err(anyhow::anyhow!("Daemon on standby — shutting down"));
                    }

                    if let Some(auth_error) = Self::check_authorization_error(&e, &daemon_id) {
                        return Err(auth_error);
                    }
                    // Backon exhausted retries - exit the daemon
                    let server_url = self.config.get_server_url().await.unwrap_or_default();
                    tracing::error!(
                        target: LOG_TARGET,
                        "Lost connection to server at {} after {} retries: {}. Check that the server is running and reachable.",
                        server_url,
                        Self::MAX_POLL_RETRIES,
                        e
                    );
                    return Err(anyhow::anyhow!(
                        "Lost connection to server at {}",
                        server_url
                    ));
                }
            }

            // Periodic health summary
            if poll_count.is_multiple_of(HEALTH_LOG_INTERVAL) {
                let uptime = start_time.elapsed();
                let uptime_str = format_uptime(uptime);
                let discovery_active = self.discovery_manager.is_discovery_running().await;

                tracing::info!(
                    target: LOG_TARGET,
                    "Health: OK | Uptime: {} | Polls: {} | Discovery: {}",
                    uptime_str,
                    poll_count,
                    if discovery_active { "active" } else { "idle" }
                );
            }
        }
    }

    /// Detect subnets from daemon's network ip_addresses.
    async fn detect_interfaced_subnets(
        &self,
    ) -> Result<Vec<crate::server::subnets::r#impl::base::Subnet>> {
        let network_id = match self.config.get_network_id().await? {
            Some(id) => id,
            None => return Ok(Vec::new()),
        };
        let interface_filter = self.config.get_interfaces().await?;

        let (_, subnets, _) = self
            .utils
            .get_own_interfaces(network_id, &interface_filter)
            .await?;

        Ok(subnets)
    }

    pub async fn initialize_services(
        &self,
        network_id: Uuid,
        api_key: String,
    ) -> Result<StartupOutcome> {
        self.config.set_network_id(network_id).await?;
        self.config.set_api_key(api_key).await?;

        let daemon_id = self.config.get_id().await?;

        match self.announce_startup(daemon_id).await {
            Ok(_) => {
                tracing::info!(target: LOG_TARGET, "  Status:          Connected");
                // A successful startup announcement means this daemon is already registered (i.e.
                // a restart, not first launch). Init-command credentials (`--credential-id`) are
                // applied only at first registration, so warn if they were passed again.
                if self
                    .config
                    .get_integration_targets()
                    .await
                    .map(|t| !t.is_empty())
                    .unwrap_or(false)
                {
                    tracing::warn!(
                        target: LOG_TARGET,
                        "Init-command credentials (--credential-id) are applied only at first \
                         registration; this daemon is already registered. Manage its credentials \
                         in the Scanopy UI instead."
                    );
                }
                return Ok(StartupOutcome::Ok);
            }
            Err(e) if Self::is_daemon_not_found_error(&e, &daemon_id) => {
                tracing::info!(target: LOG_TARGET, "  Status:          Daemon not yet registered; beginning registration");
            }
            Err(e)
                if Self::is_registered_daemon_auth_error(&e)
                    || Self::is_unregistered_auth_error(&e) =>
            {
                return Ok(StartupOutcome::AuthFailed(e));
            }
            Err(e) => {
                return Ok(StartupOutcome::ConnectionFailed(e));
            }
        }

        // ServerPoll daemons don't self-register - they're provisioned via the server UI
        // and wait for the server to poll them
        let mode = self.config.get_mode().await?;
        if mode == crate::server::daemons::r#impl::base::DaemonMode::ServerPoll {
            tracing::info!(
                target: LOG_TARGET,
                "  Status:          ServerPoll mode - skipping registration (daemon must be provisioned via server)"
            );
            return Ok(StartupOutcome::Ok);
        }

        match self.register_with_server(daemon_id, network_id).await {
            Ok(()) => Ok(StartupOutcome::Ok),
            Err(e) => Ok(StartupOutcome::ConnectionFailed(e)),
        }
    }

    // Helper function to get daemon url if override is being used, or fallback to default ip + port if not
    pub async fn get_daemon_url(&self) -> Result<String> {
        if let Some(daemon_url) = self.config.get_daemon_url().await? {
            Ok(daemon_url)
        } else {
            let bind_address = self.config.get_bind_address().await?;
            let daemon_ip = if bind_address == "0.0.0.0" || bind_address == "::" {
                self.utils.get_own_ip_address()?
            } else {
                bind_address.parse::<IpAddr>()?
            };
            let daemon_port = self.config.get_port().await?;
            Ok(format!("http://{}:{}", daemon_ip, daemon_port))
        }
    }

    /// Maximum number of registration retries (about 5 minutes with backoff)
    pub async fn register_with_server(&self, daemon_id: Uuid, network_id: Uuid) -> Result<()> {
        let config = self.api_client.config();
        let mode = config.get_mode().await?;
        let name = config.get_name().await?;
        let version = env!("CARGO_PKG_VERSION");

        let user_id = config.get_user_id().await?.unwrap_or(Uuid::nil());

        let integration_targets = config.get_integration_targets().await?;

        let registration_request = DaemonRegistrationRequest {
            daemon_id,
            network_id,
            // URL not sent - server manages this via provisioning for ServerPoll,
            // and doesn't need it for DaemonPoll
            url: None,
            name: name.clone(),
            mode,
            capabilities: DaemonCapabilities {
                interfaced_subnet_ids: Vec::new(),
            },
            user_id,
            version: Some(version.to_string()),
            integration_targets,
        };

        tracing::info!(target: LOG_TARGET, "Registering with server:");
        tracing::info!(target: LOG_TARGET, "  Daemon ID:       {}", daemon_id);
        tracing::info!(target: LOG_TARGET, "  Network ID:      {}", network_id);
        tracing::info!(target: LOG_TARGET, "  Version:         {}", version);

        let result = self
            .api_client
            .post::<_, DaemonRegistrationResponse>(
                "/api/daemons/register",
                &registration_request,
                "Registration failed",
            )
            .await;

        match result {
            Ok(response) => {
                tracing::info!(target: LOG_TARGET, "Registration successful");
                if let Some(caps) = response.server_capabilities {
                    tracing::info!(target: LOG_TARGET, "  Server version:  {}", caps.server_version);
                    tracing::info!(target: LOG_TARGET, "  Min daemon ver:  {}", caps.minimum_daemon_version);
                }
                Ok(())
            }
            Err(e) => Self::handle_registration_error(&e, daemon_id, &self.config).await,
        }
    }

    /// Handle registration errors with user-friendly messages
    async fn handle_registration_error(
        e: &anyhow::Error,
        daemon_id: Uuid,
        config: &Arc<ConfigStore>,
    ) -> Result<()> {
        // Check for API error responses first
        if let Some(api_err) = e.downcast_ref::<ApiErrorResponse>() {
            if api_err.matches_error(&ApiError::daemon_version_too_old("", "")) {
                tracing::error!(
                    target: LOG_TARGET,
                    daemon_id = %daemon_id,
                    "Daemon version is older than the server version. \
                     Please update the daemon binary to match the server. \
                     Download the latest version from the Scanopy UI under Discover > Daemons."
                );
                return Err(anyhow::anyhow!(
                    "Daemon version is older than server — update required"
                ));
            }
            if api_err.matches_error(&ApiError::daemon_key_not_yet_active()) {
                let server_url = config.get_server_url().await.unwrap_or_default();
                tracing::error!(
                    target: LOG_TARGET,
                    daemon_id = %daemon_id,
                    "API key rejected by server at {}. Re-run the install command from the Scanopy UI to generate a new key.",
                    server_url
                );
                return Err(anyhow::anyhow!("API key rejected by server"));
            }
            if api_err.matches_error(&ApiError::demo_mode_blocked()) {
                tracing::error!(
                    target: LOG_TARGET,
                    daemon_id = %daemon_id,
                    "This Scanopy instance is running in demo mode. Daemon registration is disabled."
                );
                return Err(anyhow::anyhow!(
                    "Demo mode: Daemon registration is disabled"
                ));
            }
        }

        // Connection errors still need string matching (not API responses)
        let err_str = e.to_string().to_lowercase();
        let server_url = config.get_server_url().await.unwrap_or_default();

        if err_str.contains("connection refused") {
            tracing::error!(
                target: LOG_TARGET,
                daemon_id = %daemon_id,
                server_url = %server_url,
                "Connection refused by server at {}",
                server_url
            );
            return Err(anyhow::anyhow!(
                "Connection refused by server at {}. Verify the server is running.",
                server_url
            ));
        }

        if err_str.contains("timeout") || err_str.contains("timed out") {
            tracing::error!(
                target: LOG_TARGET,
                daemon_id = %daemon_id,
                server_url = %server_url,
                "Connection timed out reaching server at {}",
                server_url
            );
            return Err(anyhow::anyhow!(
                "Connection timed out reaching server at {}",
                server_url
            ));
        }

        Err(anyhow::anyhow!("Registration failed: {}", e))
    }

    /// Announce daemon startup to the server.
    ///
    /// Called on every daemon boot (not just first registration) to:
    /// - Report daemon version to server
    /// - Receive server capabilities and deprecation warnings
    /// - Update last_seen timestamp
    pub async fn announce_startup(&self, daemon_id: Uuid) -> Result<()> {
        let path = format!("/api/daemons/{}/startup", daemon_id);

        let request = DaemonStartupRequest {
            daemon_version: semver::Version::parse(env!("CARGO_PKG_VERSION"))?,
        };

        let result: Result<ServerCapabilities, _> = self
            .api_client
            .post(&path, &request, "Startup announcement failed")
            .await;

        match result {
            Ok(capabilities) => {
                tracing::info!(target: LOG_TARGET, "  Server version:  {}", capabilities.server_version);
                tracing::info!(target: LOG_TARGET, "  Min daemon ver:  {}", capabilities.minimum_daemon_version);

                // Log any deprecation warnings from the server
                capabilities.log_warnings();

                Ok(())
            }
            Err(e) => {
                tracing::debug!(
                    target: LOG_TARGET,
                    daemon_id = %daemon_id,
                    error = %e,
                    "Startup announcement failed"
                );
                Err(e)
            }
        }
    }
}
