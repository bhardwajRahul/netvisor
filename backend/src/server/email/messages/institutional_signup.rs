use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Internal notice to the server admin (`server_admin_contact_email`) when a
/// new signup's email domain classifies as institutional. Carries the domain
/// and institution type only — never the full email address.
pub struct InstitutionalSignup<'a> {
    pub domain: &'a str,
    pub institution_type: &'a str,
}

impl Email for InstitutionalSignup<'_> {
    fn subject(&self) -> String {
        format!(
            "Institutional signup: {} ({})",
            self.domain, self.institution_type
        )
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Onboarding
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "institutional_signup"
    }

    fn body_html(&self) -> String {
        Body::new()
            .content(
                Content::new()
                    .heading("Institutional Signup")
                    .paragraph(&format!(
                        "A new user just signed up from the domain {} — classified as {}.",
                        self.domain, self.institution_type
                    ))
                    .paragraph("The full contact details are in Brevo for a personal follow-up."),
            )
            .render()
    }
}
