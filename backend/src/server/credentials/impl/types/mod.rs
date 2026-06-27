use crate::server::{
    credentials::r#impl::mapping::{
        ContainerProxyQueryCredential, ContainerSocketQueryCredential, CredentialQueryPayload,
        ResolvableSecret, ResolvableValue,
    },
    ports::r#impl::base::PortType,
    services::r#impl::definitions::ServiceDefinition,
};
use anyhow::Error;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, EnumIter};
use strum_macros::{IntoStaticStr, VariantNames};
use utoipa::ToSchema;

pub mod container_proxy;
pub mod snmp;

mod fields;
mod metadata;
mod secrets;

pub use fields::{FieldDefinition, FieldType, InlineFormat, PemTag, SelectOption};
pub use metadata::{CredentialAssignment, CredentialCategory, CredentialHostAssignment, Target};
pub use secrets::{
    ExposeSecretsGuard, FileOrInline, REDACTED_SECRET_SENTINEL, SecretValue,
    deserialize_optional_file_or_inline, deserialize_optional_secret_value,
};

// Re-export SnmpVersion and v3 protocol enums from snmp submodule
pub use snmp::{SnmpV3AuthProtocol, SnmpV3PrivProtocol, SnmpVersion};

fn default_docker_port() -> u16 {
    PortType::Docker.number()
}

/// Universal credential type — tagged enum stored as JSONB.
/// Each variant represents a different credential protocol/method.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    ToSchema,
    EnumDiscriminants,
    IntoStaticStr,
    VariantNames,
)]
#[strum_discriminants(derive(Display, Hash, Serialize, Deserialize, IntoStaticStr, EnumIter))]
#[serde(tag = "type")]
pub enum CredentialType {
    /// SNMPv1 community string — for legacy devices that only speak v1.
    SnmpV1 { community: SecretValue },
    /// SNMPv2c community string for querying network devices
    SnmpV2c { community: SecretValue },
    /// SNMPv3 USM AuthPriv — security name + auth/priv protocols and passwords.
    SnmpV3 {
        security_name: String,
        auth_protocol: SnmpV3AuthProtocol,
        auth_password: SecretValue,
        priv_protocol: SnmpV3PrivProtocol,
        priv_password: SecretValue,
        /// Optional context name (default/empty context used if unset).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context_name: Option<String>,
    },
    /// Docker API proxy credentials. Target IP determined from host ip_addresses at scan time.
    DockerProxy {
        /// Port for the Docker API proxy (default 2375)
        #[serde(default = "default_docker_port")]
        port: u16,
        /// Optional URL path prefix (e.g. "/v1.43")
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// PEM-encoded public certificate — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_cert: Option<FileOrInline>,
        /// Private key — inline PEM content or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_secret_value"
        )]
        ssl_key: Option<SecretValue>,
        /// PEM-encoded CA chain — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_chain: Option<FileOrInline>,
    },
    /// Local Docker socket access. Auto-injected by daemon when socket is available.
    /// Not user-selectable — managed automatically from daemon capabilities.
    DockerSocket {},
    /// Podman API proxy credentials. Podman exposes a Docker-compatible REST API,
    /// so the fields mirror `DockerProxy`. Target IP determined from host
    /// ip_addresses at scan time.
    PodmanProxy {
        /// Port for the Podman API proxy (default 2375)
        #[serde(default = "default_docker_port")]
        port: u16,
        /// Optional URL path prefix (e.g. "/v1.43")
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        /// PEM-encoded public certificate — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_cert: Option<FileOrInline>,
        /// Private key — inline PEM content or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_secret_value"
        )]
        ssl_key: Option<SecretValue>,
        /// PEM-encoded CA chain — inline or file path on daemon host
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_optional_file_or_inline"
        )]
        ssl_chain: Option<FileOrInline>,
    },
    /// Local Podman socket access (`/run/podman/podman.sock` or the rootless
    /// `$XDG_RUNTIME_DIR/podman/podman.sock`). Auto-injected by daemon when
    /// the socket is available. Not user-selectable.
    PodmanSocket {},
}

/// Convert a stored `SecretValue` into a daemon-bound `ResolvableSecret`,
/// exposing inline secrets and passing through file paths.
fn secret_to_resolvable(secret: &SecretValue) -> ResolvableSecret {
    match secret {
        SecretValue::Inline { value } => ResolvableSecret::Value {
            value: value.expose_secret().to_string(),
        },
        SecretValue::FilePath { path } => ResolvableSecret::FilePath { path: path.clone() },
    }
}

/// Extract the inline string value of a secret, or `None` for file-path mode
/// or the redacted sentinel.
fn inline_secret(secret: &SecretValue) -> Option<String> {
    match secret {
        SecretValue::Inline { value } => {
            let v = value.expose_secret().to_string();
            (v != REDACTED_SECRET_SENTINEL).then_some(v)
        }
        SecretValue::FilePath { .. } => None,
    }
}

impl CredentialType {
    /// Merge redacted sentinel values from the existing credential.
    /// When the API response redacts secrets to "********" and the UI sends that back,
    /// this restores the original secret from the existing record.
    pub fn merge_redacted_secrets(&mut self, existing: &CredentialType) {
        match (self, existing) {
            (
                Self::SnmpV1 { community },
                Self::SnmpV1 {
                    community: existing_community,
                },
            )
            | (
                Self::SnmpV2c { community },
                Self::SnmpV2c {
                    community: existing_community,
                },
            ) => {
                if community.is_redacted_sentinel() {
                    *community = existing_community.clone();
                }
            }
            (
                Self::SnmpV3 {
                    auth_password,
                    priv_password,
                    ..
                },
                Self::SnmpV3 {
                    auth_password: existing_auth,
                    priv_password: existing_priv,
                    ..
                },
            ) => {
                if auth_password.is_redacted_sentinel() {
                    *auth_password = existing_auth.clone();
                }
                if priv_password.is_redacted_sentinel() {
                    *priv_password = existing_priv.clone();
                }
            }
            (
                Self::DockerProxy { ssl_key, .. },
                Self::DockerProxy {
                    ssl_key: existing_key,
                    ..
                },
            )
            | (
                Self::PodmanProxy { ssl_key, .. },
                Self::PodmanProxy {
                    ssl_key: existing_key,
                    ..
                },
            ) => {
                if let Some(key) = ssl_key
                    && key.is_redacted_sentinel()
                {
                    *ssl_key = existing_key.clone();
                }
            }
            (Self::DockerSocket {}, _) | (Self::PodmanSocket {}, _) => {}
            // Type changed — no merging needed
            _ => {}
        }
    }

    pub fn credential_category(&self) -> CredentialCategory {
        match self {
            Self::SnmpV1 { .. } | Self::SnmpV2c { .. } | Self::SnmpV3 { .. } => {
                CredentialCategory::NetworkMonitoring
            }
            Self::DockerProxy { .. }
            | Self::DockerSocket {}
            | Self::PodmanProxy { .. }
            | Self::PodmanSocket {} => CredentialCategory::ContainerVirtualization,
        }
    }

    /// Where this credential type can be applied: the daemon's own host, specific
    /// hosts, and/or a whole network (broadcast).
    pub fn targets(&self) -> Vec<Target> {
        match self {
            // SNMP can target the daemon's own host too (a 127.0.0.1 IP-override),
            // a specific host, or a whole network.
            Self::SnmpV1 { .. } | Self::SnmpV2c { .. } | Self::SnmpV3 { .. } => {
                vec![Target::DaemonHost, Target::Host, Target::Network]
            }
            // Docker/Podman proxy: on the daemon host (localhost proxy) or a remote host.
            Self::DockerProxy { .. } | Self::PodmanProxy { .. } => {
                vec![Target::DaemonHost, Target::Host]
            }
            // Local socket: only the daemon's own host.
            Self::DockerSocket {} | Self::PodmanSocket {} => vec![Target::DaemonHost],
        }
    }

    /// Whether the user must provide any configuration (fields) for this type.
    /// Derived from `field_definitions()` — a type with no fields needs nothing.
    pub fn requires_config(&self) -> bool {
        !self.field_definitions().is_empty()
    }

    /// Whether this type is a zero-config local capability the daemon auto-detects
    /// (e.g. the Docker socket): no config to provide and applicable to the daemon's
    /// own host. Such types render as an on/off toggle and persist as daemon config,
    /// not as a credential. Derived — never declared per-variant.
    pub fn is_local_auto(&self) -> bool {
        !self.requires_config() && self.targets().contains(&Target::DaemonHost)
    }

    /// Whether this integration is a single service instance per host, so its
    /// access methods at a given target are mutually exclusive (e.g. a container
    /// runtime is reached by exactly one of socket/proxy). `false` for try-many
    /// auth integrations like SNMP (multiple credentials are attempted). All
    /// credential types of the same integration agree.
    pub fn single_endpoint_per_host(&self) -> bool {
        match self {
            Self::DockerProxy { .. }
            | Self::DockerSocket {}
            | Self::PodmanProxy { .. }
            | Self::PodmanSocket {} => true,
            Self::SnmpV1 { .. } | Self::SnmpV2c { .. } | Self::SnmpV3 { .. } => false,
        }
    }

    /// Get the inline string value for a field by ID.
    /// Returns None for FilePath mode, None fields, or redacted sentinels.
    fn get_inline_value(&self, field_id: &str) -> Option<String> {
        match self {
            Self::SnmpV1 { community } | Self::SnmpV2c { community } => match field_id {
                "community" => inline_secret(community),
                _ => None,
            },
            Self::SnmpV3 {
                auth_password,
                priv_password,
                ..
            } => match field_id {
                "auth_password" => inline_secret(auth_password),
                "priv_password" => inline_secret(priv_password),
                _ => None,
            },
            Self::DockerProxy {
                ssl_cert,
                ssl_key,
                ssl_chain,
                ..
            }
            | Self::PodmanProxy {
                ssl_cert,
                ssl_key,
                ssl_chain,
                ..
            } => match field_id {
                "ssl_cert" => match ssl_cert.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                "ssl_key" => inline_secret(ssl_key.as_ref()?),
                "ssl_chain" => match ssl_chain.as_ref()? {
                    FileOrInline::Inline { value } => Some(value.clone()),
                    FileOrInline::FilePath { .. } => None,
                },
                _ => None,
            },
            Self::DockerSocket {} | Self::PodmanSocket {} => None,
        }
    }

    /// Validate inline field values using field_definitions() metadata.
    /// Skips FilePath values (validated on daemon after read), redacted sentinels,
    /// and empty optionals.
    pub fn validate(&self) -> Result<(), Error> {
        for field in self.field_definitions() {
            if let Some(fmt) = &field.inline_format
                && let Some(value) = self.get_inline_value(field.id)
            {
                fmt.validate(&value, field.label)?;
            }
        }
        Ok(())
    }

    /// Returns the ServiceDefinition this credential type integrates with.
    /// Every credential type maps to exactly one service — used for logo display,
    /// metadata enrichment, and Phase 2 integration dispatch.
    pub fn associated_service(&self) -> Box<dyn ServiceDefinition> {
        match self {
            Self::SnmpV1 { .. } | Self::SnmpV2c { .. } | Self::SnmpV3 { .. } => {
                Box::new(crate::server::services::definitions::snmp::Snmp)
            }
            Self::DockerProxy { .. } | Self::DockerSocket {} => {
                Box::new(crate::server::services::definitions::docker_daemon::Docker)
            }
            Self::PodmanProxy { .. } | Self::PodmanSocket {} => {
                Box::new(crate::server::services::definitions::podman::Podman)
            }
        }
    }

    /// Whether this credential type is created as a user credential (vs. rendered
    /// as a daemon auto-capability toggle). Auto-local types (e.g. the Docker
    /// socket) are managed as daemon config, not created as credentials.
    pub fn is_user_selectable(&self) -> bool {
        !self.is_local_auto()
    }

    /// Convert to wire format payload for daemon transmission.
    /// No wildcard match — compiler forces update when new variants added.
    pub fn to_query_payload(&self) -> CredentialQueryPayload {
        use crate::server::credentials::r#impl::mapping::{SnmpQueryCredential, SnmpV3Params};
        match self {
            CredentialType::SnmpV1 { community } => {
                CredentialQueryPayload::Snmp(SnmpQueryCredential {
                    version: SnmpVersion::V1,
                    community: secret_to_resolvable(community),
                    v3: None,
                })
            }
            CredentialType::SnmpV2c { community } => {
                CredentialQueryPayload::Snmp(SnmpQueryCredential {
                    version: SnmpVersion::V2c,
                    community: secret_to_resolvable(community),
                    v3: None,
                })
            }
            CredentialType::SnmpV3 {
                security_name,
                auth_protocol,
                auth_password,
                priv_protocol,
                priv_password,
                context_name,
            } => CredentialQueryPayload::Snmp(SnmpQueryCredential {
                version: SnmpVersion::V3,
                community: ResolvableSecret::Value {
                    value: String::new(),
                },
                v3: Some(SnmpV3Params {
                    security_name: security_name.clone(),
                    auth_protocol: *auth_protocol,
                    auth_password: secret_to_resolvable(auth_password),
                    priv_protocol: *priv_protocol,
                    priv_password: secret_to_resolvable(priv_password),
                    context_name: context_name.clone(),
                }),
            }),
            CredentialType::DockerProxy {
                port,
                path,
                ssl_cert,
                ssl_key,
                ssl_chain,
            } => CredentialQueryPayload::DockerProxy(container_proxy_query(
                *port, path, ssl_cert, ssl_key, ssl_chain,
            )),
            CredentialType::DockerSocket {} => {
                CredentialQueryPayload::DockerSocket(ContainerSocketQueryCredential {})
            }
            CredentialType::PodmanProxy {
                port,
                path,
                ssl_cert,
                ssl_key,
                ssl_chain,
            } => CredentialQueryPayload::PodmanProxy(container_proxy_query(
                *port, path, ssl_cert, ssl_key, ssl_chain,
            )),
            CredentialType::PodmanSocket {} => {
                CredentialQueryPayload::PodmanSocket(ContainerSocketQueryCredential {})
            }
        }
    }
}

/// Build a container-runtime proxy query credential from the shared
/// proxy fields (Docker and Podman use the same Docker-compatible shape).
fn container_proxy_query(
    port: u16,
    path: &Option<String>,
    ssl_cert: &Option<FileOrInline>,
    ssl_key: &Option<SecretValue>,
    ssl_chain: &Option<FileOrInline>,
) -> ContainerProxyQueryCredential {
    let file_or_inline = |f: &FileOrInline| match f {
        FileOrInline::Inline { value } => ResolvableValue::Value {
            value: value.clone(),
        },
        FileOrInline::FilePath { path } => ResolvableValue::FilePath { path: path.clone() },
    };
    ContainerProxyQueryCredential {
        port,
        path: path.clone(),
        ssl_cert: ssl_cert.as_ref().map(&file_or_inline),
        ssl_key: ssl_key.as_ref().map(secret_to_resolvable),
        ssl_chain: ssl_chain.as_ref().map(&file_or_inline),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;
    use secrecy::SecretString;
    use std::collections::HashMap;
    use strum::IntoEnumIterator;

    /// `single_endpoint_per_host()` is an integration-level property expressed
    /// per credential type, so every credential type sharing an `associated_service`
    /// must agree — otherwise the integration's exclusivity rule is incoherent.
    #[test]
    fn single_endpoint_per_host_agrees_within_integration() {
        let mut by_service: HashMap<&'static str, bool> = HashMap::new();
        for disc in CredentialTypeDiscriminants::iter() {
            let ct = disc.to_credential_type();
            let service = ct.associated_service().name();
            let value = ct.single_endpoint_per_host();
            match by_service.get(service) {
                Some(&existing) => assert_eq!(
                    existing, value,
                    "credential types for integration '{service}' disagree on single_endpoint_per_host",
                ),
                None => {
                    by_service.insert(service, value);
                }
            }
        }
    }

    fn snmp_cred(community: &str) -> CredentialType {
        CredentialType::SnmpV2c {
            community: SecretValue::Inline {
                value: SecretString::from(community.to_string()),
            },
        }
    }

    fn inline(value: &str) -> SecretValue {
        SecretValue::Inline {
            value: SecretString::from(value.to_string()),
        }
    }

    fn snmpv1_cred(community: &str) -> CredentialType {
        CredentialType::SnmpV1 {
            community: inline(community),
        }
    }

    fn snmpv3_cred(auth_pw: &str, priv_pw: &str) -> CredentialType {
        CredentialType::SnmpV3 {
            security_name: "snmp-user".to_string(),
            auth_protocol: SnmpV3AuthProtocol::Sha256,
            auth_password: inline(auth_pw),
            priv_protocol: SnmpV3PrivProtocol::Aes128,
            priv_password: inline(priv_pw),
            context_name: Some("ctx-1".to_string()),
        }
    }

    fn docker_cred(ssl_key: Option<&str>) -> CredentialType {
        CredentialType::DockerProxy {
            port: 2376,
            path: None,
            ssl_cert: None,
            ssl_key: ssl_key.map(|k| SecretValue::Inline {
                value: SecretString::from(k.to_string()),
            }),
            ssl_chain: None,
        }
    }

    #[test]
    fn merge_redacted_secrets_preserves_original_when_sentinel_sent() {
        let existing = snmp_cred("my-secret-community");
        let mut updated = snmp_cred(REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::SnmpV2c { community } = &updated {
            if let SecretValue::Inline { value } = community {
                assert_eq!(value.expose_secret(), "my-secret-community");
            } else {
                panic!("Expected Inline secret");
            }
        } else {
            panic!("Expected Snmp variant");
        }
    }

    #[test]
    fn merge_redacted_secrets_allows_actual_value_changes() {
        let existing = snmp_cred("old-community");
        let mut updated = snmp_cred("new-community");
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::SnmpV2c { community } = &updated {
            if let SecretValue::Inline { value } = community {
                assert_eq!(value.expose_secret(), "new-community");
            } else {
                panic!("Expected Inline secret");
            }
        } else {
            panic!("Expected Snmp variant");
        }
    }

    #[test]
    fn merge_redacted_secrets_handles_type_mismatch() {
        let existing = snmp_cred("secret");
        let mut updated = docker_cred(Some("new-key"));
        // Should be a no-op — types don't match
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::DockerProxy { ssl_key, .. } = &updated {
            if let Some(SecretValue::Inline { value }) = ssl_key {
                assert_eq!(value.expose_secret(), "new-key");
            } else {
                panic!("Expected Some(Inline)");
            }
        } else {
            panic!("Expected DockerProxy variant");
        }
    }

    #[test]
    fn merge_redacted_secrets_handles_docker_ssl_key() {
        let existing = docker_cred(Some("original-key"));
        let mut updated = docker_cred(Some(REDACTED_SECRET_SENTINEL));
        updated.merge_redacted_secrets(&existing);

        if let CredentialType::DockerProxy { ssl_key, .. } = &updated {
            if let Some(SecretValue::Inline { value }) = ssl_key {
                assert_eq!(value.expose_secret(), "original-key");
            } else {
                panic!("Expected Some(Inline)");
            }
        } else {
            panic!("Expected DockerProxy variant");
        }
    }

    #[test]
    fn is_redacted_sentinel_detects_sentinel() {
        let sentinel = SecretValue::Inline {
            value: SecretString::from(REDACTED_SECRET_SENTINEL.to_string()),
        };
        assert!(sentinel.is_redacted_sentinel());

        let real = SecretValue::Inline {
            value: SecretString::from("real-secret".to_string()),
        };
        assert!(!real.is_redacted_sentinel());

        let filepath = SecretValue::FilePath {
            path: REDACTED_SECRET_SENTINEL.to_string(),
        };
        assert!(!filepath.is_redacted_sentinel());
    }

    #[test]
    fn merge_redacted_secrets_snmpv1_preserves_original() {
        let existing = snmpv1_cred("legacy-community");
        let mut updated = snmpv1_cred(REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);
        match &updated {
            CredentialType::SnmpV1 {
                community: SecretValue::Inline { value },
            } => assert_eq!(value.expose_secret(), "legacy-community"),
            _ => panic!("expected SnmpV1 inline"),
        }
    }

    #[test]
    fn merge_redacted_secrets_snmpv3_preserves_both_passwords_when_sentinel() {
        let existing = snmpv3_cred("auth-real", "priv-real");
        let mut updated = snmpv3_cred(REDACTED_SECRET_SENTINEL, REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);
        match &updated {
            CredentialType::SnmpV3 {
                auth_password: SecretValue::Inline { value: a },
                priv_password: SecretValue::Inline { value: p },
                ..
            } => {
                assert_eq!(a.expose_secret(), "auth-real");
                assert_eq!(p.expose_secret(), "priv-real");
            }
            _ => panic!("expected SnmpV3 inline passwords"),
        }
    }

    #[test]
    fn merge_redacted_secrets_snmpv3_independent_per_password() {
        // Only the priv password is the sentinel — auth must keep its new value.
        let existing = snmpv3_cred("auth-real", "priv-real");
        let mut updated = snmpv3_cred("auth-new", REDACTED_SECRET_SENTINEL);
        updated.merge_redacted_secrets(&existing);
        match &updated {
            CredentialType::SnmpV3 {
                auth_password: SecretValue::Inline { value: a },
                priv_password: SecretValue::Inline { value: p },
                ..
            } => {
                assert_eq!(a.expose_secret(), "auth-new");
                assert_eq!(p.expose_secret(), "priv-real");
            }
            _ => panic!("expected SnmpV3 inline passwords"),
        }
    }

    #[test]
    fn to_query_payload_snmpv1_sets_version_and_community() {
        match snmpv1_cred("comm").to_query_payload() {
            CredentialQueryPayload::Snmp(s) => {
                assert_eq!(s.version, SnmpVersion::V1);
                assert!(s.v3.is_none());
                assert_eq!(
                    s.community,
                    ResolvableSecret::Value {
                        value: "comm".to_string()
                    }
                );
            }
            _ => panic!("expected Snmp payload"),
        }
    }

    #[test]
    fn to_query_payload_snmpv3_populates_usm_params() {
        match snmpv3_cred("auth-pw", "priv-pw").to_query_payload() {
            CredentialQueryPayload::Snmp(s) => {
                assert_eq!(s.version, SnmpVersion::V3);
                let v3 = s.v3.expect("v3 params present");
                assert_eq!(v3.security_name, "snmp-user");
                assert_eq!(v3.auth_protocol, SnmpV3AuthProtocol::Sha256);
                assert_eq!(v3.priv_protocol, SnmpV3PrivProtocol::Aes128);
                assert_eq!(v3.context_name.as_deref(), Some("ctx-1"));
                assert_eq!(
                    v3.auth_password,
                    ResolvableSecret::Value {
                        value: "auth-pw".to_string()
                    }
                );
                assert_eq!(
                    v3.priv_password,
                    ResolvableSecret::Value {
                        value: "priv-pw".to_string()
                    }
                );
            }
            _ => panic!("expected Snmp payload"),
        }
    }

    #[test]
    fn snmpv3_default_serialization_redacts_passwords() {
        // The default Serialize impl (API responses) must never leak secrets.
        let json = serde_json::to_string(&snmpv3_cred("super-secret-auth", "super-secret-priv"))
            .expect("serialize");
        assert!(!json.contains("super-secret-auth"));
        assert!(!json.contains("super-secret-priv"));
        assert!(json.contains(REDACTED_SECRET_SENTINEL));
    }

    #[test]
    fn snmpv3_storage_serialization_exposes_passwords() {
        // Under an ExposeSecretsGuard (the DB-write path), secrets are exposed.
        let cred = snmpv3_cred("auth-xyz-plain", "priv-xyz-plain");
        let json = {
            let _expose = ExposeSecretsGuard::new();
            serde_json::to_string(&cred).expect("serialize")
        };
        assert!(json.contains("auth-xyz-plain"));
        assert!(json.contains("priv-xyz-plain"));
    }

    #[test]
    fn secret_exposure_resets_after_guard_drops() {
        // After the guard scope, serialization must redact again — the storage
        // exposure must not leak into subsequent default serializations.
        let cred = snmpv3_cred("leak-check-auth", "leak-check-priv");
        {
            let _expose = ExposeSecretsGuard::new();
            let _ = serde_json::to_string(&cred);
        }
        let json = serde_json::to_string(&cred).expect("serialize");
        assert!(!json.contains("leak-check-auth"));
        assert!(json.contains(REDACTED_SECRET_SENTINEL));
    }

    #[test]
    fn snmpv3_query_payload_debug_redacts_passwords() {
        let payload = snmpv3_cred("auth-dbg", "priv-dbg").to_query_payload();
        let dbg = format!("{:?}", payload);
        assert!(!dbg.contains("auth-dbg"));
        assert!(!dbg.contains("priv-dbg"));
    }

    fn podman_cred(ssl_key: Option<&str>) -> CredentialType {
        CredentialType::PodmanProxy {
            port: 2376,
            path: None,
            ssl_cert: None,
            ssl_key: ssl_key.map(|k| SecretValue::Inline {
                value: SecretString::from(k.to_string()),
            }),
            ssl_chain: None,
        }
    }

    #[test]
    fn podman_proxy_to_query_payload_round_trips() {
        match podman_cred(Some("podman-key")).to_query_payload() {
            CredentialQueryPayload::PodmanProxy(p) => {
                assert_eq!(p.port, 2376);
                assert!(matches!(
                    p.ssl_key,
                    Some(ResolvableSecret::Value { value }) if value == "podman-key"
                ));
            }
            other => panic!("expected PodmanProxy payload, got {other:?}"),
        }
    }

    #[test]
    fn podman_socket_to_query_payload() {
        assert!(matches!(
            CredentialType::PodmanSocket {}.to_query_payload(),
            CredentialQueryPayload::PodmanSocket(_)
        ));
    }

    #[test]
    fn podman_credentials_are_container_virtualization() {
        assert_eq!(
            podman_cred(None).credential_category(),
            CredentialCategory::ContainerVirtualization
        );
        assert_eq!(
            CredentialType::PodmanSocket {}.credential_category(),
            CredentialCategory::ContainerVirtualization
        );
    }

    #[test]
    fn podman_proxy_associates_podman_service() {
        assert_eq!(podman_cred(None).associated_service().name(), "Podman");
        assert_eq!(
            CredentialType::PodmanSocket {}.associated_service().name(),
            "Podman"
        );
    }

    #[test]
    fn podman_proxy_exposes_proxy_fields() {
        // Same shape as Docker proxy: port, path, and three TLS fields.
        let fields = podman_cred(None).field_definitions();
        let ids: Vec<&str> = fields.iter().map(|f| f.id).collect();
        assert_eq!(
            ids,
            vec!["port", "path", "ssl_cert", "ssl_key", "ssl_chain"]
        );
        assert!(
            CredentialType::PodmanSocket {}
                .field_definitions()
                .is_empty()
        );
    }

    #[test]
    fn storage_serialization_tag_round_trips_for_every_variant() {
        // The stored form is the credential's own derived serialization (under an
        // exposure guard). This guards that every variant's "type" tag round-trips
        // through Deserialize — so storage can't silently desync from how it reads.
        for disc in CredentialTypeDiscriminants::iter() {
            let ct = disc.to_credential_type();
            let json = {
                let _expose = ExposeSecretsGuard::new();
                serde_json::to_string(&ct).expect("serialize")
            };
            let back: CredentialType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(
                CredentialTypeDiscriminants::from(&back),
                disc,
                "storage tag did not round-trip for {disc:?}: {json}"
            );
        }
    }

    #[test]
    fn podman_proxy_merge_redacted_ssl_key() {
        let existing = podman_cred(Some("original-key"));
        let mut updated = podman_cred(Some(REDACTED_SECRET_SENTINEL));
        updated.merge_redacted_secrets(&existing);
        match &updated {
            CredentialType::PodmanProxy { ssl_key, .. } => {
                assert!(matches!(
                    ssl_key,
                    Some(SecretValue::Inline { value }) if value.expose_secret() == "original-key"
                ));
            }
            _ => panic!("expected PodmanProxy variant"),
        }
    }
}
