//! Login, logout, and current-user session handlers.
use super::*;

#[utoipa::path(
    post,
    path = "/login",
    tags = ["auth", "internal"],
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = ApiResponse<User>),
        (status = 401, description = "Invalid credentials", body = ApiErrorResponse),
        (status = 403, description = "Login forbidden", body = ApiErrorResponse),
    )
)]
pub(crate) async fn login(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    Host(host): Host,
    user_agent: Option<TypedHeader<UserAgent>>,
    session: Session,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<ApiResponse<User>>> {
    if state.config.disable_password_login {
        return Err(ApiError::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::AuthPasswordLoginDisabled,
        ));
    }

    let user_agent = user_agent.map(|u| u.to_string());

    let user = state
        .services
        .auth_service
        .login(request, ip, user_agent)
        .await?;

    // Check if user is trying to log into demo account on non-demo and visa versa
    {
        let plan = state
            .services
            .organization_service
            .get_by_id(&user.base.organization_id)
            .await?
            .and_then(|o| o.base.plan)
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);
        if plan.is_demo() && !is_demo_host(&host) {
            return Err(ApiError::forbidden(
                "You can't log in to the demo account on this instance.",
            ));
        } else if !plan.is_demo() && is_demo_only_host(&host) {
            return Err(ApiError::forbidden(
                "You can only log in to the demo account on this instance.",
            ));
        }
    }

    // Cycle session ID to prevent session fixation attacks
    session
        .cycle_id()
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to cycle session: {}", e)))?;

    session
        .insert("session_epoch", user.base.session_epoch)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;
    session
        .insert("user_id", user.id)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to save session: {}", e)))?;

    Ok(Json(ApiResponse::success(user)))
}

#[utoipa::path(
    post,
    path = "/logout",
    tags = ["auth", "internal"],
    responses(
        (status = 200, description = "Logout successful", body = EmptyApiResponse),
    )
)]
pub(crate) async fn logout(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    user_agent: Option<TypedHeader<UserAgent>>,
    session: Session,
) -> ApiResult<Json<ApiResponse<()>>> {
    if let Ok(Some(user_id)) = session.get::<Uuid>("user_id").await {
        let user_agent = user_agent.map(|u| u.to_string());

        state
            .services
            .auth_service
            .logout(user_id, ip, user_agent)
            .await?;
    }

    session
        .delete()
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to delete session: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

#[utoipa::path(
    post,
    path = "/me",
    tags = ["auth", "internal"],
    responses(
        (status = 200, description = "Current user", body = ApiResponse<User>),
        (status = 401, description = "Not authenticated", body = ApiErrorResponse),
    )
)]
pub(crate) async fn get_current_user(
    State(state): State<Arc<AppState>>,
    session: Session,
) -> ApiResult<Json<ApiResponse<User>>> {
    let user_id: Uuid = session
        .get("user_id")
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to read session: {}", e)))?
        .ok_or_else(ApiError::not_authenticated)?;

    let user = state
        .services
        .user_service
        .get_by_id(&user_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<User>(user_id))?;

    Ok(Json(ApiResponse::success(user)))
}
