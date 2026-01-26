use crate::server::auth::middleware::permissions::{Admin, Authorized, Member, Viewer};
use crate::server::shared::entities::{EntityDiscriminants, is_entity_taggable};
use crate::server::shared::handlers::ordering::OrderField;
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams,
};
use crate::server::shared::handlers::traits::create_handler;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable, Storage};
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, PaginatedApiResponse};
use crate::server::tags::r#impl::base::Tag;
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult, EmptyApiResponse},
};
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// ============================================================================
// Tag Ordering
// ============================================================================

/// Fields that tags can be ordered/grouped by.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TagOrderField {
    #[default]
    CreatedAt,
    Name,
    Color,
    UpdatedAt,
}

impl OrderField for TagOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "tags.created_at",
            Self::Name => "tags.name",
            Self::Color => "tags.color",
            Self::UpdatedAt => "tags.updated_at",
        }
    }
}

// ============================================================================
// Tag Filter Query
// ============================================================================

/// Query parameters for filtering and ordering tags.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct TagFilterQuery {
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<TagOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<TagOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl TagFilterQuery {
    /// Build the ORDER BY clause.
    pub fn apply_ordering(&self, filter: StorableFilter<Tag>) -> (StorableFilter<Tag>, String) {
        crate::server::shared::handlers::ordering::apply_ordering(
            self.group_by,
            self.order_by,
            self.order_direction,
            filter,
            "tags.created_at ASC",
        )
    }
}

impl FilterQueryExtractor for TagFilterQuery {
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

// Generated handlers for most CRUD operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Tag);
    crate::crud_update_handler!(Tag);
    crate::crud_delete_handler!(Tag);
    crate::crud_bulk_delete_handler!(Tag);
    crate::crud_export_csv_handler!(Tag);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_tags, create_tag))
        .routes(routes!(
            generated::get_by_id,
            generated::update,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
        // Entity tag assignment routes
        .routes(routes!(bulk_add_tag))
        .routes(routes!(bulk_remove_tag))
        .routes(routes!(set_entity_tags))
}

/// List all tags
///
/// Returns all tags in the authenticated user's organization.
/// Supports pagination via `limit` and `offset` query parameters,
/// and ordering via `group_by`, `order_by`, and `order_direction`.
#[utoipa::path(
    get,
    path = "",
    tag = Tag::ENTITY_NAME_PLURAL,
    params(TagFilterQuery),
    responses(
        (status = 200, description = "List of tags", body = PaginatedApiResponse<Tag>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_tags(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    crate::server::shared::extractors::Query(query): crate::server::shared::extractors::Query<
        TagFilterQuery,
    >,
) -> ApiResult<Json<PaginatedApiResponse<Tag>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let base_filter = StorableFilter::<Tag>::new().organization_id(&organization_id);

    // Apply pagination
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(base_filter);

    // Apply ordering
    let (filter, order_by) = query.apply_ordering(filter);

    let result = state
        .services
        .tag_service
        .storage()
        .get_paginated(filter, &order_by)
        .await?;

    // Get effective pagination values for response metadata
    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        result.items,
        result.total_count,
        limit,
        offset,
    )))
}

/// Create a new tag
///
/// Creates a tag scoped to your organization. Tag names must be unique within the organization.
///
/// ### Validation
///
/// - Name must be 1-100 characters (empty names are rejected)
/// - Name must be unique within your organization
#[utoipa::path(
    post,
    path = "",
    tag = Tag::ENTITY_NAME_PLURAL,
    request_body = Tag,
    responses(
        (status = 200, description = "Tag created successfully", body = ApiResponse<Tag>),
        (status = 400, description = "Validation error: name empty or too long", body = ApiErrorResponse),
        (status = 409, description = "Tag name already exists in this organization", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_tag(
    state: State<Arc<AppState>>,
    auth: Authorized<Admin>,
    Json(tag): Json<Tag>,
) -> ApiResult<Json<ApiResponse<Tag>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;
    let name_filter = StorableFilter::<Tag>::new()
        .organization_id(&organization_id)
        .name(tag.base.name.clone());

    if let Some(existing_with_name) = state.services.tag_service.get_one(name_filter).await? {
        return Err(ApiError::conflict(&format!(
            "Tag names must be unique; a tag named \"{}\" already exists",
            existing_with_name.base.name
        )));
    }

    create_handler::<Tag>(state, auth.into_permission::<Member>(), Json(tag)).await
}

/// Request body for bulk tag operations
#[derive(Debug, Deserialize, ToSchema)]
pub struct BulkTagRequest {
    /// The entity type (e.g., Host, Service, Subnet)
    pub entity_type: EntityDiscriminants,
    /// The IDs of entities to modify
    pub entity_ids: Vec<Uuid>,
    /// The tag ID to add or remove
    pub tag_id: Uuid,
}

/// Response for bulk tag operations
#[derive(Debug, Serialize, ToSchema)]
pub struct BulkTagResponse {
    /// Number of entities affected
    pub affected_count: usize,
}

/// Request body for setting all tags on an entity
#[derive(Debug, Deserialize, ToSchema)]
pub struct SetTagsRequest {
    /// The entity type (e.g., Host, Service, Subnet)
    pub entity_type: EntityDiscriminants,
    /// The entity ID
    pub entity_id: Uuid,
    /// The new list of tag IDs
    pub tag_ids: Vec<Uuid>,
}

/// Bulk add a tag to multiple entities
///
/// Adds a single tag to multiple entities of the same type. This is useful for batch tagging operations.
///
/// ### Validation
///
/// - Entity type must be taggable (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
/// - Tag must exist and belong to your organization
/// - Entities that already have the tag are silently skipped
#[utoipa::path(
    post,
    path = "/assign/bulk-add",
    tag = Tag::ENTITY_NAME_PLURAL,
    request_body = BulkTagRequest,
    responses(
        (status = 200, description = "Tag added successfully", body = ApiResponse<BulkTagResponse>),
        (status = 400, description = "Invalid entity type or tag", body = ApiErrorResponse),
        (status = 404, description = "Tag not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_add_tag(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(request): Json<BulkTagRequest>,
) -> ApiResult<Json<ApiResponse<BulkTagResponse>>> {
    // Validate entity type is taggable
    if !is_entity_taggable(request.entity_type) {
        return Err(ApiError::bad_request(&format!(
            "Entity type {:?} does not support tagging",
            request.entity_type
        )));
    }

    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let affected_count = state
        .services
        .entity_tag_service
        .bulk_add_tag(
            &request.entity_ids,
            request.entity_type,
            request.tag_id,
            organization_id,
        )
        .await?;

    Ok(Json(ApiResponse::success(BulkTagResponse {
        affected_count,
    })))
}

/// Bulk remove a tag from multiple entities
///
/// Removes a single tag from multiple entities of the same type.
///
/// ### Validation
///
/// - Entity type must be taggable (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
/// - Entities that don't have the tag are silently skipped
#[utoipa::path(
    post,
    path = "/assign/bulk-remove",
    tag = Tag::ENTITY_NAME_PLURAL,
    request_body = BulkTagRequest,
    responses(
        (status = 200, description = "Tag removed successfully", body = ApiResponse<BulkTagResponse>),
        (status = 400, description = "Invalid entity type", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_remove_tag(
    State(state): State<Arc<AppState>>,
    _auth: Authorized<Member>,
    Json(request): Json<BulkTagRequest>,
) -> ApiResult<Json<ApiResponse<BulkTagResponse>>> {
    // Validate entity type is taggable
    if !is_entity_taggable(request.entity_type) {
        return Err(ApiError::bad_request(&format!(
            "Entity type {:?} does not support tagging",
            request.entity_type
        )));
    }

    let affected_count = state
        .services
        .entity_tag_service
        .bulk_remove_tag(&request.entity_ids, request.entity_type, request.tag_id)
        .await?;

    Ok(Json(ApiResponse::success(BulkTagResponse {
        affected_count,
    })))
}

/// Set all tags for an entity
///
/// Replaces all tags on an entity with the provided list.
///
/// ### Validation
///
/// - Entity type must be taggable (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
/// - All tags must exist and belong to your organization
#[utoipa::path(
    put,
    path = "/assign",
    tag = Tag::ENTITY_NAME_PLURAL,
    request_body = SetTagsRequest,
    responses(
        (status = 200, description = "Tags set successfully", body = EmptyApiResponse),
        (status = 400, description = "Invalid entity type or tag", body = ApiErrorResponse),
        (status = 404, description = "Tag not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn set_entity_tags(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(request): Json<SetTagsRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Validate entity type is taggable
    if !is_entity_taggable(request.entity_type) {
        return Err(ApiError::bad_request(&format!(
            "Entity type {:?} does not support tagging",
            request.entity_type
        )));
    }

    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    state
        .services
        .entity_tag_service
        .set_tags(
            request.entity_id,
            request.entity_type,
            request.tag_ids,
            organization_id,
        )
        .await?;

    Ok(Json(ApiResponse::success(())))
}
