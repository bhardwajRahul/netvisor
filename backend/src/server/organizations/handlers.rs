use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::features::{BlockedInDemoMode, RequireFeature};
use crate::server::auth::middleware::permissions::{Authorized, IsUser, Member, Owner};
use crate::server::auth::service::hash_password;
use crate::server::billing::types::base::BillingPlan;
use crate::server::config::AppState;
use crate::server::networks::r#impl::Network;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::handlers::traits::{CrudHandlers, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::ApiResponse;
use crate::server::shared::types::api::ApiResult;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, EmptyApiResponse};
use crate::server::users::r#impl::base::{User, UserBase};
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use anyhow::anyhow;
use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use email_address::EmailAddress;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub const DEMO_USER_ID: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440050);

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_organization, update_org_name))
        .routes(routes!(reset))
        .routes(routes!(populate_demo_data))
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
    _demo_check: RequireFeature<BlockedInDemoMode>,
    Path(id): Path<Uuid>,
    Json(name): Json<String>,
) -> ApiResult<Json<ApiResponse<Organization>>> {
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

    reset_organization_data(&state, &org.id, entity).await?;

    Ok(Json(ApiResponse::success(())))
}

/// Populate demo data (only available for demo organizations)
#[utoipa::path(
    post,
    path = "/{id}/populate-demo",
    tags = [Organization::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Demo data populated", body = EmptyApiResponse),
        (status = 403, description = "Only available for demo organizations", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn populate_demo_data(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    use crate::server::organizations::demo_data::{DemoData, generate_groups};
    use crate::server::services::r#impl::base::Service;

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
    if !matches!(org.base.plan, Some(BillingPlan::Demo(_))) {
        return Err(ApiError::forbidden(
            "Populate demo data is only available for demo organizations",
        ));
    }

    let entity: AuthenticatedEntity = auth.into_entity();

    // First, reset all existing data
    reset_organization_data(&state, &id, entity.clone()).await?;

    // Generate demo data
    let demo_data = DemoData::generate(id, user_id);

    // Insert entities in dependency order:
    // 1. Tags (no dependencies) - keep track of created tags for group generation
    let mut created_tags = Vec::new();
    for tag in demo_data.tags {
        let created = state
            .services
            .tag_service
            .create(tag, entity.clone())
            .await?;
        created_tags.push(created);
    }

    // 2. Networks (depends on organization, tags) - keep track for group generation
    let mut created_networks = Vec::new();
    for network in demo_data.networks {
        let created = state
            .services
            .network_service
            .create(network, entity.clone())
            .await?;
        created_networks.push(created);
    }

    // 3. Subnets (depends on networks)
    for subnet in demo_data.subnets {
        state
            .services
            .subnet_service
            .create(subnet, entity.clone())
            .await?;
    }

    // 3.5 SNMP Credentials (depends on organization)
    for credential in demo_data.snmp_credentials {
        state
            .services
            .snmp_credential_service
            .create(credential, entity.clone())
            .await?;
    }

    // 4. Hosts with Services - collect created services for group generation
    let mut all_created_services: Vec<Service> = Vec::new();
    for host_with_services in demo_data.hosts_with_services {
        // Match if_entries for this host by host_id
        let host_if_entries: Vec<crate::server::if_entries::r#impl::base::IfEntry> = demo_data
            .if_entries
            .iter()
            .filter(|e| e.base.host_id == host_with_services.host.id)
            .cloned()
            .collect();

        let host_response = state
            .services
            .host_service
            .discover_host(
                host_with_services.host,
                host_with_services.interfaces,
                host_with_services.ports,
                host_with_services.services,
                host_if_entries,
                entity.clone(),
            )
            .await?;
        all_created_services.extend(host_response.services);
    }

    // 5. Daemons (depends on hosts, networks, subnets)
    for daemon in demo_data.daemons {
        state
            .services
            .daemon_service
            .create(daemon, entity.clone())
            .await?;
    }

    // 6. API Keys (depends on networks)
    for api_key in demo_data.api_keys {
        state
            .services
            .daemon_api_key_service
            .create(api_key, entity.clone())
            .await?;
    }

    // 7. Groups - generate with actual created services to get correct binding IDs
    let groups = generate_groups(&created_networks, &all_created_services, &created_tags);
    for group in groups {
        state
            .services
            .group_service
            .create(group, entity.clone())
            .await?;
    }

    // 8. Topologies (depends on networks)
    for topology in demo_data.topologies {
        state
            .services
            .topology_service
            .create(topology, entity.clone())
            .await?;
    }

    // Create admin user
    let password = hash_password("password123")?;
    let mut demo_admin = User::new(UserBase::new_password(
        EmailAddress::new_unchecked("demo@scanopy.net"),
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

    Ok(Json(ApiResponse::success(())))
}

/// Internal function to reset organization data (reused by populate_demo_data)
async fn reset_organization_data(
    state: &Arc<AppState>,
    organization_id: &Uuid,
    auth: AuthenticatedEntity,
) -> Result<(), ApiError> {
    let org_filter = StorableFilter::<Network>::new().organization_id(organization_id);
    let network_ids: Vec<Uuid> = state
        .services
        .network_service
        .get_all(org_filter.clone())
        .await?
        .iter()
        .map(|n| n.id)
        .collect();

    // Delete all data except org and owner user
    // Order matters due to foreign keys:
    // 1. Groups depend on services
    // 2. Discoveries depend on daemons/networks
    // 3. Daemons depend on hosts/networks
    // 4. Services depend on hosts
    // 5. Hosts depend on networks/subnets
    // 6. Subnets depend on networks
    // 7. API keys depend on networks
    // 8. Tags (no dependencies, but referenced by other entities)

    // Delete all data except org and owner user
    // Order matters due to foreign keys:
    // 1. Discoveries depend on daemons/networks
    // 2. Daemons depend on hosts/networks
    // 3. Hosts/services depend on networks
    // 4. API keys depend on networks
    state
        .services
        .discovery_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .daemon_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .host_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .topology_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .daemon_api_key_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .network_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .invite_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;
    state
        .services
        .tag_service
        .delete_all_for_org(organization_id, &network_ids, auth.clone())
        .await?;

    // Delete non-owner users
    let user_filter = StorableFilter::<User>::new().organization_id(organization_id);
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

    state
        .services
        .user_service
        .delete_many(&non_owner_user_ids, auth)
        .await?;

    Ok(())
}
