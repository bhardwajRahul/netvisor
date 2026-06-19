use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Confirms to the user that their paused subscription has resumed. Fired by
/// the email subscriber on `BillingOperation::Resumed`.
pub struct SubscriptionResumed;

impl Email for SubscriptionResumed {
    fn subject(&self) -> String {
        "Your Scanopy Subscription is Active Again".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "subscription_resumed"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Welcome back")
                    .paragraph("Hi there,")
                    .paragraph("Your Scanopy subscription is no longer paused. Billing has resumed on your normal cycle and the app is unlocked.")
                    .paragraph("Glad to have you back."),
            )
            .render()
    }
}
