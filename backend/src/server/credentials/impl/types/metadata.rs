use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::{
    services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt},
    shared::{
        concepts::Concept,
        types::{
            Color, Icon,
            metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
        },
    },
};

use super::{CredentialType, CredentialTypeDiscriminants, SecretValue, default_docker_port};

/// Category grouping for credential types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, IntoStaticStr, ToSchema, PartialEq, Eq)]
pub enum CredentialCategory {
    /// Network monitoring protocols (SNMP, NetFlow, sFlow)
    #[strum(serialize = "Network Monitoring")]
    NetworkMonitoring,
    /// Container and virtualization platforms (Docker, vSphere, ESXi)
    #[strum(serialize = "Container & Virtualization")]
    ContainerVirtualization,
}

/// How a credential is scoped to targets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub enum ScopeModel {
    /// Network default — try on all hosts with matching open ports
    Broadcast,
    /// Assigned to specific hosts only
    PerHost,
}

/// Where a credential / integration applies. Supersedes `ScopeModel`:
/// `Network` ⇔ Broadcast, `Host` ⇔ PerHost, plus `DaemonHost` for the daemon's
/// own host (e.g. the local Docker socket, realized as a 127.0.0.1 IP-override).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Hash)]
pub enum Target {
    /// The daemon's own host (local). Daemon-relative.
    DaemonHost,
    /// Specific discovered host(s), optionally limited to specific IP addresses.
    Host,
    /// All hosts on a network (broadcast default).
    Network,
}

/// A credential assigned to a host, optionally limited to specific ip_addresses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct CredentialAssignment {
    pub credential_id: Uuid,
    /// Interface IDs to limit this credential to. None = all host ip_addresses.
    #[serde(default, alias = "interface_ids")]
    #[schema(required)]
    pub ip_address_ids: Option<Vec<Uuid>>,
}

/// Host-keyed mirror of [`CredentialAssignment`]: a host this credential is
/// assigned to, optionally limited to specific ip_addresses. Hydrated onto a
/// credential from the `host_credentials` junction (PerHost scope).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct CredentialHostAssignment {
    pub host_id: Uuid,
    /// IP address IDs to limit this credential to on the host. None = all host ip_addresses.
    #[serde(default)]
    #[schema(required)]
    pub ip_address_ids: Option<Vec<Uuid>>,
}

impl CredentialTypeDiscriminants {
    /// Create a `CredentialType` instance with default field values for this variant.
    /// Used by `generate-fixtures` and anywhere variant iteration is needed.
    pub fn to_credential_type(&self) -> CredentialType {
        match self {
            Self::SnmpV1 => CredentialType::SnmpV1 {
                community: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            Self::SnmpV2c => CredentialType::SnmpV2c {
                community: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
            },
            Self::SnmpV3 => CredentialType::SnmpV3 {
                security_name: String::new(),
                auth_protocol: super::SnmpV3AuthProtocol::default(),
                auth_password: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
                priv_protocol: super::SnmpV3PrivProtocol::default(),
                priv_password: SecretValue::Inline {
                    value: SecretString::from(String::new()),
                },
                context_name: None,
            },
            Self::DockerProxy => CredentialType::DockerProxy {
                port: default_docker_port(),
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
            Self::DockerSocket => CredentialType::DockerSocket {},
        }
    }
}

impl HasId for CredentialTypeDiscriminants {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for CredentialTypeDiscriminants {
    fn color(&self) -> Color {
        // Derive color from associated service's category
        let service = self.to_credential_type().associated_service();
        ServiceDefinition::category(&*service).color()
    }
    fn icon(&self) -> Icon {
        // Fallback icon when the service logo is unavailable
        match self {
            Self::SnmpV1 | Self::SnmpV2c | Self::SnmpV3 => Concept::SNMP.icon(),
            Self::DockerProxy => Concept::Containerization.icon(),
            Self::DockerSocket => Concept::Containerization.icon(),
        }
    }
}

impl TypeMetadataProvider for CredentialTypeDiscriminants {
    fn name(&self) -> &'static str {
        match self {
            Self::SnmpV1 => "SNMP v1",
            Self::SnmpV2c => "SNMP v2c",
            Self::SnmpV3 => "SNMP v3",
            Self::DockerProxy => "Docker Proxy",
            Self::DockerSocket => "Docker Socket",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::SnmpV1 => "SNMPv1 community string for legacy devices that only speak v1",
            Self::SnmpV2c => "SNMPv2c community string for querying network devices",
            Self::SnmpV3 => {
                "SNMPv3 with authentication and privacy (AuthPriv) for hardened devices"
            }
            Self::DockerProxy => {
                "Reach the Docker API over the network — for Docker on another host, or exposed via a TLS proxy. Docker on the daemon's own host is scanned automatically."
            }
            Self::DockerSocket => {
                "Local Docker socket access. Auto-managed from daemon capabilities."
            }
        }
    }

    fn category(&self) -> &'static str {
        self.to_credential_type().credential_category().into()
    }

    fn metadata(&self) -> serde_json::Value {
        let ct = self.to_credential_type();
        let service = ct.associated_service();
        let url = service.logo_url();
        let logo_ext = if url.is_empty() || url.starts_with('/') {
            ""
        } else {
            url.rsplit('.')
                .next()
                .and_then(|e| e.split('?').next())
                .filter(|e| matches!(*e, "svg" | "png" | "webp"))
                .unwrap_or("svg")
        };
        serde_json::json!({
            "fields": ct.field_definitions(),
            "scope_models": ct.scope_models(),
            "targets": ct.targets(),
            "requires_config": ct.requires_config(),
            "is_local_auto": ct.is_local_auto(),
            "single_endpoint_per_host": ct.single_endpoint_per_host(),
            "associated_service": ServiceDefinition::name(&*service),
            "has_logo": service.has_logo(),
            "logo_ext": logo_ext,
            "logo_needs_white_background": service.logo_needs_white_background(),
            "is_user_selectable": ct.is_user_selectable(),
        })
    }
}
