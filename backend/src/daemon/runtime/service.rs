use crate::daemon::discovery::manager::DaemonDiscoverySessionManager;
use crate::daemon::shared::api_client::DaemonApiClient;
use crate::daemon::shared::config::ConfigStore;
use crate::daemon::utils::base::DaemonUtils;
use crate::daemon::utils::base::{PlatformDaemonUtils, create_system_utils};
use crate::server::daemons::r#impl::api::{
    DaemonCapabilities, DaemonHeartbeatPayload, DaemonRegistrationRequest,
    DaemonRegistrationResponse, DaemonStartupRequest, DiscoveryUpdatePayload, ServerCapabilities,
};
use crate::server::daemons::r#impl::version::DeprecationSeverity;
use anyhow::Result;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Number of heartbeats between health summary logs (at 30s interval = ~5 minutes)
const HEALTH_LOG_INTERVAL: u64 = 10;

/// Log target for consistent daemon logging output
const LOG_TARGET: &str = "daemon";

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

    /// Check Docker availability and return a detailed description of the connection method.
    /// Returns (is_available, description) where description explains how Docker is being accessed.
    pub async fn check_docker_availability(&self) -> (bool, String) {
        let docker_proxy = self.config.get_docker_proxy().await;
        let docker_proxy_ssl_info = self.config.get_docker_proxy_ssl_info().await;

        // Determine connection method description
        let connection_method = match &docker_proxy {
            Ok(Some(proxy_url)) => {
                if proxy_url.starts_with("https://") {
                    format!("via SSL proxy at {}", proxy_url)
                } else {
                    format!("via HTTP proxy at {}", proxy_url)
                }
            }
            _ => {
                #[cfg(target_family = "unix")]
                {
                    "via local socket (/var/run/docker.sock)".to_string()
                }
                #[cfg(target_family = "windows")]
                {
                    "via named pipe (//./pipe/docker_engine)".to_string()
                }
            }
        };

        match self
            .utils
            .new_local_docker_client(docker_proxy, docker_proxy_ssl_info)
            .await
        {
            Ok(_) => (true, format!("Available {}", connection_method)),
            Err(e) => {
                let error_hint = if e.to_string().contains("No such file") {
                    " (socket not found - is Docker running?)"
                } else if e.to_string().contains("permission denied") {
                    " (permission denied - check user is in docker group)"
                } else if e.to_string().contains("connection refused") {
                    " (connection refused - is Docker daemon running?)"
                } else {
                    ""
                };
                (
                    false,
                    format!("Not available{} - container discovery disabled", error_hint),
                )
            }
        }
    }

    /// Check if an error indicates the API key is no longer valid (rotated/revoked).
    /// Returns Some(error) if authorization failed and the daemon should stop, None otherwise.
    fn check_authorization_error(error: &anyhow::Error, daemon_id: &Uuid) -> Option<anyhow::Error> {
        let error_str = error.to_string();
        if error_str.contains("Invalid API key") || error_str.contains("HTTP 401") {
            tracing::error!(
                daemon_id = %daemon_id,
                "API key is no longer valid. The key may have been rotated or revoked. \
                 Please reconfigure the daemon with a valid API key."
            );
            Some(anyhow::anyhow!(
                "Daemon authorization failed: API key is no longer valid"
            ))
        } else {
            None
        }
    }

    /// Check if an error indicates the daemon record doesn't exist on the server.
    /// This can happen if the server's database was reset or the daemon was deleted.
    fn is_daemon_not_found_error(error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        (error_str.contains("not found") && error_str.contains("daemon"))
            || (error_str.contains("http 404") && error_str.contains("daemon"))
    }

    pub async fn request_work(&self) -> Result<()> {
        let interval_secs = self.config.get_heartbeat_interval().await?;
        let interval = Duration::from_secs(interval_secs);
        let daemon_id = self.config.get_id().await?;
        let name = self.config.get_name().await?;
        let mode = self.config.get_mode().await?;
        let url = self.get_daemon_url().await?;

        let mut interval_timer = tokio::time::interval(interval);
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut poll_count: u64 = 0;
        let mut consecutive_failures: u64 = 0;
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
            let result: Result<(Option<DiscoveryUpdatePayload>, bool), _> = self
                .api_client
                .post(
                    &path,
                    &DaemonHeartbeatPayload {
                        url: url.clone(),
                        name: name.clone(),
                        mode,
                    },
                    "Failed to request work",
                )
                .await;

            match result {
                Ok((payload, cancel_current_session)) => {
                    consecutive_failures = 0;

                    if cancel_current_session {
                        tracing::info!(target: LOG_TARGET, "Received cancellation request from server");
                        self.discovery_manager.cancel_current_session().await;
                    }

                    if let Some(payload) = payload
                        && !self.discovery_manager.is_discovery_running().await
                    {
                        tracing::info!(
                            target: LOG_TARGET,
                            "Discovery session received: {} ({:?})",
                            payload.session_id,
                            payload.discovery_type
                        );
                        self.discovery_manager
                            .initiate_session(payload.into())
                            .await;
                    }
                }
                Err(e) => {
                    if let Some(auth_error) = Self::check_authorization_error(&e, &daemon_id) {
                        return Err(auth_error);
                    }
                    consecutive_failures += 1;
                    tracing::warn!(
                        target: LOG_TARGET,
                        "Failed to poll for work: {} (failure #{})",
                        e,
                        consecutive_failures
                    );
                }
            }

            // Periodic health summary
            if poll_count.is_multiple_of(HEALTH_LOG_INTERVAL) {
                let uptime = start_time.elapsed();
                let uptime_str = format_uptime(uptime);
                let discovery_active = self.discovery_manager.is_discovery_running().await;

                tracing::info!(
                    target: LOG_TARGET,
                    "Health: {} | Uptime: {} | Polls: {} | Discovery: {}",
                    if consecutive_failures == 0 {
                        "OK"
                    } else {
                        "DEGRADED"
                    },
                    uptime_str,
                    poll_count,
                    if discovery_active { "active" } else { "idle" }
                );
            }
        }
    }

    pub async fn heartbeat(&self) -> Result<()> {
        let interval = Duration::from_secs(self.config.get_heartbeat_interval().await?);
        let daemon_id = self.config.get_id().await?;
        let name = self.config.get_name().await?;
        let mode = self.config.get_mode().await?;
        let url = self.get_daemon_url().await?;

        let mut interval_timer = tokio::time::interval(interval);
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut heartbeat_count: u64 = 0;
        let mut consecutive_failures: u64 = 0;
        let start_time = std::time::Instant::now();

        loop {
            interval_timer.tick().await;

            if self.config.get_network_id().await?.is_none() {
                tracing::warn!(target: LOG_TARGET, "Heartbeat skipped - network_id not configured");
                continue;
            }

            heartbeat_count += 1;
            tracing::debug!(target: LOG_TARGET, daemon_id = %daemon_id, "Sending heartbeat");

            let path = format!("/api/daemons/{}/heartbeat", daemon_id);
            match self
                .api_client
                .post_no_data::<_>(
                    &path,
                    &DaemonHeartbeatPayload {
                        url: url.clone(),
                        name: name.clone(),
                        mode,
                    },
                    "Heartbeat failed",
                )
                .await
            {
                Ok(_) => {
                    consecutive_failures = 0;
                    if let Err(e) = self.config.update_heartbeat().await {
                        tracing::warn!(target: LOG_TARGET, "Failed to update heartbeat timestamp: {}", e);
                    }
                }
                Err(e) => {
                    if let Some(auth_error) = Self::check_authorization_error(&e, &daemon_id) {
                        return Err(auth_error);
                    }
                    consecutive_failures += 1;
                    tracing::warn!(
                        target: LOG_TARGET,
                        "Heartbeat failed: {} (failure #{})",
                        e,
                        consecutive_failures
                    );
                }
            }

            // Periodic health summary
            if heartbeat_count.is_multiple_of(HEALTH_LOG_INTERVAL) {
                let uptime = start_time.elapsed();
                let uptime_str = format_uptime(uptime);
                let discovery_active = self.discovery_manager.is_discovery_running().await;

                tracing::info!(
                    target: LOG_TARGET,
                    "Health: {} | Uptime: {} | Heartbeats: {} | Discovery: {}",
                    if consecutive_failures == 0 {
                        "OK"
                    } else {
                        "DEGRADED"
                    },
                    uptime_str,
                    heartbeat_count,
                    if discovery_active { "active" } else { "idle" }
                );
            }
        }
    }

    pub async fn initialize_services(&self, network_id: Uuid, api_key: String) -> Result<()> {
        self.config.set_network_id(network_id).await?;
        self.config.set_api_key(api_key).await?;

        let daemon_id = self.config.get_id().await?;

        // Check Docker availability with detailed description
        let (has_docker_client, docker_description) = self.check_docker_availability().await;
        tracing::info!(target: LOG_TARGET, "  Docker:          {}", docker_description);

        tracing::info!(target: LOG_TARGET, "Connecting to server...");

        match self.announce_startup(daemon_id).await {
            Ok(_) => {
                tracing::info!(target: LOG_TARGET, "  Status:          Daemon recognized, startup announced");
                return Ok(());
            }
            Err(e) if Self::is_daemon_not_found_error(&e) => {
                tracing::info!(target: LOG_TARGET, "  Status:          Daemon not registered, registering now");
            }
            Err(e) => {
                tracing::error!(target: LOG_TARGET, "  Status:          Failed to connect: {}", e);
                return Err(e);
            }
        }

        self.register_with_server(daemon_id, network_id, has_docker_client)
            .await?;

        Ok(())
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

    pub async fn register_with_server(
        &self,
        daemon_id: Uuid,
        network_id: Uuid,
        has_docker_socket: bool,
    ) -> Result<()> {
        let config = self.api_client.config();
        let mode = config.get_mode().await?;
        let name = config.get_name().await?;
        let version = env!("CARGO_PKG_VERSION");

        let url = self.get_daemon_url().await?;

        let user_id = config.get_user_id().await?.unwrap_or(Uuid::nil());

        let registration_request = DaemonRegistrationRequest {
            daemon_id,
            network_id,
            url: url.clone(),
            name: name.clone(),
            mode,
            capabilities: DaemonCapabilities {
                has_docker_socket,
                interfaced_subnet_ids: Vec::new(),
            },
            user_id,
            version: Some(version.to_string()),
        };

        tracing::info!(target: LOG_TARGET, "Registering with server:");
        tracing::info!(target: LOG_TARGET, "  Daemon ID:       {}", daemon_id);
        tracing::info!(target: LOG_TARGET, "  Network ID:      {}", network_id);
        tracing::info!(target: LOG_TARGET, "  Version:         {}", version);
        tracing::info!(
            target: LOG_TARGET,
            "  Capabilities:    docker={}, subnets=0 (updated after self-discovery)",
            if has_docker_socket { "yes" } else { "no" }
        );

        // Retry loop for handling pending API keys (pre-registration setup flow)
        // First attempt immediately, then wait 10s (user fills form), then exponential backoff: 1, 2, 4, 8...
        // Caps at heartbeat_interval
        let heartbeat_interval = config.get_heartbeat_interval().await?;
        let mut attempt = 0;

        loop {
            attempt += 1;

            let result: Result<DaemonRegistrationResponse, _> = self
                .api_client
                .post(
                    "/api/daemons/register",
                    &registration_request,
                    "Registration failed",
                )
                .await;

            match result {
                Ok(response) => {
                    // Note: host_id is not cached locally - the server provides it
                    // in discovery requests via DiscoveryType
                    tracing::info!(target: LOG_TARGET, "Registration successful");
                    if let Some(caps) = response.server_capabilities {
                        tracing::info!(target: LOG_TARGET, "  Server version:  {}", caps.server_version);
                        tracing::info!(target: LOG_TARGET, "  Min daemon ver:  {}", caps.minimum_daemon_version);
                    }
                    return Ok(());
                }
                Err(e) => {
                    let error_str = e.to_string();

                    // Check if this is a demo mode error - provide friendly message
                    if error_str.contains("demo mode") || error_str.contains("HTTP 403") {
                        tracing::error!(
                            target: LOG_TARGET,
                            daemon_id = %daemon_id,
                            "This Scanopy instance is running in demo mode. \
                             Daemon registration is disabled. \
                             To use daemons, please create an account."
                        );
                        return Err(anyhow::anyhow!(
                            "Demo mode: Daemon registration is disabled on this server"
                        ));
                    }

                    // Check if this is an "Invalid API key" error
                    // This can happen when daemon is installed before user completes registration
                    if error_str.contains("Invalid API key") || error_str.contains("HTTP 401") {
                        // Calculate retry delay:
                        // Attempt 1 failed -> wait 10s (user filling out registration form)
                        // Attempt 2 failed -> wait 1s
                        // Attempt 3 failed -> wait 2s
                        // Attempt 4 failed -> wait 4s, etc.
                        // Capped at heartbeat_interval
                        let retry_secs = if attempt == 1 {
                            10 // Initial wait for user to complete registration
                        } else {
                            // Exponential backoff: 1, 2, 4, 8, 16...
                            (1u64 << (attempt - 2)).min(heartbeat_interval)
                        };

                        tracing::warn!(
                            target: LOG_TARGET,
                            daemon_id = %daemon_id,
                            attempt = %attempt,
                            "API key not yet active. This daemon was likely installed before account \
                             registration was completed. Waiting for account creation... \
                             Retrying in {} seconds.",
                            retry_secs
                        );

                        tokio::time::sleep(Duration::from_secs(retry_secs)).await;
                        continue;
                    }

                    // Check for connection errors - provide helpful troubleshooting message
                    let error_lower = error_str.to_lowercase();
                    let server_url = config.get_server_url().await.unwrap_or_default();

                    // Connection refused - server not running or wrong address
                    if error_lower.contains("connection refused") {
                        tracing::error!(
                            target: LOG_TARGET,
                            daemon_id = %daemon_id,
                            server_url = %server_url,
                            "Connection refused by server at {}. \
                             The server may not be running or the URL may be incorrect.",
                            server_url
                        );
                        return Err(anyhow::anyhow!(
                            "Connection refused by server at {}. Verify the server is running and SCANOPY_SERVER_URL is correct.",
                            server_url
                        ));
                    }

                    // Timeout - differentiate between connect timeout and response timeout
                    if error_lower.contains("timeout") || error_lower.contains("timed out") {
                        // Connect timeout - couldn't establish connection at all
                        if error_lower.contains("connect") {
                            tracing::error!(
                                target: LOG_TARGET,
                                daemon_id = %daemon_id,
                                server_url = %server_url,
                                "Connection timed out trying to reach server at {}. \
                                 The server may be unreachable or blocked by a firewall.",
                                server_url
                            );
                            return Err(anyhow::anyhow!(
                                "Connection timed out reaching server at {}. Check network connectivity and firewall rules.",
                                server_url
                            ));
                        }
                        // Response timeout - connected but server didn't respond
                        tracing::error!(
                            target: LOG_TARGET,
                            daemon_id = %daemon_id,
                            server_url = %server_url,
                            "Server at {} did not respond in time. \
                             The connection was established but the server did not send a response. \
                             Consider switching to Pull mode (SCANOPY_MODE=Pull) if the server cannot reach this daemon.",
                            server_url
                        );
                        return Err(anyhow::anyhow!(
                            "Server at {} connected but did not respond. Consider using Pull mode (SCANOPY_MODE=Pull) if the server cannot initiate connections to this daemon.",
                            server_url
                        ));
                    }

                    // Generic connection error
                    if error_lower.contains("connect error")
                        || error_lower.contains("tcp connect")
                        || error_lower.contains("error sending request")
                    {
                        tracing::error!(
                            target: LOG_TARGET,
                            daemon_id = %daemon_id,
                            server_url = %server_url,
                            "Failed to connect to server at {}: {}",
                            server_url,
                            error_str
                        );
                        return Err(anyhow::anyhow!(
                            "Cannot connect to server at {}. Verify the server is running and the URL is correct.",
                            server_url
                        ));
                    }

                    // For other errors, fail immediately
                    return Err(e);
                }
            }
        }
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
                self.log_deprecation_warnings(&capabilities);

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

    /// Log deprecation warnings received from the server.
    fn log_deprecation_warnings(&self, capabilities: &ServerCapabilities) {
        for warning in &capabilities.deprecation_warnings {
            let msg = format!(
                "{}{}",
                warning.message,
                warning
                    .sunset_date
                    .as_ref()
                    .map(|d| format!(" (sunset: {})", d))
                    .unwrap_or_default()
            );
            match warning.severity {
                DeprecationSeverity::Critical => {
                    tracing::error!(target: LOG_TARGET, "{}", msg);
                }
                DeprecationSeverity::Warning => {
                    tracing::warn!(target: LOG_TARGET, "{}", msg);
                }
                DeprecationSeverity::Info => {
                    tracing::info!(target: LOG_TARGET, "{}", msg);
                }
            }
        }
    }
}
