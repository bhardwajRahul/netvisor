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

/// Where a credential / integration applies. `Network` is a broadcast default
/// (all hosts on a network), `Host` targets specific hosts, and `DaemonHost` is
/// the daemon's own host (e.g. the local Docker socket, realized as a 127.0.0.1
/// IP-override).
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
            Self::PodmanProxy => CredentialType::PodmanProxy {
                port: default_docker_port(),
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
            Self::PodmanSocket => CredentialType::PodmanSocket {},
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
            Self::DockerProxy | Self::DockerSocket | Self::PodmanProxy | Self::PodmanSocket => {
                Concept::Containerization.icon()
            }
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
            Self::PodmanProxy => "Podman Proxy",
            Self::PodmanSocket => "Podman Socket",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::SnmpV1 => {
                "Discover a host's interfaces, system details, and LLDP neighbors using SNMPv1 (legacy, unencrypted)."
            }
            Self::SnmpV2c => {
                "Discover a host's interfaces, system details, and LLDP neighbors using SNMPv2c (unencrypted community string)."
            }
            Self::SnmpV3 => {
                "Discover a host's interfaces, system details, and LLDP neighbors using SNMPv3 (authenticated and encrypted)."
            }
            Self::DockerProxy => {
                "Discover Docker containers and the services they expose over TCP, optionally with TLS."
            }
            Self::DockerSocket => {
                "Discover Docker containers and the services they expose via the daemon's local socket."
            }
            Self::PodmanProxy => {
                "Discover Podman containers and the services they expose over TCP, optionally with TLS."
            }
            Self::PodmanSocket => {
                "Discover Podman containers and the services they expose via the daemon's local socket."
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
