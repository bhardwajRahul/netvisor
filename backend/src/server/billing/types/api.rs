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
