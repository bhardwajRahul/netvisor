use super::{Email, EmailCategory};

/// Sent when a trial ends without conversion: account moved to Free, with an
/// upgrade CTA to restore higher limits and scheduled discovery.
pub struct TrialExpired<'a> {
    pub plan_name: &'a str,
    pub billing_period: &'a str,
}

impl Email for TrialExpired<'_> {
    fn subject(&self) -> String {
        "Your Trial Has Ended".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        "trial_expired"
    }

    fn body_html(&self) -> String {
        BODY.replace("{plan_name}", self.plan_name)
            .replace("{billing_period}", self.billing_period)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Trial Has Ended</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} {billing_period} trial has ended and your account has been moved to the Free plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">You can still use Scanopy with up to 25 hosts and manual discovery. Upgrade anytime to restore scheduled discovery and higher limits.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=billing-plan&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Upgrade Plan</a>
                        </td>
                    </tr>
"#;
