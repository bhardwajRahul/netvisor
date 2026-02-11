use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use strum::{Display, EnumDiscriminants, EnumIter, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::snmp_credentials::r#impl::discovery::SnmpCredentialMapping;
use crate::server::{
    daemons::r#impl::api::DiscoveryUpdatePayload,
    shared::types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    IntoStaticStr,
    EnumDiscriminants,
    EnumIter,
    ToSchema,
)]
#[serde(tag = "type")]
pub enum DiscoveryType {
    #[schema(title = "SelfReport")]
    SelfReport {
        // ID of the host that the daemon is running on
        host_id: Uuid,
    },
    #[schema(title = "Network")]
    Network {
        #[schema(required)]
        subnet_ids: Option<Vec<Uuid>>,
        #[serde(default)]
        #[schema(required)]
        host_naming_fallback: HostNamingFallback,
        /// SNMP credentials for querying devices during discovery
        /// Server builds this mapping before initiating discovery
        #[serde(default)]
        snmp_credentials: SnmpCredentialMapping,
        /// Whether to probe raw-socket ports (9100-9107) during endpoint scanning.
        /// Disabled by default to prevent ghost printing on JetDirect printers.
        #[serde(default)]
        probe_raw_socket_ports: bool,
    },
    #[schema(title = "Docker")]
    Docker {
        // ID of the host that the daemon is running on
        host_id: Uuid,
        #[serde(default)]
        #[schema(required)]
        host_naming_fallback: HostNamingFallback,
    },
}

impl Default for DiscoveryType {
    fn default() -> Self {
        Self::SelfReport {
            host_id: Uuid::nil(),
        }
    }
}

impl DiscoveryType {
    /// Create a sanitized copy with sensitive data (SNMP credentials) redacted.
    /// Used when storing EntitySource to prevent credential leakage in API responses.
    pub fn sanitized(&self) -> Self {
        match self {
            DiscoveryType::Network {
                subnet_ids,
                host_naming_fallback,
                snmp_credentials,
                probe_raw_socket_ports,
            } => DiscoveryType::Network {
                subnet_ids: subnet_ids.clone(),
                host_naming_fallback: *host_naming_fallback,
                snmp_credentials: snmp_credentials.sanitized(),
                probe_raw_socket_ports: *probe_raw_socket_ports,
            },
            other => other.clone(),
        }
    }
}

impl Display for DiscoveryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryType::SelfReport { .. } => write!(f, "Self Report"),
            DiscoveryType::Network { .. } => write!(f, "Network Discovery"),
            DiscoveryType::Docker { .. } => write!(f, "Docker Discovery"),
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Copy, Deserialize, Eq, PartialEq, Hash, Display, Default, ToSchema,
)]
pub enum HostNamingFallback {
    Ip,
    #[default]
    BestService,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "type")]
pub enum RunType {
    #[schema(title = "Scheduled")]
    Scheduled {
        cron_schedule: String,
        #[serde(default)]
        #[schema(read_only)]
        last_run: Option<DateTime<Utc>>,
        enabled: bool,
    },
    #[schema(title = "Historical")]
    /// Historical discovery runs are created by the server and cannot be submitted via API
    Historical { results: DiscoveryUpdatePayload },
    #[schema(title = "AdHoc")]
    AdHoc {
        #[serde(default)]
        #[schema(read_only)]
        last_run: Option<DateTime<Utc>>,
    },
}

impl Default for RunType {
    fn default() -> Self {
        Self::AdHoc { last_run: None }
    }
}

impl HasId for DiscoveryType {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for DiscoveryType {
    fn color(&self) -> Color {
        EntityDiscriminants::Discovery.color()
    }

    fn icon(&self) -> Icon {
        EntityDiscriminants::Discovery.icon()
    }
}

impl TypeMetadataProvider for DiscoveryType {
    fn name(&self) -> &'static str {
        self.id()
    }
    fn description(&self) -> &'static str {
        match self {
            DiscoveryType::Docker { .. } => {
                "Discover Docker containers and their configurations on the daemon's host"
            }
            DiscoveryType::Network { .. } => {
                "Scan network subnets to discover hosts, open ports, and running services"
            }
            DiscoveryType::SelfReport { .. } => {
                "The daemon reports its own host configuration and network details"
            }
        }
    }
}
