use crate::server::{
    auth::middleware::{
        auth::AuthError,
        cache::CachedOrganization,
        permissions::{Authorized, Viewer},
    },
    billing::types::base::BillingPlan,
    config::AppState,
    networks::r#impl::Network,
    organizations::r#impl::base::Organization,
    shared::{
        services::traits::CrudService, storage::filter::StorableFilter, types::api::ApiError,
    },
    users::r#impl::permissions::UserOrgPermissions,
};
use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};

/// Context available for feature/quota checks
pub struct FeatureCheckContext<'a> {
    pub organization: &'a Organization,
    pub plan: BillingPlan,
    pub app_state: &'a AppState,
    pub permissions: UserOrgPermissions,
}

pub enum FeatureCheckResult {
    Allowed,
    Denied { error: ApiError },
    PaymentRequired { message: String },
}

impl FeatureCheckResult {
    pub fn denied(msg: impl Into<String>) -> Self {
        let message = msg.into();
        Self::Denied {
            error: ApiError::forbidden(&message),
        }
    }

    pub fn denied_with_error(error: ApiError) -> Self {
        Self::Denied { error }
    }

    pub fn payment_required(msg: impl Into<String>) -> Self {
        Self::PaymentRequired {
            message: msg.into(),
        }
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

#[async_trait]
pub trait FeatureCheck: Send + Sync + Default {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult;
}

// ============ Extractor ============

pub struct RequireFeature<T: FeatureCheck> {
    pub permissions: UserOrgPermissions,
    pub plan: BillingPlan,
    pub organization: Organization,
    pub _phantom: std::marker::PhantomData<T>,
}

impl<S, T> FromRequestParts<S> for RequireFeature<T>
where
    S: Send + Sync + AsRef<AppState>,
    T: FeatureCheck + Default,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = Authorized::<Viewer>::from_request_parts(parts, state).await?;
        let permissions = auth
            .entity
            .permissions()
            .ok_or_else(|| AuthError(ApiError::internal_error("No permissions")))?;
        let organization_id = auth
            .organization_id()
            .ok_or_else(|| AuthError(ApiError::internal_error("No organization")))?;

        let app_state = state.as_ref();

        // Use cached organization lookup (may have been cached by billing middleware)
        let organization = CachedOrganization::get_or_load(parts, app_state, &organization_id)
            .await
            .map_err(AuthError)?;

        let plan = organization.base.plan.unwrap_or_default();

        let ctx = FeatureCheckContext {
            organization: &organization,
            plan,
            app_state,
            permissions,
        };

        let checker = T::default();
        match checker.check(&ctx).await {
            FeatureCheckResult::Allowed => Ok(RequireFeature {
                permissions,
                plan,
                organization,
                _phantom: std::marker::PhantomData,
            }),
            FeatureCheckResult::Denied { error } => Err(AuthError(error)),
            FeatureCheckResult::PaymentRequired { message } => {
                Err(AuthError(ApiError::payment_required(&message)))
            }
        }
    }
}

// ============ Concrete Checkers ============

#[derive(Default)]
pub struct InviteUsersFeature;

#[async_trait]
impl FeatureCheck for InviteUsersFeature {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult {
        if !ctx.plan.can_invite_users() {
            return FeatureCheckResult::denied("Your plan does not include inviting users");
        }

        // Seat check happens in the handler where we have access to the request body
        FeatureCheckResult::Allowed
    }
}

#[derive(Default)]
pub struct ApiKeyFeature;

#[async_trait]
impl FeatureCheck for ApiKeyFeature {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult {
        if !ctx.plan.features().api_access {
            return FeatureCheckResult::payment_required("Your plan does not include api access");
        }

        FeatureCheckResult::Allowed
    }
}

#[derive(Default)]
pub struct ShareViewsFeature;

#[async_trait]
impl FeatureCheck for ShareViewsFeature {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult {
        if !ctx.plan.features().share_views {
            return FeatureCheckResult::payment_required(
                "Your plan does not include sharing. Upgrade to share live network diagrams.",
            );
        }

        FeatureCheckResult::Allowed
    }
}

#[derive(Default)]
pub struct CreateNetworkFeature;

#[async_trait]
impl FeatureCheck for CreateNetworkFeature {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult {
        // Check networks quota if there's a limit and user doesn't have a plan that lets them buy more networks
        if let Some(max_networks) = ctx.plan.config().included_networks
            && ctx.plan.config().network_cents.is_none()
        {
            let org_filter = StorableFilter::<Network>::new_from_org_id(&ctx.organization.id);

            let current_networks = ctx
                .app_state
                .services
                .network_service
                .get_all(org_filter)
                .await
                .map(|o| o.len())
                .unwrap_or(0);

            if current_networks >= max_networks as usize {
                return FeatureCheckResult::denied(format!(
                    "Network limit reached ({}/{}). Upgrade your plan for more networks.",
                    current_networks, max_networks
                ));
            }
        }

        FeatureCheckResult::Allowed
    }
}
