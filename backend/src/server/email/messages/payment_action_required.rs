use super::{Email, EmailCategory};

/// Asks the user to complete additional authentication (3D Secure) for a
/// payment, linking to the provider-hosted authorization page.
pub struct PaymentActionRequired<'a> {
    pub cta_href: &'a str,
}

impl Email for PaymentActionRequired<'_> {
    fn subject(&self) -> String {
        "Payment Requires Authentication".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        PAYMENT_ACTION_REQUIRED_BODY.replace("{cta_href}", self.cta_href)
    }
}

const PAYMENT_ACTION_REQUIRED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Payment Requires Authentication</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your recent payment for Scanopy requires additional authentication (3D Secure). Please complete the verification to continue your subscription.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{cta_href}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Authorize Payment</a>
                        </td>
                    </tr>
"#;
