use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// HubSpot form field for form submissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubSpotFormField {
    pub name: String,
    pub value: String,
}

impl HubSpotFormField {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Context for HubSpot form submissions
#[derive(Debug, Clone, Default)]
pub struct HubSpotFormContext {
    /// HubSpot tracking cookie (links submission to visitor)
    pub hutk: Option<String>,
    /// Client IP address (for analytics)
    pub ip_address: Option<String>,
    /// Page URI where form was submitted
    pub page_uri: String,
    /// Page name/title
    pub page_name: String,
}

impl HubSpotFormContext {
    pub fn new(page_uri: impl Into<String>, page_name: impl Into<String>) -> Self {
        Self {
            hutk: None,
            ip_address: None,
            page_uri: page_uri.into(),
            page_name: page_name.into(),
        }
    }

    pub fn with_hutk(mut self, hutk: Option<String>) -> Self {
        self.hutk = hutk;
        self
    }

    pub fn with_ip_address(mut self, ip: Option<String>) -> Self {
        self.ip_address = ip;
        self
    }
}

/// HubSpot contact properties for Scanopy users
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContactProperties {
    /// Standard HubSpot property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Standard HubSpot property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firstname: Option<String>,

    /// Standard HubSpot property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lastname: Option<String>,

    /// Standard HubSpot property - job title/role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jobtitle: Option<String>,

    /// Scanopy user UUID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_user_id: Option<String>,

    /// Scanopy organization UUID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_org_id: Option<String>,

    /// User role: owner, admin, member, viewer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_role: Option<String>,

    /// Signup source: organic, invite, enterprise_inquiry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_signup_source: Option<String>,

    /// Use case: homelab, company, msp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_use_case: Option<String>,

    /// Account creation date (ISO 8601) - when this user signed up
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_signup_date: Option<String>,

    /// Last login date (ISO 8601) - updated on LoginSuccess
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_last_login_date: Option<String>,

    /// Whether user opted in to marketing communications at registration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_marketing_opt_in: Option<bool>,
}

impl ContactProperties {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.scanopy_user_id = Some(user_id.to_string());
        self
    }

    pub fn with_org_id(mut self, org_id: Uuid) -> Self {
        self.scanopy_org_id = Some(org_id.to_string());
        self
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.scanopy_role = Some(role.into());
        self
    }

    pub fn with_signup_source(mut self, source: impl Into<String>) -> Self {
        self.scanopy_signup_source = Some(source.into());
        self
    }

    pub fn with_use_case(mut self, use_case: impl Into<String>) -> Self {
        self.scanopy_use_case = Some(use_case.into());
        self
    }

    pub fn with_signup_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_signup_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_last_login_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_last_login_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_jobtitle(mut self, title: impl Into<String>) -> Self {
        self.jobtitle = Some(title.into());
        self
    }

    pub fn with_marketing_opt_in(mut self, opt_in: bool) -> Self {
        self.scanopy_marketing_opt_in = Some(opt_in);
        self
    }
}

/// HubSpot company properties for Scanopy organizations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompanyProperties {
    /// Standard HubSpot property - company name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Scanopy organization UUID (used as unique identifier for upsert)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_org_id: Option<String>,

    /// Organization type: homelab, company, msp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_org_type: Option<String>,

    /// Company size: 1-10, 11-25, 26-50, 51-100, 101-250, 251-500, 501-1000, 1001+
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_company_size: Option<String>,

    /// Plan type: community, starter, pro, team, business, enterprise
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_plan_type: Option<String>,

    /// Plan status: trialing, active, canceled, past_due
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_plan_status: Option<String>,

    /// Monthly recurring revenue in cents
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_mrr: Option<i64>,

    /// Total networks in organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_network_count: Option<i64>,

    /// Total hosts across all networks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_host_count: Option<i64>,

    /// Total users in organization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_user_count: Option<i64>,

    /// Plan's included networks limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_network_limit: Option<i64>,

    /// Plan's included seats limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_seat_limit: Option<i64>,

    /// Organization created date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_created_date: Option<String>,

    /// Last discovery date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_last_discovery_date: Option<String>,

    /// Total discovery count (for engagement tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_discovery_count: Option<i64>,

    // === Onboarding milestone dates (org-level) ===
    /// First daemon registered date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_first_daemon_date: Option<String>,

    /// First discovery completed date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_first_discovery_date: Option<String>,

    /// Trial start date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_trial_started_date: Option<String>,

    /// Checkout/conversion completed date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_checkout_completed_date: Option<String>,

    // === Engagement milestone dates ===
    /// First network created date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_first_network_date: Option<String>,

    /// First tag created date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_first_tag_date: Option<String>,

    /// First user API key created date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_first_api_key_date: Option<String>,

    /// First SNMP credential created date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_first_snmp_credential_date: Option<String>,

    /// First invite sent date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_first_invite_sent_date: Option<String>,

    /// First invite accepted date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_first_invite_accepted_date: Option<String>,

    /// Urgency for enterprise inquiry: immediately, 1-3 months, 3-6 months, exploring
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_urgency: Option<String>,

    // === Inquiry-specific fields (separate from actual usage data) ===
    /// Plan type being inquired about (inquiry context, not actual subscription)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_inquiry_plan_type: Option<String>,

    /// Urgency from inquiry form (inquiry context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_inquiry_urgency: Option<String>,

    /// Network count from inquiry form (inquiry context, not actual count)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_inquiry_network_count: Option<i64>,

    /// Date of inquiry submission (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_inquiry_date: Option<String>,

    /// Flag for non-commercial orgs that were synced before filtering was added
    /// Used to identify records that can be bulk-deleted in HubSpot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_non_commercial: Option<bool>,
}

impl CompanyProperties {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_org_id(mut self, org_id: Uuid) -> Self {
        self.scanopy_org_id = Some(org_id.to_string());
        self
    }

    pub fn with_org_type(mut self, org_type: impl Into<String>) -> Self {
        self.scanopy_org_type = Some(org_type.into());
        self
    }

    pub fn with_company_size(mut self, size: impl Into<String>) -> Self {
        self.scanopy_company_size = Some(size.into());
        self
    }

    pub fn with_plan_type(mut self, plan_type: impl Into<String>) -> Self {
        self.scanopy_plan_type = Some(plan_type.into());
        self
    }

    pub fn with_plan_status(mut self, status: impl Into<String>) -> Self {
        self.scanopy_plan_status = Some(status.into());
        self
    }

    pub fn with_mrr(mut self, mrr_cents: i64) -> Self {
        self.scanopy_mrr = Some(mrr_cents);
        self
    }

    pub fn with_network_count(mut self, count: i64) -> Self {
        self.scanopy_network_count = Some(count);
        self
    }

    pub fn with_host_count(mut self, count: i64) -> Self {
        self.scanopy_host_count = Some(count);
        self
    }

    pub fn with_user_count(mut self, count: i64) -> Self {
        self.scanopy_user_count = Some(count);
        self
    }

    pub fn with_network_limit(mut self, limit: i64) -> Self {
        self.scanopy_network_limit = Some(limit);
        self
    }

    pub fn with_seat_limit(mut self, limit: i64) -> Self {
        self.scanopy_seat_limit = Some(limit);
        self
    }

    pub fn with_created_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_created_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_last_discovery_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_last_discovery_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_discovery_count(mut self, count: i64) -> Self {
        self.scanopy_discovery_count = Some(count);
        self
    }

    // === Onboarding milestone setters ===

    pub fn with_first_daemon_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_daemon_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_discovery_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_discovery_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_trial_started_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_trial_started_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_checkout_completed_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_checkout_completed_date = Some(date.to_rfc3339());
        self
    }

    // === Engagement milestone setters ===

    pub fn with_first_network_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_network_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_tag_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_tag_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_api_key_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_api_key_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_snmp_credential_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_snmp_credential_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_invite_sent_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_invite_sent_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_first_invite_accepted_date(mut self, date: DateTime<Utc>) -> Self {
        self.scanopy_first_invite_accepted_date = Some(date.to_rfc3339());
        self
    }

    pub fn with_urgency(mut self, urgency: impl Into<String>) -> Self {
        self.scanopy_urgency = Some(urgency.into());
        self
    }

    // === Inquiry-specific setters ===

    pub fn with_inquiry_plan_type(mut self, plan_type: impl Into<String>) -> Self {
        self.scanopy_inquiry_plan_type = Some(plan_type.into());
        self
    }

    pub fn with_inquiry_urgency(mut self, urgency: impl Into<String>) -> Self {
        self.scanopy_inquiry_urgency = Some(urgency.into());
        self
    }

    pub fn with_inquiry_network_count(mut self, count: i64) -> Self {
        self.scanopy_inquiry_network_count = Some(count);
        self
    }

    pub fn with_inquiry_date(mut self, date: DateTime<Utc>) -> Self {
        // HubSpot datetime properties use Unix timestamp in milliseconds
        self.scanopy_inquiry_date = Some(date.timestamp_millis().to_string());
        self
    }

    pub fn with_non_commercial(mut self, value: bool) -> Self {
        self.scanopy_non_commercial = Some(value);
        self
    }
}

/// HubSpot API response for contact/company creation
#[derive(Debug, Clone, Deserialize)]
pub struct HubSpotObjectResponse {
    pub id: String,
    pub properties: serde_json::Value,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
}

/// HubSpot API error response
#[derive(Debug, Clone, Deserialize)]
pub struct HubSpotError {
    pub status: String,
    pub message: String,
    #[serde(rename = "correlationId")]
    pub correlation_id: Option<String>,
    pub category: Option<String>,
}

/// HubSpot search request
#[derive(Debug, Clone, Serialize)]
pub struct HubSpotSearchRequest {
    #[serde(rename = "filterGroups")]
    pub filter_groups: Vec<HubSpotFilterGroup>,
    pub properties: Vec<String>,
    pub limit: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubSpotFilterGroup {
    pub filters: Vec<HubSpotFilter>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubSpotFilter {
    #[serde(rename = "propertyName")]
    pub property_name: String,
    pub operator: String,
    pub value: String,
}

/// HubSpot search response
#[derive(Debug, Clone, Deserialize)]
pub struct HubSpotSearchResponse {
    pub total: i64,
    pub results: Vec<HubSpotObjectResponse>,
}

/// HubSpot association request
#[derive(Debug, Clone, Serialize)]
pub struct HubSpotAssociationRequest {
    pub inputs: Vec<HubSpotAssociationInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubSpotAssociationInput {
    pub from: HubSpotAssociationObject,
    pub to: HubSpotAssociationObject,
    pub types: Vec<HubSpotAssociationType>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubSpotAssociationObject {
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubSpotAssociationType {
    #[serde(rename = "associationCategory")]
    pub association_category: String,
    #[serde(rename = "associationTypeId")]
    pub association_type_id: i32,
}

/// Enterprise inquiry form data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterpriseInquiryData {
    pub email: String,
    pub name: String,
    pub company: String,
    pub team_size: String,
    pub use_case: String,
    pub urgency: String,
    pub network_count: String,
}
