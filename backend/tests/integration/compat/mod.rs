//! API compatibility tests.
//!
//! These tests verify that the server and daemon can handle requests from
//! different versions, ensuring backwards compatibility.
//!
//! ## Fixture Generation
//!
//! Fixtures are automatically captured during integration tests when running
//! with `--features generate-fixtures`:
//!
//! - `daemon_to_server.json`: Requests the daemon makes to the server
//! - `server_to_daemon.json`: Requests the server makes to the daemon
//! - `openapi.json`: OpenAPI spec for schema validation
//!
//! ## Replay Testing
//!
//! Replay tests load fixtures and make actual HTTP requests to verify
//! compatibility. IDs in paths and bodies are substituted with test values.
//! Response bodies are validated against the captured OpenAPI schema.

mod replay;
mod schema;
mod types;

pub use replay::*;

use crate::infra::{TestClient, setup_authenticated_user};
use scanopy::server::daemon_api_keys::r#impl::api::DaemonApiKeyResponse;
use scanopy::server::daemon_api_keys::r#impl::base::{DaemonApiKey, DaemonApiKeyBase};
use scanopy::server::shared::storage::traits::Storable;
use uuid::Uuid;

const SERVER_URL: &str = "http://localhost:60072";
const DAEMON_URL: &str = "http://localhost:60073";

/// Create a daemon API key for use in compat tests.
async fn create_compat_test_api_key(network_id: Uuid) -> Result<String, String> {
    let client = TestClient::new();

    // Re-authenticate to get a session
    setup_authenticated_user(&client).await?;

    let api_key = DaemonApiKey::new(DaemonApiKeyBase {
        key: String::new(),
        name: "Compat Test API Key".to_string(),
        last_used: None,
        expires_at: None,
        network_id,
        is_enabled: true,
        tags: Vec::new(),
        plaintext: None,
    });

    let response: DaemonApiKeyResponse = client.post("/api/v1/auth/daemon", &api_key).await?;
    Ok(response.key)
}

/// Run all compatibility tests against running server and daemon.
pub async fn run_compat_tests(
    daemon_id: Uuid,
    network_id: Uuid,
    organization_id: Uuid,
    user_id: Uuid,
) -> Result<(), String> {
    // Create a daemon API key for replay requests
    let api_key = create_compat_test_api_key(network_id).await?;
    println!("  Created daemon API key for compat tests");

    let ctx = ReplayContext {
        daemon_id,
        network_id,
        user_id,
        organization_id,
        api_key,
    };

    println!("\n=== Server Compatibility (old daemon → current server) ===");
    run_server_compat_tests(SERVER_URL, &ctx).await?;

    println!("\n=== Daemon Compatibility (old server → current daemon) ===");
    run_daemon_compat_tests(DAEMON_URL, &ctx).await?;

    println!("\n✅ All compatibility tests passed!");
    Ok(())
}
