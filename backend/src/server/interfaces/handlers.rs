use crate::server::auth::middleware::permissions::{Authorized, Member};
use crate::server::config::AppState;
use crate::server::hosts::r#impl::base::Host;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::interfaces::service::InterfaceService;
use crate::server::shared::handlers::query::InterfaceQuery;
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, create_handler, update_handler,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
};
use crate::server::shared::validation::{validate_bulk_delete_access, validate_delete_access};
use crate::server::subnets::r#impl::base::Subnet;
use axum::Json;
use axum::extract::{Path, State};
use std::collections::HashSet;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

impl CrudHandlers for Interface {
    type Service = InterfaceService;
    type FilterQuery = InterfaceQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.interface_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_all_handler!(Interface);
    crate::crud_export_csv_handler!(Interface);
    crate::crud_get_by_id_handler!(Interface);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_interface))
        .routes(routes!(
            generated::get_by_id,
            update_interface,
            delete_interface
        ))
        .routes(routes!(bulk_delete_interfaces))
        .routes(routes!(generated::export_csv))
}

/// Validate that interface's host and subnet are on the same network as the interface,
/// and that the IP address falls within the subnet's CIDR range.
async fn validate_interface_consistency(
    state: &AppState,
    interface: &Interface,
) -> Result<(), ApiError> {
    // Validate host is on the same network
    if let Some(host) = state
        .services
        .host_service
        .get_by_id(&interface.base.host_id)
        .await?
        && host.base.network_id != interface.base.network_id
    {
        return Err(ApiError::entity_network_mismatch::<Host>());
    }

    // Validate subnet is on the same network AND IP is within CIDR
    if let Some(subnet) = state
        .services
        .subnet_service
        .get_by_id(&interface.base.subnet_id)
        .await?
    {
        if subnet.base.network_id != interface.base.network_id {
            return Err(ApiError::entity_network_mismatch::<Subnet>());
        }

        // Validate IP address is within subnet CIDR
        if !subnet.base.cidr.contains(&interface.base.ip_address) {
            return Err(ApiError::interface_ip_out_of_range(
                &interface.base.ip_address.to_string(),
                &subnet.base.name,
            ));
        }
    }

    Ok(())
}

/// Create a new interface
/// Position is automatically assigned to the end of the host's interface list.
#[utoipa::path(
    post,
    path = "",
    tag = Interface::ENTITY_NAME_PLURAL,
    request_body = Interface,
    responses(
        (status = 200, description = "Interface created successfully", body = ApiResponse<Interface>),
        (status = 400, description = "Network mismatch or invalid request", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_interface(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(mut interface): Json<Interface>,
) -> ApiResult<Json<ApiResponse<Interface>>> {
    validate_interface_consistency(&state, &interface).await?;

    // Auto-assign position to end of list (ignore any position in the request)
    let next_position = state
        .services
        .interface_service
        .get_next_position_for_host(&interface.base.host_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    interface.base.position = next_position;

    create_handler::<Interface>(State(state), auth, Json(interface)).await
}

/// Update an interface
/// Position must be within valid range and not conflict with other interfaces.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Interface::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Interface ID")),
    request_body = Interface,
    responses(
        (status = 200, description = "Interface updated successfully", body = ApiResponse<Interface>),
        (status = 400, description = "Network mismatch or invalid request", body = ApiErrorResponse),
        (status = 404, description = "Interface not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_interface(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
    Json(interface): Json<Interface>,
) -> ApiResult<Json<ApiResponse<Interface>>> {
    validate_interface_consistency(&state, &interface).await?;

    // Validate position is within range and doesn't conflict
    state
        .services
        .interface_service
        .validate_position_for_update(&path, &interface.base.host_id, interface.base.position)
        .await?;

    update_handler::<Interface>(State(state), auth, path, Json(interface)).await
}

/// Delete an interface
/// Remaining interfaces for the host are renumbered to maintain sequential positions.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Interface::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Interface ID")),
    responses(
        (status = 200, description = "Interface deleted successfully", body = EmptyApiResponse),
        (status = 404, description = "Interface not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn delete_interface(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let entity_auth = auth.into_entity();

    let service = &state.services.interface_service;

    // Fetch entity first to verify ownership and get host_id
    let entity = service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Interface>(id))?;

    validate_delete_access(
        Some(entity.base.network_id),
        None,
        &network_ids,
        organization_id,
    )?;

    let host_id = entity.base.host_id;

    // Delete the interface
    service
        .delete(&id, entity_auth.clone())
        .await
        .map_err(ApiError::from)?;

    // Renumber remaining interfaces for this host
    service
        .renumber_interfaces_for_host(&host_id, entity_auth)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

/// Bulk delete interfaces
/// Remaining interfaces for affected hosts are renumbered to maintain sequential positions.
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = Interface::ENTITY_NAME_PLURAL,
    request_body = Vec<Uuid>,
    responses(
        (status = 200, description = "Interfaces deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 400, description = "No IDs provided", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_delete_interfaces(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(ids): Json<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    if ids.is_empty() {
        return Err(ApiError::bulk_empty());
    }

    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let entity_auth = auth.into_entity();

    let service = &state.services.interface_service;

    // Fetch all entities by the requested IDs
    let entity_filter = StorableFilter::<Interface>::new().entity_ids(&ids);
    let entities = service.get_all(entity_filter).await?;

    // Collect affected host IDs for renumbering
    let affected_host_ids: HashSet<Uuid> = entities.iter().map(|e| e.base.host_id).collect();

    // Verify ownership of ALL entities before deleting any
    for entity in &entities {
        validate_bulk_delete_access(
            Some(entity.base.network_id),
            None,
            &network_ids,
            organization_id,
        )?;
    }

    // Only delete entities that actually exist and user has access to
    let valid_ids: Vec<Uuid> = entities.iter().map(|e| e.id).collect();
    let deleted_count = service
        .delete_many(&valid_ids, entity_auth.clone())
        .await
        .map_err(ApiError::from)?;

    // Renumber remaining interfaces for all affected hosts
    for host_id in affected_host_ids {
        service
            .renumber_interfaces_for_host(&host_id, entity_auth.clone())
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    Ok(Json(ApiResponse::success(BulkDeleteResponse {
        deleted_count,
        requested_count: ids.len(),
    })))
}
