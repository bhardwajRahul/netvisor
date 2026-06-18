use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct EmailSettings {
    pub discovery_digest: bool,
}

impl Default for EmailSettings {
    fn default() -> Self {
        Self {
            discovery_digest: true,
        }
    }
}
