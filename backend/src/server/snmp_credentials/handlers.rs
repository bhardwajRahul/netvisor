use crate::server::auth::middleware::permissions::{Admin, Authorized, Viewer};
use crate::server::shared::events::types::{TelemetryEvent, TelemetryOperation};
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, CrudHandlers, bulk_delete_handler, create_handler, delete_handler,
    update_handler,
};
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, EmptyApiResponse, PaginatedApiResponse,
};
use crate::server::snmp_credentials::r#impl::base::SnmpCredential;
use crate::server::snmp_credentials::service::SnmpCredentialService;
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult},
};
use axum::{extract::State, response::Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

impl CrudHandlers for SnmpCredential {
    type Service = SnmpCredentialService;
    type FilterQuery = SnmpCredentialFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.snmp_credential_service
    }
}

// ============================================================================
// SNMP Credential Ordering
// ============================================================================

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SnmpCredentialOrderField {
    #[default]
    CreatedAt,
    Name,
    Version,
    UpdatedAt,
}

impl OrderField for SnmpCredentialOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "snmp_credentials.created_at",
            Self::Name => "snmp_credentials.name",
            Self::Version => "snmp_credentials.version",
            Self::UpdatedAt => "snmp_credentials.updated_at",
        }
    }
}

// ============================================================================
// SNMP Credential Filter Query
// ============================================================================

#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct SnmpCredentialFilterQuery {
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<SnmpCredentialOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<SnmpCredentialOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl SnmpCredentialFilterQuery {
    pub fn apply_ordering(
        &self,
        filter: StorableFilter<SnmpCredential>,
    ) -> (StorableFilter<SnmpCredential>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "snmp_credentials.created_at ASC",
        )
    }
}

impl FilterQueryExtractor for SnmpCredentialFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        _user_network_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        filter
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

// Generated handler for read-only operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(SnmpCredential);
    crate::crud_export_csv_handler!(SnmpCredential);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_snmp_credentials, create_snmp_credential))
        .routes(routes!(generated::export_csv))
        .routes(routes!(
            generated::get_by_id,
            update_snmp_credential,
            delete_snmp_credential
        ))
        .routes(routes!(bulk_delete_snmp_credentials))
}

/// Update SNMP Credential
#[utoipa::path(
    put,
    path = "/{id}",
    tag = SnmpCredential::ENTITY_NAME_PLURAL,
    params(
        ("id" = Uuid, Path, description = "snmp_credential ID")
    ),
    request_body = SnmpCredential,
    responses(
        (status = 200, description = "snmp_credential updated successfully", body = ApiResponse<SnmpCredential>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
        (status = 404, description = "snmp_credential not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_snmp_credential(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    id: axum::extract::Path<Uuid>,
    entity: Json<SnmpCredential>,
) -> ApiResult<Json<ApiResponse<SnmpCredential>>> {
    update_handler::<SnmpCredential>(
        state,
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        id,
        entity,
    )
    .await
}

/// Delete SNMP credential
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = SnmpCredential::ENTITY_NAME_PLURAL,
    params(
        ("id" = Uuid, Path, description = "snmp_credential ID")
    ),
    responses(
        (status = 200, description = "snmp_credential deleted successfully", body = EmptyApiResponse),
        (status = 404, description = "snmp_credential not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_snmp_credential(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    id: axum::extract::Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    delete_handler::<SnmpCredential>(
        state,
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        id,
    )
    .await
}

/// Bulk delete SNMP Credential
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = SnmpCredential::ENTITY_NAME_PLURAL,
    request_body = Vec<Uuid>,
    responses(
        (status = 200, description = "SNMP Credentials deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn bulk_delete_snmp_credentials(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    ids: Json<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    bulk_delete_handler::<SnmpCredential>(
        state,
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        ids,
    )
    .await
}

/// List all SNMP Credentials
///
/// Returns all SNMP Credentials in the authenticated user's organization.
#[utoipa::path(
    get,
    path = "",
    tag = SnmpCredential::ENTITY_NAME_PLURAL,
    params(SnmpCredentialFilterQuery),
    responses(
        (status = 200, description = "List of SNMP credentials", body = PaginatedApiResponse<SnmpCredential>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_snmp_credentials(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    crate::server::shared::extractors::Query(query): crate::server::shared::extractors::Query<
        SnmpCredentialFilterQuery,
    >,
) -> ApiResult<Json<PaginatedApiResponse<SnmpCredential>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let base_filter = StorableFilter::<SnmpCredential>::new_from_org_id(&organization_id);

    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(base_filter);
    let (filter, order_by) = query.apply_ordering(filter);

    let result = state
        .services
        .snmp_credential_service
        .get_paginated_ordered(filter, &order_by)
        .await?;

    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        result.items,
        result.total_count,
        limit,
        offset,
    )))
}

/// Create a new SNMP Credential
///
/// Creates an SNMP credential scoped to your organization. Credential names must
/// be unique within the organization.
///
/// ### Validation
///
/// - Name must be 1-100 characters
/// - Name must be unique within your organization
/// - Community string is required
#[utoipa::path(
    post,
    path = "",
    tag = SnmpCredential::ENTITY_NAME_PLURAL,
    request_body = SnmpCredential,
    responses(
        (status = 200, description = "SNMP credential created successfully", body = ApiResponse<SnmpCredential>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
        (status = 409, description = "Credential name already exists in this organization", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_snmp_credential(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Admin>,
    Json(credential): Json<SnmpCredential>,
) -> ApiResult<Json<ApiResponse<SnmpCredential>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;
    let entity = auth.entity.clone();

    // Check for duplicate name
    let name_filter = StorableFilter::<SnmpCredential>::new_from_org_id(&organization_id)
        .name(credential.base.name.clone());

    if let Some(existing) = state
        .services
        .snmp_credential_service
        .get_one(name_filter)
        .await?
    {
        return Err(ApiError::conflict(&format!(
            "Credential names must be unique; a credential named \"{}\" already exists",
            existing.base.name
        )));
    }

    let response = create_handler::<SnmpCredential>(
        State(state.clone()),
        auth.into_permission::<crate::server::auth::middleware::permissions::Member>(),
        Json(credential),
    )
    .await?;

    // Emit FirstSnmpCredentialCreated telemetry event if this is the first SNMP credential
    if response.data.is_some() {
        let organization = state
            .services
            .organization_service
            .get_by_id(&organization_id)
            .await?;

        if let Some(organization) = organization
            && organization.not_onboarded(&TelemetryOperation::FirstSnmpCredentialCreated)
        {
            state
                .services
                .snmp_credential_service
                .event_bus()
                .publish_telemetry(TelemetryEvent {
                    id: Uuid::new_v4(),
                    organization_id,
                    operation: TelemetryOperation::FirstSnmpCredentialCreated,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({}),
                    authentication: entity,
                })
                .await?;
        }
    }

    Ok(response)
}
