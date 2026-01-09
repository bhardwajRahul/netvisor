use crate::server::{
    auth::middleware::{
        features::{BlockedInDemoMode, RequireFeature},
        permissions::{Authorized, Member},
    },
    config::AppState,
    daemon_api_keys::r#impl::{api::DaemonApiKeyResponse, base::DaemonApiKey},
    shared::{
        api_key_common::{ApiKeyService, ApiKeyType, generate_api_key_for_storage},
        events::types::{TelemetryEvent, TelemetryOperation},
        handlers::traits::{CrudHandlers, update_handler},
        services::traits::{CrudService, EventBusService},
        storage::traits::Entity,
        types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult},
        validation::validate_network_access,
    },
};
use axum::{
    Json,
    extract::{Path, State},
};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use chrono::Utc;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

mod generated {
    use super::*;
    crate::crud_get_all_handler!(DaemonApiKey, "daemon_api_keys", "daemon_api_key");
    crate::crud_get_by_id_handler!(DaemonApiKey, "daemon_api_keys", "daemon_api_key");
    crate::crud_delete_handler!(DaemonApiKey, "daemon_api_keys", "daemon_api_key");
    crate::crud_bulk_delete_handler!(DaemonApiKey, "daemon_api_keys");
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_daemon_api_key))
        .routes(routes!(generated::get_by_id, generated::delete))
        .routes(routes!(update_daemon_api_key))
        .routes(routes!(rotate_key_handler))
        .routes(routes!(generated::bulk_delete))
}

/// Create daemon API key
#[utoipa::path(
    post,
    path = "",
    tag = "daemon_api_keys",
    responses(
        (status = 200, description = "Daemon API key created", body = ApiResponse<DaemonApiKeyResponse>),
        (status = 400, description = "Bad request", body = ApiErrorResponse),
        (status = 403, description = "Insufficient permissions (member+ required)", body = ApiErrorResponse),
        (status = 500, description = "Internal server error", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_daemon_api_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    _demo_check: RequireFeature<BlockedInDemoMode>,
    Json(mut api_key): Json<DaemonApiKey>,
) -> ApiResult<Json<ApiResponse<DaemonApiKeyResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;
    let user_id = auth.user_id();

    tracing::debug!(
        api_key_name = %api_key.base.name,
        network_id = %api_key.base.network_id,
        user_id = ?user_id,
        "Daemon API key create request received"
    );

    validate_network_access(Some(api_key.base.network_id), &network_ids, "create")?;

    let (plaintext, hashed) = generate_api_key_for_storage(ApiKeyType::Daemon);

    let service = DaemonApiKey::get_service(&state);
    api_key.base.key = hashed;
    let entity = auth.into_entity();
    let api_key = service.create(api_key, entity.clone()).await.map_err(|e| {
        tracing::error!(
            error = %e,
            user_id = ?user_id,
            "Failed to create daemon API key"
        );
        ApiError::internal_error(&e.to_string())
    })?;

    let organization = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?;

    if let Some(organization) = organization
        && organization.not_onboarded(&TelemetryOperation::FirstApiKeyCreated)
    {
        service
            .event_bus()
            .publish_telemetry(TelemetryEvent {
                id: Uuid::new_v4(),
                organization_id,
                operation: TelemetryOperation::FirstApiKeyCreated,
                timestamp: Utc::now(),
                metadata: serde_json::json!({
                    "is_onboarding_step": true
                }),
                authentication: entity,
            })
            .await?;
    }

    Ok(Json(ApiResponse::success(DaemonApiKeyResponse {
        key: plaintext,
        api_key,
    })))
}

/// Update a daemon API key
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "daemon_api_keys",
    params(("id" = Uuid, Path, description = "Daemon API key ID")),
    responses(
        (status = 200, description = "Daemon API key updated", body = ApiResponse<DaemonApiKey>),
        (status = 404, description = "Daemon API key not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn update_daemon_api_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(mut request): Json<DaemonApiKey>,
) -> ApiResult<Json<ApiResponse<DaemonApiKey>>> {
    let network_ids = auth.network_ids();

    // Fetch existing to preserve immutable fields
    let existing = DaemonApiKey::get_service(&state)
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Daemon API key '{}' not found", id)))?;

    // Validate user has access to this key's network
    validate_network_access(Some(existing.base.network_id), &network_ids, "update")?;

    // Preserve the key hash - don't allow it to be changed via update
    request.preserve_immutable_fields(&existing);

    // Delegate to generic handler
    update_handler::<DaemonApiKey>(State(state), auth, Path(id), Json(request)).await
}

/// Rotate a daemon API key
#[utoipa::path(
    post,
    path = "/{id}/rotate",
    tag = "daemon_api_keys",
    params(("id" = Uuid, Path, description = "Daemon API key ID")),
    responses(
        (status = 200, description = "Daemon API key rotated, returns new key", body = ApiResponse<String>),
        (status = 404, description = "Daemon API key not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn rotate_key_handler(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    _demo_check: RequireFeature<BlockedInDemoMode>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    Path(api_key_id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let user_agent = user_agent.map(|u| u.to_string());
    let user_id = auth.user_id();

    let service = DaemonApiKey::get_service(&state);
    let key = service
        .rotate_key(api_key_id, ip, user_agent, auth.into_entity())
        .await
        .map_err(|e| {
            tracing::error!(
                api_key_id = %api_key_id,
                user_id = ?user_id,
                error = %e,
                "Failed to rotate daemon API key"
            );
            ApiError::internal_error(&e.to_string())
        })?;

    Ok(Json(ApiResponse::success(key)))
}
