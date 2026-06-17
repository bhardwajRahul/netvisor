use super::{Email, EmailCategory};

/// Sent when a subscription is cancelled and access has ended: account moved to
/// Free, with a resubscribe CTA.
pub struct SubscriptionCancelled<'a> {
    pub period_end_date: &'a str,
}

impl Email for SubscriptionCancelled<'_> {
    fn subject(&self) -> String {
        "Your Subscription Has Been Cancelled".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        "subscription_cancelled"
    }

    fn body_html(&self) -> String {
        BODY.replace("{period_end_date}", self.period_end_date)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Subscription Cancelled</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy subscription was cancelled and your access ended on {period_end_date}. Your account has been moved to the Free plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">You can continue using Scanopy with up to 25 hosts and manual discovery. Resubscribe anytime from your Settings page.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=billing-plan&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Resubscribe</a>
                        </td>
                    </tr>
"#;
