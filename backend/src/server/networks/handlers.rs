use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, bulk_delete_handler, create_handler, delete_handler,
    update_handler,
};
use crate::server::shared::storage::traits::Entity;
use crate::server::{
    auth::middleware::{
        features::{CreateNetworkFeature, RequireFeature},
        permissions::{Admin, Authorized, Member},
    },
    shared::types::api::{ApiErrorResponse, EmptyApiResponse},
};
use crate::server::{
    config::AppState,
    networks::r#impl::Network,
    shared::types::api::{ApiResponse, ApiResult},
};
use axum::extract::{Path, State};
use axum::response::Json;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for operations that use generic CRUD logic
mod generated {
    use super::*;
    crate::crud_get_all_handler!(Network);
    crate::crud_get_by_id_handler!(Network);
    crate::crud_export_csv_handler!(Network);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_network))
        .routes(routes!(
            generated::get_by_id,
            update_network,
            delete_network
        ))
        .routes(routes!(bulk_delete_networks))
        .routes(routes!(generated::export_csv))
}

/// Create a new network
#[utoipa::path(
    post,
    path = "",
    tag = Network::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "Network created", body = ApiResponse<Network>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_network(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    RequireFeature { .. }: RequireFeature<CreateNetworkFeature>,
    Json(network): Json<Network>,
) -> ApiResult<Json<ApiResponse<Network>>> {
    let entity = auth.entity.clone();
    let response = create_handler::<Network>(
        State(state.clone()),
        auth.into_permission::<Member>(),
        Json(network),
    )
    .await?;

    if let Some(network) = &response.data {
        let service = Network::get_service(&state);
        service
            .create_organizational_subnets(network.id, entity)
            .await?;
    }

    Ok(response)
}

/// Update a network
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Network::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Network ID")),
    request_body = Network,
    responses(
        (status = 200, description = "Network updated", body = ApiResponse<Network>),
        (status = 404, description = "Network not found", body = ApiErrorResponse),
        (status = 403, description = "User not admin", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_network(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    path: Path<Uuid>,
    json: Json<Network>,
) -> ApiResult<Json<ApiResponse<Network>>> {
    update_handler::<Network>(state, auth.into_permission::<Member>(), path, json).await
}

/// Delete a network
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Network::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Network ID")),
    responses(
        (status = 200, description = "Network deleted", body = EmptyApiResponse),
        (status = 404, description = "Network not found", body = ApiErrorResponse),
        (status = 403, description = "User not admin", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn delete_network(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    path: Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    delete_handler::<Network>(state, auth.into_permission::<Member>(), path).await
}

/// Bulk delete networks
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = Network::ENTITY_NAME_PLURAL,
    request_body(content = Vec<Uuid>, description = "Array of network IDs to delete"),
    responses(
        (status = 200, description = "Networks deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 403, description = "User not admin", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_delete_networks(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    json: Json<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    bulk_delete_handler::<Network>(state, auth.into_permission::<Member>(), json).await
}
