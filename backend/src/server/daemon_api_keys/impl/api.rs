use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::daemon_api_keys::r#impl::base::DaemonApiKey;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DaemonApiKeyResponse {
    /// The stored key record.
    pub api_key: DaemonApiKey,
    /// The plaintext API key - only returned once during creation or rotation.
    #[schema(format = "password", read_only)]
    pub key: String,
}
