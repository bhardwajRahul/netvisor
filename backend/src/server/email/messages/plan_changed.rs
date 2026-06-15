use super::{Email, EmailCategory};

/// Sent when an organization's plan changes: confirms the new plan and that the
/// change is effective immediately.
pub struct PlanChanged<'a> {
    pub plan_name: &'a str,
}

impl Email for PlanChanged<'_> {
    fn subject(&self) -> String {
        "Your Scanopy Plan Has Changed".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        "plan_changed"
    }

    fn body_html(&self) -> String {
        BODY.replace("{plan_name}", self.plan_name)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Plan Updated</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy plan has been changed to {plan_name}. The change takes effect immediately.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Open Scanopy</a>
                        </td>
                    </tr>
"#;
