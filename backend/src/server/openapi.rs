//! OpenAPI documentation configuration
//!
//! Uses utoipa-axum for automatic route documentation and utoipa-scalar for the UI.
//! Routes and schemas are collected automatically from handlers via OpenApiRouter.
//!
//! Endpoints tagged with "internal" are included in the full spec (for client generation)
//! but filtered out of the public Scalar documentation.

use axum::{Extension, Json, Router};
use std::sync::Arc;
use utoipa::OpenApi as OpenApiDerive;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::openapi::{Components, OpenApi, PathItem};
use utoipa_scalar::{Scalar, Servable};

use crate::server::config::AppState;
use crate::server::shared::handlers::query::PaginationParams;

/// Tag used to mark endpoints that should be hidden from public documentation
/// but included in the full OpenAPI spec for client generation.
const INTERNAL_TAG: &str = "internal";

/// OpenAPI base configuration
///
/// Paths, schemas, and tags are collected automatically from handler annotations by utoipa-axum.
/// Only API metadata and security schemes need to be defined here.
#[derive(OpenApiDerive)]
#[openapi(
    components(schemas(PaginationParams)),
    info(
        title = "Scanopy API",
        version = "1",
        description = r#"
Network topology discovery and visualization API.

## Authentication

Two authentication methods are supported:

| Method | Header | Use Case |
|--------|--------|----------|
| User API key | `Authorization: Bearer scp_u_...` | Programmatic access, integrations |
| Session cookie | `Cookie: session_id=...` | Web UI (via `/api/auth/login`) |

User API keys require your organization to have API access enabled. Create keys at **Platform > API Keys**.

## Rate Limiting

Limit: 300 requests/minute

Burst: 150

Response headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`

When rate limited, you'll receive HTTP `429 Too Many Requests` with a `Retry-After` header.

## Response Format

All responses use a standard envelope:

```json
{
  "success": true,
  "data": { ... },
  "meta": {
    "api_version": 1,
    "server_version": "0.12.10"
  }
}
```

Error responses include an `error` field instead of `data`:

```json
{
  "success": false,
  "error": "Resource not found",
  "meta": { ... }
}
```

**Common status codes:** `400` validation error, `401` unauthorized, `403` forbidden, `404` not found, `409` conflict, `429` rate limited.

## Versioning

The API version is an integer (`api_version: 1`) incremented only on breaking changes. API is versioned independently from the application. Endpoints are prefixed with `/api/v1/`. Check `GET /api/version` for current versions.

## Multi-Tenancy

Resources are scoped to your **organization** and **network(s)**:

- You can only access entities within your organization
- Network-level entities (hosts, services, etc.) are filtered to networks you have access to
- Use `?network_id=<UUID>` to filter list endpoints to a specific network
- API keys can be scoped to a subset of your accessible networks
"#,
        license(name = "Dual (AGPL3.0, Commercial License Available)")
    ),
    tags(
        (name = "api_keys", description = "API keys for daemon authentication. Create and manage keys that allow daemons to communicate with the server."),
        (name = "auth", description = "Authentication and session management. Handle user login, logout, and session state."),
        (name = "config", description = "Server configuration. Public configuration settings for client applications."),
        (name = "daemon_api_keys", description = "Daemon API keys for scanner authentication. Create and manage keys that allow daemons to authenticate with the server and submit discovery results."),
        (name = "discoveries", description = "Network discovery operations. Trigger and monitor scans that detect hosts, services, and network topology."),
        (name = "github", description = "GitHub integration endpoints."),
        (name = "ports", description = "Ports that have been scanned and found open on a host"),
        (name = "bindings", description = "
            ## Binding Types
            - **Interface binding**: Service is present at an interface (IP address) without a specific port.
              Used for non-port-bound services like gateways.
            - **Port binding (specific interface)**: Service listens on a specific port on a specific interface.
            - **Port binding (all interfaces)**: Service listens on a specific port on all interfaces
              (`interface_id: null`).
        "),
        (name = "groups", description = "Logical groupings of hosts. Organize hosts into groups for easier management and visualization."),
        (name = "hosts", description = "Network hosts (devices). Manage discovered or manually created hosts on your network."),
        (name = "interfaces", description = "Network interfaces on hosts. Each host can have multiple interfaces with different IP addresses."),
        (name = "internal", description = "Internal endpoints for system operations. Not part of the public API."),
        (name = "invites", description = "Organization invitations. Invite users to join your organization."),
        (name = "metadata", description = "Entity metadata registry. Schema information for all entity types in the system."),
        (name = "networks", description = "Network containers. Top-level organizational unit that contains subnets, hosts, and other entities."),
        (name = "organizations", description = "Manage organization settings."),
        (name = "services", description = "Services running on hosts. Detected or manually added services like databases, web servers, etc."),
        (name = "shares", description = "Shared network views. Create read-only shareable links to your network topology."),
        (name = "subnets", description = "IP subnets within networks. Define address ranges and organize hosts by subnet."),
        (name = "system", description = "System information endpoints. Version and compatibility checking."),
        (name = "tags", description = "Custom tags for categorization. Apply labels to entities for filtering and organization."),
        (name = "user_api_keys", description = "User API keys for programmatic access. Create and manage personal API keys with scoped permissions for automation and integrations."),
        (name = "users", description = "User account management. Manage user profiles and permissions within organizations."),
    )
)]
pub struct ApiDoc;

/// Merge the base configuration with paths/schemas/tags collected from handlers
pub fn build_openapi(paths_from_handlers: OpenApi) -> OpenApi {
    let mut base = ApiDoc::openapi();

    // Merge paths from handlers
    base.paths.paths.extend(paths_from_handlers.paths.paths);

    // Merge schemas from handlers
    if let Some(handler_components) = paths_from_handlers.components {
        if let Some(ref mut base_components) = base.components {
            base_components.schemas.extend(handler_components.schemas);
        } else {
            base.components = Some(handler_components);
        }
    }

    // Merge tags from handlers
    if let Some(handler_tags) = paths_from_handlers.tags {
        if let Some(ref mut base_tags) = base.tags {
            base_tags.extend(handler_tags);
        } else {
            base.tags = Some(handler_tags);
        }
    }

    // Add security schemes
    add_security_schemes(&mut base);

    base
}

/// Add security scheme definitions to the OpenAPI spec
fn add_security_schemes(spec: &mut OpenApi) {
    let components = spec.components.get_or_insert_with(Components::default);

    // User API key authentication (most common for API consumers)
    components.security_schemes.insert(
        "user_api_key".to_string(),
        SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
            "Authorization",
            "User API key (Bearer scp_u_...). Create in Platform > API Keys.",
        ))),
    );

    // Daemon API key authentication
    components.security_schemes.insert(
        "daemon_api_key".to_string(),
        SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::with_description(
            "Authorization",
            "Daemon API key (Bearer scp_d_...). Requires X-Daemon-ID header.",
        ))),
    );

    // Session cookie authentication (used by web UI)
    components.security_schemes.insert(
        "session".to_string(),
        SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::with_description(
            "session_id",
            "Browser session cookie. Obtained via /api/auth/login.",
        ))),
    );
}

/// Check if a path item has any operations tagged with "internal"
fn has_internal_tag(path_item: &PathItem) -> bool {
    let operations = [
        path_item.get.as_ref(),
        path_item.post.as_ref(),
        path_item.put.as_ref(),
        path_item.delete.as_ref(),
        path_item.patch.as_ref(),
        path_item.head.as_ref(),
        path_item.options.as_ref(),
        path_item.trace.as_ref(),
    ];

    // If any operation is tagged with "internal", hide the path
    operations.iter().flatten().any(|op| {
        op.tags
            .as_ref()
            .is_some_and(|tags| tags.contains(&INTERNAL_TAG.to_string()))
    })
}

/// Filter out paths tagged with "internal" from the OpenAPI spec.
/// Used to create a public documentation version while keeping the full spec
/// for client generation.
///
/// Endpoints can have multiple tags (e.g., `tags = ["billing", "internal"]`).
/// Any endpoint with "internal" as one of its tags will be filtered out.
pub fn filter_internal_paths(spec: &OpenApi) -> OpenApi {
    let mut filtered = spec.clone();

    // Remove paths that have any "internal" tag
    filtered
        .paths
        .paths
        .retain(|_path, item| !has_internal_tag(item));

    // Remove the "internal" tag from the tags list
    if let Some(ref mut tags) = filtered.tags {
        tags.retain(|tag| tag.name != INTERNAL_TAG);
    }

    filtered
}

/// Create the OpenAPI documentation router
/// Takes the OpenAPI spec collected from handlers and merges it with schema definitions.
///
/// The full spec (including internal endpoints) is served at `/api/openapi.json` for client generation.
/// The filtered spec (excluding internal endpoints) is served at `/api/docs` for public documentation.
pub fn create_docs_router(paths_from_handlers: OpenApi) -> Router<Arc<AppState>> {
    let full_openapi = Arc::new(build_openapi(paths_from_handlers));
    let public_openapi = filter_internal_paths(&full_openapi);

    Router::new()
        // Scalar docs show only public endpoints
        .merge(Scalar::with_url("/api/docs", public_openapi))
        // Full spec for client generation (includes internal endpoints)
        .route("/api/openapi.json", axum::routing::get(get_openapi_json))
        .layer(Extension(full_openapi))
}

/// Returns the OpenAPI specification as JSON
async fn get_openapi_json(Extension(openapi): Extension<Arc<OpenApi>>) -> Json<OpenApi> {
    Json((*openapi).clone())
}

/// Export the OpenAPI spec to a file for client generation.
/// This is used by the fixture generator to create the spec without running the server.
pub fn export_openapi_spec_to_file(
    openapi: OpenApi,
    path: &std::path::Path,
) -> std::io::Result<()> {
    let full_openapi = build_openapi(openapi);
    let json = serde_json::to_string_pretty(&full_openapi).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}
