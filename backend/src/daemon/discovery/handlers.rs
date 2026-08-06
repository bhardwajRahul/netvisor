use crate::daemon::runtime::types::DaemonAppState;
use crate::server::{
    daemons::r#impl::api::{DaemonDiscoveryRequest, DaemonDiscoveryResponse},
    shared::types::api::{ApiError, ApiResponse, ApiResult},
};
use axum::{Router, extract::State, response::Json, routing::post};
use std::sync::Arc;
use uuid::Uuid;

pub fn create_router() -> Router<Arc<DaemonAppState>> {
    Router::new()
        .route("/initiate", post(handle_discovery_request))
        .route("/cancel", post(handle_cancel_request))
}

pub async fn handle_discovery_request(
    State(state): State<Arc<DaemonAppState>>,
    Json(request): Json<DaemonDiscoveryRequest>,
) -> ApiResult<Json<ApiResponse<DaemonDiscoveryResponse>>> {
    let session_id = request.session_id;
    tracing::info!(
        "Received {} discovery request, session ID {}",
        request.discovery_type,
        request.session_id
    );

    let manager = &state.services.discovery_manager;

    if !manager.try_initiate_session(request).await {
        return Err(ApiError::conflict("Discovery session already in progress"));
    }

    Ok(Json(ApiResponse::success(DaemonDiscoveryResponse {
        session_id,
    })))
}

pub async fn handle_cancel_request(
    State(state): State<Arc<DaemonAppState>>,
    Json(session_id): Json<Uuid>,
) -> ApiResult<Json<ApiResponse<Uuid>>> {
    tracing::info!(
        "Received discovery cancellation request for session {}",
        session_id
    );

    let manager = state.services.discovery_manager.clone();

    // Ask the manager once. Checking liveness here and again inside
    // `cancel_current_session` is a TOCTOU: a session that ends between the two
    // reads reported an internal error for what is really "nothing to cancel".
    // Just signal cancellation, don't wait — the spawned task handles cleanup.
    if manager.cancel_current_session().await {
        // Don't clear the task - let the spawned task do it
        Ok(Json(ApiResponse::success(session_id)))
    } else {
        Err(ApiError::conflict(
            "Discovery session not currently running",
        ))
    }
}
