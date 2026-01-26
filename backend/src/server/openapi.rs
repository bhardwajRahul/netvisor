//! OpenAPI documentation configuration
//!
//! Uses utoipa-axum for automatic route documentation and utoipa-scalar for the UI.
//! Routes and schemas are collected automatically from handlers via OpenApiRouter.
//!
//! Endpoints tagged with "internal" are included in the full spec (for client generation)
//! but filtered out of the public Scalar documentation.

use axum::{Extension, Json, Router};
use serde_json::json;
use std::sync::Arc;
use utoipa::OpenApi as OpenApiDerive;
use utoipa::openapi::RefOr;
use utoipa::openapi::schema::Schema;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::openapi::{Components, OpenApi, PathItem};
use utoipa_scalar::{Scalar, Servable};

use crate::server::bindings::r#impl::base::Binding;
use crate::server::config::AppState;
use crate::server::daemon_api_keys::r#impl::base::DaemonApiKey;
use crate::server::daemons::handlers::DaemonOrderField;
use crate::server::daemons::r#impl::base::Daemon;
use crate::server::discovery::r#impl::base::Discovery;
use crate::server::groups::handlers::GroupOrderField;
use crate::server::groups::r#impl::base::Group;
use crate::server::hosts::handlers::HostOrderField;
use crate::server::hosts::r#impl::base::Host;
use crate::server::if_entries::r#impl::base::IfEntry;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::invites::r#impl::base::Invite;
use crate::server::networks::r#impl::Network;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::ports::r#impl::base::Port;
use crate::server::services::handlers::ServiceOrderField;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::handlers::query::{OrderDirection, PaginationParams};
use crate::server::shared::storage::traits::Entity;
use crate::server::shares::r#impl::base::Share;
use crate::server::snmp_credentials::handlers::SnmpCredentialOrderField;
use crate::server::snmp_credentials::r#impl::base::SnmpCredential;
use crate::server::subnets::handlers::SubnetOrderField;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::tags::handlers::TagOrderField;
use crate::server::tags::r#impl::base::Tag;
use crate::server::topology::types::base::Topology;
use crate::server::user_api_keys::r#impl::base::UserApiKey;
use crate::server::users::r#impl::base::User;

/// Tag used to mark endpoints that should be hidden from public documentation
/// but included in the full OpenAPI spec for client generation.
const INTERNAL_TAG: &str = "internal";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// OpenAPI base configuration
///
/// Paths, schemas, and tags are collected automatically from handler annotations by utoipa-axum.
/// Only API metadata and security schemes need to be defined here.
#[derive(OpenApiDerive)]
#[openapi(
    components(schemas(
        PaginationParams,
        OrderDirection,
        HostOrderField,
        ServiceOrderField,
        TagOrderField,
        GroupOrderField,
        SubnetOrderField,
        DaemonOrderField,
        SnmpCredentialOrderField
    )),
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

## Pagination

List endpoints support pagination via query parameters:

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | 50 | Maximum results to return (1-1000). Use 0 for no limit. |
| `offset` | integer | 0 | Number of results to skip |

Example: `GET /api/v1/hosts?limit=10&offset=20`

## Response Format

All responses use a standard envelope:

```json
{
  "success": true,
  "data": { ... },
  "meta": {
    "api_version": 1,
    "server_version": "{{SERVER_VERSION}}"
  }
}
```

**Paginated list responses** include pagination metadata:

```json
{
  "success": true,
  "data": [ ... ],
  "meta": {
    "api_version": 1,
    "server_version": "{{SERVER_VERSION}}",
    "pagination": {
      "total_count": 142,
      "limit": 50,
      "offset": 0,
      "has_more": true
    }
  }
}
```

| Field | Description |
|-------|-------------|
| `total_count` | Total items matching your query (ignoring pagination) |
| `limit` | Applied limit (your request or default) |
| `offset` | Applied offset |
| `has_more` | `true` if more results exist beyond this page |

**Error responses** include an `error` field instead of `data`:

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
        // Entity tags - descriptions sourced from Entity trait for consistency
        (name = Binding::ENTITY_NAME_PLURAL, description = Binding::ENTITY_DESCRIPTION),
        (name = Daemon::ENTITY_NAME_PLURAL, description = Daemon::ENTITY_DESCRIPTION),
        (name = DaemonApiKey::ENTITY_NAME_PLURAL, description = DaemonApiKey::ENTITY_DESCRIPTION),
        (name = Discovery::ENTITY_NAME_PLURAL, description = Discovery::ENTITY_DESCRIPTION),
        (name = Group::ENTITY_NAME_PLURAL, description = Group::ENTITY_DESCRIPTION),
        (name = Host::ENTITY_NAME_PLURAL, description = Host::ENTITY_DESCRIPTION),
        (name = IfEntry::ENTITY_NAME_PLURAL, description = IfEntry::ENTITY_DESCRIPTION),
        (name = Interface::ENTITY_NAME_PLURAL, description = Interface::ENTITY_DESCRIPTION),
        (name = Invite::ENTITY_NAME_PLURAL, description = Invite::ENTITY_DESCRIPTION),
        (name = Network::ENTITY_NAME_PLURAL, description = Network::ENTITY_DESCRIPTION),
        (name = Organization::ENTITY_NAME_PLURAL, description = Organization::ENTITY_DESCRIPTION),
        (name = Port::ENTITY_NAME_PLURAL, description = Port::ENTITY_DESCRIPTION),
        (name = Service::ENTITY_NAME_PLURAL, description = Service::ENTITY_DESCRIPTION),
        (name = Share::ENTITY_NAME_PLURAL, description = Share::ENTITY_DESCRIPTION),
        (name = SnmpCredential::ENTITY_NAME_PLURAL, description = SnmpCredential::ENTITY_DESCRIPTION),
        (name = Subnet::ENTITY_NAME_PLURAL, description = Subnet::ENTITY_DESCRIPTION),
        (name = Tag::ENTITY_NAME_PLURAL, description = Tag::ENTITY_DESCRIPTION),
        (name = Topology::ENTITY_NAME_PLURAL, description = Topology::ENTITY_DESCRIPTION),
        (name = User::ENTITY_NAME_PLURAL, description = User::ENTITY_DESCRIPTION),
        (name = UserApiKey::ENTITY_NAME_PLURAL, description = UserApiKey::ENTITY_DESCRIPTION),
        // Non-entity tags with inline descriptions
        (name = "auth", description = "Authentication and session management. Handle user login, logout, and session state."),
        (name = "config", description = "Server configuration. Public configuration settings for client applications."),
        (name = "github", description = "GitHub integration endpoints."),
        (name = "internal", description = "Internal endpoints for system operations. Not part of the public API."),
        (name = "metadata", description = "Entity metadata registry. Schema information for all entity types in the system."),
        (name = "system", description = "System information endpoints. Version and compatibility checking."),
    )
)]
pub struct ApiDoc;

/// Merge the base configuration with paths/schemas/tags collected from handlers
pub fn build_openapi(paths_from_handlers: OpenApi) -> OpenApi {
    let mut base = ApiDoc::openapi();

    // Replace version placeholder in description
    if let Some(ref mut description) = base.info.description {
        *description = description.replace("{{SERVER_VERSION}}", SERVER_VERSION);
    }

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

    // Fix schema examples that utoipa doesn't handle well
    fix_schema_examples(&mut base);

    base
}

/// Fix schema examples that utoipa doesn't properly generate for nested optional types
fn fix_schema_examples(spec: &mut OpenApi) {
    let Some(ref mut components) = spec.components else {
        return;
    };

    // Add proper example to PaginationMeta schema
    if let Some(RefOr::T(Schema::Object(schema))) = components.schemas.get_mut("PaginationMeta") {
        schema.example = Some(json!({
            "total_count": 142,
            "limit": 50,
            "offset": 0,
            "has_more": true
        }));
    }
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
