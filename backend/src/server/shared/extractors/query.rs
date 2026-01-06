//! Custom Query extractor that uses serde_qs for parsing nested query parameters.
//!
//! This extractor supports bracket notation for nested objects like `pagination[limit]=2`
//! which is what the OpenAPI spec generates and the frontend sends.

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use std::fmt;
use std::ops::Deref;

/// Query extractor that uses serde_qs to support nested query parameters.
///
/// Unlike axum's default Query extractor which uses serde_urlencoded,
/// this extractor can parse nested objects using bracket notation:
/// `?pagination[limit]=10&pagination[offset]=0`
#[derive(Debug, Clone, Copy, Default)]
pub struct Query<T>(pub T);

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Rejection type for Query extraction failures.
#[derive(Debug)]
pub struct QueryRejection {
    message: String,
}

impl fmt::Display for QueryRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to deserialize query string: {}", self.message)
    }
}

impl IntoResponse for QueryRejection {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

impl<S, T> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = QueryRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or("");
        let config = serde_qs::Config::new(5, false);
        let value = config.deserialize_str(query).map_err(|e| QueryRejection {
            message: e.to_string(),
        })?;
        Ok(Query(value))
    }
}
