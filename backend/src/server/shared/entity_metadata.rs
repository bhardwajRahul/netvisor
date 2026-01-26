//! Entity metadata for documentation and code generation.
//!
//! This module provides a single source of truth for entity descriptions and categories,
//! used by both OpenAPI documentation and website docs generation.

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, IntoStaticStr};

/// Categories for grouping entities in documentation.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumIter,
    IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EntityCategory {
    OrganizationsAndUsers,
    NetworkInfrastructure,
    DiscoveryAndDaemons,
    Visualization,
    Metadata,
}

impl EntityCategory {
    /// Human-readable display name for the category.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::OrganizationsAndUsers => "Organizations & Users",
            Self::NetworkInfrastructure => "Network Infrastructure",
            Self::DiscoveryAndDaemons => "Discovery & Daemons",
            Self::Visualization => "Visualization",
            Self::Metadata => "Metadata",
        }
    }
}
