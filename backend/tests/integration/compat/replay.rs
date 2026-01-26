//! Replay tests for API compatibility.
//!
//! These tests load captured fixtures and replay them against a running
//! server or daemon to verify backwards compatibility.
//!
//! ## Integration Test Usage
//!
//! The `run_server_compat_tests` and `run_daemon_compat_tests` functions are
//! called from the integration test suite to verify compatibility with older
//! daemon/server versions using the already-running containers.

use super::schema::validate_response;
use super::types::{
    CapturedExchange, FixtureManifest, get_fixture_versions, load_manifest, load_openapi_spec,
};
use regex::Regex;
use uuid::Uuid;

/// Context for replaying requests with substituted IDs.
pub struct ReplayContext {
    pub daemon_id: Uuid,
    pub network_id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub api_key: String,
}

impl ReplayContext {
    /// Substitute IDs in a path.
    /// Replaces any UUID in the path with the daemon_id.
    pub fn substitute_path(&self, path: &str) -> String {
        let uuid_regex = Regex::new(
            r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        )
        .unwrap();

        uuid_regex
            .replace_all(path, self.daemon_id.to_string().as_str())
            .to_string()
    }

    /// Substitute IDs in a request body.
    /// Replaces known ID fields with test context values.
    pub fn substitute_body(&self, body: &serde_json::Value) -> serde_json::Value {
        let mut body = body.clone();

        if let Some(obj) = body.as_object_mut() {
            // Replace known ID fields
            if obj.contains_key("daemon_id") {
                obj.insert(
                    "daemon_id".to_string(),
                    serde_json::json!(self.daemon_id.to_string()),
                );
            }
            if obj.contains_key("network_id") {
                obj.insert(
                    "network_id".to_string(),
                    serde_json::json!(self.network_id.to_string()),
                );
            }
            if obj.contains_key("user_id") {
                obj.insert(
                    "user_id".to_string(),
                    serde_json::json!(self.user_id.to_string()),
                );
            }
            if obj.contains_key("organization_id") {
                obj.insert(
                    "organization_id".to_string(),
                    serde_json::json!(self.organization_id.to_string()),
                );
            }

            // Recursively process nested objects and arrays
            for (_, value) in obj.iter_mut() {
                if value.is_object() {
                    *value = self.substitute_body(value);
                } else if value.is_array() {
                    *value = self.substitute_array(value);
                }
            }
        }

        body
    }

    /// Recursively substitute IDs in array elements.
    fn substitute_array(&self, arr: &serde_json::Value) -> serde_json::Value {
        if let Some(items) = arr.as_array() {
            serde_json::Value::Array(
                items
                    .iter()
                    .map(|item| {
                        if item.is_object() {
                            self.substitute_body(item)
                        } else if item.is_array() {
                            self.substitute_array(item)
                        } else {
                            item.clone()
                        }
                    })
                    .collect(),
            )
        } else {
            arr.clone()
        }
    }
}

/// Result of replaying an exchange.
pub struct ReplayResult {
    pub exchange: CapturedExchange,
    pub actual_status: u16,
    pub actual_body: serde_json::Value,
    pub status_ok: bool,
    pub schema_validation: Option<Result<(), String>>,
}

impl ReplayResult {
    /// Check if the replay was fully successful (2xx status and valid schema).
    pub fn is_success(&self) -> bool {
        self.status_ok && self.schema_validation.as_ref().is_none_or(|r| r.is_ok())
    }

    /// Format result for display.
    fn format_result(&self) -> String {
        if self.is_success() {
            format!(
                "  ✓ {} {} -> {} (schema: valid)",
                self.exchange.method, self.exchange.path, self.actual_status
            )
        } else if self.status_ok {
            let schema_err = self
                .schema_validation
                .as_ref()
                .and_then(|r| r.as_ref().err())
                .map(|s| s.as_str())
                .unwrap_or("unknown");
            format!(
                "  ✗ {} {} -> {} (schema validation failed)\n    {}",
                self.exchange.method, self.exchange.path, self.actual_status, schema_err
            )
        } else {
            format!(
                "  ✗ {} {} -> {} (expected 2xx)\n    Response: {}",
                self.exchange.method,
                self.exchange.path,
                self.actual_status,
                serde_json::to_string_pretty(&self.actual_body).unwrap_or_default()
            )
        }
    }
}

/// Replay a single exchange against a server/daemon.
pub async fn replay_exchange(
    client: &reqwest::Client,
    base_url: &str,
    exchange: &CapturedExchange,
    ctx: &ReplayContext,
    openapi: Option<&serde_json::Value>,
) -> Result<ReplayResult, String> {
    let path = ctx.substitute_path(&exchange.path);
    let url = format!("{}{}", base_url, path);
    let body = ctx.substitute_body(&exchange.request_body);

    let mut req = match exchange.method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url).json(&body),
        "PUT" => client.put(&url).json(&body),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url).json(&body),
        _ => client.get(&url),
    };

    // Add daemon headers for server requests
    req = req
        .header("X-Daemon-ID", ctx.daemon_id.to_string())
        .header("Authorization", format!("Bearer {}", &ctx.api_key));

    let response = req.send().await.map_err(|e| e.to_string())?;
    let actual_status = response.status().as_u16();
    let actual_body = response
        .json::<serde_json::Value>()
        .await
        .unwrap_or(serde_json::json!({}));

    // Check status is 2xx
    let status_ok = (200..300).contains(&actual_status);

    // Validate response against OpenAPI schema if available
    let schema_validation = openapi.map(|spec| {
        validate_response(
            spec,
            &exchange.path, // Use original path for schema lookup
            &exchange.method,
            actual_status,
            &actual_body,
        )
    });

    Ok(ReplayResult {
        exchange: exchange.clone(),
        actual_status,
        actual_body,
        status_ok,
        schema_validation,
    })
}

/// Paths that should be skipped during replay.
///
/// Some are skipped because they have complex entity dependencies that
/// can't be satisfied by simple ID substitution.
const SKIP_PATH_PREFIXES: &[&str] = &[
    // Creates host with interfaces referencing subnet_id that was generated
    // server-side (not the fixture's ID)
    "/api/v1/hosts/discovery",
];

/// Path suffixes that indicate deprecated endpoints.
const SKIP_PATH_SUFFIXES: &[&str] = &[
    // Deprecated: heartbeat functionality merged into /request-work
    "/heartbeat",
];

/// Check if an exchange path should be skipped.
fn should_skip_path(path: &str) -> bool {
    SKIP_PATH_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
        || SKIP_PATH_SUFFIXES.iter().any(|suffix| path.ends_with(suffix))
}

/// Replay all exchanges from a manifest.
///
/// Only replays exchanges that originally returned 2xx status codes.
/// Exchanges that originally failed (4xx, 5xx) are skipped since they
/// represent expected failure cases in the original flow (e.g., startup
/// before registration returns 404).
///
/// Some paths with complex entity dependencies are also skipped.
pub async fn replay_manifest(
    client: &reqwest::Client,
    base_url: &str,
    manifest: &FixtureManifest,
    ctx: &ReplayContext,
    openapi: Option<&serde_json::Value>,
) -> Vec<Result<ReplayResult, String>> {
    let mut results = Vec::new();

    for exchange in &manifest.exchanges {
        // Skip exchanges that originally failed - these are expected failure cases
        // in the original flow (e.g., startup before registration returns 404)
        if !(200..300).contains(&exchange.response_status) {
            continue;
        }

        // Skip paths with complex entity dependencies
        if should_skip_path(&exchange.path) {
            continue;
        }

        let result = replay_exchange(client, base_url, exchange, ctx, openapi).await;
        results.push(result);
    }

    results
}

/// Run server compatibility tests - replays old daemon requests against current server.
/// Returns Ok(()) if all fixtures replay successfully, Err with details otherwise.
pub async fn run_server_compat_tests(server_url: &str, ctx: &ReplayContext) -> Result<(), String> {
    let versions = get_fixture_versions("daemon_to_server.json");
    if versions.is_empty() {
        println!("  No daemon_to_server fixtures found, skipping");
        return Ok(());
    }

    let client = reqwest::Client::new();

    for version in versions {
        let Some(manifest) = load_manifest(&version, "daemon_to_server.json") else {
            continue;
        };

        let openapi = load_openapi_spec(&version);
        if openapi.is_none() {
            println!(
                "  Warning: No OpenAPI spec for v{}, skipping schema validation",
                version
            );
        }

        println!("  Testing server compatibility with daemon v{}", version);

        let results = replay_manifest(&client, server_url, &manifest, ctx, openapi.as_ref()).await;

        for result in results {
            match result {
                Ok(r) if r.is_success() => {
                    println!("{}", r.format_result());
                }
                Ok(r) => {
                    return Err(r.format_result());
                }
                Err(e) => {
                    return Err(format!("  ✗ Request failed: {}", e));
                }
            }
        }
    }

    Ok(())
}

/// Run daemon compatibility tests - replays old server requests against current daemon.
/// Returns Ok(()) if all fixtures replay successfully, Err with details otherwise.
///
/// Note: Daemon compat tests require the daemon to be running in a mode that exposes
/// an HTTP API (e.g., not DaemonPoll mode). If the daemon isn't reachable, tests are skipped.
pub async fn run_daemon_compat_tests(daemon_url: &str, ctx: &ReplayContext) -> Result<(), String> {
    let versions = get_fixture_versions("server_to_daemon.json");
    if versions.is_empty() {
        println!("  No server_to_daemon fixtures found, skipping");
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    // Check if daemon is reachable before running tests
    // In DaemonPoll mode, the daemon doesn't expose an HTTP API
    let health_url = format!("{}/api/health", daemon_url);
    match client.get(&health_url).send().await {
        Ok(_) => {}
        Err(e) if e.is_connect() || e.is_timeout() => {
            println!(
                "  Daemon at {} is not reachable (may be running in poll mode), skipping daemon compat tests",
                daemon_url
            );
            return Ok(());
        }
        Err(_) => {
            // Other errors (like 404) mean daemon is reachable, continue with tests
        }
    }

    for version in versions {
        let Some(manifest) = load_manifest(&version, "server_to_daemon.json") else {
            continue;
        };

        let openapi = load_openapi_spec(&version);
        if openapi.is_none() {
            println!(
                "  Warning: No OpenAPI spec for v{}, skipping schema validation",
                version
            );
        }

        println!("  Testing daemon compatibility with server v{}", version);

        let results = replay_manifest(&client, daemon_url, &manifest, ctx, openapi.as_ref()).await;

        for result in results {
            match result {
                Ok(r) if r.is_success() => {
                    println!("{}", r.format_result());
                }
                Ok(r) => {
                    return Err(r.format_result());
                }
                Err(e) => {
                    return Err(format!("  ✗ Request failed: {}", e));
                }
            }
        }
    }

    Ok(())
}
