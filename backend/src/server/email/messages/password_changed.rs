use super::{Email, EmailCategory};

/// Sent as a security notice after a user's account password is changed.
pub struct PasswordChanged<'a> {
    pub timestamp: &'a str,
}

impl Email for PasswordChanged<'_> {
    fn subject(&self) -> String {
        "Your Scanopy Password Was Changed".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Auth
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        BODY.replace("{timestamp}", self.timestamp)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Password Changed</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy password was changed on {timestamp}.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you made this change, no action is needed. If you didn't change your password, please reset it immediately and contact support.</p>
                        </td>
                    </tr>
"#;
