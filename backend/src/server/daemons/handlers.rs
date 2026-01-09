use crate::server::auth::middleware::permissions::{Authorized, IsDaemon, Viewer};
use crate::server::billing::types::base::BillingPlan;
use crate::server::daemons::r#impl::api::DaemonHeartbeatPayload;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::events::types::TelemetryOperation;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::query::{FilterQueryExtractor, NetworkFilterQuery};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::EntityFilter;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::api::ApiErrorResponse;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    config::AppState,
    daemons::r#impl::{
        api::{
            DaemonCapabilities, DaemonRegistrationRequest, DaemonRegistrationResponse,
            DaemonResponse, DaemonStartupRequest, DiscoveryUpdatePayload, ServerCapabilities,
        },
        base::{Daemon, DaemonBase},
        version::DaemonVersionPolicy,
    },
    discovery::r#impl::{
        base::{Discovery, DiscoveryBase},
        types::{DiscoveryType, HostNamingFallback, RunType},
    },
    hosts::r#impl::base::{Host, HostBase},
    shared::{
        events::types::TelemetryEvent,
        services::traits::EventBusService,
        types::{
            api::{ApiError, ApiResponse, ApiResult, EmptyApiResponse, PaginatedApiResponse},
            entities::EntitySource,
        },
    },
};
use axum::{
    extract::{Path, State},
    response::Json,
};
use chrono::Utc;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    crate::crud_delete_handler!(Daemon, "daemons", "daemon");
    crate::crud_bulk_delete_handler!(Daemon, "daemons");
}

/// User-facing daemon management endpoints (versioned at /api/v1/daemons)
pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all))
        .routes(routes!(get_by_id, generated::delete))
        .routes(routes!(generated::bulk_delete))
}

/// Daemon-internal endpoints (unversioned at /api/daemon)
/// These are called by daemons themselves, not by users.
pub fn create_internal_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(register_daemon))
        .routes(routes!(daemon_startup))
        .routes(routes!(receive_heartbeat))
        .routes(routes!(update_capabilities))
        .routes(routes!(receive_work_request))
}

/// Get all daemons
///
/// Returns all daemons accessible to the user
#[utoipa::path(
    get,
    path = "",
    tag = "daemons",
    operation_id = "get_daemons",
    summary = "Get all daemons",
    params(NetworkFilterQuery),
    responses(
        (status = 200, description = "List of daemons", body = PaginatedApiResponse<DaemonResponse>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    query: Query<NetworkFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<DaemonResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    // Apply network filter and pagination
    let base_filter = EntityFilter::unfiltered().network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    let result = state.services.daemon_service.get_paginated(filter).await?;

    let policy = DaemonVersionPolicy::default();
    let responses: Vec<DaemonResponse> = result
        .items
        .into_iter()
        .map(|d| {
            let version_status = policy.evaluate(d.base.version.as_ref());
            DaemonResponse {
                id: d.id,
                created_at: d.created_at,
                updated_at: d.updated_at,
                base: d.base,
                version_status,
            }
        })
        .collect();

    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        responses,
        result.total_count,
        limit,
        offset,
    )))
}

/// Get daemon by ID
///
/// Returns a specific daemon with computed version status.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "daemons",
    operation_id = "get_daemon_by_id",
    summary = "Get daemon by ID",
    params(("id" = Uuid, Path, description = "Daemon ID")),
    responses(
        (status = 200, description = "Daemon found", body = ApiResponse<DaemonResponse>),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_by_id(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<DaemonResponse>>> {
    let network_ids = auth.network_ids();

    let mut daemon = state
        .services
        .daemon_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Daemon '{}' not found", id)))?;

    // Validate user has access to this daemon's network
    if !network_ids.contains(&daemon.base.network_id) {
        return Err(ApiError::forbidden("You don't have access to this daemon"));
    }

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[daemon.id], EntityDiscriminants::Daemon)
        .await?;
    if let Some(tags) = tags_map.get(&daemon.id) {
        daemon.base.tags = tags.clone();
    }

    let policy = DaemonVersionPolicy::default();
    let version_status = policy.evaluate(daemon.base.version.as_ref());

    Ok(Json(ApiResponse::success(DaemonResponse {
        id: daemon.id,
        created_at: daemon.created_at,
        updated_at: daemon.updated_at,
        base: daemon.base,
        version_status,
    })))
}

const DAILY_MIDNIGHT_CRON: &str = "0 0 0 * * *";

/// Register a new daemon
///
/// Internal endpoint for daemon self-registration. Creates a host entry
/// and sets up default discovery jobs for the daemon.
#[utoipa::path(
    post,
    path = "/register",
    tags = ["daemons", "internal"],
    request_body = DaemonRegistrationRequest,
    responses(
        (status = 200, description = "Daemon registered successfully", body = ApiResponse<DaemonRegistrationResponse>),
        (status = 403, description = "Daemon registration disabled in demo mode", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn register_daemon(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Json(request): Json<DaemonRegistrationRequest>,
) -> ApiResult<Json<ApiResponse<DaemonRegistrationResponse>>> {
    // Check if this is a demo organization - block daemon registration
    let network = state
        .services
        .network_service
        .get_by_id(&request.network_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Network not found".to_string()))?;

    let org = state
        .services
        .organization_service
        .get_by_id(&network.base.organization_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Organization not found".to_string()))?;

    if matches!(org.base.plan, Some(BillingPlan::Demo(_))) {
        return Err(ApiError::forbidden(
            "Daemon registration is disabled in demo mode",
        ));
    }

    let service = &state.services.daemon_service;

    tracing::info!("{:?}", request);

    // Parse version early for use in server_capabilities
    let daemon_version = request
        .version
        .as_ref()
        .and_then(|v| semver::Version::parse(v).ok());

    // Compute server_capabilities if version was provided
    let policy = DaemonVersionPolicy::default();
    let server_capabilities = daemon_version.as_ref().map(|v| {
        let status = policy.evaluate(Some(v));
        ServerCapabilities {
            server_version: policy.latest.clone(),
            minimum_daemon_version: policy.minimum_supported.clone(),
            deprecation_warnings: status.warnings,
        }
    });

    // Check if daemon already exists (re-registration scenario)
    // This handles cases where a previous registration partially succeeded
    if let Some(mut existing_daemon) = service.get_by_id(&request.daemon_id).await? {
        tracing::info!(
            daemon_id = %request.daemon_id,
            host_id = %existing_daemon.base.host_id,
            "Daemon already registered, updating registration"
        );

        // Update daemon with current info
        existing_daemon.base.url = request.url;
        existing_daemon.base.capabilities = request.capabilities;
        existing_daemon.base.last_seen = Utc::now();
        existing_daemon.base.mode = request.mode;
        existing_daemon.base.name = request.name;
        if let Some(v) = daemon_version.clone() {
            existing_daemon.base.version = Some(v);
        }

        let updated_daemon = service
            .update(&mut existing_daemon, auth.into_entity())
            .await
            .map_err(|e| ApiError::internal_error(&format!("Failed to update daemon: {}", e)))?;

        // Return early - host and discoveries already exist from initial registration
        return Ok(Json(ApiResponse::success(DaemonRegistrationResponse {
            daemon: updated_daemon,
            host_id: existing_daemon.base.host_id,
            server_capabilities,
        })));
    }

    // New registration - create host and daemon
    let dummy_host = Host::new(HostBase {
        network_id: request.network_id,
        name: request.name.clone(),
        hostname: None,
        description: None,
        source: EntitySource::Discovery { metadata: vec![] },
        virtualization: None,
        hidden: false,
        tags: Vec::new(),
    });

    let host_response = state
        .services
        .host_service
        .discover_host(dummy_host, vec![], vec![], vec![], auth.entity.clone())
        .await?;

    // If user_id is nil (old daemon), fall back to org owner
    let user_id = if request.user_id.is_nil() {
        state
            .services
            .user_service
            .get_organization_owners(&org.id)
            .await?
            .first()
            .map(|u| u.id)
            .unwrap_or(request.user_id)
    } else {
        request.user_id
    };

    let mut daemon = Daemon::new(DaemonBase {
        host_id: host_response.id,
        network_id: request.network_id,
        url: request.url.clone(),
        capabilities: request.capabilities.clone(),
        last_seen: Utc::now(),
        mode: request.mode,
        name: request.name,
        tags: Vec::new(),
        version: daemon_version,
        user_id,
    });

    daemon.id = request.daemon_id;

    let registered_daemon = service
        .create(daemon, auth.entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to register daemon: {}", e)))?;

    let org_id = state
        .services
        .network_service
        .get_by_id(&request.network_id)
        .await?
        .map(|n| n.base.organization_id)
        .unwrap_or_default();
    let organization = state
        .services
        .organization_service
        .get_by_id(&org_id)
        .await?;

    if let Some(organization) = organization
        && organization.not_onboarded(&TelemetryOperation::FirstDaemonRegistered)
    {
        let authentication: AuthenticatedEntity = auth.into_entity();
        state
            .services
            .daemon_service
            .event_bus()
            .publish_telemetry(TelemetryEvent {
                id: Uuid::new_v4(),
                organization_id: organization.id,
                operation: TelemetryOperation::FirstDaemonRegistered,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "is_onboarding_step": true
                }),

                authentication,
            })
            .await?;
    }

    let discovery_service = state.services.discovery_service.clone();

    let self_report_discovery_type = DiscoveryType::SelfReport {
        host_id: host_response.id,
    };

    let self_report_discovery = discovery_service
        .create_discovery(
            Discovery::new(DiscoveryBase {
                run_type: RunType::Scheduled {
                    cron_schedule: DAILY_MIDNIGHT_CRON.to_string(),
                    last_run: None,
                    enabled: true,
                },
                discovery_type: self_report_discovery_type.clone(),
                name: self_report_discovery_type.to_string(),
                daemon_id: request.daemon_id,
                network_id: request.network_id,
                tags: Vec::new(),
            }),
            AuthenticatedEntity::System,
        )
        .await?;

    discovery_service
        .start_session(self_report_discovery, AuthenticatedEntity::System)
        .await?;

    if request.capabilities.has_docker_socket {
        let docker_discovery_type = DiscoveryType::Docker {
            host_id: host_response.id,
            host_naming_fallback: HostNamingFallback::BestService,
        };

        let docker_discovery = discovery_service
            .create_discovery(
                Discovery::new(DiscoveryBase {
                    run_type: RunType::Scheduled {
                        cron_schedule: DAILY_MIDNIGHT_CRON.to_string(),
                        last_run: None,
                        enabled: true,
                    },
                    discovery_type: docker_discovery_type.clone(),
                    name: docker_discovery_type.to_string(),
                    daemon_id: request.daemon_id,
                    network_id: request.network_id,
                    tags: Vec::new(),
                }),
                AuthenticatedEntity::System,
            )
            .await?;

        discovery_service
            .start_session(docker_discovery, AuthenticatedEntity::System)
            .await?;
    }

    let network_discovery_type = DiscoveryType::Network {
        subnet_ids: None,
        host_naming_fallback: HostNamingFallback::BestService,
    };

    let network_discovery = discovery_service
        .create_discovery(
            Discovery::new(DiscoveryBase {
                run_type: RunType::Scheduled {
                    cron_schedule: DAILY_MIDNIGHT_CRON.to_string(),
                    last_run: None,
                    enabled: true,
                },
                discovery_type: network_discovery_type.clone(),
                name: network_discovery_type.to_string(),
                daemon_id: request.daemon_id,
                network_id: request.network_id,
                tags: Vec::new(),
            }),
            AuthenticatedEntity::System,
        )
        .await?;

    discovery_service
        .start_session(network_discovery, AuthenticatedEntity::System)
        .await?;

    Ok(Json(ApiResponse::success(DaemonRegistrationResponse {
        daemon: registered_daemon,
        host_id: host_response.id,
        server_capabilities,
    })))
}

/// Daemon startup handshake
///
/// Internal endpoint for daemons to report their version on startup.
/// Updates the daemon's version and last_seen timestamp, returns server capabilities.
#[utoipa::path(
    post,
    path = "/{id}/startup",
    tags = ["daemons", "internal"],
    params(("id" = Uuid, Path, description = "Daemon ID")),
    request_body = DaemonStartupRequest,
    responses(
        (status = 200, description = "Startup acknowledged", body = ApiResponse<ServerCapabilities>),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn daemon_startup(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(id): Path<Uuid>,
    Json(request): Json<DaemonStartupRequest>,
) -> ApiResult<Json<ApiResponse<ServerCapabilities>>> {
    let daemon_network_id = auth.network_ids()[0];
    let service = &state.services.daemon_service;

    let mut daemon = service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Daemon '{}' not found", id)))?;

    // Validate daemon belongs to the authenticated daemon's network
    if daemon.base.network_id != daemon_network_id {
        return Err(ApiError::forbidden(
            "Cannot access daemon on a different network",
        ));
    }

    daemon.base.version = Some(request.daemon_version.clone());
    daemon.base.last_seen = Utc::now();

    service
        .update(&mut daemon, auth.into_entity())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to update daemon: {}", e)))?;

    tracing::info!(
        daemon_id = %id,
        version = %request.daemon_version,
        "Daemon startup"
    );

    let policy = DaemonVersionPolicy::default();
    let status = policy.evaluate(Some(&request.daemon_version));

    Ok(Json(ApiResponse::success(ServerCapabilities {
        server_version: policy.latest.clone(),
        minimum_daemon_version: policy.minimum_supported.clone(),
        deprecation_warnings: status.warnings,
    })))
}

/// Update daemon capabilities
///
/// Internal endpoint for daemons to report their current capabilities.
#[utoipa::path(
    post,
    path = "/{id}/update-capabilities",
    tags = ["daemons", "internal"],
    params(("id" = Uuid, Path, description = "Daemon ID")),
    request_body = DaemonCapabilities,
    responses(
        (status = 200, description = "Capabilities updated", body = EmptyApiResponse),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn update_capabilities(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(id): Path<Uuid>,
    Json(updated_capabilities): Json<DaemonCapabilities>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let daemon_network_id = auth.network_ids()[0];
    tracing::debug!(
        id = %id,
        capabilities = %updated_capabilities,
        "Updating capabilities for daemon",
    );
    let service = &state.services.daemon_service;

    let mut daemon = service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Daemon '{}' not found", &id)))?;

    // Validate daemon belongs to the authenticated daemon's network
    if daemon.base.network_id != daemon_network_id {
        return Err(ApiError::forbidden(
            "Cannot access daemon on a different network",
        ));
    }

    daemon.base.capabilities = updated_capabilities;

    service.update(&mut daemon, auth.into_entity()).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Receive daemon heartbeat
///
/// Internal endpoint for daemons to send periodic heartbeats.
/// Updates the daemon's last_seen timestamp and current status.
#[utoipa::path(
    post,
    path = "/{id}/heartbeat",
    tags = ["daemons", "internal"],
    params(("id" = Uuid, Path, description = "Daemon ID")),
    request_body = DaemonHeartbeatPayload,
    responses(
        (status = 200, description = "Heartbeat received", body = EmptyApiResponse),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn receive_heartbeat(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(id): Path<Uuid>,
    Json(request): Json<DaemonHeartbeatPayload>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let daemon_network_id = auth.network_ids()[0];
    let service = &state.services.daemon_service;

    let mut daemon = service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Daemon '{}' not found", &id)))?;

    // Validate daemon belongs to the authenticated daemon's network
    if daemon.base.network_id != daemon_network_id {
        return Err(ApiError::forbidden(
            "Cannot access daemon on a different network",
        ));
    }

    daemon.base.last_seen = Utc::now();
    daemon.base.url = request.url;
    daemon.base.name = request.name;
    daemon.base.mode = request.mode;

    service
        .update(&mut daemon, auth.into_entity())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to update heartbeat: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

/// Request work from server
///
/// Internal endpoint for daemons to poll for pending discovery sessions.
/// Also updates heartbeat and returns any pending cancellation requests.
/// Returns tuple of (next_session, should_cancel).
#[utoipa::path(
    post,
    path = "/{id}/request-work",
    tags = ["daemons", "internal"],
    params(("id" = Uuid, Path, description = "Daemon ID")),
    request_body = DaemonHeartbeatPayload,
    responses(
        (status = 200, description = "Work request processed - returns (Option<DiscoveryUpdatePayload>, bool)"),
        (status = 404, description = "Daemon not found", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn receive_work_request(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Path(daemon_id): Path<Uuid>,
    Json(request): Json<DaemonHeartbeatPayload>,
) -> ApiResult<Json<ApiResponse<(Option<DiscoveryUpdatePayload>, bool)>>> {
    let daemon_network_id = auth.network_ids()[0];
    let service = &state.services.daemon_service;

    let mut daemon = service
        .get_by_id(&daemon_id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to get daemon: {}", e)))?
        .ok_or_else(|| ApiError::not_found(format!("Daemon '{}' not found", &daemon_id)))?;

    // Validate daemon belongs to the authenticated daemon's network
    if daemon.base.network_id != daemon_network_id {
        return Err(ApiError::forbidden(
            "Cannot access daemon on a different network",
        ));
    }

    daemon.base.last_seen = Utc::now();
    daemon.base.url = request.url;
    daemon.base.name = request.name;
    daemon.base.mode = request.mode;

    service
        .update(&mut daemon, auth.entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to update heartbeat: {}", e)))?;

    let sessions = state
        .services
        .discovery_service
        .get_sessions_for_daemon(&daemon_id)
        .await;
    let (cancel, session_id_to_cancel) = state
        .services
        .discovery_service
        .pull_cancellation_for_daemon(&daemon_id)
        .await;

    let next_session = sessions.first().cloned();

    service
        .receive_work_request(
            daemon,
            cancel,
            session_id_to_cancel,
            next_session.clone(),
            auth.into_entity(),
        )
        .await?;

    Ok(Json(ApiResponse::success((next_session, cancel))))
}
