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
use stripe_billing::CancellationDetailsFeedback;
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
        comment: Option<String>,
        period_end: DateTime<Utc>,
    },
    PaymentSucceeded {
        invoice: BillingInvoice,
    },
    PaymentFailed {
        invoice_id: String,
        amount_cents: i64,
    },
    PaymentActionRequired {
        invoice_id: String,
    },
    PaymentRecovered {
        amount_cents: i64,
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
        reason_code: CancelReason,
        stripe_feedback: Option<CancellationDetailsFeedback>,
        comment: Option<String>,
        save_offer_shown: Vec<SaveOffer>,
        save_offer_redeemed: Option<SaveOffer>,
        planned_period_end: DateTime<Utc>,
    },
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
            | Self::Paused { plan, .. } => Some(plan),
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
            | Self::PlanChanged { .. }
            | Self::PaymentRecovered { .. }
            | Self::PaymentSucceeded { .. }
            | Self::Resumed { .. } => Some(PlanStatus::Active),

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

            // Telemetry-only — no state implication
            Self::CheckoutStarted { .. }
            | Self::TrialWillEnd { .. }
            | Self::FeatureLimitHit { .. }
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
