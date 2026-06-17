use lettre::{
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
    message::{Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
};

use anyhow::{Error, anyhow};
use async_trait::async_trait;
use email_address::EmailAddress;

use super::{messages::Email, transport::EmailTransport};

/// SMTP-based email transport (lettre), used as the fallback when Brevo is
/// not configured.
pub struct SmtpEmailProvider {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpEmailProvider {
    pub fn new(
        smtp_username: String,
        smtp_password: String,
        smtp_email: String,
        smtp_relay: String,
    ) -> Result<Self, Error> {
        let creds = Credentials::new(smtp_username, smtp_password);

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_relay)
            .map_err(|e| anyhow!("Failed to create SMTP transport: {}", e))?
            .credentials(creds)
            .build();

        let from = Mailbox::new(
            Some("Scanopy".to_string()),
            smtp_email
                .parse()
                .map_err(|e| anyhow!("Invalid from email address: {}", e))?,
        );

        Ok(Self { mailer, from })
    }
}

#[async_trait]
impl EmailTransport for SmtpEmailProvider {
    async fn send(&self, to: EmailAddress, email: &dyn Email, base_url: &str) -> Result<(), Error> {
        let to_mbox = Mailbox::new(
            None,
            to.email()
                .parse()
                .map_err(|e| anyhow!("Invalid recipient email address: {}", e))?,
        );

        let html = email.render_html(base_url);
        let text = email.render_text(base_url);

        let message = lettre::Message::builder()
            .from(self.from.clone())
            .to(to_mbox)
            .subject(email.subject())
            .multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(text))
                    .singlepart(SinglePart::html(html)),
            )?;

        self.mailer
            .send(message)
            .await
            .map_err(|e| anyhow!("Failed to send email: {}", e))?;

        Ok(())
    }
}
