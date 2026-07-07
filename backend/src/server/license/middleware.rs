use crate::server::{config::AppState, shared::types::api::ApiError};
use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Middleware that blocks mutating requests when the license is locked.
///
/// This is simpler than `demo_mode_middleware` — no auth introspection needed.
/// License state is global (not per-org), so we just check the service status.
///
/// Always allows:
/// - Safe methods (GET, HEAD, OPTIONS) — read-only access
/// - Auth endpoints — users must be able to log in to see the banner
/// - Config endpoint — frontend needs this to display the locked banner
/// - Health endpoint — uptime monitors must still work
pub async fn license_guard_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Safe methods always allowed (read-only mode)
    if request.method().is_safe() {
        return next.run(request).await;
    }

    let path = request.uri().path();

    // Auth endpoints always allowed
    if path.starts_with("/api/auth/") {
        return next.run(request).await;
    }

    // Config and health always allowed
    if path == "/api/config" || path == "/api/health" {
        return next.run(request).await;
    }

    // Check license status. No license service => no key configured
    // (community/cloud) => never locked.
    if let Some(license_service) = &state.license_service
        && license_service.current_status().await.is_locked()
    {
        return ApiError::license_locked().into_response();
    }

    next.run(request).await
}
