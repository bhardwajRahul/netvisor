use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::credentials::r#impl::types::CredentialTypeDiscriminants;

/// Version policy for daemons
///
/// Defines which versions are supported, recommended, and deprecated.
/// Used to evaluate daemon version status and generate warnings.
pub struct DaemonVersionPolicy {
    pub minimum_supported: Version,
    pub recommended: Version,
    pub latest: Version,
}

impl Default for DaemonVersionPolicy {
    fn default() -> Self {
        // Use CARGO_PKG_VERSION for both recommended and latest
        // so the current release is always considered "Current"
        let current = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        Self {
            minimum_supported: Version::new(0, 12, 0),
            recommended: current.clone(),
            latest: current,
        }
    }
}

/// Minimum daemon version required for unified discovery support.
pub fn minimum_unified_discovery() -> Version {
    Version::new(0, 15, 0)
}

/// Returns true if the daemon version supports unified discovery (>= 0.15.0).
pub fn supports_unified_discovery(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_unified_discovery())
}

/// Credential-type ids that a daemon at `version` cannot safely receive (their
/// per-type `minimum_daemon_version` floor is newer than the daemon). Powers the UI
/// compatibility gate: the frontend disables these types in the discovery credential
/// picker. Iterating the discriminants keeps this in lockstep with the per-type
/// declarations — a new credential type with a higher floor is covered automatically.
/// An unknown version yields every type whose floor is above the 0.16.2 wire floor.
pub fn incompatible_credential_type_ids(version: Option<&Version>) -> Vec<String> {
    use strum::IntoEnumIterator;
    CredentialTypeDiscriminants::iter()
        .filter(|d| !d.compatible_with_daemon(version))
        .map(|d| <&'static str>::from(d).to_string())
        .collect()
}

/// Returns true if the daemon predates the Interface → IPAddress binding type rename (< 0.16.0).
/// These daemons expect `"type": "Interface"` / `"interface_id"` in binding responses.
/// Legacy cleanup: remove once minimum_supported >= 0.16.0
pub fn pre_interface_to_ip_address_rename(version: Option<&str>) -> bool {
    version
        .and_then(|v| Version::parse(v).ok())
        .is_none_or(|v| v < Version::new(0, 16, 0))
}

/// First version that ships with the corrected Docker Compose daemon-config
/// volume mount (`/root/.config/scanopy/daemon`). Releases before this shipped
/// with `/root/.config/daemon`, which silently registered a new daemon on
/// upgrade because the volume mount didn't match the daemon's actual config
/// directory.
pub fn minimum_correct_docker_volume_mount() -> Version {
    Version::new(0, 16, 1)
}

/// Returns true if the daemon version is >= the first release that shipped
/// with the corrected docker-compose.yml volume mount. Used as a proxy for
/// "this user probably has the fixed compose file" — imperfect (a user could
/// be on the latest daemon with a stale compose) but catches the common case.
pub fn has_correct_docker_volume_mount(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_correct_docker_volume_mount())
}

impl DaemonVersionPolicy {
    pub fn evaluate(&self, version: Option<&Version>) -> DaemonVersionStatus {
        match version {
            None => self.evaluate_unknown(),
            Some(v) => self.evaluate_known(v),
        }
    }

    /// During migration period: unknown = outdated
    fn evaluate_unknown(&self) -> DaemonVersionStatus {
        DaemonVersionStatus {
            version: None,
            status: VersionHealthStatus::Outdated,
            warnings: vec![DeprecationWarning {
                message: format!(
                    "Daemon version unknown. Update to {} or later.",
                    self.recommended
                ),
                sunset_date: None,
                severity: DeprecationSeverity::Warning,
            }],
            supports_unified_discovery: false,
            has_correct_docker_volume_mount: false,
            incompatible_credential_type_ids: incompatible_credential_type_ids(None),
        }
    }

    fn evaluate_known(&self, v: &Version) -> DaemonVersionStatus {
        let supports_unified = supports_unified_discovery(Some(v));
        let has_correct_mount = has_correct_docker_volume_mount(Some(v));
        let incompatible = incompatible_credential_type_ids(Some(v));
        if v < &self.minimum_supported {
            DaemonVersionStatus {
                version: Some(v.to_string()),
                status: VersionHealthStatus::Deprecated,
                warnings: vec![DeprecationWarning {
                    message: format!(
                        "Daemon {} is deprecated. Update to {} or later.",
                        v, self.recommended
                    ),
                    sunset_date: Some("2025-02-01".into()),
                    severity: DeprecationSeverity::Critical,
                }],
                supports_unified_discovery: supports_unified,
                has_correct_docker_volume_mount: has_correct_mount,
                incompatible_credential_type_ids: incompatible.clone(),
            }
        } else if v < &self.recommended {
            DaemonVersionStatus {
                version: Some(v.to_string()),
                status: VersionHealthStatus::Outdated,
                warnings: vec![DeprecationWarning {
                    message: format!(
                        "Daemon {} is outdated. Update to {} for latest features.",
                        v, self.recommended
                    ),
                    sunset_date: None,
                    severity: DeprecationSeverity::Warning,
                }],
                supports_unified_discovery: supports_unified,
                has_correct_docker_volume_mount: has_correct_mount,
                incompatible_credential_type_ids: incompatible.clone(),
            }
        } else {
            DaemonVersionStatus {
                version: Some(v.to_string()),
                status: VersionHealthStatus::Current,
                warnings: vec![],
                supports_unified_discovery: supports_unified,
                has_correct_docker_volume_mount: has_correct_mount,
                incompatible_credential_type_ids: incompatible,
            }
        }
    }
}

/// Deprecation warning for daemon version
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeprecationWarning {
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sunset_date: Option<String>,
    pub severity: DeprecationSeverity,
}

/// Severity level for deprecation warnings
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, Default, PartialEq, Eq)]
pub enum DeprecationSeverity {
    #[default]
    Info,
    Warning,
    Critical,
    /// Forward-compat: a severity a newer server emits that this daemon doesn't
    /// know. `#[serde(other)]` absorbs it (treated as an ordinary warning when
    /// logged) rather than failing to deserialize the whole `ServerCapabilities`.
    #[serde(other)]
    Unknown,
}

/// Daemon version status including health and any warnings
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonVersionStatus {
    pub version: Option<String>,
    pub status: VersionHealthStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<DeprecationWarning>,
    #[serde(default)]
    pub supports_unified_discovery: bool,
    #[serde(default)]
    pub has_correct_docker_volume_mount: bool,
    /// Credential-type ids this daemon is too old to receive. The UI disables these
    /// in the discovery credential picker (see [`incompatible_credential_type_ids`]).
    #[serde(default)]
    #[schema(required)]
    pub incompatible_credential_type_ids: Vec<String>,
}

/// Health status for daemon versions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum VersionHealthStatus {
    Current,
    Outdated,
    Deprecated,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_policy() -> DaemonVersionPolicy {
        // Use fixed versions for predictable tests
        DaemonVersionPolicy {
            minimum_supported: Version::new(0, 12, 0),
            recommended: Version::new(0, 12, 8),
            latest: Version::new(0, 12, 8),
        }
    }

    #[test]
    fn test_unknown_version_is_outdated() {
        let policy = test_policy();
        let status = policy.evaluate(None);

        assert_eq!(status.status, VersionHealthStatus::Outdated);
        assert!(status.version.is_none());
        assert_eq!(status.warnings.len(), 1);
        assert_eq!(status.warnings[0].severity, DeprecationSeverity::Warning);
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_deprecated_version() {
        let policy = test_policy();
        let old_version = Version::new(0, 11, 0);
        let status = policy.evaluate(Some(&old_version));

        assert_eq!(status.status, VersionHealthStatus::Deprecated);
        assert_eq!(status.version, Some("0.11.0".to_string()));
        assert_eq!(status.warnings.len(), 1);
        assert_eq!(status.warnings[0].severity, DeprecationSeverity::Critical);
        assert!(status.warnings[0].sunset_date.is_some());
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_outdated_version() {
        let policy = test_policy();
        let outdated_version = Version::new(0, 12, 5);
        let status = policy.evaluate(Some(&outdated_version));

        assert_eq!(status.status, VersionHealthStatus::Outdated);
        assert_eq!(status.version, Some("0.12.5".to_string()));
        assert_eq!(status.warnings.len(), 1);
        assert_eq!(status.warnings[0].severity, DeprecationSeverity::Warning);
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_current_version() {
        let policy = test_policy();
        let current_version = Version::new(0, 12, 8);
        let status = policy.evaluate(Some(&current_version));

        assert_eq!(status.status, VersionHealthStatus::Current);
        assert_eq!(status.version, Some("0.12.8".to_string()));
        assert!(status.warnings.is_empty());
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_newer_than_recommended_is_current() {
        let policy = test_policy();
        let future_version = Version::new(0, 14, 0);
        let status = policy.evaluate(Some(&future_version));

        assert_eq!(status.status, VersionHealthStatus::Current);
        assert!(status.warnings.is_empty());
        assert!(!status.supports_unified_discovery);
    }

    #[test]
    fn test_supports_unified_discovery() {
        assert!(!supports_unified_discovery(None));
        assert!(!supports_unified_discovery(Some(&Version::new(0, 14, 0))));
        assert!(supports_unified_discovery(Some(&Version::new(0, 15, 0))));
        assert!(supports_unified_discovery(Some(&Version::new(0, 16, 0))));
        assert!(supports_unified_discovery(Some(&Version::new(1, 0, 0))));
    }

    #[test]
    fn test_version_status_supports_unified_at_015() {
        let policy = test_policy();
        let v015 = Version::new(0, 15, 0);
        let status = policy.evaluate(Some(&v015));
        assert!(status.supports_unified_discovery);
    }

    #[test]
    fn test_has_correct_docker_volume_mount() {
        assert!(!has_correct_docker_volume_mount(None));
        assert!(!has_correct_docker_volume_mount(Some(&Version::new(
            0, 14, 8
        ))));
        assert!(!has_correct_docker_volume_mount(Some(&Version::new(
            0, 16, 0
        ))));
        assert!(has_correct_docker_volume_mount(Some(&Version::new(
            0, 16, 1
        ))));
        assert!(has_correct_docker_volume_mount(Some(&Version::new(
            0, 17, 0
        ))));
        assert!(has_correct_docker_volume_mount(Some(&Version::new(
            1, 0, 0
        ))));
    }

    #[test]
    fn test_incompatible_credential_type_ids_by_version() {
        // 0.16.2 daemon: SnmpV1/SnmpV3/Podman* are too new.
        let at_0_16_2 = incompatible_credential_type_ids(Some(&Version::new(0, 16, 2)));
        assert!(at_0_16_2.contains(&"SnmpV1".to_string()));
        assert!(at_0_16_2.contains(&"SnmpV3".to_string()));
        assert!(at_0_16_2.contains(&"PodmanProxy".to_string()));
        assert!(at_0_16_2.contains(&"PodmanSocket".to_string()));
        assert!(!at_0_16_2.contains(&"SnmpV2c".to_string()));

        // 0.17.2 daemon: nothing incompatible.
        assert!(incompatible_credential_type_ids(Some(&Version::new(0, 17, 2))).is_empty());

        // Unknown version: everything above the 0.16.2 floor is incompatible.
        let unknown = incompatible_credential_type_ids(None);
        assert!(unknown.contains(&"PodmanProxy".to_string()));
        assert!(!unknown.contains(&"SnmpV2c".to_string()));
    }

    #[test]
    fn test_version_status_has_correct_volume_mount_flag() {
        let policy = test_policy();
        let pre_fix = Version::new(0, 16, 0);
        let at_fix = Version::new(0, 16, 1);
        assert!(
            !policy
                .evaluate(Some(&pre_fix))
                .has_correct_docker_volume_mount
        );
        assert!(
            policy
                .evaluate(Some(&at_fix))
                .has_correct_docker_volume_mount
        );
        assert!(!policy.evaluate(None).has_correct_docker_volume_mount);
    }
}
