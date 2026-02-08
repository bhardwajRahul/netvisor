use crate::server::brevo::types::{
    CompanyAttributes, CompanyListResponse, CompanyResponse, ContactAttributes,
    CreateCompanyRequest, CreateContactRequest, CreateContactResponse, CreateDealRequest,
    CreateDealResponse, EventIdentifiers, LinkUnlinkRequest, TrackEventRequest,
    UpdateCompanyRequest, UpdateContactRequest,
};
use anyhow::{Result, anyhow};
use backon::{ExponentialBuilder, Retryable};
use governor::{Quota, RateLimiter, clock::DefaultClock, state::InMemoryState};
use reqwest::Client;
use std::{collections::HashMap, num::NonZeroU32, sync::Arc};

const BREVO_API_BASE: &str = "https://api.brevo.com/v3";

/// Brevo CRM API client with rate limiting and retries
pub struct BrevoClient {
    client: Client,
    api_key: String,
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, InMemoryState, DefaultClock>>,
}

impl BrevoClient {
    /// Create a new Brevo client with rate limiting
    pub fn new(api_key: String) -> Self {
        // Conservative rate limit: ~4 req/sec with burst of 2
        // Brevo CRM endpoints have lower limits on some tiers
        let rate_limiter = Arc::new(RateLimiter::direct(
            Quota::per_second(NonZeroU32::new(4).unwrap()).allow_burst(NonZeroU32::new(2).unwrap()),
        ));

        Self {
            client: Client::new(),
            api_key,
            rate_limiter,
        }
    }

    /// Wait for rate limiter before making a request
    async fn wait_for_rate_limit(&self) {
        self.rate_limiter.until_ready().await;
    }

    /// Check if an error is retryable (429 or 5xx)
    fn is_retryable_error(status: reqwest::StatusCode) -> bool {
        status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
    }

    /// Upsert a contact (create or update by email)
    pub async fn upsert_contact(&self, email: &str, attributes: ContactAttributes) -> Result<i64> {
        let url = format!("{}/contacts", BREVO_API_BASE);
        let body = CreateContactRequest {
            email: email.to_string(),
            attributes: attributes.to_attributes(),
            update_enabled: true,
        };

        let operation = || async {
            self.wait_for_rate_limit().await;

            let response = self
                .client
                .post(&url)
                .header("api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow!("Brevo request failed: {}", e))?;

            let status = response.status();

            if status.is_success() {
                let result: CreateContactResponse = response
                    .json()
                    .await
                    .map_err(|e| anyhow!("Failed to parse Brevo response: {}", e))?;
                return Ok(result.id);
            }

            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if Self::is_retryable_error(status) {
                return Err(anyhow!(
                    "Brevo API error (retryable) {}: {}",
                    status,
                    error_body
                ));
            }

            Err(anyhow!("Brevo API error {}: {}", status, error_body))
        };

        operation
            .retry(
                ExponentialBuilder::default()
                    .with_max_times(3)
                    .with_min_delay(std::time::Duration::from_millis(500))
                    .with_max_delay(std::time::Duration::from_secs(10)),
            )
            .when(|e| e.to_string().contains("retryable"))
            .await
    }

    /// Update a contact by email (PUT /contacts/{email}, returns 204)
    pub async fn update_contact(&self, email: &str, attributes: ContactAttributes) -> Result<()> {
        let url = format!("{}/contacts/{}", BREVO_API_BASE, urlencoding::encode(email));
        let body = UpdateContactRequest {
            attributes: attributes.to_attributes(),
        };

        let operation = || async {
            self.wait_for_rate_limit().await;

            let response = self
                .client
                .put(&url)
                .header("api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow!("Brevo request failed: {}", e))?;

            let status = response.status();

            if status.is_success() {
                return Ok(());
            }

            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if Self::is_retryable_error(status) {
                return Err(anyhow!(
                    "Brevo API error (retryable) {}: {}",
                    status,
                    error_body
                ));
            }

            Err(anyhow!("Brevo API error {}: {}", status, error_body))
        };

        operation
            .retry(
                ExponentialBuilder::default()
                    .with_max_times(3)
                    .with_min_delay(std::time::Duration::from_millis(500))
                    .with_max_delay(std::time::Duration::from_secs(10)),
            )
            .when(|e| e.to_string().contains("retryable"))
            .await
    }

    /// Create a company in Brevo
    pub async fn create_company(
        &self,
        name: &str,
        attributes: CompanyAttributes,
        contact_ids: Option<Vec<i64>>,
    ) -> Result<String> {
        let url = format!("{}/companies", BREVO_API_BASE);
        let body = CreateCompanyRequest {
            name: name.to_string(),
            attributes: attributes.to_attributes(),
            linked_contacts_ids: contact_ids,
        };

        let operation = || async {
            self.wait_for_rate_limit().await;

            let response = self
                .client
                .post(&url)
                .header("api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow!("Brevo request failed: {}", e))?;

            let status = response.status();

            if status.is_success() {
                let result: CompanyResponse = response
                    .json()
                    .await
                    .map_err(|e| anyhow!("Failed to parse Brevo response: {}", e))?;
                return Ok(result.id);
            }

            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if Self::is_retryable_error(status) {
                return Err(anyhow!(
                    "Brevo API error (retryable) {}: {}",
                    status,
                    error_body
                ));
            }

            Err(anyhow!("Brevo API error {}: {}", status, error_body))
        };

        operation
            .retry(
                ExponentialBuilder::default()
                    .with_max_times(3)
                    .with_min_delay(std::time::Duration::from_millis(500))
                    .with_max_delay(std::time::Duration::from_secs(10)),
            )
            .when(|e| e.to_string().contains("retryable"))
            .await
    }

    /// Update a company in Brevo (PATCH /companies/{id})
    pub async fn update_company(
        &self,
        company_id: &str,
        attributes: CompanyAttributes,
    ) -> Result<()> {
        let url = format!("{}/companies/{}", BREVO_API_BASE, company_id);
        let body = UpdateCompanyRequest {
            name: attributes.name.clone(),
            attributes: Some(attributes.to_attributes()),
        };

        let operation = || async {
            self.wait_for_rate_limit().await;

            let response = self
                .client
                .patch(&url)
                .header("api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow!("Brevo request failed: {}", e))?;

            let status = response.status();

            if status.is_success() {
                return Ok(());
            }

            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if Self::is_retryable_error(status) {
                return Err(anyhow!(
                    "Brevo API error (retryable) {}: {}",
                    status,
                    error_body
                ));
            }

            Err(anyhow!("Brevo API error {}: {}", status, error_body))
        };

        operation
            .retry(
                ExponentialBuilder::default()
                    .with_max_times(3)
                    .with_min_delay(std::time::Duration::from_millis(500))
                    .with_max_delay(std::time::Duration::from_secs(10)),
            )
            .when(|e| e.to_string().contains("retryable"))
            .await
    }

    /// Find a company by scanopy_org_id attribute
    pub async fn find_company_by_org_id(&self, org_id: &str) -> Result<Option<String>> {
        let url = format!(
            "{}/companies?filters={}",
            BREVO_API_BASE,
            urlencoding::encode(&format!("{{\"attributes.scanopy_org_id\":\"{}\"}}", org_id))
        );

        let operation = || async {
            self.wait_for_rate_limit().await;

            let response = self
                .client
                .get(&url)
                .header("api-key", &self.api_key)
                .send()
                .await
                .map_err(|e| anyhow!("Brevo search failed: {}", e))?;

            let status = response.status();

            if status.is_success() {
                let result: CompanyListResponse = response
                    .json()
                    .await
                    .map_err(|e| anyhow!("Failed to parse Brevo search response: {}", e))?;
                return Ok(result
                    .items
                    .and_then(|items| items.into_iter().next().map(|c| c.id)));
            }

            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if Self::is_retryable_error(status) {
                return Err(anyhow!(
                    "Brevo search error (retryable) {}: {}",
                    status,
                    error_body
                ));
            }

            Err(anyhow!("Brevo search error {}: {}", status, error_body))
        };

        operation
            .retry(
                ExponentialBuilder::default()
                    .with_max_times(3)
                    .with_min_delay(std::time::Duration::from_millis(500))
                    .with_max_delay(std::time::Duration::from_secs(10)),
            )
            .when(|e| e.to_string().contains("retryable"))
            .await
    }

    /// Link a contact to a company
    pub async fn link_contact_to_company(&self, company_id: &str, contact_id: i64) -> Result<()> {
        let url = format!("{}/companies/link-unlink/{}", BREVO_API_BASE, company_id);
        let body = LinkUnlinkRequest {
            link_contacts_ids: Some(vec![contact_id]),
            unlink_contacts_ids: None,
        };

        let operation = || async {
            self.wait_for_rate_limit().await;

            let response = self
                .client
                .patch(&url)
                .header("api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow!("Brevo link failed: {}", e))?;

            let status = response.status();

            if status.is_success() {
                return Ok(());
            }

            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if Self::is_retryable_error(status) {
                return Err(anyhow!(
                    "Brevo link error (retryable) {}: {}",
                    status,
                    error_body
                ));
            }

            Err(anyhow!("Brevo link error {}: {}", status, error_body))
        };

        operation
            .retry(
                ExponentialBuilder::default()
                    .with_max_times(3)
                    .with_min_delay(std::time::Duration::from_millis(500))
                    .with_max_delay(std::time::Duration::from_secs(10)),
            )
            .when(|e| e.to_string().contains("retryable"))
            .await
    }

    /// Create a deal in Brevo CRM
    pub async fn create_deal(
        &self,
        name: &str,
        attributes: Option<HashMap<String, serde_json::Value>>,
        contact_ids: Option<Vec<i64>>,
        company_ids: Option<Vec<String>>,
    ) -> Result<String> {
        let url = format!("{}/crm/deals", BREVO_API_BASE);
        let body = CreateDealRequest {
            name: name.to_string(),
            attributes,
            linked_contacts_ids: contact_ids,
            linked_companies_ids: company_ids,
        };

        let operation = || async {
            self.wait_for_rate_limit().await;

            let response = self
                .client
                .post(&url)
                .header("api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow!("Brevo deal creation failed: {}", e))?;

            let status = response.status();

            if status.is_success() {
                let result: CreateDealResponse = response
                    .json()
                    .await
                    .map_err(|e| anyhow!("Failed to parse Brevo deal response: {}", e))?;
                return Ok(result.id);
            }

            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if Self::is_retryable_error(status) {
                return Err(anyhow!(
                    "Brevo deal error (retryable) {}: {}",
                    status,
                    error_body
                ));
            }

            Err(anyhow!("Brevo deal error {}: {}", status, error_body))
        };

        operation
            .retry(
                ExponentialBuilder::default()
                    .with_max_times(3)
                    .with_min_delay(std::time::Duration::from_millis(500))
                    .with_max_delay(std::time::Duration::from_secs(10)),
            )
            .when(|e| e.to_string().contains("retryable"))
            .await
    }

    /// Track an event in Brevo (for automation triggers)
    pub async fn track_event(
        &self,
        event_name: &str,
        email: &str,
        event_properties: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<()> {
        let url = format!("{}/events", BREVO_API_BASE);
        let body = TrackEventRequest {
            event_name: event_name.to_string(),
            identifiers: EventIdentifiers {
                email_id: Some(email.to_string()),
            },
            event_properties,
            contact_properties: None,
        };

        let operation = || async {
            self.wait_for_rate_limit().await;

            let response = self
                .client
                .post(&url)
                .header("api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| anyhow!("Brevo event tracking failed: {}", e))?;

            let status = response.status();

            if status.is_success() {
                return Ok(());
            }

            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if Self::is_retryable_error(status) {
                return Err(anyhow!(
                    "Brevo event error (retryable) {}: {}",
                    status,
                    error_body
                ));
            }

            Err(anyhow!("Brevo event error {}: {}", status, error_body))
        };

        operation
            .retry(
                ExponentialBuilder::default()
                    .with_max_times(3)
                    .with_min_delay(std::time::Duration::from_millis(500))
                    .with_max_delay(std::time::Duration::from_secs(10)),
            )
            .when(|e| e.to_string().contains("retryable"))
            .await
    }

    /// Create or update contact, then create or update company, linking them.
    /// Returns the Brevo company ID to be stored on the organization record.
    pub async fn sync_contact_and_company(
        &self,
        email: &str,
        contact_attributes: ContactAttributes,
        company_name: &str,
        company_attributes: CompanyAttributes,
    ) -> Result<(i64, String)> {
        // Upsert contact (Brevo handles create-or-update with updateEnabled: true)
        let contact_id = self.upsert_contact(email, contact_attributes).await?;

        // Check if company already exists for this org (for backfill scenarios)
        let org_id = company_attributes.scanopy_org_id.clone();
        let existing_company_id = if let Some(org_id) = &org_id {
            self.find_company_by_org_id(org_id).await?
        } else {
            None
        };

        let company_id = match existing_company_id {
            Some(id) => {
                tracing::debug!(
                    company_id = %id,
                    "Updating existing Brevo company"
                );
                self.update_company(&id, company_attributes).await?;
                self.link_contact_to_company(&id, contact_id).await?;
                id
            }
            None => {
                tracing::debug!("Creating new Brevo company");
                self.create_company(company_name, company_attributes, Some(vec![contact_id]))
                    .await?
            }
        };

        tracing::debug!(
            contact_id = %contact_id,
            company_id = %company_id,
            "Successfully synced contact and company to Brevo"
        );

        Ok((contact_id, company_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_error() {
        use reqwest::StatusCode;

        assert!(BrevoClient::is_retryable_error(
            StatusCode::TOO_MANY_REQUESTS
        ));
        assert!(BrevoClient::is_retryable_error(
            StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(!BrevoClient::is_retryable_error(StatusCode::BAD_REQUEST));
        assert!(!BrevoClient::is_retryable_error(StatusCode::OK));
    }
}
