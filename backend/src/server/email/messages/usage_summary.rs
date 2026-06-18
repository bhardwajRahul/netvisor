use super::{Body, Content, Email, EmailCategory, EmailPreference};

/// Monthly billing summary recapping an invoice's line items and total.
pub struct UsageSummary<'a> {
    pub period: &'a str,
    pub invoice_date: &'a str,
    pub line_items_html: &'a str,
    pub total: &'a str,
}

impl Email for UsageSummary<'_> {
    fn subject(&self) -> String {
        format!("Your {} Invoice ", self.period)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Billing
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Required
    }

    fn campaign(&self) -> &'static str {
        "usage_summary"
    }

    fn body_html(&self) -> String {
        let invoice_table = format!(
            r#"                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #6b7280;">Invoice date: {invoice_date}</p>

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
"#,
            invoice_date = self.invoice_date,
            line_items_html = self.line_items_html,
            total = self.total,
        );
        Body::new()
            .content(
                Content::new()
                    .heading("Monthly Billing Summary")
                    .paragraph("Hi there,")
                    .paragraph(&format!(
                        "Here's a summary of your Scanopy billing for {}.",
                        self.period
                    ))
                    .raw(&invoice_table),
            )
            .cta(
                "{base_url}/?modal=settings&tab=billing&{utm}",
                "View Billing",
            )
            .render()
    }
}
