use axum::{
    extract::{FromRequestParts, MatchedPath, Request, State},
    middleware::Next,
    response::Response,
};
use axum_client_ip::ClientIp;
use reqwest::header;
use std::{sync::Arc, time::Instant};

use crate::server::{auth::middleware::auth::AuthenticatedEntity, config::AppState};

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

    // Extract auth info  
    let (mut parts, body) = request.into_parts();  
    let entity = AuthenticatedEntity::from_request_parts(&mut parts, &state)  
        .await  
        .ok();  

    let (entity_type, entity_id) = entity  
        .map(|e| (e.entity_name(), e.entity_id()))  
        .unwrap_or(("anonymous".to_string(), None));  
    
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
    let status_str = status.to_string();  
    let entity_type_str = entity_type.to_string();  
    let ip_str = ip.to_string();  

    // Record metrics  
    metrics::counter!(  
        "http_requests_total",  
        "method" => method_str.clone(),  
        "path" => path.clone(),  
        "status" => status_str.clone(),  
        "entity_type" => entity_type_str.clone(),  
        "ip" => ip_str.clone()  
    )  
    .increment(1);  

    metrics::histogram!(  
        "http_request_duration_seconds",  
        "method" => method_str.clone(),  
        "path" => path.clone(),  
        "entity_type" => entity_type_str.clone() // ADDED for consistency  
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
            "status" => status_str.clone()  
        )  
        .record(response_size as f64);  
    }  

    // Track unique daemon instances (lower cardinality gauge)  
    if entity_type == "daemon" {  
        if let Some(eid) = entity_id {  
            metrics::gauge!(  
                "daemon_active_ips",  
                "ip" => ip_str,  
                "entity_id" => eid.to_string()  
            )  
            .set(1.0);  
        }  
    }  

    response  
}  