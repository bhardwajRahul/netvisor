use crate::server::{
    billing::types::features::Feature,
    shared::types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use stripe_product::price::CreatePriceRecurringInterval;
use strum::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;

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
            BillingPlan::CommercialSelfHosted(PlanConfig {
                base_cents: 0,
                rate: BillingRate::Month,
                trial_days: 0,
                seat_cents: None,
                network_cents: None,
                host_cents: None,
                included_networks: None,
                included_seats: None,
                included_hosts: None,
            })
        }
        #[cfg(not(feature = "commercial"))]
        {
            BillingPlan::Community(PlanConfig {
                base_cents: 0,
                rate: BillingRate::Month,
                trial_days: 0,
                seat_cents: None,
                network_cents: None,
                host_cents: None,
                included_networks: None,
                included_seats: None,
                included_hosts: None,
            })
        }
    }
}

impl BillingPlan {
    pub fn to_yearly(&self, discount: f32) -> Self {
        let mut yearly_config = self.config();
        yearly_config.rate = BillingRate::Year;

        // Round to nearest dollar (100 cents)
        yearly_config.base_cents =
            Self::round_to_dollar(yearly_config.base_cents as f32 * 12.0 * (1.0 - discount));
        yearly_config.seat_cents = yearly_config
            .seat_cents
            .map(|c| Self::round_to_dollar(c as f32 * 12.0 * (1.0 - discount)));
        yearly_config.network_cents = yearly_config
            .network_cents
            .map(|c| Self::round_to_dollar(c as f32 * 12.0 * (1.0 - discount)));
        yearly_config.host_cents = yearly_config
            .host_cents
            .map(|c| Self::round_to_dollar(c as f32 * 12.0 * (1.0 - discount)));

        let mut yearly_plan = *self;
        yearly_plan.set_config(yearly_config);
        yearly_plan
    }
    fn round_to_dollar(cents: f32) -> i64 {
        ((cents / 100.0).round() * 100.0) as i64
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
    pub community_support: bool,
    pub priority_support: bool,
    // Core features
    pub scheduled_discovery: bool,
    pub daemon_poll: bool,
    pub service_definitions: bool,
    pub docker_integration: bool,
    pub real_time_updates: bool,
    pub snmp_integration: bool,
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
            BillingPlan::Team(_)
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

    pub fn host_limit(&self) -> Option<u64> {
        self.config().included_hosts
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
                community_support: true,
                priority_support: false,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                real_time_updates: true,
                snmp_integration: true,
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
                community_support: true,
                priority_support: false,
                scheduled_discovery: false,
                daemon_poll: false,
                service_definitions: true,
                docker_integration: true,
                real_time_updates: true,
                snmp_integration: true,
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
                community_support: true,
                priority_support: false,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                real_time_updates: true,
                snmp_integration: true,
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
                community_support: true,
                priority_support: false,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                real_time_updates: true,
                snmp_integration: true,
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
                community_support: true,
                priority_support: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                real_time_updates: true,
                snmp_integration: true,
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
                community_support: true,
                priority_support: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                real_time_updates: true,
                snmp_integration: true,
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
                community_support: true,
                priority_support: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                real_time_updates: true,
                snmp_integration: true,
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
                community_support: true,
                priority_support: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                real_time_updates: true,
                snmp_integration: true,
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
                community_support: true,
                priority_support: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                real_time_updates: true,
                snmp_integration: true,
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
            community_support,
            scheduled_discovery,
            daemon_poll,
            service_definitions,
            docker_integration,
            real_time_updates,
            snmp_integration,
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

        if community_support {
            features.push(Feature::CommunitySupport)
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

        if scheduled_discovery {
            features.push(Feature::ScheduledDiscovery)
        }

        if daemon_poll {
            features.push(Feature::DaemonPoll)
        }

        if service_definitions {
            features.push(Feature::ServiceDefinitions)
        }

        if docker_integration {
            features.push(Feature::DockerIntegration)
        }

        if real_time_updates {
            features.push(Feature::RealTimeUpdates)
        }

        if snmp_integration {
            features.push(Feature::SnmpIntegration)
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
            BillingPlan::Free { .. } => {
                "Get started with Scanopy — manual discovery for up to 25 hosts"
            }
            BillingPlan::Starter { .. } => {
                "Automatically create living documentation of your network"
            }
            BillingPlan::Pro { .. } => "Visualize multiple networks and share network diagrams",
            BillingPlan::Team { .. } => {
                "Collaborate on infrastructure documentation with your team"
            }
            BillingPlan::Business { .. } => {
                "Manage multi-site and multi-customer documentation with advanced features"
            }
            BillingPlan::Enterprise { .. } => {
                "Fully managed Scanopy with dedicated support and custom deployment"
            }
            BillingPlan::Demo { .. } => "Demo mode",
            BillingPlan::CommercialSelfHosted { .. } => {
                "Commercial license for self-managed deployments — full control over configuration and integrations"
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let config = self.config();

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
            "custom_price": self.custom_price()
        })
    }
}
