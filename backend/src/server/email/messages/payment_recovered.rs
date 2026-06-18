use super::{Email, EmailCategory};

/// Tells the user a previously failed payment has gone through and their
/// subscription is active again.
pub struct PaymentRecovered<'a> {
    pub amount: &'a str,
}

impl Email for PaymentRecovered<'_> {
    fn subject(&self) -> String {
        "Your Payment is Back On Track".to_string()
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        ""
    }

    fn body_html(&self) -> String {
        PAYMENT_RECOVERED_BODY.replace("{amount}", self.amount)
    }
}

const PAYMENT_RECOVERED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Payment Recovered</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your previously failed payment of {amount} has gone through. Your subscription is active again — no action needed on your end.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Thanks for being a Scanopy customer.</p>
                        </td>
                    </tr>
"#;
