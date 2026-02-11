use crate::server::email::{
    templates::{EMAIL_VERIFICATION_TITLE, PASSWORD_RESET_TITLE},
    traits::EmailProvider,
};
use anyhow::{Error, anyhow};
use async_trait::async_trait;
use email_address::EmailAddress;
use reqwest::Client;
use serde_json::json;

/// Brevo-based email provider
pub struct BrevoEmailProvider {
    api_key: String,
    client: Client,
}

impl BrevoEmailProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }

    pub async fn send_transactional_email(
        &self,
        to: EmailAddress,
        subject: String,
        body: String,
    ) -> Result<(), Error> {
        let url = "https://api.brevo.com/v3/smtp/email";
        let payload = json!({
            "sender": {
                "name": "Scanopy",
                "email": "no-reply@email.scanopy.net"
            },
            "to": [{ "email": to.to_string() }],
            "subject": subject,
            "htmlContent": body
        });

        let response = self
            .client
            .post(url)
            .header("api-key", &self.api_key)
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!(
                "Failed to send email via Brevo: {}",
                response.text().await?
            ))
        }
    }
}

#[async_trait]
impl EmailProvider for BrevoEmailProvider {
    async fn send_password_reset(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<(), Error> {
        self.send_transactional_email(
            to,
            PASSWORD_RESET_TITLE.to_string(),
            self.build_password_reset_email(url, token),
        )
        .await
    }

    async fn send_billing_email(
        &self,
        to: EmailAddress,
        subject: String,
        body: String,
    ) -> Result<(), Error> {
        self.send_transactional_email(to, subject, body).await
    }

    async fn send_invite(
        &self,
        to: EmailAddress,
        from: EmailAddress,
        url: String,
    ) -> Result<(), Error> {
        self.send_transactional_email(
            to,
            self.build_invite_title(from.clone()),
            self.build_invite_email(url, from),
        )
        .await
    }

    async fn send_verification_email(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<(), Error> {
        self.send_transactional_email(
            to,
            EMAIL_VERIFICATION_TITLE.to_string(),
            self.build_verification_email(url, token),
        )
        .await
    }
}
