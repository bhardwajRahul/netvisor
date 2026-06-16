use super::{Email, EmailCategory};

/// Sent as a security notice when an OIDC login provider is unlinked from a
/// user's account.
pub struct OidcUnlinked<'a> {
    pub provider_name: &'a str,
}

impl Email for OidcUnlinked<'_> {
    fn subject(&self) -> String {
        format!("{} Login Disconnected", self.provider_name)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Auth
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        BODY.replace("{provider_name}", self.provider_name)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Login Method Disconnected</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {provider_name} account has been unlinked from your Scanopy account. You can no longer sign in using {provider_name}.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you didn't make this change, please sign in to your account and review your security settings.</p>
                        </td>
                    </tr>
"#;
