use crate::server::shared::extractors::Query;
use crate::server::{
    auth::middleware::permissions::{Authorized, IsUser, Member, Viewer},
    config::AppState,
    shared::{
        events::types::{TelemetryEvent, TelemetryOperation},
        handlers::{
            query::{FilterQueryExtractor, NetworkFilterQuery},
            traits::{CrudHandlers, update_handler},
        },
        services::traits::CrudService,
        storage::{filter::EntityFilter, traits::Storable},
        types::api::{
            ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
            PaginatedApiResponse,
        },
    },
    topology::{
        service::main::BuildGraphParams,
        types::base::{SetEntitiesParams, Topology, TopologyRebuildRequest},
    },
};
use axum::{
    extract::{Path, State},
    response::{
        Json, Sse,
        sse::{Event, KeepAlive},
    },
    routing::get,
};
use chrono::Utc;
use futures::{Stream, stream};
use std::{convert::Infallible, sync::Arc};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

// Generated handlers for generic CRUD operations
mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(Topology, "topologies", "topology");
    crate::crud_delete_handler!(Topology, "topologies", "topology");
}

/// Topology endpoints are internal-only (hidden from public docs)
pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_topologies, create_topology))
        .routes(routes!(
            generated::get_by_id,
            update_topology,
            generated::delete
        ))
        .routes(routes!(refresh))
        .routes(routes!(rebuild))
        .routes(routes!(lock))
        .routes(routes!(unlock))
        // SSE endpoint (not well-supported by OpenAPI)
        .route("/stream", get(staleness_stream))
}

#[utoipa::path(
    put,
    path = "/{id}",
    tags = ["topology", "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    responses(
        (status = 200, description = "Topology updated", body = ApiResponse<Topology>),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_topology(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    id: Path<Uuid>,
    topology: Json<Topology>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    update_handler::<Topology>(state, auth, id, topology).await
}

/// Get all topologies
#[utoipa::path(
    get,
    path = "",
    tags = ["topology", "internal"],
    params(NetworkFilterQuery),
    responses(
        (status = 200, description = "List of topologies", body = PaginatedApiResponse<Topology>),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_topologies(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    query: Query<NetworkFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Topology>>> {
    let network_ids = auth.network_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(|| ApiError::forbidden("Organization context required"))?;

    // Apply network filter and pagination
    let base_filter = EntityFilter::unfiltered().network_ids(&network_ids);
    let filter = query.apply_to_filter(base_filter, &network_ids, organization_id);
    let pagination = query.pagination();
    let filter = pagination.apply_to_filter(filter);

    let service = Topology::get_service(&state);
    let result = service.get_paginated(filter).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to fetch topologies");
        ApiError::internal_error(&e.to_string())
    })?;

    let limit = pagination.effective_limit().unwrap_or(0);
    let offset = pagination.effective_offset();

    Ok(Json(PaginatedApiResponse::success(
        result.items,
        result.total_count,
        limit,
        offset,
    )))
}

/// Create topology
#[utoipa::path(
    post,
    path = "",
    tags = ["topology", "internal"],
    request_body = Topology,
    responses(
        (status = 200, description = "Topology created", body = ApiResponse<Topology>),
        (status = 400, description = "Validation failed", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_topology(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Json(mut topology): Json<Topology>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    let user_id = auth.user_id();
    let network_ids = auth.network_ids();

    // Validate user has access to this network
    if !network_ids.contains(&topology.base.network_id) {
        return Err(ApiError::forbidden("You don't have access to this network"));
    }

    if let Err(err) = topology.validate() {
        tracing::warn!(
            entity_type = Topology::table_name(),
            user_id = ?user_id,
            error = %err,
            "Entity validation failed"
        );
        return Err(ApiError::bad_request(&format!(
            "{} validation failed: {}",
            Topology::entity_name(),
            err
        )));
    }

    tracing::debug!(
        entity_type = Topology::table_name(),
        user_id = ?user_id,
        "Create request received"
    );

    let service = Topology::get_service(&state);

    let (hosts, interfaces, subnets, groups, ports, bindings) =
        service.get_entity_data(topology.base.network_id).await?;

    let services = service
        .get_service_data(topology.base.network_id, &topology.base.options)
        .await?;

    let (nodes, edges) = service.build_graph(BuildGraphParams {
        options: &topology.base.options,
        hosts: &hosts,
        interfaces: &interfaces,
        subnets: &subnets,
        services: &services,
        groups: &groups,
        ports: &ports,
        bindings: &bindings,
        old_edges: &[],
        old_nodes: &[],
    });

    topology.set_entities(SetEntitiesParams {
        hosts,
        interfaces,
        services,
        subnets,
        groups,
        ports,
        bindings,
    });

    topology.set_graph(nodes, edges);

    topology.clear_stale();

    let entity = auth.into_entity();
    let created = service
        .create(topology, entity.clone())
        .await
        .map_err(|e| {
            tracing::error!(
                entity_type = Topology::table_name(),
                user_id = ?user_id,
                error = %e,
                "Failed to create entity"
            );
            ApiError::internal_error(&e.to_string())
        })?;

    tracing::info!(
        entity_type = Topology::table_name(),
        entity_id = %created.id(),
        user_id = ?user_id,
        "Entity created via API"
    );

    Ok(Json(ApiResponse::success(created)))
}

/// Refresh topology data
#[utoipa::path(
    post,
    path = "/{id}/refresh",
    tags = ["topology", "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyRebuildRequest,
    responses(
        (status = 200, description = "Topology refreshed", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn refresh(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyRebuildRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    // Validate user has access to this topology's network
    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    // Fetch the existing topology
    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    // Update options from request
    topology.base.options = request.options;

    let (hosts, interfaces, subnets, groups, ports, bindings) =
        service.get_entity_data(request.network_id).await?;

    let services = service
        .get_service_data(request.network_id, &topology.base.options)
        .await?;

    topology.set_entities(SetEntitiesParams {
        hosts,
        services,
        interfaces,
        subnets,
        groups,
        ports,
        bindings,
    });

    service.update(&mut topology, auth.into_entity()).await?;

    // Return will be handled through event subscriber which triggers SSE

    Ok(Json(ApiResponse::success(())))
}

/// Rebuild topology layout
#[utoipa::path(
    post,
    path = "/{id}/rebuild",
    tags = ["topology", "internal"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    request_body = TopologyRebuildRequest,
    responses(
        (status = 200, description = "Topology rebuilt", body = EmptyApiResponse),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn rebuild(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(request): Json<TopologyRebuildRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let network_ids = auth.network_ids();

    // Validate user has access to this topology's network
    if !network_ids.contains(&request.network_id) {
        return Err(ApiError::forbidden(
            "You don't have access to this topology's network",
        ));
    }

    let service = Topology::get_service(&state);

    // Fetch the existing topology
    let mut topology = service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("Topology {} not found", id)))?;

    // Update options from request
    topology.base.options = request.options.clone();

    let (hosts, interfaces, subnets, groups, ports, bindings) =
        service.get_entity_data(request.network_id).await?;

    let services = service
        .get_service_data(request.network_id, &topology.base.options)
        .await?;

    let (nodes, edges) = service.build_graph(BuildGraphParams {
        options: &topology.base.options,
        hosts: &hosts,
        interfaces: &interfaces,
        subnets: &subnets,
        services: &services,
        groups: &groups,
        ports: &ports,
        bindings: &bindings,
        old_nodes: &request.nodes,
        old_edges: &request.edges,
    });

    topology.set_entities(SetEntitiesParams {
        hosts,
        services,
        interfaces,
        subnets,
        groups,
        ports,
        bindings,
    });

    topology.set_graph(nodes, edges);

    topology.clear_stale();

    let organization_id = auth.organization_id();
    let entity = auth.into_entity();

    service.update(&mut topology, entity.clone()).await?;

    if let Some(org_id) = organization_id {
        let organization = state
            .services
            .organization_service
            .get_by_id(&org_id)
            .await?;

        if let Some(organization) = organization
            && organization.not_onboarded(&TelemetryOperation::FirstTopologyRebuild)
        {
            state
                .services
                .event_bus
                .publish_telemetry(TelemetryEvent {
                    id: Uuid::new_v4(),
                    organization_id: entity.organization_id().expect("User should have org_id"),
                    operation: TelemetryOperation::FirstTopologyRebuild,
                    timestamp: Utc::now(),
                    metadata: serde_json::json!({
                        "is_onboarding_step": true
                    }),
                    authentication: entity,
                })
                .await?;
        }
    }

    // Return will be handled through event subscriber which triggers SSE

    Ok(Json(ApiResponse::success(())))
}

/// Lock a topology
#[utoipa::path(
    post,
    path = "/{id}/lock",
    tags = ["topology"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    responses(
        (status = 200, description = "Topology locked", body = ApiResponse<Topology>),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn lock(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    let service = Topology::get_service(&state);
    let network_ids = auth.network_ids();
    let user_id = auth
        .user_id()
        .ok_or_else(|| ApiError::forbidden("User context required"))?;

    if let Some(mut topology) = service.get_by_id(&id).await? {
        // Validate user has access to this topology's network
        if !network_ids.contains(&topology.base.network_id) {
            return Err(ApiError::forbidden(
                "You don't have access to this topology",
            ));
        }

        topology.lock(user_id);

        let updated = service.update(&mut topology, auth.into_entity()).await?;

        Ok(Json(ApiResponse::success(updated)))
    } else {
        Err(ApiError::not_found(format!(
            "Could not find topology {}",
            id
        )))
    }
}

/// Unlock a topology
#[utoipa::path(
    post,
    path = "/{id}/unlock",
    tags = ["topology"],
    params(("id" = Uuid, Path, description = "Topology ID")),
    responses(
        (status = 200, description = "Topology unlocked", body = ApiResponse<Topology>),
        (status = 403, description = "Access denied", body = ApiErrorResponse),
        (status = 404, description = "Topology not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn unlock(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<Topology>>> {
    let service = Topology::get_service(&state);
    let network_ids = auth.network_ids();

    if let Some(mut topology) = service.get_by_id(&id).await? {
        // Validate user has access to this topology's network
        if !network_ids.contains(&topology.base.network_id) {
            return Err(ApiError::forbidden(
                "You don't have access to this topology",
            ));
        }

        topology.unlock();

        let updated = service.update(&mut topology, auth.into_entity()).await?;

        Ok(Json(ApiResponse::success(updated)))
    } else {
        Err(ApiError::not_found(format!(
            "Could not find topology {}",
            id
        )))
    }
}

async fn staleness_stream(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state
        .services
        .topology_service
        .subscribe_staleness_changes();

    let allowed_networks = auth.network_ids();

    let stream = stream::unfold(rx, move |mut rx| {
        let allowed = allowed_networks.clone();
        async move {
            loop {
                match rx.recv().await {
                    Ok(update) => {
                        // Only emit if user has access to this topology's network
                        if allowed.contains(&update.base.network_id) {
                            let json = serde_json::to_string(&update).ok()?;
                            return Some((Ok(Event::default().data(json)), rx));
                        }
                        // Otherwise skip and wait for next message
                    }
                    Err(_) => return None,
                }
            }
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
