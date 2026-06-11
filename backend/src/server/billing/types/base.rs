use crate::server::{
    billing::types::features::Feature,
    email::traits::format_cents,
    shared::types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::Hash;
use stripe_product::price::CreatePriceRecurringInterval;
use strum::{Display, EnumDiscriminants, EnumIter, IntoDiscriminant, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

// ===========================================================================
// Component enums for typed BillingOperation payloads
// ===========================================================================

/// Cancellation reason captured in `SubscriptionCancelled` /
/// `CancellationInitiated` events. Mirrors the values surfaced in the
/// in-app cancel flow (Phase 5).
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumIter,
    IntoStaticStr,
    VariantNames,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CancelReason {
    TooExpensive,
    MissingFeatures,
    SwitchedService,
    Unused,
    CustomerService,
    LowQuality,
    TooComplex,
    Other,
}

impl HasId for CancelReason {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for CancelReason {
    fn color(&self) -> Color {
        // Visual differentiation isn't load-bearing here; the modal renders
        // these as a list, not a chart. Use a single neutral palette so the
        // fixture has stable values.
        Color::Gray
    }

    fn icon(&self) -> Icon {
        match self {
            Self::TooExpensive => Icon::DollarSign,
            Self::MissingFeatures => Icon::Layers,
            Self::SwitchedService => Icon::ArrowRightLeft,
            Self::Unused => Icon::CircleSlash,
            Self::CustomerService => Icon::Headset,
            Self::LowQuality => Icon::Frown,
            Self::TooComplex => Icon::Puzzle,
            Self::Other => Icon::MessageCircle,
        }
    }
}

impl TypeMetadataProvider for CancelReason {
    fn name(&self) -> &'static str {
        match self {
            Self::TooExpensive => "Too expensive",
            Self::MissingFeatures => "Missing features",
            Self::SwitchedService => "Switched to another service",
            Self::Unused => "Not using it enough",
            Self::CustomerService => "Customer service",
            Self::LowQuality => "Low quality",
            Self::TooComplex => "Too complex",
            Self::Other => "Other",
        }
    }

    fn metadata(&self) -> serde_json::Value {
        // Reason → save-offer mapping. The frontend reads this from
        // `cancel-reasons.json` to drive step 2 of the cancel modal.
        // `Discount` is included unconditionally; the UI filters it out
        // when `discount_save_offer_available` is false on the org payload.
        let save_offers: Vec<&'static str> = match self {
            Self::TooExpensive => vec![SaveOffer::Pause.id(), SaveOffer::Discount.id()],
            Self::Unused => vec![SaveOffer::Pause.id()],
            _ => vec![],
        };
        serde_json::json!({ "save_offers": save_offers })
    }
}

/// Save-offer choices presented during in-app cancellation (Phase 5).
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumIter,
    IntoStaticStr,
    VariantNames,
    ToSchema,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SaveOffer {
    Pause,
    Discount,
    Downgrade,
}

impl HasId for SaveOffer {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for SaveOffer {
    fn color(&self) -> Color {
        match self {
            Self::Pause => Color::Amber,
            Self::Discount => Color::Green,
            Self::Downgrade => Color::Blue,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Pause => Icon::Pause,
            Self::Discount => Icon::BadgePercent,
            Self::Downgrade => Icon::TrendingDown,
        }
    }
}

impl TypeMetadataProvider for SaveOffer {
    fn name(&self) -> &'static str {
        match self {
            Self::Pause => "Pause subscription",
            Self::Discount => "Apply a discount",
            Self::Downgrade => "Switch to a smaller plan",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Pause => {
                "Take a break for 30, 60, or 90 days. We'll keep your data and resume billing when you return."
            }
            Self::Discount => "Stay subscribed at a lower rate for the next few months.",
            Self::Downgrade => "Move to a plan with fewer features but a lower price.",
        }
    }
}

/// Dimension hit when a `FeatureLimitHit` event fires.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Display, EnumIter, VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum LimitType {
    Networks,
    Hosts,
    Seats,
    Snapshots,
}

/// Origin of the request that triggered the limit hit. `Api` covers
/// user-initiated requests; `Discovery` covers automated discovery flows.
#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Display, EnumIter, VariantNames,
)]
#[serde(rename_all = "snake_case")]
pub enum LimitSource {
    Api,
    Discovery,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Display,
    IntoStaticStr,
    EnumIter,
    EnumDiscriminants,
    VariantNames,
    Eq,
    ToSchema,
)]
#[strum_discriminants(derive(IntoStaticStr, Serialize))]
#[serde(tag = "type")]
pub enum BillingPlan {
    Community(PlanConfig),
    Free(PlanConfig),
    Starter(PlanConfig),
    Pro(PlanConfig),
    Team(PlanConfig),
    Business(PlanConfig),
    Enterprise(PlanConfig),
    Demo(PlanConfig),
    CommercialSelfHosted(PlanConfig),
}

impl PartialOrd for BillingPlanDiscriminants {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        fn cloud_tier(d: &BillingPlanDiscriminants) -> Option<u8> {
            match d {
                BillingPlanDiscriminants::Free => Some(0),
                BillingPlanDiscriminants::Starter => Some(1),
                BillingPlanDiscriminants::Pro => Some(2),
                BillingPlanDiscriminants::Team => Some(3),
                BillingPlanDiscriminants::Business => Some(4),
                BillingPlanDiscriminants::Enterprise => Some(5),
                _ => None,
            }
        }
        match (cloud_tier(self), cloud_tier(other)) {
            (Some(a), Some(b)) => Some(a.cmp(&b)),
            _ if self == other => Some(std::cmp::Ordering::Equal),
            _ => None,
        }
    }
}

impl PartialEq for BillingPlan {
    fn eq(&self, other: &Self) -> bool {
        self.config() == other.config()
    }
}

impl Hash for BillingPlan {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.config().hash(state);
    }
}

impl Default for BillingPlan {
    fn default() -> Self {
        #[cfg(feature = "commercial")]
        {
            use crate::server::billing::plans::get_commercial_self_hosted_plan;

            get_commercial_self_hosted_plan()
        }
        #[cfg(not(feature = "commercial"))]
        {
            use crate::server::billing::plans::get_community_plan;

            get_community_plan()
        }
    }
}

impl BillingPlan {
    pub fn to_yearly(&self, discount: f32) -> Self {
        let mut yearly_config = self.config();
        yearly_config.rate = BillingRate::Year;

        // Round discounted monthly base to nearest dollar then subtract 1 cent
        // so yearly prices end in .99 (e.g. $14.99/mo → $11.99/mo billed yearly).
        let monthly_base = Self::round_to_99(yearly_config.base_cents as f32 * (1.0 - discount));
        yearly_config.base_cents = monthly_base * 12;
        yearly_config.seat_cents = yearly_config.seat_cents.map(|c| {
            let monthly = Self::round_to_dollar(c as f32 * (1.0 - discount));
            monthly * 12
        });
        yearly_config.network_cents = yearly_config.network_cents.map(|c| {
            let monthly = Self::round_to_dollar(c as f32 * (1.0 - discount));
            monthly * 12
        });
        yearly_config.host_cents = yearly_config.host_cents.map(|c| {
            let monthly = Self::round_to_dollar(c as f32 * (1.0 - discount));
            monthly * 12
        });

        let mut yearly_plan = *self;
        yearly_plan.set_config(yearly_config);
        yearly_plan
    }
    fn round_to_dollar(cents: f32) -> i64 {
        ((cents / 100.0).round() * 100.0) as i64
    }

    /// Round to nearest dollar, then subtract 1 cent so the price ends in .99.
    fn round_to_99(cents: f32) -> i64 {
        Self::round_to_dollar(cents) - 1
    }

    pub fn billing_period(&self) -> &str {
        self.config().rate.billing_period()
    }

    /// Format a plan's base price for display in emails (e.g. "$14.99/mo")
    pub fn base_price_formatted(&self) -> String {
        let config = self.config();
        let amount = format_cents(config.base_cents, "usd");
        match config.rate {
            BillingRate::Month => format!("{}/mo", amount),
            BillingRate::Year => format!("{}/yr", amount),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Default, Hash, ToSchema)]
pub struct PlanConfig {
    pub base_cents: i64,
    pub rate: BillingRate,
    pub trial_days: u32,

    // None = can't pay for more
    pub seat_cents: Option<i64>,
    pub network_cents: Option<i64>,
    pub host_cents: Option<i64>,

    // None = unlimited
    pub included_seats: Option<u64>,
    pub included_networks: Option<u64>,
    pub included_hosts: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Display, Copy, PartialEq, Eq, Default, Hash)]
pub enum Hosting {
    SelfHosted,
    Managed,
    #[default]
    Cloud,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Display, Default, Copy, PartialEq, Eq, Hash, ToSchema,
)]
pub enum BillingRate {
    #[default]
    Month,
    Year,
}

impl BillingRate {
    pub fn stripe_recurring_interval(&self) -> CreatePriceRecurringInterval {
        match self {
            BillingRate::Month => CreatePriceRecurringInterval::Month,
            BillingRate::Year => CreatePriceRecurringInterval::Year,
        }
    }

    pub fn billing_period(&self) -> &'static str {
        match self {
            BillingRate::Month => "Monthly",
            BillingRate::Year => "Yearly",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingPlanFeatures {
    pub share_views: bool,
    pub remove_created_with: bool,
    pub audit_logs: bool,
    pub webhooks: bool,
    pub api_access: bool,
    pub onboarding_call: bool,
    pub custom_sso: bool,
    pub managed_deployment: bool,
    pub whitelabeling: bool,
    pub live_chat_support: bool,
    pub embeds: bool,
    pub email_support: bool,
    pub priority_support: bool,
    // Core features
    pub network_mapping: bool,
    pub png_export: bool,
    pub svg_export: bool,
    pub mermaid_export: bool,
    pub confluence_export: bool,
    pub pdf_export: bool,
    pub html_export: bool,
    pub scheduled_discovery: bool,
    pub discovery_integrations: bool,
    pub csv_export: bool,
    /// How many days of snapshots the plan retains before the daily sweep
    /// deletes them. `0` means snapshots are unavailable on this plan. The
    /// env-var override (`SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE`) takes
    /// precedence at runtime — see `BillingPlan::snapshot_retention_days`.
    pub snapshot_retention_days: u32,
}

impl BillingPlan {
    pub fn config(&self) -> PlanConfig {
        match self {
            BillingPlan::Community(plan_config) => *plan_config,
            BillingPlan::Free(plan_config) => *plan_config,
            BillingPlan::Starter(plan_config) => *plan_config,
            BillingPlan::Pro(plan_config) => *plan_config,
            BillingPlan::Team(plan_config) => *plan_config,
            BillingPlan::Business(plan_config) => *plan_config,
            BillingPlan::Enterprise(plan_config) => *plan_config,
            BillingPlan::Demo(plan_config) => *plan_config,
            BillingPlan::CommercialSelfHosted(plan_config) => *plan_config,
        }
    }

    pub fn set_config(&mut self, config: PlanConfig) {
        match self {
            BillingPlan::Community(plan_config) => *plan_config = config,
            BillingPlan::Free(plan_config) => *plan_config = config,
            BillingPlan::Starter(plan_config) => *plan_config = config,
            BillingPlan::Pro(plan_config) => *plan_config = config,
            BillingPlan::Team(plan_config) => *plan_config = config,
            BillingPlan::Business(plan_config) => *plan_config = config,
            BillingPlan::Enterprise(plan_config) => *plan_config = config,
            BillingPlan::Demo(plan_config) => *plan_config = config,
            BillingPlan::CommercialSelfHosted(plan_config) => *plan_config = config,
        }
    }

    pub fn is_commercial(&self) -> bool {
        matches!(
            self,
            BillingPlan::Pro(_)
                | BillingPlan::Team(_)
                | BillingPlan::Business(_)
                | BillingPlan::Enterprise(_)
                | BillingPlan::CommercialSelfHosted(_)
                | BillingPlan::Demo(_)
        )
    }

    pub fn is_free(&self) -> bool {
        matches!(self, BillingPlan::Free(_))
    }

    pub fn is_demo(&self) -> bool {
        matches!(self, BillingPlan::Demo(_))
    }

    /// Plans where the customer hosts Scanopy themselves and Stripe is not in
    /// the loop. Use this to skip checks that only make sense for cloud plans.
    pub fn is_self_hosted(&self) -> bool {
        matches!(
            self,
            BillingPlan::Community(_) | BillingPlan::CommercialSelfHosted(_)
        )
    }

    pub fn host_limit(&self) -> Option<u64> {
        self.config().included_hosts
    }

    pub fn network_limit(&self) -> Option<u64> {
        self.config().included_networks
    }

    pub fn seat_limit(&self) -> Option<u64> {
        self.config().included_seats
    }

    /// Snapshot retention window in days for this plan. `0` means snapshots
    /// are unavailable. `env_override` (`SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE`)
    /// is a universal escape hatch — when set it wins over the fixture value
    /// for every plan tier. Self-hosted operators use it to extend retention
    /// without forking the plan fixture.
    pub fn snapshot_retention_days(&self, env_override: Option<u32>) -> u32 {
        env_override.unwrap_or_else(|| self.features().snapshot_retention_days)
    }

    pub fn can_invite_users(&self) -> bool {
        // If there's an included amount, then there's a cap and seat_cents needs to be Some to buy more
        if self.config().included_seats.is_some() {
            self.config().seat_cents.is_some()
        // If included is None, it's unlimited
        } else {
            true
        }
    }

    pub fn hosting(&self) -> Hosting {
        match self {
            BillingPlan::Community(_) => Hosting::SelfHosted,
            BillingPlan::CommercialSelfHosted(_) => Hosting::SelfHosted,
            BillingPlan::Enterprise(_) => Hosting::Managed,
            _ => Hosting::Cloud, // Free, Starter, Pro, Team, Business, Demo
        }
    }

    /// Returns the next-lower-tier cloud plan, if this is a cloud plan.
    /// Returns None for Free (no previous) and self-hosted/demo plans.
    pub fn previous_tier(&self) -> Option<BillingPlanDiscriminants> {
        let cloud_tiers: Vec<BillingPlanDiscriminants> = vec![
            BillingPlanDiscriminants::Free,
            BillingPlanDiscriminants::Starter,
            BillingPlanDiscriminants::Pro,
            BillingPlanDiscriminants::Business,
            BillingPlanDiscriminants::Enterprise,
        ];

        let my_disc = self.discriminant();
        let idx = cloud_tiers.iter().position(|d| *d == my_disc)?;
        if idx == 0 {
            return None;
        }
        Some(cloud_tiers[idx - 1])
    }

    /// Returns feature IDs added by this plan over its previous tier.
    /// For Free: returns all enabled features (it's the baseline).
    /// For self-hosted plans with no previous tier: returns all enabled non-universal features.
    /// For cloud plans: returns features new vs the previous tier.
    pub fn incremental_features(&self) -> Vec<&'static str> {
        let enabled = self.enabled_feature_ids();

        match self.previous_tier() {
            Some(prev_disc) => {
                let prev_plan = Self::default_for_discriminant(prev_disc);
                match prev_plan {
                    Some(plan) => {
                        let prev_features = plan.enabled_feature_ids();
                        enabled.difference(&prev_features).copied().collect()
                    }
                    None => enabled.into_iter().collect(),
                }
            }
            None if self.is_free() => {
                // Free plan: show all enabled features (it's the baseline)
                enabled.into_iter().collect()
            }
            None => {
                // Self-hosted/other plans: show features beyond universal (Free) baseline
                let universal = Self::universal_feature_ids();
                enabled.difference(&universal).copied().collect()
            }
        }
    }

    /// Returns set of feature IDs where the feature is enabled on this plan.
    pub fn enabled_feature_ids(&self) -> HashSet<&'static str> {
        let features = self.features();
        let json = serde_json::to_value(&features).unwrap();
        let obj = json.as_object().unwrap();
        obj.iter()
            .filter(|(_, v)| v.as_bool().unwrap_or(false))
            .map(|(k, _)| {
                // Leak the key string so we get &'static str
                // This is fine since these are a small fixed set called infrequently
                let s: &'static str = Box::leak(k.clone().into_boxed_str());
                s
            })
            .collect()
    }

    /// Features that are universal across all plans (present on Free).
    fn universal_feature_ids() -> HashSet<&'static str> {
        use crate::server::billing::plans::get_free_plan;
        get_free_plan().enabled_feature_ids()
    }

    /// Whether the feature identified by `feature_id` is enabled on this plan.
    /// Boolean features → true/false directly; numeric features (e.g.
    /// `snapshot_retention_days`) are "enabled" when the value is > 0.
    pub fn has_feature(&self, feature_id: &str) -> bool {
        let features = self.features();
        let json = serde_json::to_value(&features).unwrap();
        let Some(v) = json.get(feature_id) else {
            return false;
        };
        if let Some(b) = v.as_bool() {
            return b;
        }
        v.as_u64().map(|n| n > 0).unwrap_or(false)
    }

    /// Build a default plan instance for a given discriminant (monthly, default config).
    pub fn default_for_discriminant(disc: BillingPlanDiscriminants) -> Option<BillingPlan> {
        use crate::server::billing::plans::*;

        match disc {
            BillingPlanDiscriminants::Free => Some(get_free_plan()),
            BillingPlanDiscriminants::Community => Some(get_community_plan()),
            BillingPlanDiscriminants::Enterprise => Some(get_enterprise_plan()),
            BillingPlanDiscriminants::CommercialSelfHosted => {
                Some(get_commercial_self_hosted_plan())
            }
            // For purchasable plans, find them from the default list
            _ => get_purchasable_plans()
                .into_iter()
                .find(|p| p.discriminant() == disc),
        }
    }

    pub fn custom_price(&self) -> Option<&str> {
        match self {
            BillingPlan::Enterprise(_) => Some("Custom"),
            BillingPlan::Community(_) | BillingPlan::Free(_) => Some("Free"),
            BillingPlan::CommercialSelfHosted(_) => Some("Custom"),
            _ => None,
        }
    }

    pub fn stripe_product_id(&self) -> String {
        self.to_string().to_lowercase()
    }

    pub fn stripe_base_price_lookup_key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.stripe_product_id(),
            self.config().base_cents,
            self.config().rate
        )
    }

    pub fn stripe_seat_addon_price_lookup_key(&self) -> Option<String> {
        self.config().seat_cents.map(|c| {
            format!(
                "{}_seats_{}_{}",
                self.stripe_product_id(),
                c,
                self.config().rate
            )
        })
    }

    pub fn stripe_network_addon_price_lookup_key(&self) -> Option<String> {
        self.config().network_cents.map(|c| {
            format!(
                "{}_networks_{}_{}",
                self.stripe_product_id(),
                c,
                self.config().rate
            )
        })
    }

    pub fn features(&self) -> BillingPlanFeatures {
        match self {
            BillingPlan::Community { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: false,
                api_access: true,
                custom_sso: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: false,
                priority_support: false,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: false,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::Free { .. } => BillingPlanFeatures {
                share_views: false,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: false,
                custom_sso: false,
                api_access: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: false,
                email_support: false,
                priority_support: false,
                network_mapping: true,
                png_export: true,
                svg_export: false,
                mermaid_export: false,
                confluence_export: false,
                pdf_export: false,
                html_export: false,
                scheduled_discovery: false,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 0,
            },
            BillingPlan::Starter { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: true,
                custom_sso: false,
                api_access: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: false,
                email_support: true,
                priority_support: false,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: false,
                confluence_export: false,
                pdf_export: false,
                html_export: false,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 7,
            },
            BillingPlan::Pro { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: true,
                api_access: true,
                custom_sso: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: false,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: false,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 30,
            },
            BillingPlan::Team { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: false,
                audit_logs: false,
                remove_created_with: true,
                custom_sso: false,
                api_access: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::Business { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                custom_sso: false,
                api_access: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::Enterprise { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                custom_sso: true,
                api_access: true,
                managed_deployment: true,
                whitelabeling: true,
                live_chat_support: true,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::Demo { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                custom_sso: true,
                api_access: true,
                managed_deployment: true,
                whitelabeling: true,
                live_chat_support: true,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
            BillingPlan::CommercialSelfHosted { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                api_access: true,
                custom_sso: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                priority_support: true,
                network_mapping: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                discovery_integrations: true,
                csv_export: true,
                snapshot_retention_days: 90,
            },
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<Feature>> for BillingPlanFeatures {
    fn into(self) -> Vec<Feature> {
        let mut features = vec![];

        let BillingPlanFeatures {
            share_views,
            onboarding_call,
            webhooks,
            audit_logs,
            remove_created_with,
            custom_sso,
            managed_deployment,
            whitelabeling,
            api_access,
            live_chat_support,
            embeds,
            email_support,
            priority_support,
            network_mapping,
            png_export,
            svg_export,
            mermaid_export,
            confluence_export,
            pdf_export,
            html_export,
            scheduled_discovery,
            discovery_integrations,
            csv_export,
            snapshot_retention_days,
        } = self;

        if share_views {
            features.push(Feature::ShareViews)
        }

        if custom_sso {
            features.push(Feature::CustomSso)
        }

        if api_access {
            features.push(Feature::ApiAccess)
        }

        if managed_deployment {
            features.push(Feature::ManagedDeployment)
        }

        if embeds {
            features.push(Feature::Embeds)
        }

        if whitelabeling {
            features.push(Feature::Whitelabeling)
        }

        if live_chat_support {
            features.push(Feature::LiveChatSupport)
        }

        if priority_support {
            features.push(Feature::PrioritySupport)
        }

        if email_support {
            features.push(Feature::EmailSupport)
        }

        if onboarding_call {
            features.push(Feature::OnboardingCall)
        }

        if webhooks {
            features.push(Feature::Webhooks);
        }

        if audit_logs {
            features.push(Feature::AuditLogs)
        }

        if remove_created_with {
            features.push(Feature::RemoveCreatedWith)
        }

        if network_mapping {
            features.push(Feature::NetworkMapping)
        }

        if png_export {
            features.push(Feature::PngExport)
        }

        if svg_export {
            features.push(Feature::SvgExport)
        }

        if mermaid_export {
            features.push(Feature::MermaidExport)
        }

        if confluence_export {
            features.push(Feature::ConfluenceExport)
        }

        if pdf_export {
            features.push(Feature::PdfExport)
        }

        if html_export {
            features.push(Feature::HtmlExport)
        }

        if scheduled_discovery {
            features.push(Feature::ScheduledDiscovery)
        }

        if discovery_integrations {
            features.push(Feature::DiscoveryIntegrations)
        }

        if csv_export {
            features.push(Feature::CsvExport)
        }

        if snapshot_retention_days > 0 {
            features.push(Feature::SnapshotRetentionDays)
        }

        features
    }
}

impl HasId for BillingPlan {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for BillingPlan {
    fn icon(&self) -> Icon {
        match self {
            BillingPlan::Community { .. } => Icon::Heart,
            BillingPlan::Free { .. } => Icon::Gift,
            BillingPlan::Starter { .. } => Icon::ThumbsUp,
            BillingPlan::Pro { .. } => Icon::Zap,
            BillingPlan::Team { .. } => Icon::Users,
            BillingPlan::Business { .. } => Icon::Briefcase,
            BillingPlan::Enterprise { .. } => Icon::Building,
            BillingPlan::Demo { .. } => Icon::TestTube,
            BillingPlan::CommercialSelfHosted { .. } => Icon::ServerCog,
        }
    }

    fn color(&self) -> Color {
        match self {
            BillingPlan::Community { .. } => Color::Pink,
            BillingPlan::Free { .. } => Color::Green,
            BillingPlan::Starter { .. } => Color::Blue,
            BillingPlan::Pro { .. } => Color::Yellow,
            BillingPlan::Team { .. } => Color::Orange,
            BillingPlan::Business { .. } => Color::Indigo,
            BillingPlan::Enterprise { .. } => Color::Teal,
            BillingPlan::Demo { .. } => Color::Purple,
            BillingPlan::CommercialSelfHosted { .. } => Color::Gray,
        }
    }
}

impl TypeMetadataProvider for BillingPlan {
    fn name(&self) -> &'static str {
        match self {
            BillingPlan::Community { .. } => "Community",
            BillingPlan::Free { .. } => "Free",
            BillingPlan::Starter { .. } => "Starter",
            BillingPlan::Pro { .. } => "Pro",
            BillingPlan::Team { .. } => "Team",
            BillingPlan::Business { .. } => "Business",
            BillingPlan::Enterprise { .. } => "Enterprise",
            BillingPlan::Demo { .. } => "Demo",
            BillingPlan::CommercialSelfHosted { .. } => "On-Premise",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            BillingPlan::Community { .. } => {
                "Community plan for individuals self-hosting Scanopy - full control over configuration and integrations"
            }
            BillingPlan::Free { .. } => "For hobbyists exploring a small network",
            BillingPlan::Starter { .. } => "For homelabbers automating documentation",
            BillingPlan::Pro { .. } => "For IT pros managing multiple networks",
            BillingPlan::Team { .. } => {
                "Collaborate on infrastructure documentation with your team"
            }
            BillingPlan::Business { .. } => "For MSPs managing client infrastructure",
            BillingPlan::Enterprise { .. } => "For organizations needing custom deployment",
            BillingPlan::Demo { .. } => "Demo mode",
            BillingPlan::CommercialSelfHosted { .. } => {
                "Commercial license for self-managed deployments — full control over configuration and integrations"
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let config = self.config();
        let previous_tier = self
            .previous_tier()
            .and_then(BillingPlan::default_for_discriminant)
            .map(|p| p.id());

        serde_json::json!({
            // Pricing information
            "base_cents": config.base_cents,
            "rate": config.rate,
            "trial_days": config.trial_days,
            "seat_cents": config.seat_cents,
            "network_cents": config.network_cents,
            "host_cents": config.host_cents,
            "included_seats": config.included_seats,
            "included_networks": config.included_networks,
            "included_hosts": config.included_hosts,
            // Feature flags and metadata
            "features": self.features(),
            "is_commercial": self.is_commercial(),
            "hosting": self.hosting(),
            "custom_price": self.custom_price(),
            // Tier relationship
            "incremental_features": self.incremental_features(),
            "previous_tier": previous_tier
        })
    }
}

/// Derived subscription status — our domain enum, never Stripe's raw status.
/// Stripe webhook events map to typed `BillingOperation` variants at reception
/// (in `billing/service.rs`); each variant deterministically implies a
/// `PlanStatus` for downstream feature gates via
/// `BillingOperation::implied_status`.
#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash, strum::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum PlanStatus {
    Active,
    Trialing,
    PastDue,
    Paused,
    PendingCancellation,
    Cancelled,
}

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

#[cfg(test)]
mod cancel_modal_tests {
    use super::*;

    fn save_offers_for(reason: CancelReason) -> Vec<String> {
        let metadata = reason.metadata();
        metadata["save_offers"]
            .as_array()
            .expect("save_offers should be an array")
            .iter()
            .map(|v| v.as_str().expect("offer should be a string").to_string())
            .collect()
    }

    #[test]
    fn cancel_reason_too_expensive_offers_pause_and_discount() {
        assert_eq!(
            save_offers_for(CancelReason::TooExpensive),
            vec!["pause", "discount"]
        );
    }

    #[test]
    fn cancel_reason_unused_offers_pause_only() {
        assert_eq!(save_offers_for(CancelReason::Unused), vec!["pause"]);
    }

    #[test]
    fn cancel_reasons_without_offers_return_empty_list() {
        for reason in [
            CancelReason::MissingFeatures,
            CancelReason::SwitchedService,
            CancelReason::CustomerService,
            CancelReason::LowQuality,
            CancelReason::TooComplex,
            CancelReason::Other,
        ] {
            assert!(
                save_offers_for(reason).is_empty(),
                "{reason:?} should have no save offers"
            );
        }
    }

    #[test]
    fn cancel_reason_id_is_snake_case() {
        assert_eq!(CancelReason::TooExpensive.id(), "too_expensive");
        assert_eq!(CancelReason::Other.id(), "other");
    }

    #[test]
    fn save_offer_id_is_snake_case() {
        assert_eq!(SaveOffer::Pause.id(), "pause");
        assert_eq!(SaveOffer::Discount.id(), "discount");
        assert_eq!(SaveOffer::Downgrade.id(), "downgrade");
    }
}

#[cfg(test)]
mod snapshot_retention_tests {
    use super::*;
    use crate::server::billing::types::base::PlanConfig;

    fn cfg() -> PlanConfig {
        PlanConfig::default()
    }

    #[test]
    fn no_override_returns_plan_fixture_value() {
        assert_eq!(BillingPlan::Free(cfg()).snapshot_retention_days(None), 0);
        assert_eq!(BillingPlan::Starter(cfg()).snapshot_retention_days(None), 7);
        assert_eq!(BillingPlan::Pro(cfg()).snapshot_retention_days(None), 30);
        assert_eq!(
            BillingPlan::Business(cfg()).snapshot_retention_days(None),
            90
        );
        assert_eq!(BillingPlan::Team(cfg()).snapshot_retention_days(None), 90);
        assert_eq!(
            BillingPlan::Community(cfg()).snapshot_retention_days(None),
            90
        );
        assert_eq!(
            BillingPlan::Enterprise(cfg()).snapshot_retention_days(None),
            90
        );
        assert_eq!(BillingPlan::Demo(cfg()).snapshot_retention_days(None), 90);
        assert_eq!(
            BillingPlan::CommercialSelfHosted(cfg()).snapshot_retention_days(None),
            90
        );
    }

    #[test]
    fn env_override_wins_for_every_plan_tier() {
        let override_value = Some(365);
        assert_eq!(
            BillingPlan::Free(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Starter(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Pro(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Business(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Community(cfg()).snapshot_retention_days(override_value),
            365
        );
        assert_eq!(
            BillingPlan::Enterprise(cfg()).snapshot_retention_days(override_value),
            365
        );
    }

    #[test]
    fn override_of_zero_disables_snapshots() {
        // Universal escape hatch: an operator can set the override to 0 to
        // disable snapshots on every plan (e.g. to drain a self-hosted box).
        assert_eq!(BillingPlan::Pro(cfg()).snapshot_retention_days(Some(0)), 0);
        assert_eq!(
            BillingPlan::Business(cfg()).snapshot_retention_days(Some(0)),
            0
        );
    }
}
