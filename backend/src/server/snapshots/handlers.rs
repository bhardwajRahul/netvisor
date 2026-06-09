//! Snapshot HTTP handlers.
//!
//! `POST /snapshots` is custom: it acquires the discovery snapshot lock,
//! creates the snapshot row via the standard generic create path (which emits
//! `EntityOperation::Created` — the topology subscriber listens for this and
//! inserts the snapshot's topology row), runs `run_close_and_clone` to stamp
//! every Snapshotable entity row with `snapshot_id` + close them, and releases
//! the lock on every path.
//!
//! `GET /snapshots?network_id=X` and `DELETE /snapshots/{id}` use the standard
//! generic CRUD handlers; the cascade FKs on closed entity rows + topology rows
//! reap everything tied to the deleted snapshots automatically.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::{
    auth::middleware::{
        features::{RequireFeature, TakeSnapshotFeature},
        permissions::{Authorized, Member, Viewer},
    },
    config::AppState,
    shared::extractors::Query,
    shared::{
        handlers::traits::{delete_handler, get_all_handler, get_by_id_handler},
        services::traits::CrudService,
        storage::traits::Entity,
        types::api::{
            ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
            PaginatedApiResponse,
        },
    },
    snapshots::types::base::{Snapshot, SnapshotBase},
};

use crate::server::shared::handlers::query::NetworkFilterQuery;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSnapshotRequest {
    pub network_id: Uuid,
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_snapshots, create_snapshot))
        .routes(routes!(get_snapshot_by_id, delete_snapshot))
}

/// List snapshots for a network, sorted by `taken_at` DESC.
#[utoipa::path(
    get,
    path = "",
    tag = Snapshot::ENTITY_NAME_PLURAL,
    params(NetworkFilterQuery),
    responses(
        (status = 200, description = "List of snapshots", body = inline(PaginatedApiResponse<Snapshot>)),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_snapshots(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    query: Query<NetworkFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<Snapshot>>> {
    get_all_handler::<Snapshot>(State(state), auth, query).await
}

/// Get a snapshot by ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = Snapshot::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Snapshot ID")),
    responses(
        (status = 200, description = "Snapshot found", body = ApiResponse<Snapshot>),
        (status = 404, description = "Snapshot not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_snapshot_by_id(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    path: Path<Uuid>,
) -> ApiResult<Json<ApiResponse<Snapshot>>> {
    get_by_id_handler::<Snapshot>(State(state), auth, path).await
}

/// Take a snapshot of the current live topology + entity state for a network.
/// Acquires the discovery snapshot lock, creates the snapshots row, runs
/// close-and-clone to stamp every Snapshotable entity row with `snapshot_id`
/// and close them. The topology subscriber inserts the snapshot's topology
/// row off the back of the `Snapshot::Created` event.
#[utoipa::path(
    post,
    path = "",
    tag = Snapshot::ENTITY_NAME_PLURAL,
    request_body = CreateSnapshotRequest,
    responses(
        (status = 200, description = "Snapshot created", body = ApiResponse<Snapshot>),
        (status = 402, description = "Snapshots not available on plan", body = ApiErrorResponse),
        (status = 409, description = "Network is busy with discovery; retry shortly", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn create_snapshot(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    RequireFeature { .. }: RequireFeature<TakeSnapshotFeature>,
    Json(req): Json<CreateSnapshotRequest>,
) -> ApiResult<Json<ApiResponse<Snapshot>>> {
    // Tenant isolation: require the network to be in the caller's set.
    let network_ids = auth.network_ids();
    if !network_ids.contains(&req.network_id) {
        return Err(ApiError::forbidden(
            "Network not accessible to the current user",
        ));
    }

    // Discovery lock: blocks queueing of new discoveries on this network for
    // the duration of close-and-clone, and rejects this request if a scan is
    // currently in-flight.
    let acquired = state
        .services
        .discovery_service
        .try_acquire_network_for_snapshot(req.network_id)
        .await;
    if !acquired {
        return Err(ApiError::conflict(
            "Network is busy with an in-flight discovery; retry shortly.",
        ));
    }

    let result = async {
        let snapshot = Snapshot {
            base: SnapshotBase::new(req.network_id, Utc::now(), auth.user_id()),
            ..Default::default()
        };

        // Generic create: validates access, INSERTs, emits Created event.
        // The topology subscriber catches that event and creates the
        // snapshot's topology row.
        let created = state
            .services
            .snapshot_service
            .create(snapshot, auth.entity.clone())
            .await
            .map_err(ApiError::from)?;

        // Close-and-clone the live entity set, stamping `snapshot_id` on each
        // closed copy. Single transaction; rolls back on any failure.
        state
            .services
            .snapshot_service
            .run_close_and_clone(created.base.network_id, created.base.taken_at, created.id)
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;

        Ok::<Snapshot, ApiError>(created)
    }
    .await;

    state
        .services
        .discovery_service
        .release_network_for_snapshot(req.network_id)
        .await;

    let created = result?;
    Ok(Json(ApiResponse::success(created)))
}

/// Delete a snapshot. The cascade FK on closed entity rows + topology rows
/// reaps everything tied to this snapshot automatically.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = Snapshot::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Snapshot ID")),
    responses(
        (status = 200, description = "Snapshot deleted", body = EmptyApiResponse),
        (status = 404, description = "Snapshot not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn delete_snapshot(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    delete_handler::<Snapshot>(State(state), auth, path).await
}
