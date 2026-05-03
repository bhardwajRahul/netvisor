//! Snapshot retention windows by plan tier.
//!
//! The retention window is **not** stored on `PlanConfig` (DB-persisted; would
//! prevent env-var overrides) — it's a static lookup over `&BillingPlan` plus
//! an optional override read from `ServerConfig` at startup. SaaS-tier plans
//! (Free / Starter / Pro / Business / Team) have fixed values; self-hosted /
//! community / enterprise / demo deployments respect the env-var override
//! (`SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE`) and default to 90 days.
//!
//! Returning `0` disables snapshots entirely for that plan (Free).

use crate::server::billing::types::base::BillingPlan;

/// Resolve the retention window in days for a plan. `0` disables snapshots.
pub fn snapshot_retention_days(plan: &BillingPlan, env_override: Option<u32>) -> u32 {
    match plan {
        BillingPlan::Free(_) => 0,
        BillingPlan::Starter(_) => 7,
        BillingPlan::Pro(_) => 30,
        BillingPlan::Business(_) | BillingPlan::Team(_) => 90,
        BillingPlan::Community(_)
        | BillingPlan::CommercialSelfHosted(_)
        | BillingPlan::Enterprise(_)
        | BillingPlan::Demo(_) => env_override.unwrap_or(90),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::types::base::PlanConfig;

    fn cfg() -> PlanConfig {
        PlanConfig::default()
    }

    #[test]
    fn default_per_tier_no_override() {
        assert_eq!(snapshot_retention_days(&BillingPlan::Free(cfg()), None), 0);
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Starter(cfg()), None),
            7
        );
        assert_eq!(snapshot_retention_days(&BillingPlan::Pro(cfg()), None), 30);
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Business(cfg()), None),
            90
        );
        assert_eq!(snapshot_retention_days(&BillingPlan::Team(cfg()), None), 90);
    }

    #[test]
    fn env_override_applies_to_self_hosted_plans() {
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Community(cfg()), Some(42)),
            42
        );
        assert_eq!(
            snapshot_retention_days(&BillingPlan::CommercialSelfHosted(cfg()), Some(42)),
            42
        );
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Enterprise(cfg()), Some(180)),
            180
        );
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Demo(cfg()), Some(7)),
            7
        );
    }

    #[test]
    fn env_override_ignored_for_saas_tiers() {
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Free(cfg()), Some(365)),
            0
        );
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Starter(cfg()), Some(365)),
            7
        );
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Pro(cfg()), Some(365)),
            30
        );
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Business(cfg()), Some(365)),
            90
        );
    }

    #[test]
    fn self_hosted_default_is_90_when_unset() {
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Community(cfg()), None),
            90
        );
        assert_eq!(
            snapshot_retention_days(&BillingPlan::Enterprise(cfg()), None),
            90
        );
    }
}
