use super::{Email, EmailCategory};

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

    fn campaign(&self) -> &'static str {
        "cancellation_initiated"
    }

    fn body_html(&self) -> String {
        CANCELLATION_INITIATED_BODY.replace("{period_end}", self.period_end)
    }
}

const CANCELLATION_INITIATED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Cancellation Scheduled</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy subscription is scheduled to cancel on <strong>{period_end}</strong>. You'll keep full access until then; after that you'll move to the Free plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Changed your mind? You can resubscribe or switch plans any time from your billing settings.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=billing-plan&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Manage Subscription</a>
                        </td>
                    </tr>
"#;
