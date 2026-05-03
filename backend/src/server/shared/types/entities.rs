use crate::server::services::r#impl::patterns::MatchDetails;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumDiscriminants, VariantNames};
use utoipa::ToSchema;

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
    #[schema(title = "Unknown")]
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
