use super::{Email, EmailCategory};

/// Sent when an organization hits a plan limit (hosts/networks/seats).
pub struct PlanLimitReached<'a> {
    pub first_name: Option<&'a str>,
    pub limit_type: &'a str,
    pub current_count: u64,
    pub limit: u64,
    pub plan_name: &'a str,
    pub has_overage: bool,
}

impl Email for PlanLimitReached<'_> {
    fn subject(&self) -> String {
        format!("You've Reached Your {} Limit", self.limit_type)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        "plan_limit_reached"
    }

    fn body_html(&self) -> String {
        let (limit_message, cta_modal, cta_label) = if self.has_overage {
            (
                format!(
                    "Additional {} beyond your included amount are being billed automatically.",
                    self.limit_type
                ),
                "settings&tab=billing",
                "View Billing",
            )
        } else {
            (
                format!(
                    "You won't be able to add new {} until you upgrade.",
                    self.limit_type
                ),
                "billing-plan",
                "Upgrade Plan",
            )
        };
        BODY.replace("{first_name}", self.first_name.unwrap_or("there"))
            .replace("{limit_type}", self.limit_type)
            .replace("{current_count}", &self.current_count.to_string())
            .replace("{limit}", &self.limit.to_string())
            .replace("{plan_name}", self.plan_name)
            .replace("{limit_message}", &limit_message)
            .replace("{cta_modal}", cta_modal)
            .replace("{cta_label}", cta_label)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Plan Limit Reached</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi {first_name},</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">You've reached <strong>{current_count}</strong> of your <strong>{limit}</strong> included {limit_type} on the {plan_name} plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">{limit_message}</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal={cta_modal}&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">{cta_label}</a>
                        </td>
                    </tr>
"#;
