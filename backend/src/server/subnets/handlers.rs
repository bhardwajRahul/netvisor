use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, IsDaemon, Member, Or, Viewer};
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::query::{FilterQueryExtractor, NetworkFilterQuery};
use crate::server::shared::handlers::traits::{CrudHandlers, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::EntityFilter;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiJson, ApiResponse, ApiResult,
};
use crate::server::{config::AppState, subnets::r#impl::base::Subnet};
use axum::extract::{Path, State};
use axum::response::Json;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for most CRUD operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Subnet, "subnets", "subnet");
    crate::crud_delete_handler!(Subnet, "subnets", "subnet");
    crate::crud_bulk_delete_handler!(Subnet, "subnets");
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_subnets, create_subnet))
        .routes(routes!(
            generated::get_by_id,
            update_subnet,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
}

/// Get all subnets
///
/// Returns all subnets accessible to the authenticated user or daemon.
/// Daemons can only access subnets within their assigned network.
#[utoipa::path(
    get,
    path = "",
    tag = "subnets",
    operation_id = "list_subnets",
    summary = "List all subnets",
    params(NetworkFilterQuery),
    responses(
        (status = 200, description = "List of subnets", body = ApiResponse<Vec<Subnet>>),
    ),
    security( ("user_api_key" = []),("session" = []), ("daemon_api_key" = []))
)]
async fn get_all_subnets(
    state: State<Arc<AppState>>,
    auth: Authorized<Or<Viewer, IsDaemon>>,
    query: Query<NetworkFilterQuery>,
) -> ApiResult<Json<ApiResponse<Vec<Subnet>>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth.organization_id();
    let entity = auth.into_entity();

    match entity {
        AuthenticatedEntity::Daemon { network_id, .. } => {
            // Daemons can only access subnets in their network
            // Don't apply pagination for daemon requests
            let filter = EntityFilter::unfiltered().network_ids(&[network_id]);
            let service = Subnet::get_service(&state);
            let result = service.get_all(filter).await.map_err(|e| {
                tracing::error!(
                    error = %e,
                    network_id = %network_id,
                    "Failed to fetch subnets for daemon"
                );
                ApiError::internal_error(&e.to_string())
            })?;
            Ok(Json(ApiResponse::success(result)))
        }
        _ => {
            // Users/API keys - use standard filter with query params
            let org_id = organization_id
                .ok_or_else(|| ApiError::forbidden("Organization context required"))?;
            let base_filter = EntityFilter::unfiltered().network_ids(&network_ids);
            let filter = query.apply_to_filter(base_filter, &network_ids, org_id);
            // Apply pagination
            let pagination = query.pagination();
            let filter = pagination.apply_to_filter(filter);
            let service = Subnet::get_service(&state);
            let result = service.get_paginated(filter).await.map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch subnets");
                ApiError::internal_error(&e.to_string())
            })?;
            let limit = pagination.effective_limit().unwrap_or(0);
            let offset = pagination.effective_offset();
            Ok(Json(ApiResponse::success_paginated(
                result.items,
                result.total_count,
                limit,
                offset,
            )))
        }
    }
}

/// Create a new subnet
#[utoipa::path(
    post,
    path = "",
    tag = "subnets",
    request_body = Subnet,
    responses(
        (status = 200, description = "Subnet created successfully", body = ApiResponse<Subnet>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    ),
    security( ("user_api_key" = []),("session" = []), ("daemon_api_key" = []))
)]
async fn create_subnet(
    state: State<Arc<AppState>>,
    auth: Authorized<Or<Member, IsDaemon>>,
    ApiJson(request): ApiJson<Subnet>,
) -> ApiResult<Json<ApiResponse<Subnet>>> {
    let network_ids = auth.network_ids();
    let entity = auth.into_entity();

    tracing::debug!(
        subnet_name = %request.base.name,
        subnet_cidr = %request.base.cidr,
        network_id = %request.base.network_id,
        entity_id = %entity.entity_id(),
        "Subnet create request received"
    );

    if let Err(err) = request.validate() {
        tracing::warn!(
            subnet_name = %request.base.name,
            subnet_cidr = %request.base.cidr,
            entity_id = %entity.entity_id(),
            error = %err,
            "Subnet validation failed"
        );
        return Err(ApiError::bad_request(&format!(
            "Subnet validation failed: {}",
            err
        )));
    }

    let created = match &entity {
        AuthenticatedEntity::Daemon { network_id, .. } => {
            if *network_id == request.base.network_id {
                let service = Subnet::get_service(&state);
                let created = service.create(request, entity).await.map_err(|e| {
                    tracing::error!(
                        error = %e,
                        "Failed to create subnet"
                    );
                    ApiError::internal_error(&e.to_string())
                })?;
                Json(ApiResponse::success(created))
            } else {
                return Err(ApiError::bad_request(
                    "Daemon cannot create subnets on other networks",
                ));
            }
        }
        _ => {
            // User/API key - validate network access and create
            if !network_ids.contains(&request.base.network_id) {
                return Err(ApiError::forbidden("You don't have access to this network"));
            }
            let service = Subnet::get_service(&state);
            let created = service.create(request, entity).await.map_err(|e| {
                tracing::error!(error = %e, "Failed to create subnet");
                ApiError::internal_error(&e.to_string())
            })?;
            Json(ApiResponse::success(created))
        }
    };

    Ok(created)
}

/// Update a subnet
///
/// Updates subnet properties. If the CIDR is being changed, validates that
/// all existing interfaces on this subnet have IPs within the new CIDR range.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "subnets",
    params(("id" = Uuid, Path, description = "Subnet ID")),
    request_body = Subnet,
    responses(
        (status = 200, description = "Subnet updated", body = ApiResponse<Subnet>),
        (status = 400, description = "CIDR change would orphan existing interfaces", body = ApiErrorResponse),
        (status = 404, description = "Subnet not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_subnet(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    ApiJson(subnet): ApiJson<Subnet>,
) -> ApiResult<Json<ApiResponse<Subnet>>> {
    // Check if CIDR is being changed
    let current = state
        .services
        .subnet_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::not_found(format!("Subnet {} not found", id)))?;

    if current.base.cidr != subnet.base.cidr {
        // CIDR is changing - validate that all existing interfaces are within the new CIDR
        let filter = EntityFilter::unfiltered().subnet_id(&id);
        let interfaces = state
            .services
            .interface_service
            .get_all(filter)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        for interface in &interfaces {
            if !subnet.base.cidr.contains(&interface.base.ip_address) {
                return Err(ApiError::bad_request(&format!(
                    "Cannot change CIDR to {}: interface \"{}\" has IP {} which would be outside the new range",
                    subnet.base.cidr,
                    interface.base.name.as_deref().unwrap_or("unnamed"),
                    interface.base.ip_address
                )));
            }
        }
    }

    // Delegate to generic handler
    update_handler::<Subnet>(State(state), auth, Path(id), Json(subnet)).await
}
