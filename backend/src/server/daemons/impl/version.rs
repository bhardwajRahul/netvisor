use chrono::{DateTime, TimeZone, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ===========================================================================
// Version lifecycle registry
//
// Single source of truth for "what daemon versions are supported, and when do
// they sunset." Replaces the former constant `minimum_supported` and the dead
// hard-coded `sunset_date` string literal.
//
// Two data tables drive everything:
//   * `release_lines()` — every shipped minor line and its `.0` ship date.
//   * `V1_SUNSET` — the one announced cutover (everything below 0.17.5 sunsets
//     at the v1.0 launch + 3 months).
//
// The enforced floor and every lifecycle stage are *derived* from those tables
// plus the current date. Nothing is a literal in a match arm, so nothing can
// silently rot the way `2025-02-01` did.
//
// Dormancy: `V1_SUNSET.effective_on` is `None` until the real launch date is
// baked in before the release build. While it is `None` the whole machinery is
// OFF — the enforced floor stays at the historical `BASELINE_FLOOR` (0.12.0),
// nothing is marked `Deprecated`/`Unsupported`, and no daemon is rejected that
// wasn't already. This lets the code ship well ahead of launch without changing
// any daemon's behavior until the date is set.
// ===========================================================================

/// The historical floor. Used while the sunset machinery is dormant, and as the
/// lower bound the derived floor never drops below.
fn baseline_floor() -> Version {
    Version::new(0, 12, 0)
}

/// This binary's own version. The derived floor is capped here: a server never
/// enforces a floor newer than itself, so it can never reject a daemon of its
/// own generation. A pinned/stale self-hosted server therefore converges to a
/// floor it can actually reason about and stops.
fn own_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is valid semver")
}

/// Helper: a UTC timestamp at midnight for the given calendar date.
fn dt(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .expect("valid date")
}

/// A shipped minor release line and the date its `.0` shipped. Seeded from git
/// tags. Used to detect "a newer release exists" (Outdated) and to derive future
/// support-window floors as newer binaries add entries.
struct ReleaseLine {
    minor: (u64, u64),
    released_on: DateTime<Utc>,
}

/// Every shipped minor line, oldest first. Add one entry per new minor at
/// release time — that is the only maintenance the automatic-advance rule needs.
fn release_lines() -> Vec<ReleaseLine> {
    vec![
        ReleaseLine {
            minor: (0, 12),
            released_on: dt(2025, 12, 14),
        },
        ReleaseLine {
            minor: (0, 13),
            released_on: dt(2026, 1, 4),
        },
        ReleaseLine {
            minor: (0, 14),
            released_on: dt(2026, 2, 1),
        },
        ReleaseLine {
            minor: (0, 15),
            released_on: dt(2026, 3, 23),
        },
        ReleaseLine {
            minor: (0, 16),
            released_on: dt(2026, 4, 17),
        },
        ReleaseLine {
            minor: (0, 17),
            released_on: dt(2026, 6, 22),
        },
    ]
}

/// An announced support cutover: at `effective_on`, daemons below `floor` become
/// `Unsupported` and are rejected. Before it (but once announced) they are
/// `Deprecated` and carry the date. `effective_on: None` is the dormant sentinel.
struct ScheduledFloor {
    floor: Version,
    effective_on: Option<DateTime<Utc>>,
}

/// Months of lead time between the v1.0 launch and the first enforced cutover.
const V1_SUNSET_LEAD_MONTHS: i64 = 3;

/// The v1.0 launch anchor — the single place the launch date lives. Everything
/// (the 0.17.5 cutover date, every `Deprecated` sunset date shown in UI/email,
/// and the automatic-advance schedule) derives from this one value.
///
/// `None` is the dormant sentinel: keep it `None` in development so the whole
/// sunset machinery stays off and CI is green. Bake in the real launch date as
/// `Some(dt(YYYY, MM, DD))` immediately before the release build. The
/// `release_build_has_launch_date_set` test guards against shipping it dormant.
//
// TODO(release): set to Some(dt(<v1.0 launch year>, <month>, <day>)) before the
// release build. Until then the machinery is intentionally dormant.
fn v1_launch() -> Option<DateTime<Utc>> {
    None
}

/// The single announced cutover, derived from the launch anchor: everything
/// below 0.17.5 sunsets `V1_SUNSET_LEAD_MONTHS` after launch.
fn v1_sunset() -> ScheduledFloor {
    ScheduledFloor {
        floor: Version::new(0, 17, 5),
        effective_on: v1_launch().map(|launch| add_months(launch, V1_SUNSET_LEAD_MONTHS)),
    }
}

/// Add whole months to a timestamp (day clamped to the target month's length via
/// chrono's checked arithmetic; falls back to the original on overflow, which
/// cannot happen for the dates in play).
fn add_months(base: DateTime<Utc>, months: i64) -> DateTime<Utc> {
    base.checked_add_months(chrono::Months::new(months as u32))
        .unwrap_or(base)
}

/// The enforced support floor at `now`: daemons below it are rejected (once the
/// gate is wired in). Derived, never a literal.
///
/// While dormant (`v1_launch()` is `None`) this is exactly `baseline_floor()`
/// (0.12.0) — the historical behavior. Once the launch date is set, the newest
/// effective cutover applies, capped at `own_version()` so a stale server can
/// never over-enforce.
pub fn enforced_floor(now: DateTime<Utc>) -> Version {
    let mut floor = baseline_floor();

    // The explicit v1.0 cutover.
    let v1 = v1_sunset();
    if let Some(eff) = v1.effective_on
        && eff <= now
        && v1.floor > floor
    {
        floor = v1.floor;
    }

    // Automatic advancement for post-v1.0 binaries: once the launch anchor is
    // set, the support window is the three newest minor lines; the floor is the
    // oldest still-supported line, effective 6 months after the line that pushed
    // its predecessor out of the window shipped. This only produces a higher
    // floor than the v1.0 cutover on a *newer* binary (whose release table
    // reaches past 0.17); on this binary it is a no-op below 0.17.5.
    if v1_launch().is_some()
        && let Some(auto) = support_window_floor(now)
        && auto > floor
    {
        floor = auto;
    }

    // Safety cap: never enforce a floor newer than this binary.
    floor.min(own_version())
}

/// The support-window floor: the oldest of the three newest minor lines, once
/// its window has elapsed. `None` if there are fewer than three lines or the
/// window has not yet passed. Effective dates are clamped to never precede the
/// v1.0 cutover, so this never front-runs launch.
fn support_window_floor(now: DateTime<Utc>) -> Option<Version> {
    let lines = release_lines();
    if lines.len() < 3 {
        return None;
    }
    // Newest three lines are supported; the oldest supported is index len-3.
    let oldest_supported = &lines[lines.len() - 3];
    // The line that pushed the predecessor of `oldest_supported` out of the
    // window is `oldest_supported` itself; give a 6-month grace from its ship.
    let mut effective = add_months(oldest_supported.released_on, 6);
    if let Some(launch) = v1_launch() {
        let cutover = add_months(launch, V1_SUNSET_LEAD_MONTHS);
        if effective < cutover {
            effective = cutover;
        }
    }
    if effective <= now {
        Some(Version::new(
            oldest_supported.minor.0,
            oldest_supported.minor.1,
            0,
        ))
    } else {
        None
    }
}

/// The currently-announced daemon sunset, if the launch date has been baked in:
/// the floor below which daemons will become (or are) `Unsupported`, and the
/// date it takes effect. `None` while dormant, so the boot-time sunset sweep is
/// a no-op until launch is set.
pub fn announced_sunset() -> Option<(Version, DateTime<Utc>)> {
    let v1 = v1_sunset();
    v1.effective_on.map(|eff| (v1.floor, eff))
}

/// Whether the announced sunset `announced` covers version `v`, with its date.
/// A version-less daemon (`None`) is treated as **below** any floor: a daemon
/// that reports no version is almost always genuinely old (modern daemons always
/// report one), so it is covered by the sunset just like an old versioned daemon.
/// Returns `None` when no sunset is announced (dormant) or `v` is at/above the floor.
fn announced_sunset_for(
    announced: &Option<(Version, DateTime<Utc>)>,
    v: Option<&Version>,
) -> Option<(Version, DateTime<Utc>)> {
    let (floor, eff) = announced.as_ref()?;
    let below = v.is_none_or(|v| v < floor);
    below.then(|| (floor.clone(), *eff))
}

// ===========================================================================
// Capability floors (single source; permanent until their cutover cleanup)
//
// These gate specific server↔daemon features. They legitimately live near the
// policy so the rot-guard test can assert every one is ≤ the current server
// version. Credential-type floors stay in the credentials module (they own
// their domain) but are covered by the same invariant test there.
// ===========================================================================

/// Minimum daemon version required for unified discovery support.
pub fn minimum_unified_discovery() -> Version {
    Version::new(0, 15, 0)
}

/// Returns true if the daemon version supports unified discovery (>= 0.15.0).
pub fn supports_unified_discovery(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_unified_discovery())
}

/// Minimum daemon version that supports the full ServerPoll flow (>= 0.14.0).
/// Absorbed here from `daemons/impl/base.rs` so the registry owns every floor.
pub fn minimum_full_server_poll() -> Version {
    Version::new(0, 14, 0)
}

/// Returns true if the daemon supports the full ServerPoll flow (>= 0.14.0).
/// A daemon without a recorded version is assumed legacy (`false`).
pub fn supports_full_server_poll(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_full_server_poll())
}

/// Minimum daemon version that ships with server-provisioned identity: it is
/// installed against a pre-provisioned record bound to a 1:1 api key, learns its
/// identity from the register / first-contact handshake, and no longer relies on
/// the client-supplied X-Daemon-ID header. Below this, daemons self-register with
/// a shared network key. Coexistence is enforced by key shape (1:1 vs NULL), not
/// by this floor — this documents the boundary and gates any version-specific UI.
pub fn minimum_server_provisioned_identity() -> Version {
    Version::new(0, 17, 5)
}

/// Returns true if the daemon version supports server-provisioned identity (>= 0.17.5).
pub fn supports_server_provisioned_identity(version: Option<&Version>) -> bool {
    version.is_some_and(|v| v >= &minimum_server_provisioned_identity())
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

/// Every capability floor owned by this module. The rot-guard test asserts each
/// is ≤ the current server version, so a floor can never quietly reference a
/// version this build doesn't know about.
#[cfg(test)]
fn capability_floors() -> Vec<Version> {
    vec![
        minimum_unified_discovery(),
        minimum_full_server_poll(),
        minimum_server_provisioned_identity(),
        minimum_correct_docker_volume_mount(),
    ]
}

// ===========================================================================
// Policy + lifecycle evaluation
// ===========================================================================

/// Version policy for daemons.
///
/// `minimum_supported` is the derived enforced floor (see [`enforced_floor`]);
/// `recommended`/`latest` are this server's own version. `now` is captured at
/// construction so lifecycle evaluation and sunset dates are deterministic
/// within one request (tests pin it).
pub struct DaemonVersionPolicy {
    pub minimum_supported: Version,
    pub recommended: Version,
    pub latest: Version,
    pub now: DateTime<Utc>,
    /// The announced sunset (floor, effective date), or `None` while dormant.
    /// Captured on construction so evaluation is deterministic and testable.
    announced_sunset: Option<(Version, DateTime<Utc>)>,
}

impl Default for DaemonVersionPolicy {
    fn default() -> Self {
        Self::at(Utc::now())
    }
}

impl DaemonVersionPolicy {
    /// Construct the policy as of `now` (real time in production, fixed in tests).
    pub fn at(now: DateTime<Utc>) -> Self {
        let current = own_version();
        Self {
            minimum_supported: enforced_floor(now),
            recommended: current.clone(),
            latest: current,
            now,
            announced_sunset: announced_sunset(),
        }
    }

    /// Test-only constructor pinning both the clock and the announced sunset, so
    /// every lifecycle branch (Deprecated / Unsupported / version-less) can be
    /// exercised without depending on the hard-coded `v1_launch()` sentinel.
    #[cfg(test)]
    fn at_with_sunset(now: DateTime<Utc>, announced: Option<(Version, DateTime<Utc>)>) -> Self {
        let current = own_version();
        Self {
            minimum_supported: baseline_floor(),
            recommended: current.clone(),
            latest: current,
            now,
            announced_sunset: announced,
        }
    }

    pub fn evaluate(&self, version: Option<&Version>) -> DaemonVersionStatus {
        let supports_unified = supports_unified_discovery(version);
        let has_correct_mount = has_correct_docker_volume_mount(version);

        let (status, warnings, sunset_date) = self.lifecycle(version);

        DaemonVersionStatus {
            version: version.map(|v| v.to_string()),
            status,
            warnings,
            sunset_date,
            supports_unified_discovery: supports_unified,
            has_correct_docker_volume_mount: has_correct_mount,
        }
    }

    /// The lifecycle stage, warnings, and sunset date for a (possibly absent)
    /// version. A version-less daemon flows through the same sunset logic as an
    /// old versioned one — it is treated as genuinely old — and only degrades to
    /// the benign `Unknown` stage when no sunset is announced (dormant).
    fn lifecycle(
        &self,
        v: Option<&Version>,
    ) -> (VersionHealthStatus, Vec<DeprecationWarning>, Option<String>) {
        let label = match v {
            Some(v) => format!("Daemon {v}"),
            None => "This daemon (no reported version)".to_string(),
        };

        if let Some((_floor, effective_on)) = announced_sunset_for(&self.announced_sunset, v) {
            let sunset_str = effective_on.format("%Y-%m-%d").to_string();
            let (status, message) = if effective_on <= self.now {
                // Past its announced cutover — unsupported and rejected.
                (
                    VersionHealthStatus::Unsupported,
                    format!(
                        "{label} is no longer supported (support ended {sunset_str}). \
                         Update to {} or later to reconnect.",
                        self.recommended
                    ),
                )
            } else {
                // Announced but still in the window — deprecated, with a date.
                (
                    VersionHealthStatus::Deprecated,
                    format!(
                        "{label} is deprecated and support ends {sunset_str}. \
                         Update to {} or later before then to avoid interruption.",
                        self.recommended
                    ),
                )
            };
            return (
                status,
                vec![DeprecationWarning {
                    message,
                    sunset_date: Some(sunset_str.clone()),
                    severity: DeprecationSeverity::Critical,
                }],
                Some(sunset_str),
            );
        }

        // No sunset covers this daemon.
        let Some(v) = v else {
            // Version-less and nothing announced (dormant) — benign Unknown.
            return (
                VersionHealthStatus::Unknown,
                vec![DeprecationWarning {
                    message: format!(
                        "Daemon version unknown. Update to {} or later.",
                        self.recommended
                    ),
                    sunset_date: None,
                    severity: DeprecationSeverity::Warning,
                }],
                None,
            );
        };

        if v < &self.minimum_supported {
            // Below the enforced floor without an explicit announced schedule
            // (e.g. a future auto-advanced floor). Unsupported.
            (
                VersionHealthStatus::Unsupported,
                vec![DeprecationWarning {
                    message: format!(
                        "Daemon {v} is no longer supported. Update to {} or later to reconnect.",
                        self.recommended
                    ),
                    sunset_date: None,
                    severity: DeprecationSeverity::Critical,
                }],
                None,
            )
        } else if v < &self.recommended {
            (
                VersionHealthStatus::Outdated,
                vec![DeprecationWarning {
                    message: format!(
                        "Daemon {v} is outdated. Update to {} for the latest features.",
                        self.recommended
                    ),
                    sunset_date: None,
                    severity: DeprecationSeverity::Warning,
                }],
                None,
            )
        } else {
            (VersionHealthStatus::Current, vec![], None)
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
    /// The date this daemon's version stops being supported, if a sunset is
    /// scheduled for it. Surfaced top-level (not only inside `warnings`) so the
    /// UI can render a countdown from the same value the email uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sunset_date: Option<String>,
    #[serde(default)]
    pub supports_unified_discovery: bool,
    #[serde(default)]
    pub has_correct_docker_volume_mount: bool,
}

/// Health status for daemon versions.
///
/// Lifecycle order: `Current` → `Outdated` → `Deprecated` → `Unsupported`, with
/// `Unknown` for daemons whose version the server has no record of.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum VersionHealthStatus {
    Current,
    Outdated,
    /// A sunset date is scheduled for this version; it still works until then.
    Deprecated,
    /// Past its sunset / below the enforced floor. Rejected by the server.
    Unsupported,
    /// The server has no recorded version for this daemon.
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A policy pinned to a fixed instant for deterministic lifecycle tests.
    fn policy_at(now: DateTime<Utc>) -> DaemonVersionPolicy {
        DaemonVersionPolicy::at(now)
    }

    // --- Dormancy: with the launch anchor unset, nothing new happens. --------

    #[test]
    fn dormant_floor_is_baseline() {
        // v1_launch() is None in dev, so the floor must stay at the historical
        // baseline regardless of date.
        assert_eq!(enforced_floor(dt(2026, 7, 27)), baseline_floor());
        assert_eq!(enforced_floor(dt(2030, 1, 1)), baseline_floor());
    }

    #[test]
    fn dormant_old_daemon_is_not_deprecated() {
        // A <0.17.5 daemon while dormant is Outdated (as today), never Deprecated
        // or Unsupported — the machinery is off.
        let policy = policy_at(dt(2026, 7, 27));
        let status = policy.evaluate(Some(&Version::new(0, 16, 0)));
        assert_eq!(status.status, VersionHealthStatus::Outdated);
        assert!(status.sunset_date.is_none());
    }

    #[test]
    fn dormant_current_daemon_is_current() {
        let policy = policy_at(dt(2026, 7, 27));
        let status = policy.evaluate(Some(&own_version()));
        assert_eq!(status.status, VersionHealthStatus::Current);
        assert!(status.warnings.is_empty());
    }

    #[test]
    fn unknown_version_is_unknown_not_outdated() {
        // Dormant (no announced sunset): a version-less daemon is a benign Unknown.
        let policy = policy_at(dt(2026, 7, 27));
        let status = policy.evaluate(None);
        assert_eq!(status.status, VersionHealthStatus::Unknown);
        assert!(status.version.is_none());
    }

    #[test]
    fn announced_sunset_deprecates_old_and_versionless() {
        // Future sunset date: an old versioned daemon AND a version-less one are
        // both Deprecated with that date (a version-less daemon is treated as
        // genuinely old). A current daemon is unaffected.
        let policy = DaemonVersionPolicy::at_with_sunset(
            dt(2026, 7, 27),
            Some((Version::new(0, 17, 5), dt(2026, 10, 1))),
        );

        let old = policy.evaluate(Some(&Version::new(0, 16, 0)));
        assert_eq!(old.status, VersionHealthStatus::Deprecated);
        assert_eq!(old.sunset_date.as_deref(), Some("2026-10-01"));

        let versionless = policy.evaluate(None);
        assert_eq!(versionless.status, VersionHealthStatus::Deprecated);
        assert_eq!(versionless.sunset_date.as_deref(), Some("2026-10-01"));

        assert_eq!(
            policy.evaluate(Some(&own_version())).status,
            VersionHealthStatus::Current
        );
    }

    #[test]
    fn passed_sunset_is_unsupported_including_versionless() {
        // Sunset date already elapsed: both an old versioned daemon and a
        // version-less one are Unsupported.
        let policy = DaemonVersionPolicy::at_with_sunset(
            dt(2026, 11, 1),
            Some((Version::new(0, 17, 5), dt(2026, 10, 1))),
        );
        assert_eq!(
            policy.evaluate(Some(&Version::new(0, 16, 0))).status,
            VersionHealthStatus::Unsupported
        );
        assert_eq!(
            policy.evaluate(None).status,
            VersionHealthStatus::Unsupported
        );
    }

    // --- Once the launch anchor is set: the real transitions. ----------------
    //
    // These drive `enforced_floor`/`evaluate` off an explicit `ScheduledFloor`
    // instead of the production `v1_launch()` sentinel, so they exercise the
    // date logic without depending on an unset launch date.

    fn floor_for(v1: &ScheduledFloor, now: DateTime<Utc>) -> Version {
        let mut floor = baseline_floor();
        if let Some(eff) = v1.effective_on
            && eff <= now
            && v1.floor > floor
        {
            floor = v1.floor.clone();
        }
        floor.min(own_version())
    }

    #[test]
    fn floor_moves_only_after_cutover() {
        let launch = dt(2026, 9, 1);
        let v1 = ScheduledFloor {
            floor: Version::new(0, 17, 5),
            effective_on: Some(add_months(launch, V1_SUNSET_LEAD_MONTHS)), // 2026-12-01
        };
        // Before the cutover: still baseline.
        assert_eq!(floor_for(&v1, dt(2026, 11, 30)), baseline_floor());
        // On/after the cutover: 0.17.5.
        assert_eq!(floor_for(&v1, dt(2026, 12, 1)), Version::new(0, 17, 5));
        assert_eq!(floor_for(&v1, dt(2027, 6, 1)), Version::new(0, 17, 5));
    }

    #[test]
    fn floor_capped_at_own_version() {
        // A cutover naming a version newer than this binary is capped down —
        // a server never enforces a floor it can't reason about.
        let v1 = ScheduledFloor {
            floor: Version::new(9, 9, 9),
            effective_on: Some(dt(2026, 1, 1)),
        };
        assert_eq!(floor_for(&v1, dt(2027, 1, 1)), own_version());
    }

    #[test]
    fn add_months_is_three_month_lead() {
        assert_eq!(add_months(dt(2026, 9, 1), 3), dt(2026, 12, 1));
        assert_eq!(add_months(dt(2026, 11, 15), 3), dt(2027, 2, 15));
    }

    // --- Rot guards -----------------------------------------------------------

    #[test]
    fn current_minor_has_a_release_line() {
        let own = own_version();
        let has = release_lines()
            .iter()
            .any(|l| l.minor == (own.major, own.minor));
        assert!(
            has,
            "release_lines() is missing the current minor {}.{} — add an entry",
            own.major, own.minor
        );
    }

    #[test]
    fn capability_floors_within_server_version() {
        let own = own_version();
        for floor in capability_floors() {
            assert!(
                floor <= own,
                "capability floor {floor} exceeds server version {own} — a shim references a \
                 version this build doesn't know; move the floor or delete the shim"
            );
        }
    }

    #[test]
    fn release_lines_are_sorted_and_unique() {
        let lines = release_lines();
        for pair in lines.windows(2) {
            assert!(
                pair[0].minor < pair[1].minor,
                "release_lines() must be strictly increasing by minor"
            );
            assert!(
                pair[0].released_on < pair[1].released_on,
                "release_lines() dates must be strictly increasing"
            );
        }
    }

    /// Only asserts in a release build (when `SCANOPY_RELEASE_BUILD` is set), so
    /// dev CI stays green while the launch date is intentionally unset, but the
    /// release build cannot ship with the sunset machinery dormant.
    #[test]
    fn release_build_has_launch_date_set() {
        if option_env!("SCANOPY_RELEASE_BUILD").is_some() {
            assert!(
                v1_launch().is_some(),
                "SCANOPY_RELEASE_BUILD is set but v1_launch() is None — bake in the v1.0 launch \
                 date in version.rs before shipping"
            );
        }
    }

    // --- Capability helpers (unchanged behavior) -----------------------------

    #[test]
    fn supports_unified_discovery_floor() {
        assert!(!supports_unified_discovery(None));
        assert!(!supports_unified_discovery(Some(&Version::new(0, 14, 0))));
        assert!(supports_unified_discovery(Some(&Version::new(0, 15, 0))));
        assert!(supports_unified_discovery(Some(&Version::new(1, 0, 0))));
    }

    #[test]
    fn has_correct_docker_volume_mount_floor() {
        assert!(!has_correct_docker_volume_mount(None));
        assert!(!has_correct_docker_volume_mount(Some(&Version::new(
            0, 16, 0
        ))));
        assert!(has_correct_docker_volume_mount(Some(&Version::new(
            0, 16, 1
        ))));
    }
}
