use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::email::messages::{EmailPreference, PausableCategory};

/// Per-user toggles for the user-pausable email categories. Each field maps
/// 1:1 to a [`PausableCategory`]; required emails are never gated here.
///
/// Stored as a JSONB blob, so new categories are added as new fields rather
/// than via migration. New fields carry `#[serde(default = "default_true")]`
/// so a category is opted in by default if its key is absent from the stored
/// JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct EmailSettings {
    /// Send a periodic summary of what discovery found.
    pub discovery_digest: bool,
    /// Send getting-started guidance.
    #[serde(default = "default_true")]
    pub product_onboarding: bool,
    /// Send an alert when a daemon stops reporting.
    #[serde(default = "default_true")]
    pub daemon_alerts: bool,
    /// Send trial reminders and plan-usage warnings.
    #[serde(default = "default_true")]
    pub trial_and_usage: bool,
}

fn default_true() -> bool {
    true
}

impl Default for EmailSettings {
    fn default() -> Self {
        Self {
            discovery_digest: true,
            product_onboarding: true,
            daemon_alerts: true,
            trial_and_usage: true,
        }
    }
}

impl EmailSettings {
    /// Whether an email with the given preference may be delivered to this
    /// user. Required emails always send; pausable ones project onto the
    /// matching flag. The exhaustive match means a new [`PausableCategory`]
    /// will not compile until it is wired to a field here.
    pub fn allows(&self, preference: EmailPreference) -> bool {
        match preference {
            EmailPreference::Required => true,
            EmailPreference::Pausable(category) => match category {
                PausableCategory::DiscoveryDigest => self.discovery_digest,
                PausableCategory::ProductOnboarding => self.product_onboarding,
                PausableCategory::DaemonAlerts => self.daemon_alerts,
                PausableCategory::TrialAndUsage => self.trial_and_usage,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_always_allowed_even_when_everything_off() {
        let settings = EmailSettings {
            discovery_digest: false,
            product_onboarding: false,
            daemon_alerts: false,
            trial_and_usage: false,
        };
        assert!(settings.allows(EmailPreference::Required));
    }

    #[test]
    fn pausable_follows_its_flag() {
        let settings = EmailSettings {
            discovery_digest: true,
            product_onboarding: false,
            daemon_alerts: true,
            trial_and_usage: false,
        };
        assert!(settings.allows(EmailPreference::Pausable(PausableCategory::DiscoveryDigest)));
        assert!(!settings.allows(EmailPreference::Pausable(
            PausableCategory::ProductOnboarding
        )));
        assert!(settings.allows(EmailPreference::Pausable(PausableCategory::DaemonAlerts)));
        assert!(!settings.allows(EmailPreference::Pausable(PausableCategory::TrialAndUsage)));
    }
}
