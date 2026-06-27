use crate::server::auth::r#impl::oidc::OidcProviderMetadata;
use crate::server::license::service::LicenseService;
use crate::server::shared::types::api::ApiResponse;
use crate::server::{
    auth::r#impl::oidc::OidcProviderConfig, shared::services::factory::ServiceFactory,
};
use anyhow::{Error, Result};
use axum::Json;
use axum::extract::State;
use axum::http::header::CACHE_CONTROL;
use axum::response::IntoResponse;
use clap::Parser;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::PostgresStore;
use utoipa::ToSchema;

use crate::server::shared::storage::factory::StorageFactory;
use sqlx::PgPool;

#[derive(Parser)]
#[command(name = "scanopy-server")]
#[command(about = "Scanopy server")]
pub struct ServerCli {
    /// Override server port
    #[arg(long)]
    server_port: Option<u16>,

    /// Override log level
    #[arg(long)]
    log_level: Option<String>,

    /// Override rust system log level
    #[arg(long)]
    rust_log: Option<String>,

    /// Override database path
    #[arg(long)]
    database_url: Option<String>,

    /// Override integrated daemon url
    #[arg(long)]
    integrated_daemon_url: Option<String>,

    /// Use secure session cookies (if serving UI behind HTTPS)
    #[arg(long)]
    use_secure_session_cookies: Option<bool>,

    /// Enable or disable registration flow
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    disable_registration: Option<bool>,

    /// Disable email/password login (use when OIDC is configured)
    #[arg(long, num_args = 0..=1, default_missing_value = "true")]
    disable_password_login: Option<bool>,

    /// Stripe publishable key (exposed to the frontend for Stripe Elements)
    #[arg(long)]
    stripe_key: Option<String>,

    /// Stripe secret key
    #[arg(long)]
    stripe_secret: Option<String>,

    /// Stripe webhook signing secret
    #[arg(long)]
    stripe_webhook_secret: Option<String>,

    #[arg(long)]
    smtp_username: Option<String>,

    #[arg(long)]
    smtp_password: Option<String>,

    /// Email used as to/from in emails send by Scanopy using SMTP
    #[arg(long)]
    smtp_email: Option<String>,

    #[arg(long)]
    smtp_relay: Option<String>,

    #[arg(long)]
    smtp_port: Option<u16>,

    /// Server URL used in features like password reset and invite links
    #[arg(long)]
    public_url: Option<String>,

    /// Configure what proxy (if any) is providing IP address for requests, ie in a reverse proxy setup, for accurate IP in auth event logging
    #[arg(long)]
    pub client_ip_source: Option<String>,

    /// List of OIDC providers
    #[arg(long)]
    pub oidc_providers: Option<String>,

    #[arg(long)]
    pub posthog_key: Option<String>,

    #[arg(long)]
    pub metrics_token: Option<String>,

    #[arg(long)]
    pub brevo_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server_port: u16,
    pub log_level: String,
    pub rust_log: String,
    pub database_url: String,
    pub web_external_path: Option<PathBuf>,
    pub public_url: String,
    pub integrated_daemon_url: Option<String>,
    pub use_secure_session_cookies: bool,
    pub disable_registration: bool,
    pub disable_password_login: bool,
    pub client_ip_source: Option<String>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_relay: Option<String>,
    pub smtp_email: Option<String>,
    /// SMTP port. Defaults to 465 (implicit TLS) when unset. Ports other than
    /// 465 use STARTTLS (e.g. 587, 25).
    pub smtp_port: Option<u16>,
    #[serde(default)]
    pub oidc_providers: Option<Vec<OidcProviderConfig>>,

    // Used in SaaS deployment
    pub stripe_key: Option<String>,
    pub stripe_secret: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub posthog_key: Option<String>,

    // Testing
    #[serde(default)]
    pub enforce_billing_for_testing: bool,

    // Metrics
    pub metrics_token: Option<String>,

    // Brevo CRM integration
    pub brevo_api_key: Option<String>,

    // External service IP restrictions
    // Maps service name (lowercase) to list of allowed IPs/CIDRs
    // Populated from SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS env vars
    #[serde(default)]
    pub external_service_allowed_ips: HashMap<String, Vec<String>>,

    // License key for commercial self-hosted deployments
    pub license_key: Option<String>,

    /// Override the snapshot retention window for self-hosted / community /
    /// enterprise / demo plans. Default 90 days when unset. Ignored for the
    /// SaaS-tier plans (Free / Starter / Pro / Business / Team) which carry
    /// fixed per-tier retention values. Picks up `SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE`
    /// from the env automatically via the existing Figment env-var pipeline.
    #[serde(default)]
    pub snapshot_retention_days_override: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentType {
    Cloud,
    Commercial,
    Community,
}

impl DeploymentType {
    /// True for deployments the customer hosts themselves (Commercial /
    /// Community). Cloud is the only Scanopy-LLC-operated deployment, so this
    /// is the "is Scanopy LLC the sender" signal the email footer gates on.
    pub fn is_self_hosted(&self) -> bool {
        !matches!(self, DeploymentType::Cloud)
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PublicConfigResponse {
    pub server_port: u16,
    pub disable_registration: bool,
    pub disable_password_login: bool,
    pub oidc_providers: Vec<OidcProviderMetadata>,
    pub billing_enabled: bool,
    /// Stripe publishable key, exposed so the frontend can mount Stripe
    /// Elements (Payment Element) for in-app card collection. `None` when
    /// billing isn't configured. Publishable keys are safe to expose to the
    /// browser (same as `posthog_key`).
    pub stripe_publishable_key: Option<String>,
    /// `STRIPE_SAVE_OFFER_COUPON_ID` env var is set. When false, the
    /// cancel modal hides the discount save-offer panel so the user
    /// doesn't see an option the deployment can't fulfil.
    pub discount_save_offer_available: bool,
    pub has_integrated_daemon: bool,
    pub has_email_service: bool,
    pub has_email_opt_in: bool,
    pub public_url: String,
    pub posthog_key: Option<String>,
    pub needs_cookie_consent: bool,
    pub deployment_type: DeploymentType,
    pub license_status: Option<String>,
    /// Hard expiry — the drop-dead date after which the server rejects
    /// the key. Referenced by the grace-period banner.
    pub license_expiry: Option<String>,
    /// User-visible expiry — the date displayed to end users under
    /// normal operation. 7 days earlier than `license_expiry` for keys
    /// issued after grace-period support landed.
    pub license_intended_expiry: Option<String>,
    /// True when the license is past `intended_exp` but not yet past
    /// the hard `exp` — the silent grace window.
    pub license_in_grace_period: bool,
    /// `SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE` if set on this instance.
    /// Frontend uses it inside the plan-comparison view to display the
    /// effective retention for this deployment rather than the per-plan
    /// fixture default.
    pub snapshot_retention_days_override: Option<u32>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_port: 60072,
            log_level: "info".to_string(),
            rust_log: "".to_string(),
            database_url: "postgresql://postgres:password@localhost:5432/scanopy".to_string(),
            public_url: "http://localhost:60072".to_string(),
            web_external_path: None,
            use_secure_session_cookies: false,
            integrated_daemon_url: None,
            disable_registration: false,
            disable_password_login: false,
            stripe_key: None,
            stripe_secret: None,
            stripe_webhook_secret: None,
            smtp_username: None,
            smtp_password: None,
            smtp_email: None,
            smtp_relay: None,
            smtp_port: None,
            client_ip_source: None,
            oidc_providers: None,
            posthog_key: None,
            enforce_billing_for_testing: false,
            metrics_token: None,
            brevo_api_key: None,
            external_service_allowed_ips: HashMap::new(),
            license_key: None,
            snapshot_retention_days_override: None,
        }
    }
}

impl ServerConfig {
    pub fn load(cli_args: ServerCli) -> anyhow::Result<Self> {
        // Standard configuration layering: Defaults → Env → CLI (highest priority)
        let mut figment = Figment::from(Serialized::defaults(ServerConfig::default()))
            .merge(Toml::file("../oidc.toml"))
            .merge(Env::prefixed("SCANOPY_"));

        // Add CLI overrides (highest priority) - only if explicitly provided
        if let Some(server_port) = cli_args.server_port {
            figment = figment.merge(("server_port", server_port));
        }
        if let Some(log_level) = cli_args.log_level {
            figment = figment.merge(("log_level", log_level));
        }
        if let Some(rust_log) = cli_args.rust_log {
            figment = figment.merge(("rust_log", rust_log));
        }
        if let Some(database_url) = cli_args.database_url {
            figment = figment.merge(("database_url", database_url));
        }
        if let Some(integrated_daemon_url) = cli_args.integrated_daemon_url {
            figment = figment.merge(("integrated_daemon_url", integrated_daemon_url));
        }
        if let Some(use_secure_session_cookies) = cli_args.use_secure_session_cookies {
            figment = figment.merge(("use_secure_session_cookies", use_secure_session_cookies));
        }
        if let Some(stripe_key) = cli_args.stripe_key {
            figment = figment.merge(("stripe_key", stripe_key));
        }
        if let Some(stripe_secret) = cli_args.stripe_secret {
            figment = figment.merge(("stripe_secret", stripe_secret));
        }
        if let Some(stripe_webhook_secret) = cli_args.stripe_webhook_secret {
            figment = figment.merge(("stripe_webhook_secret", stripe_webhook_secret));
        }
        if let Some(smtp_username) = cli_args.smtp_username {
            figment = figment.merge(("smtp_username", smtp_username));
        }
        if let Some(smtp_password) = cli_args.smtp_password {
            figment = figment.merge(("smtp_password", smtp_password));
        }
        if let Some(smtp_relay) = cli_args.smtp_relay {
            figment = figment.merge(("smtp_relay", smtp_relay));
        }
        if let Some(smtp_email) = cli_args.smtp_email {
            figment = figment.merge(("smtp_email", smtp_email));
        }
        if let Some(smtp_port) = cli_args.smtp_port {
            figment = figment.merge(("smtp_port", smtp_port));
        }
        if let Some(public_url) = cli_args.public_url {
            figment = figment.merge(("public_url", public_url));
        }
        if let Some(client_ip_source) = cli_args.client_ip_source {
            figment = figment.merge(("client_ip_source", client_ip_source));
        }
        if let Some(oidc_providers) = cli_args.oidc_providers {
            figment = figment.merge(("oidc_providers", oidc_providers));
        }
        if let Some(posthog_key) = cli_args.posthog_key {
            figment = figment.merge(("posthog_key", posthog_key));
        }
        if let Some(disable_registration) = cli_args.disable_registration {
            figment = figment.merge(("disable_registration", disable_registration));
        }
        if let Some(disable_password_login) = cli_args.disable_password_login {
            figment = figment.merge(("disable_password_login", disable_password_login));
        }
        if let Some(metrics_token) = cli_args.metrics_token {
            figment = figment.merge(("metrics_token", metrics_token));
        }
        if let Some(brevo_api_key) = cli_args.brevo_api_key {
            figment = figment.merge(("brevo_api_key", brevo_api_key));
        }

        let mut config: ServerConfig = figment
            .extract()
            .map_err(|e| Error::msg(format!("Configuration error: {}", e)))?;

        // Parse external service IP restrictions from env vars
        // Format: SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS=192.168.1.0/24,10.0.0.1
        config.external_service_allowed_ips = Self::load_external_service_allowed_ips();

        Ok(config)
    }

    /// Load external service IP restrictions from environment variables.
    ///
    /// Scans for env vars matching `SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS`
    /// and parses them into a HashMap of service name -> allowed IPs.
    fn load_external_service_allowed_ips() -> HashMap<String, Vec<String>> {
        let mut result = HashMap::new();
        let prefix = "SCANOPY_EXTERNAL_SERVICE_";
        let suffix = "_ALLOWED_IPS";

        for (key, value) in std::env::vars() {
            if key.starts_with(prefix) && key.ends_with(suffix) {
                // Extract service name from: SCANOPY_EXTERNAL_SERVICE_<NAME>_ALLOWED_IPS
                let service_name = key
                    .strip_prefix(prefix)
                    .and_then(|s| s.strip_suffix(suffix))
                    .map(|s| s.to_lowercase());

                if let Some(name) = service_name
                    && !name.is_empty()
                {
                    let ips: Vec<String> = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    if !ips.is_empty() {
                        result.insert(name, ips);
                    }
                }
            }
        }

        result
    }

    pub fn database_url(&self) -> String {
        self.database_url.to_string()
    }
}

pub struct AppState {
    pub config: ServerConfig,
    pub services: ServiceFactory,
    pub session_store: SessionManagerLayer<PostgresStore>,
    pub license_service: Arc<LicenseService>,
    pub pool: PgPool,
}

impl AppState {
    pub async fn new(config: ServerConfig) -> Result<Arc<Self>, Error> {
        let storage =
            StorageFactory::new(&config.database_url(), config.use_secure_session_cookies).await?;
        let services = ServiceFactory::new(&storage, config.clone()).await?;

        let is_commercial = cfg!(feature = "commercial");
        let license_service = Arc::new(LicenseService::new(
            config.license_key.clone(),
            is_commercial,
        ));

        Ok(Arc::new(Self {
            config,
            services,
            session_store: storage.sessions,
            license_service,
            pool: storage.pool,
        }))
    }
}

pub fn get_deployment_type(config: &ServerConfig) -> DeploymentType {
    if config.stripe_secret.is_some() {
        DeploymentType::Cloud
    } else {
        #[cfg(feature = "commercial")]
        {
            DeploymentType::Commercial
        }
        #[cfg(not(feature = "commercial"))]
        {
            DeploymentType::Community
        }
    }
}

/// Get public server configuration
///
/// Returns public configuration settings like OIDC providers, billing status, etc.
#[utoipa::path(
    get,
    path = "/api/config",
    tags = ["config", "internal"],
    responses(
        (status = 200, description = "Public server configuration", body = ApiResponse<PublicConfigResponse>)
    )
)]
pub async fn get_public_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let oidc_providers = state
        .services
        .oidc_service
        .as_ref()
        .map(|o| o.as_ref().list_providers())
        .unwrap_or_default();

    let deployment_type = get_deployment_type(&state.config);
    let current_license = state.license_service.current_status().await;
    let license_status = current_license.as_api_string().map(String::from);
    let license_expiry = current_license.expiry_date();
    let license_intended_expiry = current_license.intended_expiry_date();
    let license_in_grace_period = current_license.in_grace_period();

    (
        [(CACHE_CONTROL, "no-store, no-cache, must-revalidate")],
        Json(ApiResponse::success(PublicConfigResponse {
            server_port: state.config.server_port,
            disable_registration: state.config.disable_registration,
            disable_password_login: state.config.disable_password_login,
            oidc_providers,
            billing_enabled: state.config.stripe_secret.is_some(),
            stripe_publishable_key: state.config.stripe_key.clone(),
            discount_save_offer_available: std::env::var("STRIPE_SAVE_OFFER_COUPON_ID").is_ok(),
            has_integrated_daemon: state.config.integrated_daemon_url.is_some(),
            has_email_service: state.config.brevo_api_key.is_some()
                || (state.config.smtp_password.is_some()
                    && state.config.smtp_username.is_some()
                    && state.config.smtp_email.is_some()
                    && state.config.smtp_relay.is_some()),
            public_url: state.config.public_url.clone(),
            has_email_opt_in: state.config.brevo_api_key.is_some(),
            posthog_key: state.config.posthog_key.clone(),
            needs_cookie_consent: state.config.posthog_key.is_some()
                || state.config.brevo_api_key.is_some(),
            deployment_type,
            license_status,
            license_expiry,
            license_intended_expiry,
            license_in_grace_period,
            snapshot_retention_days_override: state.config.snapshot_retention_days_override,
        })),
    )
}
