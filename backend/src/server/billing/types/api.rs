use crate::server::billing::types::base::{BillingPlan, BillingRate, CancelReason, SaveOffer};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCheckoutRequest {
    pub plan: BillingPlan,
    pub url: String,
}

/// Pause subscription duration. The cancel modal's `RadioGroup` posts
/// one of these enum variants verbatim — no integer parsing at the API
/// boundary, the type is the contract.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PauseDuration {
    Days30,
    Days60,
    Days90,
}

impl PauseDuration {
    pub fn days(self) -> u32 {
        match self {
            Self::Days30 => 30,
            Self::Days60 => 60,
            Self::Days90 => 90,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PauseSubscriptionRequest {
    pub duration_days: PauseDuration,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelSubscriptionRequest {
    pub reason_code: CancelReason,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub save_offer_shown: Vec<SaveOffer>,
    #[serde(default)]
    pub save_offer_redeemed: Option<SaveOffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelSubscriptionResponse {
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetupPaymentMethodRequest {
    pub url: String,
}

/// Response for creating a SetupIntent — the client secret the frontend
/// Payment Element uses to collect and confirm a card in-app.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetupIntentResponse {
    pub client_secret: String,
}

/// Request to finalize a client-confirmed SetupIntent (set the collected card
/// as the customer's default payment method).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FinalizePaymentMethodRequest {
    pub setup_intent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangePlanRequest {
    pub plan: BillingPlan,
    pub rate: BillingRate,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChangePlanPreview {
    pub excess_hosts: u64,
    pub excess_networks: u64,
    pub excess_seats: u64,
}

/// Live terms for the configured save-offer coupon, read directly from
/// Stripe. Used by the cancel modal's Discount panel to render the offer
/// dynamically instead of hard-coding the percent/duration.
///
/// Only returned when the coupon would actually catch the user's next
/// invoice — i.e. `next_renewal_at` falls within the coupon's `duration_in_months`
/// window. Yearly subscribers partway through a cycle whose next renewal
/// lands after the coupon's window get `None` from the endpoint and the
/// cancel modal's Discount panel doesn't render.
///
/// `billing_rate` lets the frontend pick monthly vs yearly copy: a monthly
/// subscriber thinks in terms of "N months of discount"; a yearly subscriber
/// thinks in terms of "my next renewal on {date}."
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SaveOfferCoupon {
    pub percent_off: i64,
    pub duration_in_months: i64,
    pub next_renewal_at: DateTime<Utc>,
    pub billing_rate: BillingRate,
}
