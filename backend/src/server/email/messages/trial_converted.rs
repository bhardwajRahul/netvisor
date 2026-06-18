use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Sent when a trial converts to a paid subscription: confirms the active plan
/// and the going-forward billing amount.
pub struct TrialConverted<'a> {
    pub plan_name: &'a str,
    pub billing_period: &'a str,
    pub base_price: &'a str,
}

impl Email for TrialConverted<'_> {
    fn subject(&self) -> String {
        "Your Subscription is Now Active".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "trial_converted"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Your Subscription is Active!")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your {} {} trial has ended and your subscription is now active. You'll be billed {}* going forward.",
                        self.plan_name, self.billing_period, self.base_price
                    ))
                    .fine_print("*Price excludes applicable taxes. Additional usage beyond included seats, networks, or hosts is billed separately."),
            )
            .cta("{base_url}/?{utm}", "Open Scanopy")
            .render()
    }
}
