use super::{Email, EmailCategory};

/// Confirms that an organization and all of its data have been deleted.
pub struct OrganizationDeleted;

impl Email for OrganizationDeleted {
    fn subject(&self) -> String {
        "Your Scanopy Organization Has Been Deleted".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Account
    }

    fn campaign(&self) -> &'static str {
        "organization_deleted"
    }

    fn body_html(&self) -> String {
        ORGANIZATION_DELETED_BODY.to_string()
    }
}

const ORGANIZATION_DELETED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Organization Deleted</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy organization has been deleted. All of its data, along with every user account in the organization, has been removed.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you'd like to use Scanopy again, you can sign up for a new account at any time.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Create a New Account</a>
                        </td>
                    </tr>
"#;
