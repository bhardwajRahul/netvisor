//! Domain invoice snapshot types and Stripe conversions.
use super::*;

// ===========================================================================
// Domain invoice snapshot — typed projection of `stripe_billing::Invoice` for
// event payloads. Carries exactly the fields the usage-summary email needs to
// render the line-item breakdown without reaching back into Stripe.
// ===========================================================================

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Display, VariantNames,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum BillingReason {
    /// Recurring renewal — triggers the usage-summary email.
    SubscriptionCycle,
    /// Initial subscription creation invoice.
    SubscriptionCreate,
    /// Plan change / proration invoice.
    SubscriptionUpdate,
    /// Manually-issued invoice.
    Manual,
    /// Anything else Stripe sends us.
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BillingInvoiceLineItem {
    pub description: Option<String>,
    pub amount_cents: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct BillingInvoice {
    pub stripe_invoice_id: String,
    pub amount_paid_cents: i64,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub billing_reason: BillingReason,
    pub line_items: Vec<BillingInvoiceLineItem>,
    /// Public link to Stripe's rendered PDF for this invoice. Stripe generates
    /// it lazily, so it can be `None` immediately after payment.
    pub invoice_pdf: Option<String>,
    /// Public link to Stripe's hosted invoice page — the fallback when the PDF
    /// isn't ready in time to attach.
    pub hosted_invoice_url: Option<String>,
}

// Stripe ships unix-epoch i64 timestamps; fall back to `Utc::now()` on a
// malformed value rather than failing the event publish.
fn ts_to_chrono(ts: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
}

impl From<&stripe_billing::Invoice> for BillingInvoice {
    fn from(inv: &stripe_billing::Invoice) -> Self {
        Self {
            stripe_invoice_id: inv.id.as_ref().map(|id| id.to_string()).unwrap_or_default(),
            amount_paid_cents: inv.amount_paid,
            currency: inv.currency.to_string(),
            created_at: ts_to_chrono(inv.created),
            period_start: ts_to_chrono(inv.period_start),
            period_end: ts_to_chrono(inv.period_end),
            billing_reason: inv.billing_reason.into(),
            line_items: inv
                .lines
                .data
                .iter()
                .map(BillingInvoiceLineItem::from)
                .collect(),
            invoice_pdf: inv.invoice_pdf.clone(),
            hosted_invoice_url: inv.hosted_invoice_url.clone(),
        }
    }
}

impl From<&stripe_billing::InvoiceLineItem> for BillingInvoiceLineItem {
    fn from(item: &stripe_billing::InvoiceLineItem) -> Self {
        Self {
            description: item.description.clone(),
            amount_cents: item.amount,
            period_start: ts_to_chrono(item.period.start),
            period_end: ts_to_chrono(item.period.end),
        }
    }
}

impl From<Option<stripe_billing::InvoiceBillingReason>> for BillingReason {
    fn from(reason: Option<stripe_billing::InvoiceBillingReason>) -> Self {
        use stripe_billing::InvoiceBillingReason::*;
        match reason {
            Some(SubscriptionCycle) => Self::SubscriptionCycle,
            Some(SubscriptionCreate) => Self::SubscriptionCreate,
            Some(SubscriptionUpdate) => Self::SubscriptionUpdate,
            Some(Manual) => Self::Manual,
            _ => Self::Other,
        }
    }
}
