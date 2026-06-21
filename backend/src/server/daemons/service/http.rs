//! Daemon HTTP client (GET/POST with retry) and the request wrappers built on it.
use super::*;

impl DaemonService {
    // ========================================================================
    // Daemon HTTP helpers with built-in retry
    // ========================================================================

    /// Send GET request to daemon with auth and retry.
    /// Uses exponential backoff: 5 retries, 5-30s delays.
    async fn get_from_daemon<T: serde::de::DeserializeOwned>(
        &self,
        daemon: &Daemon,
        api_key: &str,
        path: &str,
    ) -> Result<T> {
        let url = format!("{}{}", daemon.base.url, path);
        let daemon_id = daemon.id;

        (|| async {
            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await?;

            if !response.status().is_success() {
                anyhow::bail!("GET {} failed: HTTP {}", path, response.status());
            }

            let api_response: ApiResponse<T> = response.json().await?;

            if !api_response.success {
                anyhow::bail!(
                    "GET {} failed: {}",
                    path,
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                );
            }

            api_response
                .data
                .ok_or_else(|| anyhow::anyhow!("GET {} response missing data", path))
        })
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(Duration::from_secs(5))
                .with_max_delay(Duration::from_secs(30))
                .with_max_times(UNREACHABLE_THRESHOLD),
        )
        .notify(|e, dur| {
            tracing::warn!(
                daemon_id = %daemon_id,
                path = %path,
                "Request failed, retrying in {:?}: {}",
                dur,
                e
            );
        })
        .await
    }

    /// Send POST request to daemon with optional auth and retry.
    /// Uses exponential backoff: 5 retries, 5-30s delays.
    /// Returns `Option<T>` - `Some(data)` if response contains data, `None` otherwise.
    /// For endpoints that don't return data, use `::<serde_json::Value>` and ignore result.
    ///
    /// If `api_key` is `None`, the request is sent without an Authorization header.
    /// This is used for legacy daemons (< v0.14.0) that don't require authentication.
    async fn post_to_daemon<T: serde::de::DeserializeOwned>(
        &self,
        daemon: &Daemon,
        api_key: Option<&str>,
        path: &str,
        body: &impl serde::Serialize,
    ) -> Result<Option<T>> {
        let url = format!("{}{}", daemon.base.url, path);
        let daemon_id = daemon.id;
        let body_json = serde_json::to_value(body)?;
        let api_key_owned = api_key.map(|s| s.to_owned());

        (|| async {
            let mut request = self.client.post(&url).json(&body_json);

            // Only add auth header if API key provided (v0.14.0+ daemons)
            if let Some(ref key) = api_key_owned {
                request = request.header("Authorization", format!("Bearer {}", key));
            }

            let response = request.send().await?;

            if !response.status().is_success() {
                anyhow::bail!("POST {} failed: HTTP {}", path, response.status());
            }

            let api_response: ApiResponse<T> = response.json().await?;

            if !api_response.success {
                anyhow::bail!(
                    "POST {} failed: {}",
                    path,
                    api_response
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                );
            }

            Ok(api_response.data)
        })
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(Duration::from_secs(5))
                .with_max_delay(Duration::from_secs(30))
                .with_max_times(UNREACHABLE_THRESHOLD),
        )
        .notify(|e, dur| {
            tracing::warn!(
                daemon_id = %daemon_id,
                path = %path,
                "Request failed, retrying in {:?}: {}",
                dur,
                e
            );
        })
        .await
    }

    // ========================================================================
    // Daemon HTTP methods (using helpers)
    // ========================================================================

    /// Poll daemon status via GET /api/status
    pub(crate) async fn poll_status(&self, daemon: &Daemon, api_key: &str) -> Result<DaemonStatus> {
        self.get_from_daemon(daemon, api_key, "/api/status").await
    }

    /// Poll daemon discovery via GET /api/poll
    pub(crate) async fn poll_discovery(
        &self,
        daemon: &Daemon,
        api_key: &str,
    ) -> Result<DiscoveryPollResponse> {
        self.get_from_daemon(daemon, api_key, "/api/poll").await
    }

    /// Send created entities back to daemon via POST /api/discovery/entities-created
    pub(crate) async fn send_created_entities(
        &self,
        daemon: &Daemon,
        api_key: &str,
        created_entities: CreatedEntitiesPayload,
    ) -> Result<()> {
        // Skip if there's nothing to send
        if created_entities.hosts.is_empty() && created_entities.subnets.is_empty() {
            return Ok(());
        }

        // Legacy cleanup: remove once minimum_supported >= 0.16.0
        // Pre-0.16.0 daemons expect old entity/field names in responses
        let version_str = daemon.base.version.as_ref().map(|v| v.to_string());
        if pre_interface_to_ip_address_rename(version_str.as_deref()) {
            let mut json = serde_json::to_value(&created_entities)?;
            rewrite_response_for_legacy_daemon(&mut json);
            let _: Option<serde_json::Value> = self
                .post_to_daemon(
                    daemon,
                    Some(api_key),
                    "/api/discovery/entities-created",
                    &json,
                )
                .await?;
        } else {
            let _: Option<serde_json::Value> = self
                .post_to_daemon(
                    daemon,
                    Some(api_key),
                    "/api/discovery/entities-created",
                    &created_entities,
                )
                .await?;
        }

        tracing::debug!(
            daemon_id = %daemon.id,
            hosts_count = created_entities.hosts.len(),
            subnets_count = created_entities.subnets.len(),
            "Sent created entities to ServerPoll daemon"
        );

        Ok(())
    }

    /// Send discovery request to daemon (HTTP only, no event publishing).
    ///
    /// If `api_key` is `None`, the request is sent without authentication.
    /// This is used for legacy daemons (< v0.14.0) that don't require auth.
    pub async fn send_discovery_request_to_daemon(
        &self,
        daemon: &Daemon,
        api_key: Option<&str>,
        request: DaemonDiscoveryRequest,
    ) -> Result<(), Error> {
        Self::warn_if_insecure_daemon_url(&daemon.base.url);

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %request.session_id,
            "Sending discovery request to daemon"
        );

        // Unified: serialize with credential_mappings via with_exposed_credentials().
        // Legacy: serialize with SNMP inline via with_exposed_snmp().
        let payload = if matches!(request.discovery_type, DiscoveryType::Unified { .. }) {
            request.with_exposed_credentials()
        } else {
            request.with_exposed_snmp()
        };
        let _: Option<serde_json::Value> = self
            .post_to_daemon(daemon, api_key, "/api/discovery/initiate", &payload)
            .await?;

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %request.session_id,
            "Discovery request sent successfully"
        );

        Ok(())
    }

    /// Send discovery cancellation to daemon (HTTP only, no event publishing).
    ///
    /// If `api_key` is `None`, the request is sent without authentication.
    /// This is used for legacy daemons (< v0.14.0) that don't require auth.
    pub async fn send_discovery_cancellation_to_daemon(
        &self,
        daemon: &Daemon,
        api_key: Option<&str>,
        session_id: Uuid,
    ) -> Result<(), Error> {
        let _: Option<serde_json::Value> = self
            .post_to_daemon(daemon, api_key, "/api/discovery/cancel", &session_id)
            .await?;

        tracing::info!(
            daemon_id = %daemon.id,
            session_id = %session_id,
            "Discovery cancellation sent successfully"
        );

        Ok(())
    }

    /// Send first contact request to ServerPoll daemon.
    /// This assigns the daemon its server-side ID and returns the daemon's status.
    pub(crate) async fn send_first_contact(
        &self,
        daemon: &Daemon,
        api_key: &str,
    ) -> Result<DaemonStatus> {
        let policy = DaemonVersionPolicy::default();
        let server_capabilities = ServerCapabilities {
            server_version: policy.latest.clone(),
            minimum_daemon_version: policy.minimum_supported.clone(),
            deprecation_warnings: vec![],
        };

        let request = FirstContactRequest {
            daemon_id: daemon.id,
            server_capabilities,
        };

        self.post_to_daemon(daemon, Some(api_key), "/api/first-contact", &request)
            .await?
            .ok_or_else(|| anyhow::anyhow!("First contact response missing daemon status"))
    }

    /// Initialize a local daemon (for integrated daemon setup)
    pub async fn initialize_local_daemon(
        &self,
        daemon_url: String,
        network_id: Uuid,
        api_key: String,
    ) -> Result<(), Error> {
        match self
            .client
            .post(format!("{}/api/initialize", daemon_url))
            .json(&InitializeDaemonRequest {
                network_id,
                api_key,
            })
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    tracing::info!("Successfully initialized daemon");
                } else {
                    let body = resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Could not read body".to_string());
                    tracing::warn!(status = %status, body = %body, "Daemon returned error");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to reach daemon");
            }
        }

        Ok(())
    }
}
