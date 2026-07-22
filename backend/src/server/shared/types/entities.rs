use crate::server::services::r#impl::patterns::MatchDetails;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumDiscriminants, VariantNames};
use utoipa::ToSchema;

/// How recently discovery last observed an entity.
///
/// Derived, never persisted — computed from `last_seen_at` against the
/// entity's network staleness window (`Network::stale_cutoff`). Shared by the
/// discovery digest email and the UI so a host reported stale in the digest is
/// the same host badged stale in the inventory and topology; running two
/// different measures let them disagree (a scan-count measure calls an entity
/// missing after 3 scans, which is 45 minutes on one network and 3 months on
/// another).
///
/// Only discovery-managed entities can be `Stale` — see
/// [`DiscoveryTracked::is_discovery_managed`](crate::server::shared::storage::snapshot::DiscoveryTracked::is_discovery_managed).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityFreshness {
    /// First observed during the scan window being reported on. Only the
    /// digest distinguishes this; the inventory surfaces `created_at` directly.
    New,
    /// Observed within the network's staleness window, or not discovery-managed.
    #[default]
    Current,
    /// Discovery-managed and not observed within the network's staleness
    /// window. Asserts only "not seen recently" — never "removed".
    Stale,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Default,
    Eq,
    PartialEq,
    Hash,
    EnumDiscriminants,
    VariantNames,
    ToSchema,
)]
#[strum_discriminants(derive(Hash))]
#[serde(tag = "type")]
pub enum EntitySource {
    #[schema(title = "Manual")]
    Manual,
    #[default]
    #[schema(title = "System")]
    System,
    #[schema(title = "Discovery")]
    Discovery,
    #[schema(title = "DiscoveryWithMatch")]
    DiscoveryWithMatch { details: MatchDetails },
    // `other` makes an EntitySource variant a newer server adds degrade to
    // `Unknown` on an older daemon instead of failing the whole response.
    // EntitySource is embedded as `.source` on nearly every entity, so this is
    // the highest-blast-radius forward-compat guard.
    #[schema(title = "Unknown")]
    #[serde(other)]
    Unknown,
}

impl EntitySource {
    /// Returns true if this entity was created via discovery (network, Docker, etc.)
    pub fn is_from_discovery(&self) -> bool {
        matches!(
            self,
            EntitySource::Discovery | EntitySource::DiscoveryWithMatch { .. }
        )
    }
}

// DiscoveryMetadata removed — its only consumer was a list of "last 5
// discoveries" that has been superseded by the typed
// last_discovery_id / first_discovery_id / last_seen_at columns on
// DiscoveryTracked entities. The metadata-strip migration (20260502000007)
// removes the field from existing source JSONB on disk.

#[cfg(test)]
mod forward_compat_tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn unknown_variant_degrades_to_unknown() {
        // A source kind a newer server adds degrades to `Unknown` on an older
        // daemon instead of failing the entire response.
        let parsed: EntitySource =
            serde_json::from_value(serde_json::json!({ "type": "FutureSource" })).unwrap();
        assert_eq!(parsed, EntitySource::Unknown);
    }

    #[test]
    fn tolerates_legacy_metadata_field() {
        // Reverse-direction skew: a NEW daemon reading an OLD server's payload
        // that still carries the removed `metadata` field — extra field ignored.
        let parsed: EntitySource =
            serde_json::from_value(serde_json::json!({ "type": "Discovery", "metadata": [] }))
                .unwrap();
        assert_eq!(parsed, EntitySource::Discovery);
    }

    #[test]
    fn known_variants_round_trip() {
        for variant in [
            EntitySource::Manual,
            EntitySource::System,
            EntitySource::Discovery,
            EntitySource::Unknown,
        ] {
            let json = serde_json::to_value(&variant).unwrap();
            let back: EntitySource = serde_json::from_value(json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn historical_required_metadata_failure_is_reproduced() {
        // Documents the original production failure: the pre-`ed235fa28`
        // EntitySource required `metadata` on `Discovery`, so the new server's
        // `{"type":"Discovery"}` could not be deserialized by an old daemon.
        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum OldEntitySource {
            #[allow(dead_code)]
            Discovery { metadata: Vec<serde_json::Value> },
        }
        let result: Result<OldEntitySource, _> =
            serde_json::from_value(serde_json::json!({ "type": "Discovery" }));
        let Err(err) = result else {
            panic!("expected old EntitySource to require `metadata`");
        };
        let err = err.to_string();
        assert!(err.contains("missing field `metadata`"), "got: {err}");
    }
}
