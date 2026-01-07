use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, IsDaemon, Member, Or, Viewer};
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::query::FilterQueryExtractor;
use crate::server::shared::handlers::traits::{
    BulkDeleteResponse, bulk_delete_handler, delete_handler,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::EntityFilter;
use crate::server::shared::types::api::{ApiErrorResponse, EmptyApiResponse};
use crate::server::shared::validation::{validate_network_access, validate_read_access};
use crate::server::{
    config::AppState,
    hosts::r#impl::{
        api::{CreateHostRequest, DiscoveryHostRequest, HostResponse, UpdateHostRequest},
        base::Host,
        legacy::{HostCreateRequestBody, HostCreateResponse, LegacyHostWithServicesResponse},
    },
    shared::types::api::{ApiError, ApiResponse, ApiResult},
};
use axum::extract::{Path, State};
use axum::response::Json;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;
use validator::Validate;

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_hosts, create_host))
        .routes(routes!(get_host_by_id, update_host, delete_host))
        .routes(routes!(bulk_delete_hosts))
        .routes(routes!(consolidate_hosts))
        .routes(routes!(create_host_discovery))
}

/// List all hosts
///
/// Returns all hosts the authenticated user has access to, with their
/// interfaces, ports, and services included. Supports pagination via
/// `limit` and `offset` query parameters.
#[utoipa::path(
    get,
    path = "",
    tag = "hosts",
    params(crate::server::shared::handlers::query::NetworkFilterQuery),
    responses(
        (status = 200, description = "List of hosts with their children", body = ApiResponse<Vec<HostResponse>>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<crate::server::shared::handlers::query::NetworkFilterQuery>,
) -> ApiResult<Json<ApiResponse<Vec<HostResponse>>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let base_filter = EntityFilter::unfiltered().network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);

    // Apply pagination
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    let result = state
        .services
        .host_service
        .get_all_host_responses_paginated(filter)
        .await?;

    // Get effective pagination values for response metadata
    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(ApiResponse::success_paginated(
        result.items,
        result.total_count,
        limit,
        offset,
    )))
}

/// Get a host by ID
///
/// Returns a single host with its interfaces, ports, and services.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "hosts",
    params(("id" = Uuid, Path, description = "Host ID")),
    responses(
        (status = 200, description = "Host found", body = ApiResponse<HostResponse>),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_host_by_id(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let mut host = state
        .services
        .host_service
        .get_host_response(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Host {} not found", id)))?;

    validate_read_access(Some(host.network_id), None, &network_ids, organization_id)?;

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[host.id], EntityDiscriminants::Host)
        .await?;
    if let Some(tags) = tags_map.get(&host.id) {
        host.tags = tags.clone();
    }

    Ok(Json(ApiResponse::success(host)))
}

/// Create a new host
///
/// Creates a host with optional interfaces, ports, and services.
/// The `source` field is automatically set to `Manual`.
///
/// ### Tag Validation
///
/// - Tags must exist and belong to your organization
/// - Duplicate tag UUIDs are automatically deduplicated
/// - Invalid or cross-organization tag UUIDs return a 400 error
///
#[utoipa::path(
    post,
    path = "",
    tag = "hosts",
    request_body = CreateHostRequest,
    responses(
        (status = 200, description = "Host created successfully", body = ApiResponse<HostResponse>),
        (status = 400, description = "Validation error: network not found, subnet mismatch, or invalid tags", body = ApiErrorResponse),
        (status = 401, description = "No access to the specified network", body = ApiErrorResponse),
    ),
    security( ("user_api_key" = []),("session" = []), ("daemon_api_key" = []))
)]
async fn create_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Or<Member, IsDaemon>>,
    Json(request): Json<HostCreateRequestBody>,
) -> ApiResult<Json<ApiResponse<HostCreateResponse>>> {
    let network_ids = auth.network_ids();
    let entity = auth.into_entity();
    let host_service = &state.services.host_service;

    match (request, &entity) {
        // New format - standard flow
        (HostCreateRequestBody::New(request), _) => {
            // Validate request (name length, etc.)
            request
                .validate()
                .map_err(|e| ApiError::bad_request(&e.to_string()))?;

            // Validate user has access to the network
            validate_network_access(Some(request.network_id), &network_ids, "create")?;

            // Validate network_id exists
            let _network = state
                .services
                .network_service
                .get_by_id(&request.network_id)
                .await?
                .ok_or_else(|| {
                    ApiError::bad_request(&format!("Network {} not found", request.network_id))
                })?;

            // Check interface subnets are on the same network
            for interface in &request.interfaces {
                if let Some(subnet) = state
                    .services
                    .subnet_service
                    .get_by_id(&interface.subnet_id)
                    .await?
                    && subnet.base.network_id != request.network_id
                {
                    return Err(ApiError::bad_request(&format!(
                        "Host is on network {}, cannot have an interface with a subnet \"{}\" which is on network {}.",
                        request.network_id, subnet.base.name, subnet.base.network_id
                    )));
                }
            }

            let host_response = host_service.create_from_request(request, entity).await?;

            Ok(Json(ApiResponse::success(HostCreateResponse::New(
                host_response,
            ))))
        }

        // Legacy format from daemon - transform and process
        (
            HostCreateRequestBody::Legacy(legacy_request),
            AuthenticatedEntity::Daemon { daemon_id, .. },
        ) => {
            tracing::warn!(
                daemon_id = %daemon_id,
                "Legacy daemon request to POST /api/hosts - daemon should be updated"
            );

            let discovery_request = legacy_request.into_discovery_request();

            // Validate daemon has access to the network
            validate_network_access(
                Some(discovery_request.host.base.network_id),
                &network_ids,
                "create",
            )?;

            let DiscoveryHostRequest {
                host,
                interfaces,
                ports,
                services,
            } = discovery_request;

            let host_response = host_service
                .discover_host(host, interfaces, ports, services, entity)
                .await?;

            let legacy_response = LegacyHostWithServicesResponse::from_host_response(host_response);

            Ok(Json(ApiResponse::success(HostCreateResponse::Legacy(
                legacy_response,
            ))))
        }

        // Legacy format from non-daemon (user) - reject
        (HostCreateRequestBody::Legacy(_), _) => Err(ApiError::bad_request(
            "Invalid request format. Please use the CreateHostRequest format.",
        )),
    }
}

/// Update a host
///
/// Updates host properties. Children (interfaces, ports, services)
/// are managed via their own endpoints.
///
/// ### Tag Validation
///
/// - Tags must exist and belong to your organization
/// - Duplicate tag UUIDs are automatically deduplicated
/// - Invalid or cross-organization tag UUIDs return a 400 error
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "hosts",
    params(("id" = Uuid, Path, description = "Host ID")),
    request_body = UpdateHostRequest,
    responses(
        (status = 200, description = "Host updated", body = ApiResponse<HostResponse>),
        (status = 400, description = "Validation error: invalid tags", body = ApiErrorResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(mut request): Json<UpdateHostRequest>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    // Validate request (name length, etc.)
    request
        .validate()
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;

    let host_service = &state.services.host_service;

    // Path ID is canonical - override any ID in the body
    request.id = id;

    // Fetch existing host to validate network access
    let existing_host = host_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Host {} not found", id)))?;

    validate_read_access(
        Some(existing_host.base.network_id),
        None,
        &network_ids,
        organization_id,
    )?;

    let mut host_response = host_service
        .update_from_request(request, auth.into_entity())
        .await?;

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[host_response.id], EntityDiscriminants::Host)
        .await?;
    if let Some(tags) = tags_map.get(&host_response.id) {
        host_response.tags = tags.clone();
    }

    Ok(Json(ApiResponse::success(host_response)))
}

/// Internal endpoint for daemon discovery
///
/// Used by daemons to report discovered hosts. Accepts full entities with
/// pre-generated IDs. Uses upsert behavior to merge with existing hosts.
///
/// Tagged as "internal" - included in OpenAPI spec for client generation
/// but hidden from public documentation.
#[utoipa::path(
    post,
    path = "/discovery",
    tags = ["hosts", "internal"],
    request_body = DiscoveryHostRequest,
    responses(
        (status = 200, description = "Host discovered/updated successfully", body = ApiResponse<HostResponse>),
        (status = 403, description = "Daemon cannot create hosts on other networks", body = ApiErrorResponse),
    ),
    security(("daemon_api_key" = []))
)]
async fn create_host_discovery(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsDaemon>,
    Json(request): Json<DiscoveryHostRequest>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let host_service = &state.services.host_service;

    let DiscoveryHostRequest {
        host,
        interfaces,
        ports,
        services,
    } = request;

    // Get daemon network_id from entity
    let daemon_network_id = auth
        .network_ids()
        .first()
        .copied()
        .ok_or_else(|| ApiError::forbidden("Daemon has no network assignment"))?;

    if host.base.network_id != daemon_network_id {
        return Err(ApiError::forbidden(
            "Daemon cannot create hosts on networks it's not assigned to",
        ));
    }

    let host_response = host_service
        .discover_host(host, interfaces, ports, services, auth.into_entity())
        .await?;

    Ok(Json(ApiResponse::success(host_response)))
}

/// Consolidate hosts
///
/// Merges all interfaces, ports, and services from `other_host` into
/// `destination_host`, then deletes `other_host`. Both hosts must be
/// on the same network.
///
/// ### Merge Behavior
///
/// - **Interfaces**: Transferred to destination. If an interface with matching subnet+IP or MAC
///   already exists on destination, bindings are remapped to use the existing interface.
/// - **Ports**: Transferred to destination. If a port with the same number and protocol already
///   exists, bindings are remapped to use the existing port.
/// - **Services**: Transferred to destination with deduplication.
///   See [upsert behavior](https://scanopy.net/docs/discovery/#upsert-behavior) for details.
///
/// ### Restrictions
///
/// - Cannot consolidate a host with itself.
/// - Cannot consolidate a host that has a daemon - consolidate into it instead.
#[utoipa::path(
    put,
    path = "/{destination_host}/consolidate/{other_host}",
    tag = "hosts",
    params(
        ("destination_host" = Uuid, Path, description = "Destination host ID - will receive all children"),
        ("other_host" = Uuid, Path, description = "Host to merge into destination - will be deleted")
    ),
    responses(
        (status = 200, description = "Hosts consolidated successfully", body = ApiResponse<HostResponse>),
        (status = 404, description = "One or both hosts not found", body = ApiErrorResponse),
        (status = 400, description = "Validation error: same host, has daemon, or different networks", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn consolidate_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path((destination_host_id, other_host_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<ApiResponse<HostResponse>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    let host_service = &state.services.host_service;

    let destination_host = host_service
        .get_by_id(&destination_host_id)
        .await?
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Could not find destination host {}",
                destination_host_id
            ))
        })?;
    let other_host = host_service
        .get_by_id(&other_host_id)
        .await?
        .ok_or_else(|| {
            ApiError::not_found(format!(
                "Could not find host to consolidate {}",
                other_host_id
            ))
        })?;

    // Validate user has access to both hosts
    validate_read_access(
        Some(destination_host.base.network_id),
        None,
        &network_ids,
        organization_id,
    )?;
    validate_read_access(
        Some(other_host.base.network_id),
        None,
        &network_ids,
        organization_id,
    )?;

    // Make sure hosts are on same network
    if destination_host.base.network_id != other_host.base.network_id {
        return Err(ApiError::bad_request(&format!(
            "Destination Host is on network {}, other host \"{}\" can't be on a different network ({}).",
            destination_host.base.network_id, other_host.base.name, other_host.base.network_id
        )));
    }

    let mut host_response = host_service
        .consolidate_hosts(destination_host, other_host, auth.into_entity())
        .await?;

    // Hydrate tags from junction table
    let tags_map = state
        .services
        .entity_tag_service
        .get_tags_map(&[host_response.id], EntityDiscriminants::Host)
        .await?;
    if let Some(tags) = tags_map.get(&host_response.id) {
        host_response.tags = tags.clone();
    }

    Ok(Json(ApiResponse::success(host_response)))
}

/// Delete a host
///
/// Prevents deletion if the host has a daemon associated with it
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "hosts",
    params(
        ("id" = Uuid, Path, description = "Host ID")
    ),
    responses(
        (status = 200, description = "Host deleted", body = EmptyApiResponse),
        (status = 404, description = "Host not found", body = ApiErrorResponse),
        (status = 409, description = "Host has associated daemon", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn delete_host(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    // Pre-validation: Can't delete a host with an associated daemon
    let host_filter = EntityFilter::unfiltered().host_id(&id);
    if state
        .services
        .daemon_service
        .get_one(host_filter)
        .await?
        .is_some()
    {
        return Err(ApiError::conflict(
            "Can't delete a host with an associated daemon. Delete the daemon first.",
        ));
    }

    // Delegate to generic handler (handles auth checks, deletion)
    delete_handler::<Host>(State(state), auth, Path(id)).await
}

/// Bulk delete hosts
///
/// Deletes multiple hosts in a single request. The request body should be
/// an array of host IDs to delete. Fails if any host has an associated daemon.
#[utoipa::path(
    post,
    path = "/bulk-delete",
    tag = "hosts",
    request_body(content = Vec<Uuid>, description = "Array of host IDs to delete"),
    responses(
        (status = 200, description = "Hosts deleted successfully", body = ApiResponse<BulkDeleteResponse>),
        (status = 409, description = "One or more hosts has an associated daemon - delete daemons first", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn bulk_delete_hosts(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(ids): Json<Vec<Uuid>>,
) -> ApiResult<Json<ApiResponse<BulkDeleteResponse>>> {
    let daemon_service = &state.services.daemon_service;

    let host_filter = EntityFilter::unfiltered().host_ids(&ids);

    if !daemon_service.get_all(host_filter).await?.is_empty() {
        return Err(ApiError::conflict(
            "One or more hosts has an associated daemon, and can't be deleted. Delete the daemon(s) first.",
        ));
    }

    bulk_delete_handler::<Host>(axum::extract::State(state), auth, axum::extract::Json(ids)).await
}
