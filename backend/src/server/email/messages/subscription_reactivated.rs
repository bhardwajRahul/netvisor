use super::{Email, EmailCategory};

/// Confirms to the user that their pending cancellation was cleared and
/// their subscription is active again. Fired by the email subscriber on
/// `BillingOperation::Reactivated`.
pub struct SubscriptionReactivated;

impl Email for SubscriptionReactivated {
    fn subject(&self) -> String {
        "Your Scanopy subscription is active again".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        SUBSCRIPTION_REACTIVATED_BODY.to_string()
    }
}

const SUBSCRIPTION_REACTIVATED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">You're back on Scanopy</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your pending cancellation has been cleared and your subscription is active again. Nothing else needs to change — billing will continue on your normal cycle.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Glad to have you back.</p>
                        </td>
                    </tr>
"#;
