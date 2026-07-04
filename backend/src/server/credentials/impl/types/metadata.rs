use std::borrow::Cow;

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
            metadata::{EntityMetadataProvider, HasId, MetadataProvider, TypeMetadata},
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
            Self::DockerSocket => CredentialType::DockerSocket { socket_path: None },
            Self::PodmanProxy => CredentialType::PodmanProxy {
                port: default_docker_port(),
                path: None,
                ssl_cert: None,
                ssl_key: None,
                ssl_chain: None,
            },
            Self::PodmanSocket => CredentialType::PodmanSocket { socket_path: None },
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

impl CredentialTypeDiscriminants {
    /// Display name for this credential transport (e.g. "Docker Socket").
    pub(crate) fn display_name(&self) -> &'static str {
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

    /// Canonical "what's discovered" for the integration this credential targets.
    /// One arm per associated service, shared by all of that service's transports,
    /// so the text has a single source of truth. The per-transport credential
    /// description ([`full_description`](Self::full_description)) and the
    /// `integrations` fixture both derive from this. Exhaustive (no wildcard): a
    /// new credential variant cannot compile until it declares its integration's
    /// discovery text.
    pub(crate) fn integration_discovers(&self) -> &'static str {
        match self {
            Self::SnmpV1 | Self::SnmpV2c | Self::SnmpV3 => {
                "Discover a host's interfaces, system details, and CDP/LLDP neighbors."
            }
            Self::DockerProxy | Self::DockerSocket => {
                "Discover Docker containers and the services they expose."
            }
            Self::PodmanProxy | Self::PodmanSocket => {
                "Discover Podman containers and the services they expose."
            }
        }
    }

    /// Transport-specific note appended after the canonical discovery text. This is
    /// the only per-transport prose; the shared "what's discovered" stem lives in
    /// [`integration_discovers`](Self::integration_discovers).
    pub(crate) fn transport_note(&self) -> &'static str {
        match self {
            Self::SnmpV1 => "Uses SNMPv1.",
            Self::SnmpV2c => "Uses SNMPv2c.",
            Self::SnmpV3 => "Uses SNMPv3.",
            Self::DockerProxy | Self::PodmanProxy => "Connects over TCP, optionally with TLS.",
            Self::DockerSocket | Self::PodmanSocket => "Connects via the daemon's local socket.",
        }
    }

    /// Short transport label within an integration (e.g. "Socket", "Proxy", "v2c").
    pub(crate) fn transport_label(&self) -> &'static str {
        match self {
            Self::SnmpV1 => "v1",
            Self::SnmpV2c => "v2c",
            Self::SnmpV3 => "v3",
            Self::DockerProxy | Self::PodmanProxy => "Proxy",
            Self::DockerSocket | Self::PodmanSocket => "Socket",
        }
    }

    /// Full credential description shown in the wizard and `credential-types.json`:
    /// the canonical discovery text plus the transport note. Derived, never
    /// hand-written per transport, so the two cannot drift.
    pub(crate) fn full_description(&self) -> String {
        format!("{} {}", self.integration_discovers(), self.transport_note())
    }

    fn category_str(&self) -> &'static str {
        self.to_credential_type().credential_category().into()
    }

    /// Minimum daemon version that can safely receive credential mappings of this
    /// type over the server→daemon wire. Exhaustive (no wildcard): a new credential
    /// variant will not compile until it declares its floor. This single declaration
    /// drives server-side dispatch filtering (never send a mapping an older daemon
    /// can't deserialize), assignment-time rejection, and the UI compatibility gate.
    ///
    /// Gated on the 7-way `CredentialType` discriminant, NOT the collapsed
    /// `CredentialQueryPayload` wire tag: SnmpV1/V3 carry a higher floor than SnmpV2c
    /// despite all three sharing the single `Snmp` wire variant.
    ///
    /// Distinct from the global [`DaemonVersionPolicy::minimum_supported`] floor —
    /// same `semver` comparison, different purpose.
    pub fn minimum_daemon_version(&self) -> semver::Version {
        match self {
            // Unified credential-wire floor. Older daemons ignore `credential_mappings`
            // via #[serde(default)], so filtering these out is harmless.
            Self::SnmpV2c | Self::DockerProxy | Self::DockerSocket => {
                semver::Version::new(0, 16, 2)
            }
            // SnmpV1/SnmpV3 inner `SnmpVersion` values shipped in 0.17.0.
            Self::SnmpV1 | Self::SnmpV3 => semver::Version::new(0, 17, 0),
            // Podman variants shipped in 0.17.2.
            Self::PodmanProxy | Self::PodmanSocket => semver::Version::new(0, 18, 2),
        }
    }

    /// Whether a daemon at `daemon_version` can safely receive credential mappings of
    /// this type. A missing version is treated conservatively: only types at the
    /// 0.16.2 unified-wire floor are considered compatible. Shared by server-side
    /// dispatch filtering and the UI compatibility gate so the two never diverge.
    pub fn compatible_with_daemon(&self, daemon_version: Option<&semver::Version>) -> bool {
        match daemon_version {
            Some(v) => *v >= self.minimum_daemon_version(),
            None => self.minimum_daemon_version() <= semver::Version::new(0, 16, 2),
        }
    }

    fn metadata_json(&self) -> serde_json::Value {
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
            // The frontend derives "daemon-host-only" (former `is_local_auto`) from `targets`.
            "targets": ct.targets(),
            "requires_config": ct.requires_config(),
            "single_endpoint_per_host": ct.single_endpoint_per_host(),
            // Minimum daemon version that can receive this type (message-only on the
            // frontend; the actual gate uses the server-computed compat flag).
            "minimum_daemon_version": self.minimum_daemon_version().to_string(),
            "associated_service": ServiceDefinition::name(&*service),
            "has_logo": service.has_logo(),
            "logo_ext": logo_ext,
            "logo_needs_white_background": service.logo_needs_white_background(),
        })
    }
}

// Credential types build their `TypeMetadata` directly (rather than via the
// `TypeMetadataProvider` blanket) because their description is composed at build
// time from the centralized integration text — see [`full_description`].
impl MetadataProvider<TypeMetadata> for CredentialTypeDiscriminants {
    fn to_metadata(&self) -> TypeMetadata {
        TypeMetadata {
            id: self.id(),
            name: Some(self.display_name()),
            description: Some(Cow::Owned(self.full_description())),
            category: Some(self.category_str()),
            icon: Some(self.icon()),
            color: self.color(),
            metadata: Some(self.metadata_json()),
        }
    }
}
