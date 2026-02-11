use crate::server::billing::types::base::{BillingPlan, BillingRate};
use serde::Deserialize;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateCheckoutRequest {
    pub plan: BillingPlan,
    pub url: String,
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
