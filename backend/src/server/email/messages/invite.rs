use super::{Email, EmailCategory};

/// Sent to invite a recipient to join an existing Scanopy organization.
pub struct Invite<'a> {
    pub url: &'a str,
    pub inviter: &'a str,
}

impl Email for Invite<'_> {
    fn subject(&self) -> String {
        format!("You've been invited to join {} on Scanopy", self.inviter)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Onboarding
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        INVITE_LINK_BODY
            .replace("{invite_url}", self.url)
            .replace("{inviter_name}", self.inviter)
    }
}

const INVITE_LINK_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">You've Been Invited to Scanopy</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">{inviter_name} has invited you to join their Scanopy instance to visualize and explore their network infrastructure.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Click the button below to accept the invitation and create your account:</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{invite_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Accept Invitation</a>
                        </td>
                    </tr>

                    <!-- Alternative Link -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">If the button doesn't work, copy and paste this link into your browser:</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #2563eb; word-break: break-all;">{invite_url}</p>
                        </td>
                    </tr>

                    <!-- Expiration Notice -->
                    <tr>
                        <td style="padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;">
                            <p style="margin: 20px 0 0 0; font-size: 14px; line-height: 20px; color: #6b7280;">This invitation link will expire in 7 days. If you didn't expect this invitation, you can safely ignore this email.</p>
                        </td>
                    </tr>
"#;
