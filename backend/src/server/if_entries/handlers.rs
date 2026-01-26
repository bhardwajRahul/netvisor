use axum::Json;
use axum::extract::{Path, State};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member};
use crate::server::config::AppState;
use crate::server::hosts::r#impl::base::Host;
use crate::server::if_entries::r#impl::base::{IfEntry, Neighbor};
use crate::server::if_entries::service::IfEntryService;
use crate::server::shared::handlers::query::HostChildQuery;
use crate::server::shared::handlers::traits::{CrudHandlers, create_handler, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult};

impl CrudHandlers for IfEntry {
    type Service = IfEntryService;
    type FilterQuery = HostChildQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.if_entry_service
    }
}

// Generated handlers
mod generated {
    use super::*;
    crate::crud_get_all_handler!(IfEntry);
    crate::crud_get_by_id_handler!(IfEntry);
    crate::crud_delete_handler!(IfEntry);
    crate::crud_bulk_delete_handler!(IfEntry);
    crate::crud_export_csv_handler!(IfEntry);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_if_entry))
        .routes(routes!(generated::export_csv))
        .routes(routes!(
            generated::get_by_id,
            update_if_entry,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
}

/// Validate that if entry's host is on the same network as the entry
async fn validate_if_entry_network_consistency(
    state: &AppState,
    if_entry: &IfEntry,
) -> Result<(), ApiError> {
    if let Some(host) = state
        .services
        .host_service
        .get_by_id(&if_entry.base.host_id)
        .await?
        && host.base.network_id != if_entry.base.network_id
    {
        return Err(ApiError::entity_network_mismatch::<Host>());
    }

    Ok(())
}

/// Validate Neighbor::Host references (requires HostService access, not available in IfEntryService)
async fn validate_neighbor_host(state: &AppState, if_entry: &IfEntry) -> Result<(), ApiError> {
    if let Some(Neighbor::Host(neighbor_host_id)) = &if_entry.base.neighbor {
        // Cannot connect to self (same host)
        if *neighbor_host_id == if_entry.base.host_id {
            return Err(ApiError::bad_request(
                "IfEntry cannot have a neighbor pointing to its own host",
            ));
        }

        // Verify the neighbor host exists and is in the same network
        let neighbor_host = state
            .services
            .host_service
            .get_by_id(neighbor_host_id)
            .await?
            .ok_or_else(|| ApiError::bad_request("neighbor references a non-existent host"))?;

        if neighbor_host.base.network_id != if_entry.base.network_id {
            return Err(ApiError::bad_request(
                "neighbor host must be in the same network",
            ));
        }
    }

    Ok(())
}

/// Create a new IfEntry
///
/// Creates an SNMP ifTable entry for a host. These are typically created by
/// SNMP discovery, but can also be created manually.
#[utoipa::path(
    post,
    path = "",
    tag = IfEntry::ENTITY_NAME_PLURAL,
    request_body = IfEntry,
    responses(
        (status = 200, description = "If entry created successfully", body = ApiResponse<IfEntry>),
        (status = 400, description = "Network mismatch or duplicate if_index", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn create_if_entry(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(if_entry): Json<IfEntry>,
) -> ApiResult<Json<ApiResponse<IfEntry>>> {
    validate_if_entry_network_consistency(&state, &if_entry).await?;
    state
        .services
        .if_entry_service
        .validate_relationships(&if_entry)
        .await
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;
    validate_neighbor_host(&state, &if_entry).await?;
    create_handler::<IfEntry>(State(state), auth, Json(if_entry)).await
}

/// Update an IfEntry
#[utoipa::path(
    put,
    path = "/{id}",
    tag = IfEntry::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "If entry ID")),
    request_body = IfEntry,
    responses(
        (status = 200, description = "If entry updated successfully", body = ApiResponse<IfEntry>),
        (status = 400, description = "Network mismatch or invalid request", body = ApiErrorResponse),
        (status = 404, description = "If entry not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_if_entry(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
    Json(if_entry): Json<IfEntry>,
) -> ApiResult<Json<ApiResponse<IfEntry>>> {
    validate_if_entry_network_consistency(&state, &if_entry).await?;
    state
        .services
        .if_entry_service
        .validate_relationships(&if_entry)
        .await
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;
    validate_neighbor_host(&state, &if_entry).await?;
    update_handler::<IfEntry>(State(state), auth, path, Json(if_entry)).await
}
