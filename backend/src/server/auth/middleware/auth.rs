use std::{fmt::Display, net::IpAddr, str::FromStr};

use cidr::IpCidr;

use crate::server::{
    config::AppState,
    daemon_api_keys::r#impl::base::DaemonApiKey,
    networks::r#impl::Network,
    shared::{
        api_key_common::{ApiKeyCommon, ApiKeyType, check_key_validity, hash_api_key},
        events::types::{AuthEvent, AuthOperation},
        services::traits::CrudService,
        storage::filter::StorableFilter,
        types::api::ApiError,
    },
    users::r#impl::{base::User, permissions::UserOrgPermissions},
};
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, request::Parts},
    response::{IntoResponse, Response},
};
use axum_client_ip::ClientIp;
use chrono::Utc;
use email_address::EmailAddress;
use serde::Deserialize;
use serde::Serialize;
use tower_sessions::Session;
use uuid::Uuid;

pub struct AuthError(pub ApiError);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

/// Represents how an entity authenticated - used for audit logging
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// User authenticated via session cookie
    Session,
    /// Authenticated via user API key (scp_u_ prefix)
    UserApiKey,
    /// Authenticated via daemon API key (scp_d_ prefix)
    DaemonApiKey,
    /// External service authentication (e.g., Prometheus)
    ExternalService,
    /// System-level operation (internal)
    System,
    /// No authentication
    Anonymous,
}

impl Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMethod::Session => write!(f, "session"),
            AuthMethod::UserApiKey => write!(f, "user_api_key"),
            AuthMethod::DaemonApiKey => write!(f, "daemon_api_key"),
            AuthMethod::ExternalService => write!(f, "external_service"),
            AuthMethod::System => write!(f, "system"),
            AuthMethod::Anonymous => write!(f, "anonymous"),
        }
    }
}

/// Represents either an authenticated user, daemon, or user API key
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthenticatedEntity {
    User {
        user_id: Uuid,
        organization_id: Uuid,
        permissions: UserOrgPermissions,
        network_ids: Vec<Uuid>,
        email: EmailAddress,
    },
    Daemon {
        network_id: Uuid,
        api_key_id: Uuid,
        daemon_id: Uuid,
    },
    /// User API key authentication - acts on behalf of a user with potentially restricted permissions
    ApiKey {
        api_key_id: Uuid,
        user_id: Uuid,
        organization_id: Uuid,
        permissions: UserOrgPermissions,
        network_ids: Vec<Uuid>,
    },
    /// External service authentication (e.g., Prometheus, Grafana)
    ExternalService {
        /// Service name from X-Service-Name header
        name: String,
    },
    System,
    Anonymous,
}

impl Display for AuthenticatedEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthenticatedEntity::Anonymous => write!(f, "Anonymous"),
            AuthenticatedEntity::System => write!(f, "System"),
            AuthenticatedEntity::Daemon { .. } => write!(f, "Daemon"),
            AuthenticatedEntity::User {
                user_id,
                permissions,
                ..
            } => write!(
                f,
                "User {{ user_id: {}, permissions: {} }}",
                user_id, permissions
            ),
            AuthenticatedEntity::ApiKey {
                api_key_id,
                user_id,
                permissions,
                ..
            } => write!(
                f,
                "ApiKey {{ api_key_id: {}, user_id: {}, permissions: {} }}",
                api_key_id, user_id, permissions
            ),
            AuthenticatedEntity::ExternalService { name } => {
                write!(f, "ExternalService {{ name: {} }}", name)
            }
        }
    }
}

impl AuthenticatedEntity {
    /// Get the user_id if this is a User or ApiKey, otherwise None
    pub fn user_id(&self) -> Option<Uuid> {
        match self {
            AuthenticatedEntity::User { user_id, .. } => Some(*user_id),
            AuthenticatedEntity::ApiKey { user_id, .. } => Some(*user_id),
            _ => None,
        }
    }

    /// Get the organization_id if this is a User or ApiKey, otherwise None
    pub fn organization_id(&self) -> Option<Uuid> {
        match self {
            AuthenticatedEntity::User {
                organization_id, ..
            } => Some(*organization_id),
            AuthenticatedEntity::ApiKey {
                organization_id, ..
            } => Some(*organization_id),
            _ => None,
        }
    }

    /// Get permissions if this is a User or ApiKey, otherwise None
    pub fn permissions(&self) -> Option<UserOrgPermissions> {
        match self {
            AuthenticatedEntity::User { permissions, .. } => Some(*permissions),
            AuthenticatedEntity::ApiKey { permissions, .. } => Some(*permissions),
            _ => None,
        }
    }

    pub fn entity_id(&self) -> String {
        match self {
            AuthenticatedEntity::User { user_id, .. } => user_id.to_string(),
            AuthenticatedEntity::Daemon { daemon_id, .. } => daemon_id.to_string(),
            AuthenticatedEntity::ApiKey { api_key_id, .. } => api_key_id.to_string(),
            AuthenticatedEntity::ExternalService { name } => format!("external_service:{}", name),
            AuthenticatedEntity::System => "System".to_string(),
            AuthenticatedEntity::Anonymous => "Anonymous".to_string(),
        }
    }

    /// Get network_ids that daemon / user / API key have access to
    pub fn network_ids(&self) -> Vec<Uuid> {
        match self {
            AuthenticatedEntity::Daemon { network_id, .. } => vec![*network_id],
            AuthenticatedEntity::User { network_ids, .. } => network_ids.clone(),
            AuthenticatedEntity::ApiKey { network_ids, .. } => network_ids.clone(),
            AuthenticatedEntity::ExternalService { .. } => vec![], // Global scope, no network restriction
            AuthenticatedEntity::System => vec![],
            AuthenticatedEntity::Anonymous => vec![],
        }
    }

    /// Check if this is a user (session-based authentication)
    pub fn is_user(&self) -> bool {
        matches!(self, AuthenticatedEntity::User { .. })
    }

    /// Check if this is a daemon
    pub fn is_daemon(&self) -> bool {
        matches!(self, AuthenticatedEntity::Daemon { .. })
    }

    /// Check if this is a user API key
    pub fn is_api_key(&self) -> bool {
        matches!(self, AuthenticatedEntity::ApiKey { .. })
    }

    /// Check if this is an external service
    pub fn is_external_service(&self) -> bool {
        matches!(self, AuthenticatedEntity::ExternalService { .. })
    }

    /// Check if this is a user or API key (has user-level permissions)
    pub fn is_user_or_api_key(&self) -> bool {
        matches!(
            self,
            AuthenticatedEntity::User { .. } | AuthenticatedEntity::ApiKey { .. }
        )
    }

    /// Check if this entity has access to the specified network
    pub fn has_network_access(&self, network_id: &Uuid) -> bool {
        self.network_ids().contains(network_id)
    }

    /// Get the email address if this is a User or ApiKey
    pub fn email(&self) -> Option<&EmailAddress> {
        match self {
            AuthenticatedEntity::User { email, .. } => Some(email),
            _ => None,
        }
    }

    /// Get daemon_id if this is a Daemon, otherwise None
    pub fn daemon_id(&self) -> Option<Uuid> {
        match self {
            AuthenticatedEntity::Daemon { daemon_id, .. } => Some(*daemon_id),
            _ => None,
        }
    }
}

impl From<User> for AuthenticatedEntity {
    fn from(value: User) -> Self {
        AuthenticatedEntity::User {
            user_id: value.id,
            organization_id: value.base.organization_id,
            permissions: value.base.permissions,
            network_ids: vec![],
            email: value.base.email,
        }
    }
}

/// Marker to cache failed auth attempts and prevent duplicate event publishing
#[derive(Clone)]
struct AuthAttemptFailed(ApiError);

// Generic authenticated entity extractor - accepts users, daemons, and user API keys
impl<S> FromRequestParts<S> for AuthenticatedEntity
where
    S: Send + Sync + AsRef<AppState>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Check if already extracted (cached in extensions) to avoid duplicate auth
        // This prevents multiple middleware/extractors from triggering repeated DB updates
        if let Some(cached) = parts.extensions.get::<AuthenticatedEntity>() {
            return Ok(cached.clone());
        }

        // Check if auth already failed for this request (prevents duplicate event publishing)
        if let Some(cached_failure) = parts.extensions.get::<AuthAttemptFailed>() {
            return Err(AuthError(cached_failure.0.clone()));
        }

        let result = Self::extract_auth(parts, state).await;

        // Cache result in extensions for subsequent extractors
        match &result {
            Ok(entity) => {
                parts.extensions.insert(entity.clone());
            }
            Err(AuthError(api_error)) => {
                parts
                    .extensions
                    .insert(AuthAttemptFailed(api_error.clone()));
            }
        }

        result
    }
}

impl AuthenticatedEntity {
    /// Internal auth extraction logic - called once and cached
    async fn extract_auth<S>(parts: &mut Parts, state: &S) -> Result<Self, AuthError>
    where
        S: Send + Sync + AsRef<AppState>,
    {
        let app_state = state.as_ref();

        // Extract IP and user agent for failed auth logging
        let ip = ClientIp::from_request_parts(parts, state)
            .await
            .map(|c| c.0)
            .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
        let user_agent = extract_user_agent(&parts.headers);

        // Check for Bearer token in Authorization header
        if let Some(auth_header) = parts.headers.get(axum::http::header::AUTHORIZATION)
            && let Ok(auth_str) = auth_header.to_str()
            && let Some(api_key_raw) = auth_str.strip_prefix("Bearer ")
        {
            // Check for external service token (e.g., Prometheus metrics endpoint)
            // This must come before API key type detection since external tokens don't have prefixes
            if let Some(metrics_token) = &app_state.config.metrics_token
                && !metrics_token.is_empty()
                && api_key_raw == metrics_token
            {
                // External service authentication
                let service_name = parts
                    .headers
                    .get("X-Service-Name")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| "unknown".to_string());

                // Check IP restrictions for this service
                if let Some(allowed_ips) = app_state
                    .config
                    .external_service_allowed_ips
                    .get(&service_name)
                    && !is_ip_allowed(ip, allowed_ips)
                {
                    return Err(AuthError(ApiError::forbidden(&format!(
                        "IP {} not allowed for external service '{}'",
                        ip, service_name
                    ))));
                }

                return Ok(AuthenticatedEntity::ExternalService { name: service_name });
            }

            let hashed_key = hash_api_key(api_key_raw);
            // Extract key prefix for logging (first 8 chars, safe for logging)
            let key_prefix = api_key_raw.get(..8);

            // Detect key type from prefix
            let (key_type, _is_prefixed) = ApiKeyType::from_key(api_key_raw);

            match key_type {
                ApiKeyType::User => {
                    // User API key authentication
                    if let Ok(Some(mut user_api_key)) = app_state
                        .services
                        .user_api_key_service
                        .get_by_key(&hashed_key)
                        .await
                    {
                        let api_key_id = user_api_key.id;
                        let user_id = user_api_key.base.user_id;
                        let organization_id = user_api_key.base.organization_id;
                        let permissions = user_api_key.base.permissions;
                        let service = app_state.services.user_api_key_service.clone();

                        // Check validity using shared trait
                        if let Err(e) = check_key_validity(&user_api_key) {
                            let reason = if user_api_key.is_expired() {
                                "expired"
                            } else {
                                "disabled"
                            };
                            publish_api_key_auth_failed(
                                app_state,
                                ip,
                                user_agent.clone(),
                                key_type,
                                reason,
                                key_prefix,
                            )
                            .await;

                            // Auto-disable expired keys
                            if user_api_key.is_expired() {
                                user_api_key.set_is_enabled(false);
                                tokio::spawn(async move {
                                    let _ = service
                                        .update(&mut user_api_key, AuthenticatedEntity::System)
                                        .await;
                                });
                            }
                            return Err(AuthError(e));
                        }

                        let organization_has_api_access = app_state
                            .services
                            .organization_service
                            .get_by_id(&organization_id)
                            .await
                            .unwrap_or_default()
                            .map(|o| {
                                o.base
                                    .plan
                                    .map(|p| p.features().api_access)
                                    .unwrap_or(false)
                            })
                            .unwrap_or(false);

                        if !organization_has_api_access {
                            return Err(AuthError(ApiError::payment_required(
                                "Your plan does not include api access",
                            )));
                        }

                        // Get network access from junction table
                        let network_ids = app_state
                            .services
                            .user_api_key_service
                            .get_network_ids(&api_key_id)
                            .await
                            .unwrap_or_default();

                        // Update last used asynchronously (don't block auth)
                        user_api_key.set_last_used(Some(Utc::now()));
                        tokio::spawn(async move {
                            let _ = service
                                .update(&mut user_api_key, AuthenticatedEntity::System)
                                .await;
                        });

                        return Ok(AuthenticatedEntity::ApiKey {
                            api_key_id,
                            user_id,
                            organization_id,
                            permissions,
                            network_ids,
                        });
                    }

                    publish_api_key_auth_failed(
                        app_state,
                        ip,
                        user_agent.clone(),
                        key_type,
                        "invalid_key",
                        key_prefix,
                    )
                    .await;
                    return Err(AuthError(ApiError::not_authenticated()));
                }
                ApiKeyType::Daemon => {
                    // Daemon API key authentication - requires X-Daemon-ID header
                    let daemon_id = parts
                        .headers
                        .get("X-Daemon-ID")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .ok_or_else(|| AuthError(ApiError::daemon_required()))?;

                    let api_key_filter = StorableFilter::<DaemonApiKey>::new().api_key(hashed_key);
                    if let Ok(Some(mut api_key)) = app_state
                        .services
                        .daemon_api_key_service
                        .get_one(api_key_filter)
                        .await
                    {
                        let network_id = api_key.base.network_id;
                        let service = app_state.services.daemon_api_key_service.clone();
                        let api_key_id = api_key.id;

                        // Check validity using shared trait
                        if let Err(e) = check_key_validity(&api_key) {
                            let reason = if api_key.is_expired() {
                                "expired"
                            } else {
                                "disabled"
                            };
                            publish_api_key_auth_failed(
                                app_state,
                                ip,
                                user_agent.clone(),
                                key_type,
                                reason,
                                key_prefix,
                            )
                            .await;

                            // Auto-disable expired keys
                            if api_key.is_expired() {
                                api_key.set_is_enabled(false);
                                tokio::spawn(async move {
                                    let _ = service
                                        .update(&mut api_key, AuthenticatedEntity::System)
                                        .await;
                                });
                            }
                            return Err(AuthError(e));
                        }

                        // Update last used asynchronously (don't block auth)
                        api_key.set_last_used(Some(Utc::now()));
                        tokio::spawn(async move {
                            let _ = service
                                .update(&mut api_key, AuthenticatedEntity::System)
                                .await;
                        });

                        // Note: We don't validate that the daemon exists here because:
                        // 1. The daemon may be registering for the first time (doesn't exist yet)
                        // 2. The API key's network_id is the source of truth for authorization
                        // 3. Individual handlers validate daemon-network consistency as needed
                        return Ok(AuthenticatedEntity::Daemon {
                            network_id,
                            api_key_id,
                            daemon_id,
                        });
                    }

                    // Check if this daemon exists to provide a better error message
                    // - If daemon exists: key was rotated/revoked, fail immediately
                    // - If daemon doesn't exist: onboarding scenario, daemon should retry
                    let daemon_exists = app_state
                        .services
                        .daemon_service
                        .get_by_id(&daemon_id)
                        .await
                        .map(|d| d.is_some())
                        .unwrap_or(false);

                    publish_api_key_auth_failed(
                        app_state,
                        ip,
                        user_agent.clone(),
                        key_type,
                        "invalid_key",
                        key_prefix,
                    )
                    .await;

                    if daemon_exists {
                        return Err(AuthError(ApiError::not_authenticated()));
                    }
                    return Err(AuthError(ApiError::not_authenticated()));
                }
            }
        }

        // Try user authentication (session cookie)
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError(ApiError::not_authenticated()))?;

        let user_id: Uuid = session
            .get("user_id")
            .await
            .map_err(|_| AuthError(ApiError::not_authenticated()))?
            .ok_or_else(|| AuthError(ApiError::not_authenticated()))?;

        let user = app_state
            .services
            .user_service
            .get_by_id(&user_id)
            .await
            .map_err(|_| AuthError(ApiError::not_authenticated()))?
            .ok_or_else(|| AuthError(ApiError::not_authenticated()))?;

        let network_ids: Vec<Uuid> = if matches!(
            user.base.permissions,
            UserOrgPermissions::Owner | UserOrgPermissions::Admin
        ) {
            let org_filter =
                StorableFilter::<Network>::new().organization_id(&user.base.organization_id);

            app_state
                .services
                .network_service
                .get_all(org_filter)
                .await
                .map_err(|_| AuthError(ApiError::internal_error("Failed to load networks")))?
                .iter()
                .map(|n| n.id)
                .collect()
        } else {
            // Load network_ids from junction table for non-admin users
            app_state
                .services
                .user_service
                .get_network_ids(&user.id)
                .await
                .map_err(|_| AuthError(ApiError::internal_error("Failed to load user networks")))?
        };

        Ok(AuthenticatedEntity::User {
            user_id: user.id,
            organization_id: user.base.organization_id,
            permissions: user.base.permissions,
            network_ids,
            email: user.base.email,
        })
    }
}

/// Helper to extract user agent from headers
fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

/// Check if an IP address is allowed based on a list of IP/CIDR strings.
///
/// Returns true if:
/// - The allowed list is empty (no restriction)
/// - The IP matches any entry in the list (exact IP or CIDR range)
///
/// Invalid CIDR entries in the list are logged and skipped.
fn is_ip_allowed(ip: IpAddr, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true; // No restriction = allow all
    }

    for entry in allowed {
        // Try to parse as CIDR first
        if let Ok(cidr) = IpCidr::from_str(entry) {
            if cidr.contains(&ip) {
                return true;
            }
        } else if let Ok(exact_ip) = IpAddr::from_str(entry) {
            // Try as exact IP address
            if ip == exact_ip {
                return true;
            }
        } else {
            // Log invalid entry but don't fail
            tracing::warn!(
                "Invalid IP/CIDR entry in external service allowed IPs: {}",
                entry
            );
        }
    }

    false
}

/// Publish a failed API key authentication event
async fn publish_api_key_auth_failed(
    app_state: &AppState,
    ip: IpAddr,
    user_agent: Option<String>,
    key_type: ApiKeyType,
    reason: &str,
    key_prefix: Option<&str>,
) {
    let key_type_str = match key_type {
        ApiKeyType::User => "user",
        ApiKeyType::Daemon => "daemon",
    };

    let metadata = serde_json::json!({
        "key_type": key_type_str,
        "reason": reason,
        "key_prefix": key_prefix,
    });

    let event = AuthEvent::new(
        Uuid::new_v4(),
        None, // No user_id for failed auth
        None, // No organization_id
        AuthOperation::ApiKeyAuthFailed,
        Utc::now(),
        ip,
        user_agent,
        metadata,
        AuthenticatedEntity::Anonymous,
    );

    // Fire and forget - don't block auth on event publishing
    let event_bus = app_state.services.event_bus.clone();
    tokio::spawn(async move {
        if let Err(e) = event_bus.publish_auth(event).await {
            tracing::warn!(error = %e, "Failed to publish API key auth failed event");
        }
    });
}

/// Extractor that only accepts user API key authentication (rejects users and daemons)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthenticatedApiKey {
    pub api_key_id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub permissions: UserOrgPermissions,
    pub network_ids: Vec<Uuid>,
}

impl From<AuthenticatedApiKey> for AuthenticatedEntity {
    fn from(value: AuthenticatedApiKey) -> Self {
        AuthenticatedEntity::ApiKey {
            api_key_id: value.api_key_id,
            user_id: value.user_id,
            organization_id: value.organization_id,
            permissions: value.permissions,
            network_ids: value.network_ids,
        }
    }
}

impl<S> FromRequestParts<S> for AuthenticatedApiKey
where
    S: Send + Sync + AsRef<AppState>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let entity = AuthenticatedEntity::from_request_parts(parts, state).await?;

        match entity {
            AuthenticatedEntity::ApiKey {
                api_key_id,
                user_id,
                organization_id,
                permissions,
                network_ids,
            } => Ok(AuthenticatedApiKey {
                api_key_id,
                user_id,
                organization_id,
                permissions,
                network_ids,
            }),
            _ => Err(AuthError(ApiError::api_key_required())),
        }
    }
}
