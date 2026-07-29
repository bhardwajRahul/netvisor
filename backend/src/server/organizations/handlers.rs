use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, IsUser, Member, Owner};
use crate::server::auth::service::hash_password;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::config::AppState;
use crate::server::networks::r#impl::{Network, NetworkBase};
use crate::server::organizations::demo_status::DemoPopulateStatus;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::{OnboardingOperation, OnboardingOperationDiscriminants};
use crate::server::shared::handlers::traits::{CrudHandlers, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable, Storage};
use crate::server::shared::types::api::ApiResponse;
use crate::server::shared::types::api::ApiResult;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, EmptyApiResponse};
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::snapshots::types::base::{Snapshot, SnapshotBase};
use crate::server::tags::entity_tags::EntityTag;
use crate::server::topology::types::base::Topology;
use crate::server::users::r#impl::base::{User, UserBase};
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use anyhow::anyhow;
use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use email_address::EmailAddress;
use serde::Deserialize;
use std::sync::Arc;
use strum::IntoEnumIterator;
use tower_sessions::Session;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub const DEMO_USER_ID: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440050);

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_organization, update_org_name))
        .routes(routes!(update_profile))
        .routes(routes!(submit_referral_source))
        .routes(routes!(daemon_prompt_response))
        .routes(routes!(reset))
        .routes(routes!(delete_organization))
        .routes(routes!(populate_demo_data))
        .routes(routes!(populate_demo_status))
}

/// Get the current user's organization
#[utoipa::path(
    get,
    path = "",
    tag = Organization::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "Organization details", body = ApiResponse<Organization>),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn get_organization(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
) -> ApiResult<Json<ApiResponse<Organization>>> {
    let organization_id = auth.require_organization_id()?;
    let service = Organization::get_service(&state);
    let entity = service
        .get_by_id(&organization_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(organization_id))?;

    Ok(Json(ApiResponse::success(entity)))
}

/// Update organization name
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Organization::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Organization ID")),
    request_body = String,
    responses(
        (status = 200, description = "Organization updated", body = ApiResponse<Organization>),
        (status = 403, description = "Only owners can update organization", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn update_org_name(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
    Json(name): Json<String>,
) -> ApiResult<Json<ApiResponse<Organization>>> {
    // Organization's scoping fields are both None, so the generic update_handler
    // below cannot enforce tenant ownership — check it explicitly here, matching
    // the sibling reset/delete/populate-demo handlers.
    let user_org_id = auth.require_organization_id()?;
    if id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    let mut org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| anyhow!("Could not find org"))?;

    org.base.name = name;

    update_handler::<Organization>(
        axum::extract::State(state),
        auth.into_permission::<Member>(),
        axum::extract::Path(id),
        axum::extract::Json(org),
    )
    .await
}

/// Request to update user profile (deferred marketing fields)
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProfileUpdateRequest {
    /// The user's job title, collected during onboarding.
    pub job_title: Option<String>,
    /// Company size bracket, collected during onboarding.
    pub company_size: Option<String>,
}

/// Update user profile with deferred marketing fields
#[utoipa::path(
    post,
    path = "/profile",
    tag = Organization::ENTITY_NAME_PLURAL,
    request_body = ProfileUpdateRequest,
    responses(
        (status = 200, description = "Profile updated", body = EmptyApiResponse),
    )
)]
async fn update_profile(
    auth: Authorized<IsUser>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<ProfileUpdateRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let org_id = auth.organization_id().unwrap();
    let authentication: AuthenticatedEntity = auth.into();

    state
        .services
        .event_bus
        .publish(Event::new(
            OrgScope {
                organization_id: org_id,
            },
            OnboardingOperation::ProfileCompleted {
                job_title: request.job_title,
                company_size: request.company_size,
            },
            authentication,
        ))
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to publish profile event: {}", e))
        })?;

    Ok(Json(ApiResponse::success(())))
}

/// How a user first heard about Scanopy, as offered by the onboarding prompt.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReferralSource {
    SearchEngine,
    AiAssistant,
    Youtube,
    Tiktok,
    BlogArticle,
    Reddit,
    HackerNews,
    SocialMedia,
    WordOfMouth,
    ProxmoxCommunityScripts,
    SelfHosted,
    Other,
    PreferNotToSay,
}

/// Request to submit referral source
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReferralSourceRequest {
    /// How the user heard about Scanopy.
    pub referral_source: ReferralSource,
    /// Free-text detail, sent when `referral_source` is `other`.
    pub referral_source_other: Option<String>,
}

/// Submit referral source (how did you hear about us)
#[utoipa::path(
    post,
    path = "/referral-source",
    tag = Organization::ENTITY_NAME_PLURAL,
    request_body = ReferralSourceRequest,
    responses(
        (status = 200, description = "Referral source recorded", body = EmptyApiResponse),
    )
)]
async fn submit_referral_source(
    auth: Authorized<IsUser>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<ReferralSourceRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let org_id = auth.organization_id().unwrap();
    let authentication: AuthenticatedEntity = auth.into();

    state
        .services
        .event_bus
        .publish(Event::new(
            OrgScope {
                organization_id: org_id,
            },
            OnboardingOperation::ReferralSourceCompleted {
                referral_source: request.referral_source,
                referral_source_other: request.referral_source_other,
            },
            authentication,
        ))
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to publish referral source event: {}", e))
        })?;

    Ok(Json(ApiResponse::success(())))
}

/// Which daemon-prompt CTA the user chose.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DaemonPromptAction {
    Dismissed,
    Accepted,
}

/// Request recording the user's response to the "Start Discovering Your Network" prompt.
#[derive(Debug, Deserialize, ToSchema)]
pub struct DaemonPromptResponseRequest {
    /// What the user chose to do about the daemon prompt.
    pub action: DaemonPromptAction,
}

/// Record the user's response to the daemon-install prompt so it is not shown again.
/// Each CTA persists a distinct onboarding milestone (the org subscriber dedups); the
/// PostHog subscriber turns these into funnel events, so no client-side telemetry is needed.
#[utoipa::path(
    post,
    path = "/daemon-prompt-response",
    tag = Organization::ENTITY_NAME_PLURAL,
    request_body = DaemonPromptResponseRequest,
    responses(
        (status = 200, description = "Response recorded", body = EmptyApiResponse),
    )
)]
async fn daemon_prompt_response(
    auth: Authorized<Member>,
    State(state): State<Arc<AppState>>,
    Json(request): Json<DaemonPromptResponseRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let org_id = auth.organization_id().unwrap();
    let authentication: AuthenticatedEntity = auth.into();

    let operation = match request.action {
        DaemonPromptAction::Dismissed => OnboardingOperation::DaemonPromptDismissed,
        DaemonPromptAction::Accepted => OnboardingOperation::DaemonPromptAccepted,
    };

    state
        .services
        .event_bus
        .publish(Event::new(
            OrgScope {
                organization_id: org_id,
            },
            operation,
            authentication,
        ))
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to publish daemon prompt event: {}", e))
        })?;

    Ok(Json(ApiResponse::success(())))
}

/// Reset all organization data (delete all entities except organization and owner user)
#[utoipa::path(
    post,
    path = "/{id}/reset",
    tags = [Organization::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Organization reset", body = EmptyApiResponse),
        (status = 403, description = "Cannot reset another organization", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn reset(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Verify organization exists
    let org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(id))?;

    if org.id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    let entity: AuthenticatedEntity = auth.into_entity();

    reset_organization_data(&state, &org.id, entity.clone()).await?;

    // Create a default network so the org always has at least one
    let network = Network::new(NetworkBase::new(org.id));
    let network = state
        .services
        .network_service
        .create(network, entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to create network: {}", e)))?;

    state
        .services
        .network_service
        .create_organizational_subnets(network.id, entity.clone())
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to seed data: {}", e)))?;

    // Create a default topology for the new network
    use crate::server::topology::types::base::TopologyBase;
    let base = TopologyBase::new(network.id);
    let topology = Topology {
        id: Uuid::new_v4(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        base,
    };
    state
        .services
        .topology_service
        .create(topology, entity)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to create topology: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

/// Delete the organization entirely, including all data and users
#[utoipa::path(
    delete,
    path = "/{id}",
    tags = [Organization::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Organization deleted", body = EmptyApiResponse),
        (status = 403, description = "Cannot delete another organization", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("session" = []))
)]
pub async fn delete_organization(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
    session: Session,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Verify organization exists
    let org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(id))?;

    if org.id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    let has_active_paid_sub = if let Some(billing) = &state.services.billing_service {
        billing.has_active_paid_subscription(org.id).await?
    } else {
        false
    };
    if has_active_paid_sub {
        return Err(ApiError::coded(
            axum::http::StatusCode::CONFLICT,
            ErrorCode::OrganizationHasActiveSubscription,
        ));
    }

    let entity: AuthenticatedEntity = auth.into_entity();

    // Best-effort Stripe teardown. Stripe auto-cancels active subscriptions
    // on customer-delete; failures here don't block deletion — the webhook
    // handlers no-op on missing org if Stripe still fires events afterwards.
    if let (Some(customer_id), Some(billing)) = (
        org.base.stripe_customer_id.as_deref(),
        state.services.billing_service.as_ref(),
    ) && let Err(e) = billing.delete_stripe_customer(customer_id).await
    {
        tracing::error!(
            organization_id = %org.id,
            stripe_customer_id = %customer_id,
            error = %e,
            "Failed to delete Stripe customer during org deletion — proceeding"
        );
    }

    // 1. Delete all child entities (reuse reset logic)
    reset_organization_data(&state, &org.id, entity.clone()).await?;

    // 2. Delete ALL users (including owner)
    let user_filter = StorableFilter::<User>::new_from_org_id(&org.id);
    let all_user_ids: Vec<Uuid> = state
        .services
        .user_service
        .get_all(user_filter)
        .await?
        .iter()
        .map(|u| u.id)
        .collect();

    // Org-deleted confirmation email is dispatched by the email subscriber
    // reacting to `EntityOperation::Deleted` for `Entity::Organization`. The
    // event's `authentication` field carries the initiating user's email.

    if !all_user_ids.is_empty() {
        state
            .services
            .user_service
            .storage()
            .delete_many(&all_user_ids)
            .await?;
    }

    // 3. Delete the organization itself via the CRUD service so the
    //    `EntityOperation::Deleted` event fires; the email subscriber for
    //    `Entity::Organization { Deleted }` dispatches the confirmation
    //    email to the initiating user (carried on `event.authentication`).
    state
        .services
        .organization_service
        .delete(&org.id, entity)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to delete organization: {}", e)))?;

    // 4. Invalidate caller's session
    session
        .delete()
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to delete session: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

/// Populate demo data (only available for demo organizations).
///
/// Runs the population off the request thread (a `tokio::spawn`) and returns
/// `202` immediately — the work is a few hundred sequential DB round-trips and
/// would otherwise exceed the reverse-proxy request timeout against a remote
/// database. Poll `GET /{id}/populate-demo/status` for completion/failure.
#[utoipa::path(
    post,
    path = "/{id}/populate-demo",
    tags = [Organization::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 202, description = "Demo data population started", body = ApiResponse<DemoPopulateStatus>),
        (status = 403, description = "Only available for demo organizations", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
        (status = 409, description = "Population already in progress", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn populate_demo_data(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
) -> ApiResult<(StatusCode, Json<ApiResponse<DemoPopulateStatus>>)> {
    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

    let org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(id))?;

    if org.id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    // Only available for demo organizations
    let plan = org
        .base
        .plan
        .unwrap_or_else(crate::server::billing::plans::get_free_plan);
    if !plan.is_demo() {
        return Err(ApiError::forbidden(
            "Populate demo data is only available for demo organizations",
        ));
    }

    let entity: AuthenticatedEntity = auth.into_entity();

    // Single-flight per org: a retry while a run is in flight must not stack a
    // second population (it would race the reset). `try_begin_demo` sets the
    // `Running` status we hand back in the 202 body.
    let Some(started) = state.services.organization_service.try_begin_demo(id).await else {
        return Err(ApiError::conflict(
            "Demo data population is already in progress for this organization",
        ));
    };

    let task_state = state.clone();
    tokio::spawn(async move {
        let org_service = task_state.services.organization_service.clone();
        let terminal = match run_populate_demo(task_state.clone(), id, user_id, entity, org).await {
            Ok(()) => DemoPopulateStatus::Complete {
                finished_at: chrono::Utc::now(),
            },
            Err(e) => {
                tracing::error!(
                    organization_id = %id,
                    error = %e,
                    "Demo data population failed",
                );
                DemoPopulateStatus::Failed {
                    error: e.to_string(),
                    finished_at: chrono::Utc::now(),
                }
            }
        };
        org_service.set_demo_status(id, terminal).await;
    });

    Ok((StatusCode::ACCEPTED, Json(ApiResponse::success(started))))
}

/// Poll the status of an org's background demo-populate task.
#[utoipa::path(
    get,
    path = "/{id}/populate-demo/status",
    tags = [Organization::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Demo populate status", body = ApiResponse<DemoPopulateStatus>),
        (status = 404, description = "No demo-populate task for this organization", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn populate_demo_status(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<DemoPopulateStatus>>> {
    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    if id != user_org_id {
        return Err(ApiError::permission_denied());
    }
    let status = state
        .services
        .organization_service
        .get_demo_status(&id)
        .await
        .ok_or_else(|| {
            ApiError::not_found("No demo-populate task for this organization".to_string())
        })?;
    Ok(Json(ApiResponse::success(status)))
}

/// The population work itself — runs in the spawned task, off the request
/// thread. Returns `Ok(())` on success; the caller records the terminal status.
async fn run_populate_demo(
    state: Arc<AppState>,
    id: Uuid,
    user_id: Uuid,
    entity: AuthenticatedEntity,
    mut org: Organization,
) -> ApiResult<()> {
    use crate::server::organizations::demo_data::DemoData;
    use crate::server::services::r#impl::base::Service;

    // First, reset all existing data
    reset_organization_data(&state, &id, entity.clone()).await?;

    org.base.onboarding = OnboardingOperationDiscriminants::iter().collect();

    state
        .services
        .organization_service
        .update(&mut org, entity.clone())
        .await?;

    // Generate demo data
    let demo_data = DemoData::generate(id, user_id);

    // Collect all entity tags to bulk insert at the end (single INSERT).
    let mut all_entity_tags: Vec<EntityTag> = Vec::new();

    /// Collect EntityTag records from tagged entities into the accumulator.
    fn collect_entity_tags<T: Entity>(entities: &[T], out: &mut Vec<EntityTag>) {
        use crate::server::tags::entity_tags::{EntityTag, EntityTagBase};
        for entity in entities {
            if let Some(tags) = entity.get_tags() {
                for &tag_id in tags {
                    out.push(EntityTag::new(EntityTagBase::new(
                        entity.id(),
                        T::entity_type(),
                        tag_id,
                    )));
                }
            }
        }
    }

    // Insert entities in dependency order using bulk inserts.
    // Since we just reset the org, there are no collisions — we use service-level
    // create_many (publishes one event per scope per entity type) instead of
    // per-entity create() for speed. Entities without event subscribers use
    // storage().create_many() directly.

    // 1. Tags (no dependencies)
    state
        .services
        .tag_service
        .storage()
        .create_many(&demo_data.tags)
        .await?;

    // 2. Credentials (depends on organization)
    state
        .services
        .credential_service
        .storage()
        .create_many(&demo_data.credentials)
        .await?;

    // 3. Networks (depends on organization, tags)
    let created_networks = state
        .services
        .network_service
        .storage()
        .create_many(&demo_data.networks)
        .await?;
    collect_entity_tags(&created_networks, &mut all_entity_tags);

    // 3.5. Network-credential associations — one bulk insert across all
    // networks (no per-network lock/delete; the org was just reset).
    let network_cred_pairs: Vec<(Uuid, Uuid)> = demo_data
        .network_credential_assignments
        .iter()
        .flat_map(|a| {
            let network_id = a.network_id;
            a.credential_ids
                .iter()
                .map(move |&cred_id| (network_id, cred_id))
        })
        .collect();
    state
        .services
        .credential_service
        .create_network_credentials(&network_cred_pairs)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // 4. Subnets (depends on networks)
    let created_subnets = state
        .services
        .subnet_service
        .create_many(&demo_data.subnets, entity.clone())
        .await?;
    collect_entity_tags(&created_subnets, &mut all_entity_tags);

    // 4.5. VLANs (depends on networks)
    state
        .services
        .vlan_service
        .storage()
        .create_many(&demo_data.vlans)
        .await?;

    // 4.6. Subnet↔VLAN junction rows (depend on subnets + VLANs). One bulk
    // insert, no per-subnet lock (fresh org). Derived to mirror the discovery
    // reconciler, so demo subnets show their VLANs like a real deployment.
    state
        .services
        .vlan_service
        .subnet_vlan_storage
        .create_many(&demo_data.subnet_vlan_records)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    // 5. Hosts + children — bypass discover_host (no collisions in fresh org)
    // Flatten hosts, ip_addresses, ports, services from HostWithServices bundles
    let mut all_hosts = Vec::new();
    let mut all_ip_addresses = Vec::new();
    let mut all_ports = Vec::new();
    let mut all_services: Vec<Service> = Vec::new();
    for hws in &demo_data.hosts_with_services {
        let host_id = hws.host.id;
        let network_id = hws.host.base.network_id;
        all_hosts.push(hws.host.clone());
        all_ip_addresses.extend(hws.ip_addresses.clone());
        all_ports.extend(
            hws.ports
                .iter()
                .cloned()
                .map(|p| p.with_host(host_id, network_id)),
        );
        all_services.extend(hws.services.clone());
    }

    let created_hosts = state
        .services
        .host_service
        .create_many(&all_hosts, entity.clone())
        .await?;
    collect_entity_tags(&created_hosts, &mut all_entity_tags);

    // 5.3. Resolve interface neighbor links in memory, so neighbor_interface_id
    // is set at insert time and we avoid N post-insert UPDATEs.
    let interfaces = {
        use crate::server::interfaces::r#impl::base::Neighbor;
        use std::collections::HashMap;

        let host_id_to_name: HashMap<Uuid, String> = all_hosts
            .iter()
            .map(|h| (h.id, h.base.name.clone()))
            .collect();

        let mut if_entry_lookup: HashMap<(String, i32), Uuid> = HashMap::new();
        for entry in &demo_data.interfaces {
            if let Some(host_name) = host_id_to_name.get(&entry.base.host_id) {
                if_entry_lookup.insert((host_name.clone(), entry.base.if_index), entry.id);
            }
        }

        // Build a map of source interface ID -> target interface ID
        let mut neighbor_map: HashMap<Uuid, Uuid> = HashMap::new();
        for neighbor_update in &demo_data.neighbor_updates {
            let source_key = (
                neighbor_update.source_host_name.clone(),
                neighbor_update.source_if_index,
            );
            let target_key = (
                neighbor_update.target_host_name.clone(),
                neighbor_update.target_if_index,
            );
            if let (Some(&source_id), Some(&target_id)) = (
                if_entry_lookup.get(&source_key),
                if_entry_lookup.get(&target_key),
            ) {
                neighbor_map.insert(source_id, target_id);
            }
        }

        // Apply neighbors to interfaces before inserting
        let mut interfaces = demo_data.interfaces;
        for entry in &mut interfaces {
            if let Some(&target_id) = neighbor_map.get(&entry.id) {
                entry.base.neighbor = Some(Neighbor::Interface(target_id));
            }
        }
        interfaces
    };

    // 5.4. ip_addresses must be committed before interfaces: interfaces.ip_address_id
    // FKs into ip_addresses(id), and create_many is not transactional — each chunk
    // autocommits on its own pooled connection. Run concurrently, the child batch can
    // reach the server before the parent rows exist and fail the FK, which is exactly
    // what connection-acquisition skew against a remote database produces. Awaiting
    // here makes the ordering unconditional, at the cost of one serialized round trip.
    state
        .services
        .ip_address_service
        .create_many(&all_ip_addresses, entity.clone())
        .await?;

    // 5.5. The remaining three host-children have no interdependency, so they stay
    // concurrent — each takes its own pooled connection. The services result is
    // needed downstream for bindings + entity tags.
    let (_, created_services, _) = tokio::try_join!(
        async {
            state
                .services
                .port_service
                .create_many(&all_ports, entity.clone())
                .await
                .map_err(ApiError::from)
        },
        async {
            state
                .services
                .service_service
                .create_many(&all_services, entity.clone())
                .await
                .map_err(ApiError::from)
        },
        async {
            state
                .services
                .interface_service
                .create_many(&interfaces, entity.clone())
                .await
                .map_err(ApiError::from)
        },
    )?;
    collect_entity_tags(&created_services, &mut all_entity_tags);

    // 5.6. Bindings (child entities of services, stored in a separate table).
    let all_bindings: Vec<Binding> = created_services
        .iter()
        .flat_map(|s| {
            s.base
                .bindings
                .iter()
                .cloned()
                .map(|b| b.with_service(s.id, s.base.network_id))
        })
        .collect();
    state
        .services
        .binding_service
        .create_many(&all_bindings, entity.clone())
        .await?;

    // 6. Daemons (depends on hosts, networks, subnets)
    state
        .services
        .daemon_service
        .storage()
        .create_many(&demo_data.daemons)
        .await?;

    // 7. Daemon API Keys (depends on networks)
    state
        .services
        .daemon_api_key_service
        .storage()
        .create_many(&demo_data.api_keys)
        .await?;

    // 8. Discoveries (depends on daemons, networks, subnets)
    state
        .services
        .discovery_service
        .storage()
        .create_many(&demo_data.discoveries)
        .await?;

    // 9. Dependencies + members — pre-generated during DemoData::generate().
    // create_many bypasses per-entity service logic, so persist members
    // separately — as one bulk insert across all dependencies, resolving each
    // Bindings member's service_id from the in-memory bindings (avoiding a
    // SELECT per binding) and skipping the per-dependency lock/delete.
    let created_deps = state
        .services
        .dependency_service
        .create_many(&demo_data.dependencies, entity.clone())
        .await?;
    {
        use crate::server::dependencies::dependency_members::{
            DependencyMemberRecord, DependencyMemberRecordBase,
        };
        use crate::server::dependencies::r#impl::base::DependencyMembers;
        use std::collections::{HashMap, HashSet};

        let binding_to_service: HashMap<Uuid, Uuid> = all_bindings
            .iter()
            .map(|b| (b.id, b.base.service_id))
            .collect();

        let mut member_records: Vec<DependencyMemberRecord> = Vec::new();
        for dep in &created_deps {
            match &dep.base.members {
                DependencyMembers::Services { service_ids } => {
                    let mut seen = HashSet::new();
                    for (position, service_id) in service_ids
                        .iter()
                        .filter(|id| seen.insert(**id))
                        .enumerate()
                    {
                        member_records.push(DependencyMemberRecord::new(
                            DependencyMemberRecordBase::new(
                                dep.id,
                                *service_id,
                                None,
                                position as i32,
                            ),
                        ));
                    }
                }
                DependencyMembers::Bindings { binding_ids } => {
                    for (position, binding_id) in binding_ids.iter().enumerate() {
                        if let Some(&service_id) = binding_to_service.get(binding_id) {
                            member_records.push(DependencyMemberRecord::new(
                                DependencyMemberRecordBase::new(
                                    dep.id,
                                    service_id,
                                    Some(*binding_id),
                                    position as i32,
                                ),
                            ));
                        }
                    }
                }
            }
        }
        state
            .services
            .dependency_service
            .create_members(&member_records)
            .await?;
    }

    // 10. Topologies (depends on networks + the entities created above).
    // The graph is built on request from the persisted entities, so `create`
    // just persists the row + options. Must run before shares (step 11), whose
    // `topology_id` FK references these rows.
    for topology in demo_data.topologies {
        state
            .services
            .topology_service
            .create(topology, entity.clone())
            .await?;
    }

    // 10.5. Bulk insert all entity tags (single INSERT for all tagged entities)
    if !all_entity_tags.is_empty() {
        state
            .services
            .entity_tag_service
            .create_many(&all_entity_tags)
            .await?;
    }

    // 11. Shares (depends on topologies)
    state
        .services
        .share_service
        .storage()
        .create_many(&demo_data.shares)
        .await?;

    // 12. Demo admin user
    let password = hash_password("password123")?;
    let mut demo_admin = User::new(UserBase::new_password(
        EmailAddress::new_unchecked("demo@scanopy.net"),
        true,
        password,
        org.id,
        UserOrgPermissions::Admin,
        vec![],
        None,
    ));
    demo_admin.base.email_verified = true;
    demo_admin.id = DEMO_USER_ID;
    state
        .services
        .user_service
        .create(demo_admin, entity.clone())
        .await?;

    // 13. User API Keys (depends on demo admin user + network access junction table)
    for (api_key, network_ids) in demo_data.user_api_keys {
        state
            .services
            .user_api_key_service
            .create_with_networks(api_key, network_ids, entity.clone())
            .await
            .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    }

    // 14. One snapshot per network so the snapshot UI is exercised in demo orgs.
    // Must run last: close-and-clone captures the live entity set, so all demo
    // entities (and their entity-tags + live topology rows) must already exist.
    // Each network's snapshot is scoped to its own network_id and runs in its
    // own transaction, so the networks' snapshots run concurrently.
    let snapshot_futures = created_networks.iter().map(|network| {
        let state = &state;
        let entity = entity.clone();
        async move {
            let snapshot = Snapshot {
                base: SnapshotBase::new(network.id, chrono::Utc::now(), Some(user_id)),
                ..Default::default()
            };
            let created = state
                .services
                .snapshot_service
                .create(snapshot, entity)
                .await
                .map_err(ApiError::from)?;
            state
                .services
                .snapshot_service
                .run_close_and_clone(created.base.network_id, created.base.taken_at, created.id)
                .await
                .map_err(|e| ApiError::internal_error(&e.to_string()))?;
            // No snapshot topology row — the graph is built on request from the
            // closed copies stamped above.
            Ok::<(), ApiError>(())
        }
    });
    futures::future::try_join_all(snapshot_futures).await?;

    // 15. "Recently discovered" hosts — created AFTER the snapshot so the
    // snapshot captures the earlier state and the live view visibly differs
    // (these hosts/services appear only in live). Self-contained (no
    // dependencies, neighbors, or interfaces), so the main flatten→create
    // pattern applies, minus interfaces.
    if !demo_data.recent_hosts_with_services.is_empty() {
        let mut recent_hosts = Vec::new();
        let mut recent_ips = Vec::new();
        let mut recent_ports = Vec::new();
        let mut recent_services: Vec<Service> = Vec::new();
        for hws in &demo_data.recent_hosts_with_services {
            let host_id = hws.host.id;
            let network_id = hws.host.base.network_id;
            recent_hosts.push(hws.host.clone());
            recent_ips.extend(hws.ip_addresses.clone());
            recent_ports.extend(
                hws.ports
                    .iter()
                    .cloned()
                    .map(|p| p.with_host(host_id, network_id)),
            );
            recent_services.extend(hws.services.clone());
        }

        let mut recent_entity_tags: Vec<EntityTag> = Vec::new();
        let created_recent_hosts = state
            .services
            .host_service
            .create_many(&recent_hosts, entity.clone())
            .await?;
        collect_entity_tags(&created_recent_hosts, &mut recent_entity_tags);

        let (_, _, created_recent_services) = tokio::try_join!(
            async {
                state
                    .services
                    .ip_address_service
                    .create_many(&recent_ips, entity.clone())
                    .await
                    .map_err(ApiError::from)
            },
            async {
                state
                    .services
                    .port_service
                    .create_many(&recent_ports, entity.clone())
                    .await
                    .map_err(ApiError::from)
            },
            async {
                state
                    .services
                    .service_service
                    .create_many(&recent_services, entity.clone())
                    .await
                    .map_err(ApiError::from)
            },
        )?;
        collect_entity_tags(&created_recent_services, &mut recent_entity_tags);

        let recent_bindings: Vec<Binding> = created_recent_services
            .iter()
            .flat_map(|s| {
                s.base
                    .bindings
                    .iter()
                    .cloned()
                    .map(|b| b.with_service(s.id, s.base.network_id))
            })
            .collect();
        state
            .services
            .binding_service
            .create_many(&recent_bindings, entity.clone())
            .await?;

        if !recent_entity_tags.is_empty() {
            state
                .services
                .entity_tag_service
                .create_many(&recent_entity_tags)
                .await?;
        }
    }

    Ok(())
}

/// Internal function to reset organization data (reused by populate_demo_data).
///
/// Uses direct storage-level bulk deletes instead of service-level `delete_all_for_org`
/// to avoid O(N) per-entity tag removal and event publishing. This is safe because:
/// - We're deleting the entire org's data, not selective entities
/// - Tags are deleted first, and `tag_id REFERENCES tags(id) ON DELETE CASCADE`
///   automatically cleans up all entity_tags — no per-entity removal needed
/// - Event publishing during a full demo reset is unnecessary
async fn reset_organization_data(
    state: &Arc<AppState>,
    organization_id: &Uuid,
    _auth: AuthenticatedEntity,
) -> Result<(), ApiError> {
    use crate::server::credentials::r#impl::base::Credential;
    use crate::server::invites::r#impl::base::Invite;
    use crate::server::tags::r#impl::base::Tag;
    use crate::server::user_api_keys::r#impl::base::UserApiKey;

    // Deletes run sequentially: several of these cascades overlap on shared
    // junction rows (e.g. networks and user_api_keys both cascade
    // user_api_key_network_access; tags cascade entity_tags), so running them
    // concurrently risks lock-ordering deadlocks for negligible gain — the
    // reset is a handful of deletes and was never the slow part.

    // 1. Delete tags — CASCADE on tag_id cleans up entity_tags automatically.
    state
        .services
        .tag_service
        .storage()
        .delete_by_filter(StorableFilter::<Tag>::new_from_org_id(organization_id))
        .await?;

    // 2. Delete networks — CASCADE handles all network-scoped entities
    //    (hosts, services, subnets, topologies, shares, daemons, discoveries,
    //    ports, bindings, interfaces, IP addresses, daemon API keys, etc.)
    state
        .services
        .network_service
        .storage()
        .delete_by_filter(StorableFilter::<Network>::new_from_org_id(organization_id))
        .await?;

    // 3. Delete org-scoped entities not tied to networks
    state
        .services
        .user_api_key_service
        .storage()
        .delete_by_filter(StorableFilter::<UserApiKey>::new_from_org_id(
            organization_id,
        ))
        .await?;
    state
        .services
        .invite_service
        .storage()
        .delete_by_filter(StorableFilter::<Invite>::new_from_org_id(organization_id))
        .await?;
    state
        .services
        .credential_service
        .storage()
        .delete_by_filter(StorableFilter::<Credential>::new_from_org_id(
            organization_id,
        ))
        .await?;

    // 4. Delete non-owner users
    let user_filter = StorableFilter::<User>::new_from_org_id(organization_id);
    let non_owner_user_ids: Vec<Uuid> = state
        .services
        .user_service
        .get_all(user_filter)
        .await?
        .iter()
        .filter_map(|u| {
            if u.base.permissions != UserOrgPermissions::Owner {
                Some(u.id)
            } else {
                None
            }
        })
        .collect();

    if !non_owner_user_ids.is_empty() {
        state
            .services
            .user_service
            .storage()
            .delete_many(&non_owner_user_ids)
            .await?;
    }

    Ok(())
}
