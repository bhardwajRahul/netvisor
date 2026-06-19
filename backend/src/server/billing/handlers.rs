use crate::server::auth::middleware::permissions::{Authorized, Owner, RequireVerified, Viewer};
use crate::server::billing::types::api::{
    CancelSubscriptionRequest, CancelSubscriptionResponse, ChangePlanPreview, ChangePlanRequest,
    CreateCheckoutRequest, PauseSubscriptionRequest, SaveOfferCoupon, SetupPaymentMethodRequest,
};
use crate::server::billing::types::base::BillingPlan;
use crate::server::config::AppState;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::types::ErrorCode;
use crate::server::shared::types::api::{ApiError, ApiResult};
use crate::server::shared::types::api::{ApiErrorResponse, ApiResponse, EmptyApiResponse};
use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::CACHE_CONTROL;
use axum::response::IntoResponse;
use axum_client_ip::ClientIp;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

/// Enterprise plan inquiry request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnterpriseInquiryRequest {
    /// Contact email
    pub email: String,
    /// Contact name
    pub name: String,
    /// Company name
    pub company: String,
    /// Team/company size: 1-10, 11-25, 26-50, 51-100, 101-250, 251-500, 501-1000, 1001+
    pub team_size: String,
    /// Message/use case description
    pub message: String,
    /// Urgency: immediately, 1-3 months, 3-6 months, exploring
    #[serde(default)]
    pub urgency: Option<String>,
    /// Number of networks/sites
    #[serde(default)]
    pub network_count: Option<i64>,
    /// Plan type being inquired about
    #[serde(default)]
    pub plan_type: Option<String>,
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_billing_plans))
        .routes(routes!(create_checkout_session))
        .routes(routes!(setup_payment_method))
        .routes(routes!(change_plan))
        .routes(routes!(preview_plan_change))
        .routes(routes!(handle_webhook))
        .routes(routes!(create_portal_session))
        .routes(routes!(submit_enterprise_inquiry))
        .routes(routes!(pause_subscription))
        .routes(routes!(resume_subscription))
        .routes(routes!(extend_trial))
        .routes(routes!(cancel_subscription))
        .routes(routes!(reactivate_subscription))
        .routes(routes!(apply_discount_save_offer))
        .routes(routes!(get_save_offer_coupon))
}

/// Get available billing plans
#[utoipa::path(
    get,
    path = "/plans",
    tags = ["billing", "internal"],
    responses(
        (status = 200, description = "List of available billing plans", body = ApiResponse<Vec<BillingPlan>>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_billing_plans(
    State(state): State<Arc<AppState>>,
    _auth: Authorized<Owner>,
) -> Result<impl IntoResponse, ApiError> {
    if let Some(billing_service) = state.services.billing_service.clone() {
        let plans = billing_service.get_plans();
        Ok((
            [(CACHE_CONTROL, "no-store, no-cache, must-revalidate")],
            Json(ApiResponse::success(plans)),
        ))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Create a checkout session
#[utoipa::path(
    post,
    path = "/checkout",
    tags = ["billing", "internal"],
    request_body = CreateCheckoutRequest,
    responses(
        (status = 200, description = "Checkout session URL", body = ApiResponse<String>),
        (status = 400, description = "Invalid plan or billing not enabled", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_checkout_session(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<CreateCheckoutRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let current_plans = billing_service.get_plans();

        if !current_plans.contains(&request.plan) {
            return Err(ApiError::validation(ErrorCode::ValidationInvalidFormat {
                field: "plan".to_string(),
            }));
        }

        let success_url = format!("{}?billing_flow=checkout", request.url);

        // Check if org already has a plan — route based on target plan and payment state
        let org = billing_service.get_organization(organization_id).await?;
        let plan_status = org.base.plan_status;

        if plan_status.is_some() && org.base.stripe_customer_id.is_some() {
            if request.plan.is_free() {
                // Downgrade to Free — schedule cancellation at end of billing cycle
                let result = billing_service
                    .schedule_downgrade(organization_id, auth.into_entity())
                    .await?;
                Ok(Json(ApiResponse::success(result)))
            } else {
                // Paid target — check trial eligibility and payment state
                use crate::server::billing::types::base::PlanStatus;
                let is_currently_trialing = plan_status == Some(PlanStatus::Trialing);

                if is_currently_trialing {
                    // Currently trialing — switch plan via subscription update (preserves trial)
                    let result = billing_service
                        .change_plan(organization_id, request.plan, auth.into_entity())
                        .await?;
                    return Ok(Json(ApiResponse::success(result)));
                }

                let has_non_free_plan = org.base.plan.as_ref().is_some_and(|p| !p.is_free());
                let is_returning = has_non_free_plan || org.base.trial_end_date.is_some();
                let is_trial_eligible = !is_returning && request.plan.config().trial_days > 0;

                if is_trial_eligible {
                    // Trial-eligible — create subscription directly, skip Checkout
                    let result = billing_service
                        .create_trial_subscription(
                            organization_id,
                            request.plan,
                            auth.into_entity(),
                        )
                        .await?;
                    Ok(Json(ApiResponse::success(result)))
                } else if !org.base.has_payment_method {
                    // No payment method, not trial-eligible — collect via Checkout
                    let cancel_url = request.url.clone();
                    let session = billing_service
                        .create_checkout_session(
                            organization_id,
                            request.plan,
                            success_url,
                            cancel_url,
                            auth.into_entity(),
                        )
                        .await?;
                    Ok(Json(ApiResponse::success(session.url.unwrap())))
                } else {
                    // Has payment, not trial-eligible — direct subscription update
                    let result = billing_service
                        .change_plan(organization_id, request.plan, auth.into_entity())
                        .await?;
                    Ok(Json(ApiResponse::success(result)))
                }
            }
        } else {
            // First-time subscriber
            if request.plan.is_free() {
                // Free plan — activate directly, no Stripe needed
                let result = billing_service
                    .activate_free_plan(organization_id, request.plan, auth.into_entity())
                    .await?;
                Ok(Json(ApiResponse::success(result)))
            } else if request.plan.config().trial_days > 0 {
                // Trial plan — create subscription directly, skip Checkout
                let result = billing_service
                    .create_trial_subscription(organization_id, request.plan, auth.into_entity())
                    .await?;
                Ok(Json(ApiResponse::success(result)))
            } else {
                // No trial — go through Stripe Checkout
                let cancel_url = request.url.clone();
                let session = billing_service
                    .create_checkout_session(
                        organization_id,
                        request.plan,
                        success_url,
                        cancel_url,
                        auth.into_entity(),
                    )
                    .await?;
                Ok(Json(ApiResponse::success(session.url.unwrap())))
            }
        }
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Setup payment method (collect card without charging)
#[utoipa::path(
    post,
    path = "/setup-payment-method",
    tags = ["billing", "internal"],
    request_body = SetupPaymentMethodRequest,
    responses(
        (status = 200, description = "Setup session URL", body = ApiResponse<String>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn setup_payment_method(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<SetupPaymentMethodRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let success_url = format!("{}?billing_flow=payment_setup", request.url);
    let cancel_url = request.url;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let session = billing_service
            .create_setup_payment_method_session(
                organization_id,
                success_url,
                cancel_url,
                auth.into_entity(),
            )
            .await?;

        Ok(Json(ApiResponse::success(session.url.unwrap())))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Change billing plan
///
/// Upgrades or downgrades the organization's billing plan.
#[utoipa::path(
    post,
    path = "/change-plan",
    tags = ["billing", "internal"],
    request_body = ChangePlanRequest,
    responses(
        (status = 200, description = "Plan change initiated", body = ApiResponse<String>),
        (status = 400, description = "Invalid plan or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn change_plan(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<ChangePlanRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .change_plan(organization_id, request.plan, auth.into_entity())
            .await?;

        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Preview plan change (shows overage counts)
#[utoipa::path(
    get,
    path = "/change-plan/preview",
    tags = ["billing", "internal"],
    params(
        ("plan" = String, Query, description = "Target plan (JSON)"),
    ),
    responses(
        (status = 200, description = "Plan change preview", body = ApiResponse<ChangePlanPreview>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn preview_plan_change(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<ApiResponse<ChangePlanPreview>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let plan_str = params
        .get("plan")
        .ok_or_else(|| ApiError::bad_request("Missing plan parameter"))?;

    let plan: BillingPlan =
        serde_json::from_str(plan_str).map_err(|_| ApiError::bad_request("Invalid plan format"))?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let preview = billing_service
            .preview_plan_change(organization_id, plan)
            .await?;

        Ok(Json(ApiResponse::success(preview)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Handle Stripe webhook
///
/// Internal endpoint for Stripe webhook callbacks.
#[utoipa::path(
    post,
    path = "/webhooks",
    tags = ["billing", "internal"],
    responses(
        (status = 200, description = "Webhook processed", body = EmptyApiResponse),
        (status = 400, description = "Invalid signature or billing not enabled", body = ApiErrorResponse),
    )
)]
async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> ApiResult<Json<ApiResponse<()>>> {
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            ApiError::validation(ErrorCode::ValidationRequired {
                field: "stripe-signature".to_string(),
            })
        })?;

    if let Some(billing_service) = &state.services.billing_service {
        billing_service.handle_webhook(&body, signature).await?;
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Create a billing portal session
#[utoipa::path(
    post,
    path = "/portal",
    tags = ["billing", "internal"],
    request_body = String,
    responses(
        (status = 200, description = "Portal session URL", body = ApiResponse<String>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_portal_session(
    State(state): State<Arc<AppState>>,
    auth: Authorized<RequireVerified<Owner>>,
    Json(return_url): Json<String>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = &state.services.billing_service {
        let session_url = billing_service
            .create_portal_session(organization_id, return_url)
            .await?;

        Ok(Json(ApiResponse::success(session_url)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Submit enterprise plan inquiry
///
/// Updates Brevo contact/company with inquiry data, creates a deal, and
/// tracks an event for automation triggers. Requires authentication to
/// link the inquiry to an organization.
#[utoipa::path(
    post,
    path = "/inquiry",
    tags = ["billing", "internal"],
    request_body = EnterpriseInquiryRequest,
    responses(
        (status = 200, description = "Inquiry submitted successfully", body = EmptyApiResponse),
        (status = 400, description = "Invalid request or Brevo not configured", body = ApiErrorResponse),
        (status = 401, description = "Authentication required", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn submit_enterprise_inquiry(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    auth: Authorized<Viewer>,
    Json(request): Json<EnterpriseInquiryRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if request.email.is_empty() || request.name.is_empty() || request.company.is_empty() {
        return Err(ApiError::validation(ErrorCode::ValidationRequired {
            field: "email, name, company".to_string(),
        }));
    }

    let brevo_service = state
        .services
        .brevo_service
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("Enterprise inquiries are not enabled"))?;

    // 1. Create a deal in the Sales pipeline
    let org = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?
        .ok_or_else(ApiError::organization_required)?;

    let deal_name = format!("Enterprise Inquiry - {}", &request.company);

    let company_ids = org.base.brevo_company_id.map(|id| vec![id]);
    let mut deal_attributes = serde_json::to_value(&request)?;
    if let Some(obj) = deal_attributes.as_object_mut() {
        obj.remove("email");
        obj.remove("company");
        obj.remove("name");
        // Brevo deal attributes must all be strings
        for value in obj.values_mut() {
            if let serde_json::Value::Number(n) = value {
                *value = serde_json::Value::String(n.to_string());
            }
        }
    }
    if let Err(e) = brevo_service
        .client
        .create_deal(&deal_name, Some(deal_attributes), company_ids)
        .await
    {
        tracing::warn!(
            error = %e,
            organization_id = %organization_id,
            "Failed to create Brevo deal for inquiry"
        );
    }

    // 2. Fire event for tracking and triggers
    if let Err(e) = brevo_service
        .client
        .track_event(
            "enterprise_inquiry",
            &request.email,
            Some(serde_json::to_value(&request)?),
        )
        .await
    {
        tracing::warn!(
            error = %e,
            "Failed to track enterprise_inquiry event in Brevo"
        );
    }

    tracing::info!(
        email = %request.email,
        company = %request.company,
        organization_id = %organization_id,
        plan_type = ?request.plan_type,
        client_ip = %ip,
        "Enterprise inquiry submitted to Brevo"
    );

    Ok(Json(ApiResponse::success(())))
}

/// Pause subscription billing
///
/// Pauses billing for a 30/60/90 day window. Eligibility: rolling 6-month
/// cooldown anchored on the org's `last_paused_at`.
#[utoipa::path(
    post,
    path = "/pause",
    tags = ["billing", "internal"],
    request_body = PauseSubscriptionRequest,
    responses(
        (status = 200, description = "Subscription paused", body = ApiResponse<String>),
        (status = 400, description = "Ineligible or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn pause_subscription(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<PauseSubscriptionRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .pause_subscription(organization_id, request.duration_days, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Resume a paused subscription
///
/// Clears Stripe pause collection and re-activates billing. Available while
/// `plan_status === 'paused'`. The prorated pause credit is posted to the
/// customer's Stripe balance asynchronously by the webhook arm that fires
/// for the `pause_collection` clear — the endpoint just returns success.
#[utoipa::path(
    post,
    path = "/resume",
    tags = ["billing", "internal"],
    responses(
        (status = 200, description = "Subscription resumed", body = ApiResponse<String>),
        (status = 400, description = "No paused subscription or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn resume_subscription(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .resume_subscription(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Reactivate a subscription pending cancellation
///
/// Clears Stripe's scheduled-cancellation state (`cancel_at` → None).
/// Available while `plan_status === 'pending_cancellation'`.
#[utoipa::path(
    post,
    path = "/reactivate",
    tags = ["billing", "internal"],
    responses(
        (status = 200, description = "Subscription reactivated", body = ApiResponse<String>),
        (status = 400, description = "No pending cancellation or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn reactivate_subscription(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .reactivate_subscription(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Self-serve trial extend (+7 days, once per org lifetime)
#[utoipa::path(
    post,
    path = "/extend-trial",
    tags = ["billing", "internal"],
    responses(
        (status = 200, description = "Trial extended", body = ApiResponse<String>),
        (status = 400, description = "Ineligible or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn extend_trial(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .extend_trial(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Cancel subscription
///
/// In-app cancel modal endpoint. Sets Stripe `cancel_at` to the current
/// period end (via Stripe's `MaxPeriodEnd` sentinel), stashes the canonical
/// Scanopy reason in subscription metadata, returns the period end so the
/// modal can render the retention disclosure.
#[utoipa::path(
    post,
    path = "/cancel",
    tags = ["billing", "internal"],
    request_body = CancelSubscriptionRequest,
    responses(
        (status = 200, description = "Cancellation initiated", body = ApiResponse<CancelSubscriptionResponse>),
        (status = 400, description = "No active subscription or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn cancel_subscription(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<CancelSubscriptionRequest>,
) -> ApiResult<Json<ApiResponse<CancelSubscriptionResponse>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .cancel_subscription(organization_id, request, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Apply the discount save offer
///
/// Applies the configured Stripe coupon to the subscription. Returns 400
/// when `STRIPE_SAVE_OFFER_COUPON_ID` is unset.
#[utoipa::path(
    post,
    path = "/cancel/apply-discount",
    tags = ["billing", "internal"],
    responses(
        (status = 200, description = "Discount applied", body = ApiResponse<String>),
        (status = 400, description = "Discount not configured or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn apply_discount_save_offer(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .apply_discount_save_offer(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Read live terms for the configured save-offer coupon
///
/// Returns the coupon's `percent_off` and `duration_in_months` so the
/// cancel modal's Discount panel can render the offer dynamically. The
/// payload is `null` when `STRIPE_SAVE_OFFER_COUPON_ID` is unset — the
/// modal hides the panel in that case.
#[utoipa::path(
    get,
    path = "/save-offer-coupon",
    tags = ["billing", "internal"],
    responses(
        (status = 200, description = "Save-offer coupon terms, or null when not configured", body = ApiResponse<Option<SaveOfferCoupon>>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_save_offer_coupon(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
) -> ApiResult<Json<ApiResponse<Option<SaveOfferCoupon>>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let coupon = billing_service
            .get_save_offer_coupon(organization_id)
            .await?;
        Ok(Json(ApiResponse::success(coupon)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}
