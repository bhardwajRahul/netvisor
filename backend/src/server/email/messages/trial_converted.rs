use super::{Email, EmailCategory};

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

    fn campaign(&self) -> &'static str {
        "trial_converted"
    }

    fn body_html(&self) -> String {
        BODY.replace("{plan_name}", self.plan_name)
            .replace("{billing_period}", self.billing_period)
            .replace("{base_price}", self.base_price)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Subscription is Active!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} {billing_period} trial has ended and your subscription is now active. You'll be billed {base_price}* going forward.</p>
                            <p style="margin: 0 0 20px 0; font-size: 12px; line-height: 18px; color: #9ca3af;">*Price excludes applicable taxes. Additional usage beyond included seats, networks, or hosts is billed separately.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Open Scanopy</a>
                        </td>
                    </tr>
"#;
