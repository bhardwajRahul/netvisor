use super::{Email, EmailCategory};

/// Monthly billing summary recapping an invoice's line items and total.
pub struct UsageSummary<'a> {
    pub period: &'a str,
    pub invoice_date: &'a str,
    pub line_items_html: &'a str,
    pub total: &'a str,
}

impl Email for UsageSummary<'_> {
    fn subject(&self) -> String {
        format!("Your Scanopy Invoice — {}", self.period)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn campaign(&self) -> &'static str {
        "usage_summary"
    }

    fn body_html(&self) -> String {
        BODY.replace("{period}", self.period)
            .replace("{invoice_date}", self.invoice_date)
            .replace("{line_items_html}", self.line_items_html)
            .replace("{total}", self.total)
    }
}

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Monthly Billing Summary</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Here's a summary of your Scanopy billing for {period}.</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #6b7280;">Invoice date: {invoice_date}</p>

                            <!-- Line Items Table -->
                            <table role="presentation" style="width: 100%; border-collapse: collapse; margin: 0 0 20px 0;">
                                <tr>
                                    <td style="padding: 8px 0; border-bottom: 2px solid #1a1a1a; font-size: 14px; font-weight: 600; color: #1a1a1a;">Description</td>
                                    <td style="padding: 8px 0; border-bottom: 2px solid #1a1a1a; font-size: 14px; font-weight: 600; color: #1a1a1a; text-align: right;">Amount</td>
                                </tr>
                                {line_items_html}
                                <tr>
                                    <td style="padding: 12px 0 0 0; font-size: 16px; font-weight: 600; color: #1a1a1a;">Total</td>
                                    <td style="padding: 12px 0 0 0; font-size: 16px; font-weight: 600; color: #1a1a1a; text-align: right;">{total}</td>
                                </tr>
                            </table>
                            <p style="margin: 0; font-size: 14px; line-height: 20px; color: #6b7280;">Questions? Please reach out to <a href="mailto:billing@scanopy.net" style="color: #2563eb; text-decoration: none;">billing@scanopy.net</a></p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{base_url}/?modal=settings&tab=billing&{utm}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">View Billing</a>
                        </td>
                    </tr>
"#;
