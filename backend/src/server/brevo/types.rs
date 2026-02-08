use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Brevo contact attributes for Scanopy users.
/// Brevo uses UPPERCASE attribute names in an `attributes` map.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContactAttributes {
    pub email: Option<String>,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub jobtitle: Option<String>,
    pub scanopy_user_id: Option<String>,
    pub scanopy_org_id: Option<String>,
    pub scanopy_role: Option<String>,
    pub scanopy_signup_source: Option<String>,
    pub scanopy_use_case: Option<String>,
    pub scanopy_signup_date: Option<String>,
    pub scanopy_last_login_date: Option<String>,
    pub scanopy_marketing_opt_in: Option<bool>,

    /// How user heard about Scanopy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_referral_source: Option<String>,
}

impl ContactAttributes {
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

    pub fn with_referral_source(mut self, source: impl Into<String>) -> Self {
        self.scanopy_referral_source = Some(source.into());
        self
    }

    /// Convert to Brevo API attributes map (UPPERCASE keys)
    pub fn to_attributes(&self) -> HashMap<String, serde_json::Value> {
        let mut attrs = HashMap::new();

        if let Some(v) = &self.firstname {
            attrs.insert("FIRSTNAME".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.lastname {
            attrs.insert("LASTNAME".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.jobtitle {
            attrs.insert("JOBTITLE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_user_id {
            attrs.insert("SCANOPY_USER_ID".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_org_id {
            attrs.insert("SCANOPY_ORG_ID".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_role {
            attrs.insert("SCANOPY_ROLE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_signup_source {
            attrs.insert("SCANOPY_SIGNUP_SOURCE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_use_case {
            attrs.insert("SCANOPY_USE_CASE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_signup_date {
            attrs.insert("SCANOPY_SIGNUP_DATE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_last_login_date {
            attrs.insert("SCANOPY_LAST_LOGIN_DATE".to_string(), serde_json::json!(v));
        }
        if let Some(v) = &self.scanopy_marketing_opt_in {
            attrs.insert("SCANOPY_MARKETING_OPT_IN".to_string(), serde_json::json!(v));
        }

        attrs
    }
}

/// Brevo company attributes for Scanopy organizations.
/// Attributes are free-form key/value in an `attributes` map.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompanyAttributes {
    pub name: Option<String>,
    pub scanopy_org_id: Option<String>,
    pub scanopy_org_type: Option<String>,
    pub scanopy_company_size: Option<String>,
    pub scanopy_plan_type: Option<String>,
    pub scanopy_plan_status: Option<String>,
    pub scanopy_mrr: Option<i64>,
    pub scanopy_network_count: Option<i64>,
    pub scanopy_host_count: Option<i64>,
    pub scanopy_user_count: Option<i64>,
    pub scanopy_network_limit: Option<i64>,
    pub scanopy_seat_limit: Option<i64>,
    pub scanopy_created_date: Option<String>,
    pub scanopy_last_discovery_date: Option<String>,
    pub scanopy_discovery_count: Option<i64>,
    pub scanopy_first_daemon_date: Option<String>,
    pub scanopy_first_discovery_date: Option<String>,
    pub scanopy_trial_started_date: Option<String>,
    pub scanopy_checkout_completed_date: Option<String>,
    pub scanopy_first_network_date: Option<String>,
    pub scanopy_first_tag_date: Option<String>,
    pub scanopy_first_api_key_date: Option<String>,
    pub scanopy_first_snmp_credential_date: Option<String>,
    pub scanopy_first_invite_sent_date: Option<String>,
    pub scanopy_first_invite_accepted_date: Option<String>,
    pub scanopy_urgency: Option<String>,
    pub scanopy_inquiry_plan_type: Option<String>,
    pub scanopy_inquiry_urgency: Option<String>,
    pub scanopy_inquiry_network_count: Option<i64>,
    pub scanopy_inquiry_date: Option<String>,

    /// How user heard about Scanopy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanopy_referral_source: Option<String>,
}

impl CompanyAttributes {
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
        self.scanopy_inquiry_date = Some(date.to_rfc3339());
        self
    }

    /// Convert to Brevo API attributes map
    pub fn to_attributes(&self) -> HashMap<String, serde_json::Value> {
        let mut attrs = HashMap::new();

        macro_rules! insert_opt {
            ($field:ident, $key:expr) => {
                if let Some(v) = &self.$field {
                    attrs.insert($key.to_string(), serde_json::json!(v));
                }
            };
        }

        macro_rules! insert_opt_num {
            ($field:ident, $key:expr) => {
                if let Some(v) = self.$field {
                    attrs.insert($key.to_string(), serde_json::json!(v));
                }
            };
        }

        insert_opt!(scanopy_org_id, "scanopy_org_id");
        insert_opt!(scanopy_org_type, "scanopy_org_type");
        insert_opt!(scanopy_company_size, "scanopy_company_size");
        insert_opt!(scanopy_plan_type, "scanopy_plan_type");
        insert_opt!(scanopy_plan_status, "scanopy_plan_status");
        insert_opt_num!(scanopy_mrr, "scanopy_mrr");
        insert_opt_num!(scanopy_network_count, "scanopy_network_count");
        insert_opt_num!(scanopy_host_count, "scanopy_host_count");
        insert_opt_num!(scanopy_user_count, "scanopy_user_count");
        insert_opt_num!(scanopy_network_limit, "scanopy_network_limit");
        insert_opt_num!(scanopy_seat_limit, "scanopy_seat_limit");
        insert_opt!(scanopy_created_date, "scanopy_created_date");
        insert_opt!(scanopy_last_discovery_date, "scanopy_last_discovery_date");
        insert_opt_num!(scanopy_discovery_count, "scanopy_discovery_count");
        insert_opt!(scanopy_first_daemon_date, "scanopy_first_daemon_date");
        insert_opt!(scanopy_first_discovery_date, "scanopy_first_discovery_date");
        insert_opt!(scanopy_trial_started_date, "scanopy_trial_started_date");
        insert_opt!(
            scanopy_checkout_completed_date,
            "scanopy_checkout_completed_date"
        );
        insert_opt!(scanopy_first_network_date, "scanopy_first_network_date");
        insert_opt!(scanopy_first_tag_date, "scanopy_first_tag_date");
        insert_opt!(scanopy_first_api_key_date, "scanopy_first_api_key_date");
        insert_opt!(
            scanopy_first_snmp_credential_date,
            "scanopy_first_snmp_credential_date"
        );
        insert_opt!(
            scanopy_first_invite_sent_date,
            "scanopy_first_invite_sent_date"
        );
        insert_opt!(
            scanopy_first_invite_accepted_date,
            "scanopy_first_invite_accepted_date"
        );
        insert_opt!(scanopy_urgency, "scanopy_urgency");
        insert_opt!(scanopy_inquiry_plan_type, "scanopy_inquiry_plan_type");
        insert_opt!(scanopy_inquiry_urgency, "scanopy_inquiry_urgency");
        insert_opt_num!(
            scanopy_inquiry_network_count,
            "scanopy_inquiry_network_count"
        );
        insert_opt!(scanopy_inquiry_date, "scanopy_inquiry_date");

        attrs
    }
    
    pub fn with_referral_source(mut self, source: impl Into<String>) -> Self {
        self.scanopy_referral_source = Some(source.into());
        self
    }
}

// ============================================================================
// Brevo API request/response types
// ============================================================================

/// POST /contacts - create or update contact
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContactRequest {
    pub email: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub update_enabled: bool,
}

/// PUT /contacts/{email} - update contact attributes
#[derive(Debug, Clone, Serialize)]
pub struct UpdateContactRequest {
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Response from POST /contacts
#[derive(Debug, Clone, Deserialize)]
pub struct CreateContactResponse {
    pub id: i64,
}

/// POST /companies - create company
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompanyRequest {
    pub name: String,
    pub attributes: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_contacts_ids: Option<Vec<i64>>,
}

/// PATCH /companies/{id} - update company
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCompanyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

/// Response from POST /companies
#[derive(Debug, Clone, Deserialize)]
pub struct CompanyResponse {
    pub id: String,
}

/// Response from GET /companies (list/filter)
#[derive(Debug, Clone, Deserialize)]
pub struct CompanyListResponse {
    pub items: Option<Vec<CompanyItem>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompanyItem {
    pub id: String,
}

/// PATCH /companies/link-unlink/{id}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkUnlinkRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_contacts_ids: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlink_contacts_ids: Option<Vec<i64>>,
}

/// POST /crm/deals - create deal
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDealRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_contacts_ids: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_companies_ids: Option<Vec<String>>,
}

/// Response from POST /crm/deals
#[derive(Debug, Clone, Deserialize)]
pub struct CreateDealResponse {
    pub id: String,
}

/// POST /events - track event
#[derive(Debug, Clone, Serialize)]
pub struct TrackEventRequest {
    pub event_name: String,
    pub identifiers: EventIdentifiers,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_properties: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_properties: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventIdentifiers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contact_attributes_builder() {
        let attrs = ContactAttributes::new()
            .with_email("test@example.com")
            .with_user_id(uuid::Uuid::nil())
            .with_role("owner")
            .with_signup_source("organic");

        assert_eq!(attrs.email, Some("test@example.com".to_string()));
        assert_eq!(attrs.scanopy_user_id, Some(uuid::Uuid::nil().to_string()));
        assert_eq!(attrs.scanopy_role, Some("owner".to_string()));
        assert_eq!(attrs.scanopy_signup_source, Some("organic".to_string()));

        let map = attrs.to_attributes();
        assert_eq!(map.get("SCANOPY_ROLE"), Some(&serde_json::json!("owner")));
    }

    #[test]
    fn test_company_attributes_builder() {
        let attrs = CompanyAttributes::new()
            .with_name("Acme Inc")
            .with_org_id(uuid::Uuid::nil())
            .with_org_type("company")
            .with_plan_type("pro");

        assert_eq!(attrs.name, Some("Acme Inc".to_string()));
        assert_eq!(attrs.scanopy_org_id, Some(uuid::Uuid::nil().to_string()));
        assert_eq!(attrs.scanopy_org_type, Some("company".to_string()));
        assert_eq!(attrs.scanopy_plan_type, Some("pro".to_string()));

        let map = attrs.to_attributes();
        assert_eq!(
            map.get("scanopy_plan_type"),
            Some(&serde_json::json!("pro"))
        );
    }
}
