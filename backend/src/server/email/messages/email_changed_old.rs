use super::{Email, EmailCategory};

/// Sent to a user's previous email address as a security notice when the
/// account email is changed.
pub struct EmailChangedOld<'a> {
    pub new_email: &'a str,
}

impl Email for EmailChangedOld<'_> {
    fn subject(&self) -> String {
        "Your Email Was Changed".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Auth
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        BODY.replace("{new_email}", self.new_email)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Email Address Changed</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">The email address on your Scanopy account was changed to <strong>{new_email}</strong>.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you made this change, no action is needed. If you didn't request this change, please contact support immediately.</p>
                        </td>
                    </tr>
"#;
