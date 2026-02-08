use crate::server::auth::middleware::permissions::{Authorized, Owner, Viewer};
use crate::server::billing::types::api::{
    ChangePlanPreview, ChangePlanRequest, CreateCheckoutRequest, SetupPaymentMethodRequest,
};
use crate::server::billing::types::base::BillingPlan;
use crate::server::brevo::types::CompanyAttributes;
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
use chrono::Utc;
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

    // Build success/cancel URLs
    let success_url = format!("{}?session_id={{CHECKOUT_SESSION_ID}}", request.url);
    let cancel_url = format!("{}/billing", request.url);

    if let Some(billing_service) = state.services.billing_service.clone() {
        let current_plans = billing_service.get_plans();

        if !current_plans.contains(&request.plan) {
            return Err(ApiError::validation(ErrorCode::ValidationInvalidFormat {
                field: "plan".to_string(),
            }));
        }

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

    let success_url = format!("{}?setup_complete=true", request.url);
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
    auth: Authorized<Owner>,
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

    // 1. Update Company via CRM API (sets inquiry-specific properties)
    let org = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?
        .ok_or_else(ApiError::organization_required)?;

    if let Some(company_id) = &org.base.brevo_company_id {
        let mut company_attrs = CompanyAttributes::new().with_inquiry_date(Utc::now());

        if let Some(urgency) = &request.urgency {
            company_attrs = company_attrs.with_inquiry_urgency(urgency);
        }
        if let Some(network_count) = request.network_count {
            company_attrs = company_attrs.with_inquiry_network_count(network_count);
        }
        if let Some(plan_type) = &request.plan_type {
            company_attrs = company_attrs.with_inquiry_plan_type(plan_type);
        }

        if let Err(e) = brevo_service
            .client
            .update_company(company_id, company_attrs)
            .await
        {
            tracing::warn!(
                error = %e,
                organization_id = %organization_id,
                "Failed to update Brevo company with inquiry properties"
            );
        }

        // 2. Create a deal for the inquiry
        let deal_name = format!("Enterprise Inquiry - {}", &request.company);
        let mut deal_attrs = std::collections::HashMap::new();
        deal_attrs.insert("message".to_string(), serde_json::json!(&request.message));
        deal_attrs.insert(
            "team_size".to_string(),
            serde_json::json!(&request.team_size),
        );
        if let Some(plan_type) = &request.plan_type {
            deal_attrs.insert("plan_type".to_string(), serde_json::json!(plan_type));
        }

        if let Err(e) = brevo_service
            .client
            .create_deal(
                &deal_name,
                Some(deal_attrs),
                None,
                Some(vec![company_id.clone()]),
            )
            .await
        {
            tracing::warn!(
                error = %e,
                organization_id = %organization_id,
                "Failed to create Brevo deal for inquiry"
            );
        }
    }

    // 3. Track event for automation triggers (notifications)
    let mut event_props = std::collections::HashMap::new();
    event_props.insert("company".to_string(), serde_json::json!(&request.company));
    event_props.insert(
        "team_size".to_string(),
        serde_json::json!(&request.team_size),
    );
    event_props.insert("message".to_string(), serde_json::json!(&request.message));
    if let Some(urgency) = &request.urgency {
        event_props.insert("urgency".to_string(), serde_json::json!(urgency));
    }
    if let Some(plan_type) = &request.plan_type {
        event_props.insert("plan_type".to_string(), serde_json::json!(plan_type));
    }

    if let Err(e) = brevo_service
        .client
        .track_event("enterprise_inquiry", &request.email, Some(event_props))
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
