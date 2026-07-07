use std::sync::Arc;
use tokio::sync::RwLock;

use super::key::LicenseKey;
use super::types::LicenseStatus;

pub struct LicenseService {
    status: Arc<RwLock<LicenseStatus>>,
    license_key_raw: Option<LicenseKey>,
}

impl LicenseService {
    /// Create a new license service.
    ///
    /// A deployment is "commercial" iff a license key is configured. With no
    /// key (`SCANOPY_LICENSE_KEY` unset) the deployment is the free community
    /// edition and licensing is `NotRequired`; with a key present the key is
    /// validated and drives `Valid`/`Expired`/`Invalid`. This is the single
    /// runtime signal — there is no separate commercial build.
    ///
    /// - `license_key`: the effective key for this deployment (`None` on cloud
    ///   or community, per `ServerConfig::effective_license_key`).
    pub fn new(license_key: Option<LicenseKey>) -> Self {
        let status = match &license_key {
            None => LicenseStatus::NotRequired,
            Some(key) => key.validate(),
        };

        Self {
            status: Arc::new(RwLock::new(status)),
            license_key_raw: license_key,
        }
    }

    /// Get the current license status.
    pub async fn current_status(&self) -> LicenseStatus {
        self.status.read().await.clone()
    }

    /// Re-validate the license key. Called by the periodic background task
    /// to catch time-based expiry transitions without requiring a restart.
    pub async fn revalidate(&self) {
        if let Some(key) = &self.license_key_raw {
            let new_status = key.validate();
            let mut status = self.status.write().await;

            let was_locked = status.is_locked();
            let now_locked = new_status.is_locked();

            if was_locked != now_locked {
                if now_locked {
                    tracing::warn!(
                        target: "server",
                        "License status changed to locked: {}",
                        new_status.as_api_string().unwrap_or("unknown")
                    );
                } else {
                    tracing::info!(
                        target: "server",
                        "License status changed to valid"
                    );
                }
            }

            *status = new_status;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::LicenseClaims;
    use super::*;

    fn claims(iat: i64, exp: i64, intended_exp: i64) -> LicenseClaims {
        LicenseClaims {
            sub: "scanopy-license".to_string(),
            iss: "scanopy".to_string(),
            iat,
            exp,
            intended_exp,
            org_id: None,
            plan: None,
        }
    }

    #[test]
    fn no_key_is_not_required() {
        // No license key configured => free community edition, not locked.
        let service = LicenseService::new(None);
        let status = service.status.blocking_read();
        assert!(!status.is_locked());
        assert_eq!(status.as_api_string(), None);
    }

    #[test]
    fn garbage_key_is_invalid() {
        // A key is present but does not verify => commercial deployment, locked.
        let service = LicenseService::new(Some(LicenseKey::new("not-a-jwt".to_string())));
        let status = service.status.blocking_read();
        assert!(status.is_locked());
        assert_eq!(status.as_api_string(), Some("invalid"));
    }

    #[test]
    fn license_claims_json_roundtrip_preserves_intended_exp() {
        let original = claims(1_700_000_000, 1_800_000_000, 1_799_395_200);
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: LicenseClaims = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.exp, original.exp);
        assert_eq!(decoded.intended_exp, 1_799_395_200);
    }

    #[test]
    fn license_claims_rejects_missing_intended_exp() {
        let json_missing_intended_exp = r#"{
            "sub": "scanopy-license",
            "iss": "scanopy",
            "iat": 1700000000,
            "exp": 1800000000
        }"#;
        assert!(serde_json::from_str::<LicenseClaims>(json_missing_intended_exp).is_err());
    }

    #[test]
    fn in_grace_period_true_between_intended_and_hard() {
        let now = 1_700_000_000;
        let status = LicenseStatus::Valid(claims(
            now - 86_400 * 30,
            now + 86_400 * 6, // hard exp: 6 days from now
            now - 86_400,     // intended exp: 1 day ago
        ));
        assert!(status.in_grace_period_at(now));
    }

    #[test]
    fn in_grace_period_false_before_intended_expiry() {
        let now = 1_700_000_000;
        let status =
            LicenseStatus::Valid(claims(now - 86_400, now + 86_400 * 372, now + 86_400 * 365));
        assert!(!status.in_grace_period_at(now));
    }

    #[test]
    fn in_grace_period_false_after_hard_expiry() {
        let now = 1_700_000_000;
        // Even if status happens to be Valid at construction time, the
        // grace window ends at `exp`.
        let status = LicenseStatus::Valid(claims(
            now - 86_400 * 400,
            now - 86_400,     // hard exp: 1 day ago
            now - 86_400 * 8, // intended exp: 8 days ago
        ));
        assert!(!status.in_grace_period_at(now));
    }

    #[test]
    fn in_grace_period_false_when_expired_variant() {
        let now = 1_700_000_000;
        let status =
            LicenseStatus::Expired(claims(now - 86_400 * 400, now - 86_400, now - 86_400 * 8));
        assert!(!status.in_grace_period_at(now));
    }

    #[test]
    fn intended_expiry_date_uses_intended_exp_not_hard_exp() {
        let now = 1_700_000_000;
        let status =
            LicenseStatus::Valid(claims(now - 86_400, now + 86_400 * 372, now + 86_400 * 365));
        assert_ne!(status.intended_expiry_date(), status.expiry_date());
    }
}
