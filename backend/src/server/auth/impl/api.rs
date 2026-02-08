use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Login request from client
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,

    #[validate(length(min = 10, message = "Password must be at least 10 characters"))]
    pub password: String,
}

/// Registration request from client
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,

    #[validate(length(min = 10, message = "Password must be at least 10 characters"))]
    #[validate(custom(function = "validate_password_complexity"))]
    pub password: String,
    pub terms_accepted: bool,
    #[serde(default)]
    pub marketing_opt_in: bool,
}

/// Validate password complexity requirements
fn validate_password_complexity(password: &str) -> Result<(), validator::ValidationError> {
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());

    if !has_uppercase || !has_lowercase || !has_digit {
        let mut err = validator::ValidationError::new("password_complexity");
        err.message = Some("Password must contain uppercase, lowercase, and number".into());
        return Err(err);
    }

    Ok(())
}

/// Session user info (stored in session, not in database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUser {
    pub user_id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct OidcCallbackParams {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateEmailPasswordRequest {
    pub password: Option<String>,
    #[schema(value_type = Option<String>, format = "email")]
    pub email: Option<EmailAddress>,
}

#[derive(Debug, Deserialize)]
pub struct OidcAuthorizeParams {
    pub flow: Option<String>, // "login", "register", or "link"
    pub return_url: Option<String>,
    pub terms_accepted: Option<bool>,
    pub marketing_opt_in: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

/// Network configuration for setup
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkSetup {
    pub name: String,
    /// Whether SNMP is enabled for this network
    #[serde(default)]
    pub snmp_enabled: bool,
    /// SNMP version ("V2c" or "V3")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snmp_version: Option<String>,
    /// SNMP community string (for V2c)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snmp_community: Option<String>,
}

/// Setup request for pre-registration org/network configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetupRequest {
    pub organization_name: String,
    pub networks: Vec<NetworkSetup>,
}

/// Response from setup endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetupResponse {
    pub network_ids: Vec<Uuid>,
}

/// Daemon setup request for pre-registration daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonSetupRequest {
    pub daemon_name: String,
    pub network_id: Uuid,
    #[serde(default)]
    pub install_later: bool,
}

/// Response from daemon setup endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonSetupResponse {
    pub api_key: Option<String>,
}

/// Request to verify email using token
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailRequest {
    pub token: String,
}

/// Request to resend verification email
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResendVerificationRequest {
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,
}

/// Request to save onboarding step
#[derive(Debug, Deserialize, ToSchema)]
pub struct OnboardingStepRequest {
    pub step: String,
    /// Use case selection (homelab, company, msp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_case: Option<String>,
    /// Job title/role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_title: Option<String>,
    /// Company size
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_size: Option<String>,
    /// Referral source (how they heard about Scanopy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral_source: Option<String>,
    /// Free-text referral source (when "other" is selected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referral_source_other: Option<String>,
}

/// Network data in onboarding state response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnboardingNetworkState {
    /// Network ID (if created)
    pub id: Option<Uuid>,
    /// Network name
    pub name: String,
    /// Whether SNMP is enabled
    #[serde(default)]
    pub snmp_enabled: bool,
    /// SNMP version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snmp_version: Option<String>,
    /// SNMP community string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snmp_community: Option<String>,
}

/// Daemon setup data in onboarding state response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnboardingDaemonSetupState {
    /// Network ID this daemon is for
    pub network_id: Uuid,
    /// Daemon name
    pub daemon_name: String,
    /// API key (only returned if user chose to install now)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Response from onboarding state endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnboardingStateResponse {
    /// Current onboarding step (if any)
    pub step: Option<String>,
    /// Use case selection (homelab, company, msp)
    pub use_case: Option<String>,
    /// Organization name from pending setup
    pub org_name: Option<String>,
    /// Networks from pending setup (with names and IDs)
    pub networks: Vec<OnboardingNetworkState>,
    /// Network IDs from pending setup (if any) - kept for backwards compatibility
    pub network_ids: Vec<Uuid>,
    /// Daemon setups (if any)
    #[serde(default)]
    pub daemon_setups: Vec<OnboardingDaemonSetupState>,
}
