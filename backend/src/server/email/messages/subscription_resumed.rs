use super::{Email, EmailCategory};

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

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        SUBSCRIPTION_RESUMED_BODY.to_string()
    }
}

const SUBSCRIPTION_RESUMED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Welcome back</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy subscription is no longer paused. Billing has resumed on your normal cycle and the app is unlocked.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Glad to have you back.</p>
                        </td>
                    </tr>
"#;
