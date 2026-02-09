use std::sync::Arc;

use anyhow::{Error, Result};
use async_trait::async_trait;
use email_address::EmailAddress;

use crate::server::{
    email::templates::{
        EMAIL_FOOTER, EMAIL_HEADER, EMAIL_VERIFICATION_BODY, INVITE_LINK_BODY, PASSWORD_RESET_BODY,
        PAYMENT_METHOD_ADDED_BODY, PAYMENT_METHOD_ADDED_TITLE, PLAN_CHANGED_BODY,
        PLAN_CHANGED_TITLE, SUBSCRIPTION_CANCELLED_BODY, SUBSCRIPTION_CANCELLED_TITLE,
        TRIAL_ENDING_BODY, TRIAL_ENDING_TITLE, TRIAL_EXPIRED_BODY, TRIAL_EXPIRED_TITLE,
        TRIAL_STARTED_BODY, TRIAL_STARTED_TITLE,
    },
    users::service::UserService,
};

/// Trait for email provider implementations
#[async_trait]
pub trait EmailProvider: Send + Sync {
    // Example usage function
    fn build_email(&self, body: String) -> String {
        format!("{}{}{}", EMAIL_HEADER, body, EMAIL_FOOTER)
    }

    fn build_invite_title(&self, from_user: EmailAddress) -> String {
        format!("You've been invited to join {} on Scanopy", from_user)
    }

    fn build_password_reset_email(&self, url: String, token: String) -> String {
        self.build_email(PASSWORD_RESET_BODY.replace(
            "{reset_url}",
            &format!(
                "{}/reset-password?token={}",
                url.trim_end_matches('/'),
                token
            ),
        ))
    }

    fn build_invite_email(&self, url: String, from: EmailAddress) -> String {
        self.build_email(
            INVITE_LINK_BODY
                .replace("{invite_url}", &url)
                .replace("{inviter_name}", from.as_str()),
        )
    }

    fn build_verification_email(&self, url: String, token: String) -> String {
        self.build_email(EMAIL_VERIFICATION_BODY.replace(
            "{verify_url}",
            &format!("{}/verify-email?token={}", url.trim_end_matches('/'), token),
        ))
    }

    /// Send an HTML email
    async fn send_password_reset(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<(), Error>;

    /// Send an invite via email
    async fn send_invite(
        &self,
        to: EmailAddress,
        from: EmailAddress,
        url: String,
    ) -> Result<(), Error>;

    /// Send email verification link
    async fn send_verification_email(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<(), Error>;

    /// Send a billing lifecycle email
    async fn send_billing_email(
        &self,
        to: EmailAddress,
        subject: String,
        body: String,
    ) -> Result<(), Error>;

    async fn send_trial_started_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        trial_days: u32,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_trial_started_email(plan_name, trial_days);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_trial_ending_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_trial_ending_email(plan_name);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_trial_expired_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_trial_expired_email(plan_name);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_plan_changed_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
    ) -> Result<(), Error> {
        let (subject, body) = self.build_plan_changed_email(plan_name);
        self.send_billing_email(to, subject, body).await
    }

    async fn send_subscription_cancelled_email(&self, to: EmailAddress) -> Result<(), Error> {
        let (subject, body) = self.build_subscription_cancelled_email();
        self.send_billing_email(to, subject, body).await
    }

    fn build_trial_started_email(&self, plan_name: &str, trial_days: u32) -> (String, String) {
        let body = self.build_email(
            TRIAL_STARTED_BODY
                .replace("{plan_name}", plan_name)
                .replace("{trial_days}", &trial_days.to_string()),
        );
        (TRIAL_STARTED_TITLE.to_string(), body)
    }

    fn build_trial_ending_email(&self, plan_name: &str) -> (String, String) {
        let body = self.build_email(TRIAL_ENDING_BODY.replace("{plan_name}", plan_name));
        (TRIAL_ENDING_TITLE.to_string(), body)
    }

    fn build_trial_expired_email(&self, plan_name: &str) -> (String, String) {
        let body = self.build_email(TRIAL_EXPIRED_BODY.replace("{plan_name}", plan_name));
        (TRIAL_EXPIRED_TITLE.to_string(), body)
    }

    fn build_plan_changed_email(&self, plan_name: &str) -> (String, String) {
        let body = self.build_email(PLAN_CHANGED_BODY.replace("{plan_name}", plan_name));
        (PLAN_CHANGED_TITLE.to_string(), body)
    }

    fn build_subscription_cancelled_email(&self) -> (String, String) {
        let body = self.build_email(SUBSCRIPTION_CANCELLED_BODY.to_string());
        (SUBSCRIPTION_CANCELLED_TITLE.to_string(), body)
    }

    fn build_payment_method_added_email(&self) -> (String, String) {
        let body = self.build_email(PAYMENT_METHOD_ADDED_BODY.to_string());
        (PAYMENT_METHOD_ADDED_TITLE.to_string(), body)
    }
}

/// Email service that wraps the provider
pub struct EmailService {
    provider: Box<dyn EmailProvider>,
    pub user_service: Arc<UserService>,
}

impl EmailService {
    pub fn new(provider: Box<dyn EmailProvider>, user_service: Arc<UserService>) -> Self {
        Self {
            provider,
            user_service,
        }
    }

    /// Send an HTML email
    pub async fn send_password_reset(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<()> {
        self.provider.send_password_reset(to, url, token).await
    }

    pub async fn send_invite(
        &self,
        to: EmailAddress,
        from: EmailAddress,
        url: String,
    ) -> Result<()> {
        self.provider.send_invite(to, from, url).await
    }

    /// Send email verification link
    pub async fn send_verification_email(
        &self,
        to: EmailAddress,
        url: String,
        token: String,
    ) -> Result<()> {
        self.provider.send_verification_email(to, url, token).await
    }

    /// Send billing lifecycle email
    pub async fn send_billing_email(
        &self,
        to: EmailAddress,
        subject: String,
        body: String,
    ) -> Result<()> {
        self.provider.send_billing_email(to, subject, body).await
    }

    pub async fn send_trial_started_email(
        &self,
        to: EmailAddress,
        plan_name: &str,
        trial_days: u32,
    ) -> Result<()> {
        self.provider
            .send_trial_started_email(to, plan_name, trial_days)
            .await
    }

    pub async fn send_trial_ending_email(&self, to: EmailAddress, plan_name: &str) -> Result<()> {
        self.provider.send_trial_ending_email(to, plan_name).await
    }

    pub async fn send_trial_expired_email(&self, to: EmailAddress, plan_name: &str) -> Result<()> {
        self.provider.send_trial_expired_email(to, plan_name).await
    }

    pub async fn send_plan_changed_email(&self, to: EmailAddress, plan_name: &str) -> Result<()> {
        self.provider.send_plan_changed_email(to, plan_name).await
    }

    pub async fn send_subscription_cancelled_email(&self, to: EmailAddress) -> Result<()> {
        self.provider.send_subscription_cancelled_email(to).await
    }
}

/// Strip HTML tags for plain text fallback
pub fn strip_html_tags(html: String) -> String {
    html2text::from_read(html.as_bytes(), 80).unwrap_or_else(|_| html.to_string())
}
