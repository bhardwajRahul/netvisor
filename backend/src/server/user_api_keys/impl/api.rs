use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::user_api_keys::r#impl::base::UserApiKey;

/// Response for user API key creation/rotation
/// Contains the full API key record plus the plaintext key (shown only once)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserApiKeyResponse {
    /// The stored key record.
    pub api_key: UserApiKey,
    /// The plaintext API key - only returned once during creation or rotation
    #[schema(format = "password", read_only)]
    pub key: String,
}
