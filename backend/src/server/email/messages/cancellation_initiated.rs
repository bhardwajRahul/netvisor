use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Confirms a subscription is scheduled to cancel at the end of the current
/// billing period.
pub struct CancellationInitiated<'a> {
    pub period_end: &'a str,
}

impl Email for CancellationInitiated<'_> {
    fn subject(&self) -> String {
        format!("Your Subscription Will End on {}", self.period_end)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "cancellation_initiated"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Cancellation Scheduled")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your Scanopy subscription is scheduled to cancel on <strong>{}</strong>. You'll keep full access until then; after that you'll move to the Free plan.",
                        self.period_end
                    ))
                    .paragraph("Changed your mind? You can resubscribe or switch plans any time from your billing settings."),
            )
            .cta("{base_url}/?modal=billing-plan&{utm}", "Manage Subscription")
            .render()
    }
}
