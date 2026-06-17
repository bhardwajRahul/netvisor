use super::{Email, EmailCategory};

/// Confirms to the user that their subscription has been paused. Fired by the
/// email subscriber on `BillingOperation::Paused`.
pub struct SubscriptionPaused<'a> {
    pub resumes_at: &'a str,
}

impl<'a> Email for SubscriptionPaused<'a> {
    fn subject(&self) -> String {
        "Your Scanopy Subscription is Paused".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        SUBSCRIPTION_PAUSED_BODY.replace("{resumes_at}", self.resumes_at)
    }
}

const SUBSCRIPTION_PAUSED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your subscription is paused</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Billing on your Scanopy subscription is paused. We won't charge you again until {resumes_at}, or until you click <strong>Resume now</strong> in your billing settings — whichever comes first.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">While paused, your network data stays put but the app is locked behind a billing prompt. Resume any time to pick back up.</p>
                        </td>
                    </tr>
"#;
