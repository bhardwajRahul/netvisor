use super::{Email, EmailCategory};

/// Sent when a checkout completes: welcomes the user to their newly active paid
/// plan.
pub struct CheckoutCompleted<'a> {
    pub plan_name: &'a str,
}

impl Email for CheckoutCompleted<'_> {
    fn subject(&self) -> String {
        format!("Welcome to Scanopy {}", self.plan_name)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        "checkout_completed"
    }

    fn body_html(&self) -> String {
        BODY.replace("{plan_name}", self.plan_name)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">You're On Scanopy {plan_name}</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your subscription to <strong>{plan_name}</strong> is active. Thanks for picking Scanopy — we're glad to have you.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Open Scanopy</a>
                        </td>
                    </tr>
"#;
