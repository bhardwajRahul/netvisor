use axum::{
    extract::{FromRequestParts, MatchedPath, Request, State},
    middleware::Next,
    response::Response,
};
use axum_client_ip::ClientIp;
use reqwest::header;
use std::{sync::Arc, time::Instant};

use crate::server::{auth::middleware::auth::AuthenticatedEntity, config::AppState};

/// Normalizes a path for metrics labels to prevent high cardinality.
fn normalize_path_for_metrics(path: &str) -> String {
    // SvelteKit immutable assets (cache-busted hashes)
    if path.starts_with("/_app/immutable/") {
        return "static_immutable".to_string();
    }

    // Use mime_guess to detect static files by extension
    // This covers ~800 file extensions without manual maintenance
    if mime_guess::from_path(path).first().is_some() {
        return "static_file".to_string();
    }

    path.to_string()
}

pub async fn request_logging_middleware(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();

    // Extract info before consuming request
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| uri.path().to_owned());
    let normalized_path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|p| normalize_path_for_metrics(p.as_str()))
        .unwrap_or_else(|| "unmatched".to_string());

    // Extract auth info
    let (mut parts, body) = request.into_parts();
    let entity = AuthenticatedEntity::from_request_parts(&mut parts, &state)
        .await
        .ok();

    let (entity_type, entity_id, daemon_version_raw) = entity
        .map(|e| {
            (
                e.entity_name(),
                e.entity_id(),
                e.daemon_version().map(|v| v.to_string()),
            )
        })
        .unwrap_or(("anonymous".to_string(), None, None));

    // Bounded daemon-version label: reduce the (client-supplied) X-Daemon-Version
    // to `major.minor` so cardinality is capped at the finite set of release lines
    // (a garbage or spoofed header collapses to "none"). Lets us see the installed
    // base's version distribution from live traffic, not just the DB.
    let daemon_version_label = daemon_version_raw
        .as_deref()
        .and_then(|v| semver::Version::parse(v).ok())
        .map(|v| format!("{}.{}", v.major, v.minor))
        .unwrap_or_else(|| "none".to_string());

    // Capture request size (approximate from Content-Length header)
    let request_size = parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let request = Request::from_parts(parts, body);

    // Track in-flight requests (BEFORE processing)
    metrics::gauge!(
        "http_requests_in_flight",
        "entity_type" => entity_type.clone(),
        "method" => method.to_string()
    )
    .increment(1.0);

    // Process request
    let response = next.run(request).await;

    // Capture response info
    let duration = start.elapsed();
    let status = response.status().as_u16();

    // Application error code, if the response came from a coded `ApiError`.
    // Bounded snake_case value (or "none"); see `MetricErrorCode`.
    let error_code = response
        .extensions()
        .get::<crate::server::shared::types::api::MetricErrorCode>()
        .map(|c| c.0)
        .unwrap_or("none");

    // Capture response size (approximate from Content-Length header)
    let response_size = response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    // Track in-flight requests (AFTER processing - decrement)
    metrics::gauge!(
        "http_requests_in_flight",
        "entity_type" => entity_type.clone(),
        "method" => method.to_string()
    )
    .decrement(1.0);

    // Log the request
    tracing::debug!(
        target: "request_log",
        method = %method,
        path = %path,
        status = status,
        duration_ms = duration.as_millis() as u64,
        ip = %ip,
        entity_type = &entity_type,
        entity_id = entity_id.unwrap_or_default().to_string(),
        request_size = request_size,
        response_size = response_size,
        "request completed"
    );

    // Shared label values
    let method_str = method.to_string();
    let status_class = match status / 100 {
        2 => "2xx",
        3 => "3xx",
        4 => "4xx",
        5 => "5xx",
        _ => "other",
    };
    let entity_type_str = entity_type.to_string();

    // Record metrics.
    // `status` keeps the coarse class (dashboards/alerts filter on it); `status_code`
    // (exact) and `error_code` (coded `ApiError` reason, "none" otherwise) are additive
    // for diagnosing which specific 4xx/5xx is occurring. Both are bounded.
    metrics::counter!(
        "http_requests_total",
        "method" => method_str.clone(),
        "path" => normalized_path.clone(),
        "status" => status_class.to_string(),
        "status_code" => status.to_string(),
        "error_code" => error_code.to_string(),
        "entity_type" => entity_type_str.clone(),
        "daemon_version" => daemon_version_label
    )
    .increment(1);

    metrics::histogram!(
        "http_request_duration_seconds",
        "method" => method_str.clone(),
        "path" => normalized_path.clone(),
        "entity_type" => entity_type_str.clone()
    )
    .record(duration.as_secs_f64());

    // Track request/response sizes
    if request_size > 0 {
        metrics::histogram!(
            "http_request_size_bytes",
            "entity_type" => entity_type_str.clone(),
            "method" => method_str.clone()
        )
        .record(request_size as f64);
    }

    if response_size > 0 {
        metrics::histogram!(
            "http_response_size_bytes",
            "entity_type" => entity_type_str.clone(),
            "status" => status_class.to_string()
        )
        .record(response_size as f64);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_sveltekit_immutable() {
        // SvelteKit immutable assets with cache-busted hashes
        assert_eq!(
            normalize_path_for_metrics("/_app/immutable/chunks/Bl4lrTMV.js"),
            "static_immutable"
        );
        assert_eq!(
            normalize_path_for_metrics("/_app/immutable/nodes/0.Dv1n4FpZ.js"),
            "static_immutable"
        );
        assert_eq!(
            normalize_path_for_metrics("/_app/immutable/assets/app.Cx2a3bYz.css"),
            "static_immutable"
        );
    }

    #[test]
    fn test_normalize_path_static_files() {
        // Common static file extensions
        assert_eq!(normalize_path_for_metrics("/favicon.ico"), "static_file");
        assert_eq!(normalize_path_for_metrics("/logo.png"), "static_file");
        assert_eq!(normalize_path_for_metrics("/styles.css"), "static_file");
        assert_eq!(normalize_path_for_metrics("/bundle.js"), "static_file");
        assert_eq!(normalize_path_for_metrics("/font.woff2"), "static_file");
        assert_eq!(normalize_path_for_metrics("/data.json"), "static_file");
    }

    #[test]
    fn test_normalize_path_api_routes_unchanged() {
        // API routes should pass through unchanged
        assert_eq!(normalize_path_for_metrics("/api/hosts"), "/api/hosts");
        assert_eq!(
            normalize_path_for_metrics("/api/hosts/:id"),
            "/api/hosts/:id"
        );
        assert_eq!(
            normalize_path_for_metrics("/api/networks/:network_id/hosts"),
            "/api/networks/:network_id/hosts"
        );
        assert_eq!(normalize_path_for_metrics("/api/metrics"), "/api/metrics");
    }

    #[test]
    fn test_normalize_path_html_routes_unchanged() {
        // HTML page routes (no extension) should pass through unchanged
        assert_eq!(normalize_path_for_metrics("/"), "/");
        assert_eq!(normalize_path_for_metrics("/login"), "/login");
        assert_eq!(normalize_path_for_metrics("/dashboard"), "/dashboard");
        assert_eq!(
            normalize_path_for_metrics("/networks/:id/hosts"),
            "/networks/:id/hosts"
        );
    }
}
