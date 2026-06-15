use super::{Email, EmailCategory};

/// Sent when a payment fails: prompts the user to update their payment method to
/// avoid service interruption.
pub struct PaymentFailed;

impl Email for PaymentFailed {
    fn subject(&self) -> String {
        "Payment Failed - Action Required".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        "payment_failed"
    }

    fn body_html(&self) -> String {
        BODY.to_string()
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Payment Failed</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your recent payment for Scanopy failed. Please update your payment method to avoid service interruption.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If you believe this is an error, check with your bank or try a different payment method.</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Update Payment Method</a>
                        </td>
                    </tr>
"#;
