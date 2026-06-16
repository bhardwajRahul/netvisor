use crate::server::{
    auth::r#impl::oidc::OidcProviderMetadata,
    billing::types::base::{
        BillingInvoice, BillingPlan, CancelReason, LimitSource, LimitType, SaveOffer,
    },
    discovery::r#impl::types::DiscoveryType,
    organizations::r#impl::base::UseCase,
    shared::api_key_common::ApiKeyType,
};
use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use stripe_billing::{CancellationDetailsFeedback, CancellationDetailsReason};
use strum::EnumIter;
use strum_macros::EnumDiscriminants;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize)]
pub enum EventLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Authentication method for user-flow auth events. API-key auth lives on
/// dedicated variants (`RotateKey`, `ApiKeyAuthFailed`) — not here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type")]
pub enum AuthMethod {
    Password,
    Oidc(OidcProviderMetadata),
}

/// Struct used for operations where an email + token is used: email verification, password reset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmailAndToken {
    pub email: EmailAddress,
    pub token: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::Display, EnumDiscriminants,
)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(Hash, EnumIter, strum::Display, Serialize, Deserialize,))]
pub enum AuthOperation {
    // User Auth
    Register {
        method: AuthMethod,
        marketing_opt_in: bool,
        // If None, user was invited or OIDC and email verification is not required
        email_and_token: Option<EmailAndToken>,
    },
    LoginSuccess {
        method: AuthMethod,
        via_register_flow: bool,
    },
    LoginFailed {
        method: AuthMethod,
        attempted_email: EmailAddress,
    },
    PasswordResetRequested {
        email_and_token: EmailAndToken,
    },
    PasswordResetCompleted,
    PasswordChanged {
        had_password: bool,
        email: EmailAddress,
        timestamp: DateTime<Utc>,
    },
    EmailVerified,
    OidcLinked {
        email: EmailAddress,
        provider: OidcProviderMetadata,
    },
    OidcUnlinked {
        email: EmailAddress,
        provider: OidcProviderMetadata,
    },
    EmailVerificationRequested {
        email_and_token: EmailAndToken,
    },
    EmailChangeRequested {
        email_and_token: EmailAndToken,
    },
    EmailChanged {
        old_email: EmailAddress,
        new_email: EmailAddress,
    },
    LoggedOut,

    // Api Key Auth
    RotateKey {
        api_key_id: Uuid,
        key_type: ApiKeyType,
    },
    ApiKeyAuthFailed {
        key_type: ApiKeyType,
        reason: String,
        key_prefix: String,
    },
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, strum::Display, EnumDiscriminants,
)]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(Hash, EnumIter, strum::Display, Serialize, Deserialize,))]
pub enum EntityOperation {
    Get,
    GetAll,
    Created,
    Updated,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum::Display, EnumDiscriminants)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(Hash, EnumIter, strum::Display, Serialize, Deserialize,))]
pub enum BillingOperation {
    CheckoutStarted {
        plan: BillingPlan,
        has_trial: bool,
    },
    CheckoutCompleted {
        plan: BillingPlan,
        included_networks: Option<u64>,
        included_seats: Option<u64>,
        mrr_amount_cents: i64,
        is_trialing: bool,
    },
    TrialStarted {
        plan: BillingPlan,
        trial_end: DateTime<Utc>,
        trial_days: u32,
    },
    TrialWillEnd {
        plan: BillingPlan,
        has_payment_method: bool,
    },
    TrialEnded {
        plan: BillingPlan,
        converted: bool,
    },
    PlanChanged {
        from: BillingPlan,
        to: BillingPlan,
        is_downgrade: bool,
    },
    SubscriptionCancelled {
        plan: BillingPlan,
        reason_code: Option<CancelReason>,
        stripe_feedback: Option<CancellationDetailsFeedback>,
        stripe_reason: Option<CancellationDetailsReason>,
        internal_reason: Option<String>,
        comment: Option<String>,
        period_end: DateTime<Utc>,
        was_trialing: bool,
        mrr_amount_cents: i64,
        tenure_days: u32,
    },
    PaymentSucceeded {
        invoice: BillingInvoice,
    },
    PaymentFailed {
        invoice_id: String,
        amount_cents: i64,
        plan: BillingPlan,
        attempt_count: u32,
    },
    PaymentActionRequired {
        invoice_id: String,
        /// Stripe-hosted authorization URL (3DS/SCA). Set in the cloud
        /// invoice payload; the email CTA links here directly so the user
        /// completes authorization on Stripe's page instead of navigating
        /// our settings modal.
        hosted_invoice_url: Option<String>,
    },
    PaymentRecovered {
        invoice_id: String,
        amount_cents: i64,
        plan: BillingPlan,
        attempt_count: u32,
    },
    FeatureLimitHit {
        limit_type: LimitType,
        current_count: u64,
        limit: u64,
        plan: BillingPlan,
        source: LimitSource,
    },
    Paused {
        plan: BillingPlan,
        duration_days: u32,
        resumes_at: DateTime<Utc>,
    },
    Resumed {
        was_early: bool,
    },
    TrialExtended {
        days_added: u32,
        new_trial_end: DateTime<Utc>,
    },
    CancellationInitiated {
        reason_code: Option<CancelReason>,
        stripe_feedback: Option<CancellationDetailsFeedback>,
        stripe_reason: Option<CancellationDetailsReason>,
        comment: Option<String>,
        save_offer_shown: Vec<SaveOffer>,
        save_offer_redeemed: Option<SaveOffer>,
        planned_period_end: DateTime<Utc>,
    },
    /// User cleared a pending cancellation (via in-app reactivate). Stripe's
    /// `cancel_at_period_end` flips from true to false; we emit this so the
    /// org subscriber's `implied_status` mirror flips `plan_status` back to
    /// `active` and analytics subscribers can attribute the un-churn.
    Reactivated,
    PaymentMethodAdded,
    PaymentMethodRemoved,
}

impl BillingOperation {
    /// Plan carried by the event, where the variant has one. Used by analytics
    /// subscribers (PostHog person properties, Brevo CRM sync) that need the
    /// plan name without a per-call-site exhaustive match.
    pub fn plan(&self) -> Option<&BillingPlan> {
        match self {
            Self::CheckoutStarted { plan, .. }
            | Self::CheckoutCompleted { plan, .. }
            | Self::TrialStarted { plan, .. }
            | Self::TrialWillEnd { plan, .. }
            | Self::TrialEnded { plan, .. }
            | Self::SubscriptionCancelled { plan, .. }
            | Self::FeatureLimitHit { plan, .. }
            | Self::Paused { plan, .. }
            | Self::PaymentFailed { plan, .. }
            | Self::PaymentRecovered { plan, .. } => Some(plan),
            Self::PlanChanged { to, .. } => Some(to),
            _ => None,
        }
    }

    /// Canonical mapping from a billing event to the `PlanStatus` it implies
    /// — or `None` for telemetry-only variants that don't affect status.
    /// Single source of truth used by Brevo's plan_status sync and PostHog
    /// person properties.
    pub fn implied_status(&self) -> Option<crate::server::billing::types::base::PlanStatus> {
        use crate::server::billing::types::base::PlanStatus;
        match self {
            Self::CheckoutCompleted { .. }
            | Self::PaymentRecovered { .. }
            | Self::Resumed { .. }
            | Self::Reactivated => Some(PlanStatus::Active),

            Self::TrialStarted { .. } | Self::TrialExtended { .. } => Some(PlanStatus::Trialing),
            Self::TrialEnded {
                converted: true, ..
            } => Some(PlanStatus::Active),
            Self::TrialEnded {
                converted: false, ..
            } => Some(PlanStatus::Cancelled),

            Self::PaymentFailed { .. } | Self::PaymentActionRequired { .. } => {
                Some(PlanStatus::PastDue)
            }

            Self::Paused { .. } => Some(PlanStatus::Paused),

            Self::CancellationInitiated { .. } => Some(PlanStatus::PendingCancellation),
            Self::SubscriptionCancelled { .. } => Some(PlanStatus::Cancelled),

            // Telemetry-only — no state implication.
            //
            // - `PlanChanged` describes a plan transition, not a status
            //   transition. At its only emission site (tier switch on an
            //   active sub) `plan_status` was `Active` and stays `Active`;
            //   the lifecycle event that triggered the switch (or didn't
            //   trigger one — for paid→paid tier switches there's no
            //   accompanying status change) owns the status. The chained
            //   PlanChanged-for-Brevo-sync at the cancel site that used to
            //   make this return `Active` is gone — see
            //   `process_subscription_deleted_side_effects`; Brevo now
            //   handles the Free plan_type write off `SubscriptionCancelled`
            //   directly.
            // - `PaymentSucceeded` fires on every invoice.paid webhook
            //   including the $0 trial-setup invoice Stripe creates
            //   alongside `customer.subscription.created`. Treating it as
            //   `Active` would race the `TrialStarted` write and clobber
            //   `plan_status='trialing'`. Subscription lifecycle is owned by
            //   `CheckoutCompleted` / `TrialStarted` / `TrialEnded` /
            //   `Paused` / `Resumed` / `Cancelled`, and dunning recovery by
            //   `PaymentRecovered` — which fires inside `handle_invoice_paid`
            //   BEFORE `PaymentSucceeded` for the was-past-due case, so we
            //   lose nothing.
            Self::CheckoutStarted { .. }
            | Self::PlanChanged { .. }
            | Self::TrialWillEnd { .. }
            | Self::FeatureLimitHit { .. }
            | Self::PaymentSucceeded { .. }
            | Self::PaymentMethodAdded
            | Self::PaymentMethodRemoved => None,
        }
    }
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    strum::Display,
    utoipa::ToSchema,
    EnumDiscriminants,
)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(
    Hash,
    EnumIter,
    strum::Display,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
    strum::IntoStaticStr,
    strum::VariantNames,
))]
pub enum OnboardingOperation {
    OrgCreated {
        org_name: String,
        plan: BillingPlan,
        use_case: UseCase,
    },
    OnboardingModalCompleted,
    PlanSelected {
        plan: BillingPlan,
    },
    FirstDaemonRegistered {
        daemon_name: String,
        network_name: String,
    },
    FirstTopologyRebuild,
    FirstDiscoveryCompleted {
        discovery_type: DiscoveryType,
    },
    FirstHostDiscovered,
    SecondNetworkCreated {
        network_id: Uuid,
        network_name: String,
        total_networks: u32,
    },
    FirstTagCreated,
    #[serde(alias = "FirstGroupCreated")]
    FirstDependencyCreated,
    FirstUserApiKeyCreated,
    FirstSnmpCredentialCreated,
    FirstApplicationTagCreated,
    FirstCredentialCreated,
    FirstSnapshotCreated {
        snapshot_id: Uuid,
        network_id: Uuid,
    },
    InviteSent,
    InviteAccepted,
    ProfileCompleted {
        job_title: Option<String>,
        company_size: Option<String>,
    },
    ReferralSourceCompleted {
        referral_source: String,
        referral_source_other: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, strum::Display, EnumDiscriminants)]
#[serde(tag = "type")]
#[strum(serialize_all = "snake_case")]
#[strum_discriminants(derive(Hash, EnumIter, strum::Display, Serialize, Deserialize,))]
pub enum AnalyticsOperation {
    TopologyShareViewed { share_id: Uuid, has_password: bool },
    TopologyEmbedViewed { share_id: Uuid, has_password: bool },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::plans::get_free_plan;

    fn round_trip(op: BillingOperation) {
        let json = serde_json::to_string(&op).expect("serialize");
        let back: BillingOperation = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(op, back, "round-trip mismatch for {json}");
    }

    #[test]
    fn subscription_cancelled_round_trip_with_all_optionals_some() {
        round_trip(BillingOperation::SubscriptionCancelled {
            plan: get_free_plan(),
            reason_code: Some(crate::server::billing::types::base::CancelReason::TooExpensive),
            stripe_feedback: Some(CancellationDetailsFeedback::TooExpensive),
            stripe_reason: Some(CancellationDetailsReason::PaymentFailed),
            internal_reason: Some("admin".to_string()),
            comment: Some("not for me".to_string()),
            period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            was_trialing: true,
            mrr_amount_cents: 9900,
            tenure_days: 42,
        });
    }

    #[test]
    fn subscription_cancelled_round_trip_with_all_optionals_none() {
        round_trip(BillingOperation::SubscriptionCancelled {
            plan: get_free_plan(),
            reason_code: None,
            stripe_feedback: None,
            stripe_reason: None,
            internal_reason: None,
            comment: None,
            period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            was_trialing: false,
            mrr_amount_cents: 0,
            tenure_days: 0,
        });
    }

    #[test]
    fn checkout_completed_round_trip_paid() {
        round_trip(BillingOperation::CheckoutCompleted {
            plan: get_free_plan(),
            included_networks: Some(3),
            included_seats: Some(5),
            mrr_amount_cents: 4900,
            is_trialing: false,
        });
    }

    #[test]
    fn checkout_completed_round_trip_trialing() {
        round_trip(BillingOperation::CheckoutCompleted {
            plan: get_free_plan(),
            included_networks: Some(3),
            included_seats: Some(5),
            mrr_amount_cents: 4900,
            is_trialing: true,
        });
    }

    #[test]
    fn cancellation_initiated_round_trip_with_stripe_details() {
        round_trip(BillingOperation::CancellationInitiated {
            reason_code: None,
            stripe_feedback: Some(CancellationDetailsFeedback::TooExpensive),
            stripe_reason: Some(CancellationDetailsReason::CancellationRequested),
            comment: Some("not for me".to_string()),
            save_offer_shown: vec![],
            save_offer_redeemed: None,
            planned_period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
        });
    }

    #[test]
    fn cancellation_initiated_round_trip_all_none() {
        round_trip(BillingOperation::CancellationInitiated {
            reason_code: None,
            stripe_feedback: None,
            stripe_reason: None,
            comment: None,
            save_offer_shown: vec![],
            save_offer_redeemed: None,
            planned_period_end: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
        });
    }

    #[test]
    fn payment_failed_round_trip() {
        round_trip(BillingOperation::PaymentFailed {
            invoice_id: "in_123".to_string(),
            amount_cents: 9900,
            plan: get_free_plan(),
            attempt_count: 3,
        });
    }

    #[test]
    fn payment_recovered_round_trip() {
        round_trip(BillingOperation::PaymentRecovered {
            invoice_id: "in_456".to_string(),
            amount_cents: 9900,
            plan: get_free_plan(),
            attempt_count: 2,
        });
    }
}
