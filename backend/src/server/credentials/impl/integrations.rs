//! The `integrations` aggregate: a website-facing view that joins a
//! [`ServiceDefinition`] (logo, name) with the [`CredentialType`]s that target it
//! (its transports) and one canonical "what's discovered" description.
//!
//! An "integration" is defined entirely by the credentials that point at a
//! service: it is the set of distinct `associated_service()` values over the
//! credential discriminants. There is no parallel store and no runtime endpoint —
//! [`all_integrations`] is computed from the existing credential/service metadata
//! and emitted as the `integrations.json` fixture by `generate-fixtures`, then
//! synced to the website the same way `service-definitions.json` is.

use serde::Serialize;
use strum::IntoEnumIterator;

use crate::server::services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt};
use crate::server::shared::fixtures::logo_slug;

use super::types::{CredentialTypeDiscriminants, Target};

/// One integration: a service plus the transports (credential types) that reach
/// it, with a single canonical discovery description and a one-line summary.
#[derive(Debug, Clone, Serialize)]
pub struct Integration {
    /// Service id (e.g. "Docker", "Podman", "SNMP").
    pub id: String,
    pub name: String,
    /// Credential category grouping (e.g. "Container & Virtualization").
    pub category: String,
    pub has_logo: bool,
    /// File extension of the service logo (matches the downloaded
    /// `logos/services/{logo_slug}.{logo_ext}`). Empty when there is no logo.
    pub logo_ext: String,
    /// Filename stem of the service logo under `logos/services/`.
    pub logo_slug: String,
    pub logo_needs_white_background: bool,
    /// Canonical "what's discovered" text — the single source shared by every
    /// transport of this integration.
    pub discovers: String,
    /// One-line summary: the discovery text plus the available transports.
    pub summary: String,
    pub transports: Vec<IntegrationTransport>,
}

/// One transport (credential type) of an integration.
#[derive(Debug, Clone, Serialize)]
pub struct IntegrationTransport {
    /// Credential type discriminant (e.g. "DockerSocket").
    pub id: String,
    /// Short transport label (e.g. "Socket", "Proxy", "v2c").
    pub name: String,
    /// Transport-specific note, derived from the credential's `transport_note`.
    pub description: String,
    pub requires_config: bool,
    pub single_endpoint_per_host: bool,
    /// Where this transport can be applied (daemon host, hosts, network).
    pub targets: Vec<Target>,
}

/// Build every integration by grouping credential discriminants on their
/// associated service. Output is deterministic (sorted by integration id, then
/// transport id) so re-running `generate-fixtures` yields no diff.
pub fn all_integrations() -> Vec<Integration> {
    let mut integrations: Vec<Integration> = Vec::new();

    for disc in CredentialTypeDiscriminants::iter() {
        let ct = disc.to_credential_type();
        let service = ct.associated_service();
        let service_id = ServiceDefinition::name(&*service).to_string();

        let transport = IntegrationTransport {
            id: <&'static str>::from(disc).to_string(),
            name: disc.transport_label().to_string(),
            description: disc.transport_note().to_string(),
            requires_config: ct.requires_config(),
            single_endpoint_per_host: ct.single_endpoint_per_host(),
            targets: ct.targets(),
        };

        if let Some(existing) = integrations.iter_mut().find(|i| i.id == service_id) {
            existing.transports.push(transport);
            continue;
        }

        integrations.push(Integration {
            id: service_id,
            name: ServiceDefinition::name(&*service).to_string(),
            category: <&'static str>::from(ct.credential_category()).to_string(),
            has_logo: service.has_logo(),
            logo_ext: logo_ext(service.logo_url()).to_string(),
            logo_slug: logo_slug(ServiceDefinition::name(&*service)),
            logo_needs_white_background: service.logo_needs_white_background(),
            discovers: disc.integration_discovers().to_string(),
            summary: String::new(), // filled in after all transports are collected
            transports: vec![transport],
        });
    }

    for integration in &mut integrations {
        integration.transports.sort_by(|a, b| a.id.cmp(&b.id));
        let labels: Vec<String> = integration
            .transports
            .iter()
            .map(|t| t.name.to_lowercase())
            .collect();
        integration.summary = format!(
            "{} Supports {}.",
            integration.discovers,
            join_oxford(&labels)
        );
    }

    integrations.sort_by(|a, b| a.id.cmp(&b.id));
    integrations
}

/// Oxford-comma join: `["a"] -> "a"`, `["a","b"] -> "a and b"`,
/// `["a","b","c"] -> "a, b, and c"`.
fn join_oxford(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {}", rest.join(", "), last),
    }
}

/// Derive a logo file extension from a logo URL, mirroring the logic used by the
/// service/credential metadata. Empty for missing or locally-served (`/`-prefixed)
/// logos.
fn logo_ext(url: &str) -> &str {
    if url.is_empty() || url.starts_with('/') {
        return "";
    }
    url.rsplit('.')
        .next()
        .and_then(|e| e.split('?').next())
        .filter(|e| matches!(*e, "svg" | "png" | "webp"))
        .unwrap_or("svg")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generic structural test (no per-integration assertions): every integration
    /// is well-formed and its credential descriptions derive from the centralized
    /// text. Mirrors the spirit of the service-definition specificity tests.
    #[test]
    fn integrations_are_well_formed() {
        let integrations = all_integrations();
        assert!(
            !integrations.is_empty(),
            "expected at least one integration"
        );

        for integration in &integrations {
            assert!(!integration.id.is_empty(), "integration id is empty");
            assert!(
                !integration.discovers.trim().is_empty(),
                "{} has no discovery text",
                integration.id
            );
            assert!(
                !integration.summary.trim().is_empty(),
                "{} has no summary",
                integration.id
            );
            assert!(
                integration.summary.contains(&integration.discovers),
                "{} summary should build on its discovery text",
                integration.id
            );
            assert!(
                !integration.transports.is_empty(),
                "{} has no transports",
                integration.id
            );

            // Transports of one integration must be distinguishable.
            let mut notes: Vec<&str> = integration
                .transports
                .iter()
                .map(|t| t.description.as_str())
                .collect();
            notes.sort_unstable();
            let unique = notes.len();
            notes.dedup();
            assert_eq!(
                notes.len(),
                unique,
                "{} has duplicate transport descriptions",
                integration.id
            );
        }
    }

    /// Locks the de-duplication: each credential's full description is exactly the
    /// canonical discovery text + its transport note (single source, derived).
    #[test]
    fn credential_description_derives_from_canonical_text() {
        for disc in CredentialTypeDiscriminants::iter() {
            let expected = format!("{} {}", disc.integration_discovers(), disc.transport_note());
            assert_eq!(disc.full_description(), expected);
            // The shared stem must actually be the leading text.
            assert!(
                disc.full_description()
                    .starts_with(disc.integration_discovers())
            );
        }
    }
}
