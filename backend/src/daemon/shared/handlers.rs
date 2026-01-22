use crate::{
    daemon::{
        discovery::handlers as discovery_handlers,
        runtime::{
            state::{CreatedEntitiesPayload, DaemonStatus, DiscoveryPollResponse},
            types::{DaemonAppState, InitializeDaemonRequest},
        },
        shared::auth::server_auth_middleware,
    },
    server::shared::types::api::{ApiResponse, ApiResult},
};
use axum::{
    Json, Router,
    extract::State,
    middleware,
    routing::{get, post},
};
use std::sync::Arc;

/// Create daemon HTTP router.
/// The `state` parameter is required for applying authentication middleware
/// to ServerPoll mode endpoints.
pub fn create_router(state: Arc<DaemonAppState>) -> Router<Arc<DaemonAppState>> {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/api/health", get(get_health))
        .route("/api/initialize", post(initialize));

    // Authenticated routes (ServerPoll mode - server must provide valid API key)
    // Discovery initiate/cancel require auth to prevent unauthorized scans
    let authenticated_routes = Router::new()
        .route("/api/status", get(get_status))
        .route("/api/poll", get(get_discovery_poll))
        .route(
            "/api/discovery/entities-created",
            post(receive_created_entities),
        )
        .route(
            "/api/discovery/initiate",
            post(discovery_handlers::handle_discovery_request),
        )
        .route(
            "/api/discovery/cancel",
            post(discovery_handlers::handle_cancel_request),
        )
        .route_layer(middleware::from_fn_with_state(
            state,
            server_auth_middleware,
        ));

    public_routes.merge(authenticated_routes)
}

async fn get_health() -> ApiResult<Json<ApiResponse<String>>> {
    tracing::info!("Received healthcheck request");

    Ok(Json(ApiResponse::success(
        "Scanopy Daemon Running".to_string(),
    )))
}

async fn initialize(
    State(state): State<Arc<DaemonAppState>>,
    Json(request): Json<InitializeDaemonRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    // Check if daemon is already initialized (once-only guard)
    // Prevents re-initialization attacks - if both network_id and api_key are set,
    // return success without modifying the configuration
    let existing_network_id = state.config.get_network_id().await.ok().flatten();
    let existing_api_key = state.config.get_api_key().await.ok().flatten();

    if existing_network_id.is_some() && existing_api_key.is_some() {
        tracing::warn!(
            network_id = %request.network_id,
            "Received initialization request but daemon is already initialized - ignoring"
        );
        return Ok(Json(ApiResponse::success(
            "Daemon already initialized".to_string(),
        )));
    }

    tracing::info!(
        network_id = %request.network_id,
        api_key = %request.api_key,
        "Received initialization signal",
    );

    state
        .services
        .runtime_service
        .initialize_services(request.network_id, request.api_key)
        .await?;

    Ok(Json(ApiResponse::success(
        "Daemon initialized successfully".to_string(),
    )))
}

/// Get daemon status (for ServerPoll mode).
/// Returns lightweight status: url, name, mode, version.
async fn get_status(
    State(state): State<Arc<DaemonAppState>>,
) -> ApiResult<Json<ApiResponse<DaemonStatus>>> {
    let status = state.services.daemon_state.get_status().await;
    Ok(Json(ApiResponse::success(status)))
}

/// Get discovery poll data (for ServerPoll mode).
/// Returns current progress and any buffered entities since last poll.
async fn get_discovery_poll(
    State(state): State<Arc<DaemonAppState>>,
) -> ApiResult<Json<ApiResponse<DiscoveryPollResponse>>> {
    let progress = state.services.daemon_state.get_progress().await;
    let entities = state.services.daemon_state.drain_entities().await;

    Ok(Json(ApiResponse::success(DiscoveryPollResponse {
        progress,
        entities,
    })))
}

/// Receive created entity confirmations from server (for ServerPoll mode).
/// Server sends back actual entities (with deduped IDs) after processing polled entities.
async fn receive_created_entities(
    State(state): State<Arc<DaemonAppState>>,
    Json(payload): Json<CreatedEntitiesPayload>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let buffer = state.services.daemon_state.entity_buffer();

    // Mark subnets as created with actual server data
    for (pending_id, actual_subnet) in payload.subnets {
        if let Some((old_id, new_id)) = buffer.mark_subnet_created(pending_id, actual_subnet).await
        {
            tracing::debug!(
                old_id = %old_id,
                new_id = %new_id,
                "Subnet ID changed after server deduplication"
            );
        }
    }

    // Mark hosts as created with actual server data
    for (pending_id, actual_host) in payload.hosts {
        buffer.mark_host_created(pending_id, actual_host).await;
    }

    tracing::info!("Received created entities confirmation from server");

    Ok(Json(ApiResponse::success(
        "Created entities acknowledged".to_string(),
    )))
}
