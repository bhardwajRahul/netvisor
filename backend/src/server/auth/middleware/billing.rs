//! Billing middleware that checks subscription status for authenticated requests.
//!
//! This middleware examines each request and:
//! - For authenticated requests (user, API key, daemon): Checks billing status, returns 402 if not active
//! - For unauthenticated requests: Passes through (handler auth will reject if needed)
//!
//! Exemptions (always allowed regardless of billing):
//! - Community plan
//! - CommercialSelfHosted plan
//! - Demo plan
//! - Self-hosted instances (no stripe_secret configured)

use crate::server::{
    auth::middleware::{auth::AuthenticatedEntity, cache::CachedNetwork},
    billing::types::base::BillingPlan,
    config::AppState,
};
use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

/// Middleware that enforces billing requirements for authenticated requests.
///
/// Apply this middleware at the router level to groups of routes that require billing.
/// Checks billing for users (session), user API keys, and daemons.
/// Caches looked-up entities in request extensions for subsequent handlers.
pub async fn require_billing_for_users(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Check if billing is enabled:
    // - Production: enabled when stripe_secret is configured
    // - Testing: enabled when enforce_billing_for_testing is true
    let billing_enabled =
        state.config.stripe_secret.is_some() || state.config.enforce_billing_for_testing;
    if !billing_enabled {
        return next.run(request).await;
    }

    // Split request to access parts for caching
    let (mut parts, body) = request.into_parts();

    // Extract authenticated entity (cached in extensions)
    let entity = AuthenticatedEntity::from_request_parts(&mut parts, &state)
        .await
        .ok();

    // Get organization ID based on auth type, caching network lookup for daemons
    let organization_id: Option<Uuid> = match &entity {
        Some(AuthenticatedEntity::User {
            organization_id, ..
        }) => Some(*organization_id),
        Some(AuthenticatedEntity::ApiKey {
            organization_id, ..
        }) => Some(*organization_id),
        Some(AuthenticatedEntity::Daemon { network_id, .. }) => {
            // Use cached network lookup
            match CachedNetwork::get_or_load(&mut parts, &state, network_id).await {
                Ok(network) => Some(network.base.organization_id),
                Err(_) => None,
            }
        }
        _ => None,
    };

    let Some(organization_id) = organization_id else {
        // Not authenticated or no org - pass through (handler auth will reject if needed)
        let request = Request::from_parts(parts, body);
        return next.run(request).await;
    };

    // Cached organization for the request — used by both this middleware
    // and any downstream feature checks.
    let organization =
        match super::cache::CachedOrganization::get_or_load(&mut parts, &state, &organization_id)
            .await
        {
            Ok(org) => org,
            Err(e) => return e.into_response(),
        };

    // Reassemble request with cached entities in extensions
    let request = Request::from_parts(parts, body);

    // Check plan type - some plans are exempt from billing checks.
    // `unwrap_or_default()` treats `plan = None` (newly-registered orgs that
    // haven't selected a plan yet) as the build's default plan
    // (Community / CommercialSelfHosted), which matches the exempt arm and
    // lets `/api/v1/organizations` succeed. The frontend reads the resulting
    // `plan = null` and opens BillingPlanModal to force plan selection.
    let plan = organization.base.plan.unwrap_or_default();
    if matches!(
        plan,
        BillingPlan::Community(_)
            | BillingPlan::Free(_)
            | BillingPlan::CommercialSelfHosted(_)
            | BillingPlan::SelfHostedStandard(_)
            | BillingPlan::SelfHostedPlus(_)
            | BillingPlan::Demo(_)
    ) {
        return next.run(request).await;
    }

    // Check subscription status. None = no subscription yet (must select a
    // plan); Cancelled = revoke access; everything else allows the request
    // through (Active / Trialing / PendingCancellation / PastDue / Paused all
    // keep features available — Paused is a Stripe collection pause, not a
    // feature lockout).
    use crate::server::billing::types::base::PlanStatus;
    match organization.base.plan_status {
        Some(PlanStatus::Cancelled) => {
            billing_error_response("Your subscription has been canceled. Please renew to continue.")
        }
        Some(_) => next.run(request).await,
        None => billing_error_response("Active billing plan required. Please select a plan."),
    }
}

fn billing_error_response(message: &str) -> Response {
    (
        StatusCode::PAYMENT_REQUIRED,
        axum::Json(serde_json::json!({
            "success": false,
            "error": {
                "code": 402,
                "message": message
            }
        })),
    )
        .into_response()
}
