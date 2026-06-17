use super::{Email, EmailCategory};

/// Sent when a user requests a password reset, providing a time-limited link
/// to choose a new password.
pub struct PasswordReset<'a> {
    pub url: &'a str,
    pub token: &'a str,
}

impl Email for PasswordReset<'_> {
    fn subject(&self) -> String {
        "Password Reset".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Auth
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        BODY.replace(
            "{reset_url}",
            &format!(
                "{}/reset-password?token={}",
                self.url.trim_end_matches('/'),
                self.token
            ),
        )
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Reset Your Password</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">We received a request to reset your password for your Scanopy account. Click the button below to create a new password:</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{reset_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Reset Password</a>
                        </td>
                    </tr>

                    <!-- Alternative Link -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">If the button doesn't work, copy and paste this link into your browser:</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #2563eb; word-break: break-all;">{reset_url}</p>
                        </td>
                    </tr>

                    <!-- Security Notice -->
                    <tr>
                        <td style="padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;">
                            <p style="margin: 20px 0 0 0; font-size: 14px; line-height: 20px; color: #6b7280;">This password reset link will expire in 24 hours. If you didn't request a password reset, you can safely ignore this email.</p>
                        </td>
                    </tr>
"#;
