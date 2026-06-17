use super::{Email, EmailCategory};

/// Confirms to the user that a payment method was added to their account.
pub struct PaymentMethodAdded;

impl Email for PaymentMethodAdded {
    fn subject(&self) -> String {
        "Payment Method Added".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        PAYMENT_METHOD_ADDED_BODY.to_string()
    }
}

const PAYMENT_METHOD_ADDED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Payment Method Added</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">A payment method has been added to your Scanopy account.</p>
                        </td>
                    </tr>
"#;
