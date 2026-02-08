use crate::server::users::r#impl::permissions::UserOrgPermissions;
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

pub struct LoginRegisterParams {
    pub org_id: Option<Uuid>,
    pub permissions: Option<UserOrgPermissions>,
    pub ip: IpAddr,
    pub user_agent: Option<String>,
    pub network_ids: Vec<Uuid>,
}

pub struct ProvisionUserParams {
    pub email: EmailAddress,
    pub password_hash: Option<String>,
    pub oidc_subject: Option<String>,
    pub oidc_provider: Option<String>,
    pub org_id: Option<Uuid>,
    pub permissions: Option<UserOrgPermissions>,
    pub network_ids: Vec<Uuid>,
    pub terms_accepted_at: Option<DateTime<Utc>>,
    /// Whether billing is enabled (if false, sets default billing plan for self-hosted)
    pub billing_enabled: bool,
    /// Whether user opted in to marketing communications
    pub marketing_opt_in: bool,
}

/// Network setup data for a single network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingNetworkSetup {
    pub name: String,
    pub network_id: Uuid,
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

/// Setup data collected before registration (org name, networks, seed preference)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSetup {
    pub org_name: String,
    pub networks: Vec<PendingNetworkSetup>,
    /// Use case selection (homelab, company, msp)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_case: Option<String>,
    /// Company size selection
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_size: Option<String>,
    /// Job title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_title: Option<String>,
    /// How they heard about Scanopy
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referral_source: Option<String>,
    /// Free-text referral source (when "other" is selected)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referral_source_other: Option<String>,
}

/// Daemon setup data collected before registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDaemonSetup {
    pub daemon_name: String,
    pub network_id: Uuid,
    pub api_key_raw: Option<String>, // None if install_later
}
