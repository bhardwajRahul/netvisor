use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Sent when an organization's plan changes: confirms the new plan and that the
/// change is effective immediately.
pub struct PlanChanged<'a> {
    pub plan_name: &'a str,
}

impl Email for PlanChanged<'_> {
    fn subject(&self) -> String {
        "Your Plan Has Changed".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "plan_changed"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Plan Updated")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Your Scanopy plan has been changed to {}. The change takes effect immediately.",
                        self.plan_name
                    )),
            )
            .cta("{base_url}/?{utm}", "Open Scanopy")
            .render()
    }
}
