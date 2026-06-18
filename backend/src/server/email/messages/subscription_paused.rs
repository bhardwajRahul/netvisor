use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Confirms to the user that their subscription has been paused. Fired by the
/// email subscriber on `BillingOperation::Paused`.
///
/// Copy bifurcates on plan rate because "we won't charge you again until X"
/// only carries meaning when a charge was imminent (monthly). For yearly
/// plans the next charge is already months away, so we frame the pause as
/// the renewal date being pushed back day-for-day instead.
pub struct SubscriptionPaused<'a> {
    pub resumes_at: &'a str,
    pub is_yearly: bool,
    pub duration_days: u32,
}

impl Email for SubscriptionPaused<'_> {
    fn subject(&self) -> String {
        "Your Scanopy Subscription is Paused".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "subscription_paused"
    }

    fn body_html(&self) -> String {
        let lead = if self.is_yearly {
            format!(
                "Billing on your Scanopy subscription is paused. Your annual renewal date is being pushed back day-for-day while you're paused — billing auto-resumes on {} (adding {} days to your current term), or sooner if you click <strong>Resume now</strong> in your billing settings.",
                self.resumes_at, self.duration_days
            )
        } else {
            format!(
                "Billing on your Scanopy subscription is paused. Your next monthly charge is held until {}, or until you click <strong>Resume now</strong> in your billing settings — whichever comes first.",
                self.resumes_at
            )
        };
        Body::new()
            .content(
                Content::new()
                    .heading("Your subscription is paused")
                    .paragraph("Hi there,")
                    .paragraph(&lead)
                    .paragraph("While paused, your network data stays put but the app is locked behind a billing prompt. Resume any time to pick back up."),
            )
            .render()
    }
}
