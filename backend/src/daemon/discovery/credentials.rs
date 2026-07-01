//! Generic credential resolution and summarization for discovery integrations.

use std::collections::HashMap;
use std::net::IpAddr;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::{
    CredentialMapping, CredentialQueryPayload, CredentialQueryPayloadDiscriminants,
};
use crate::server::hosts::r#impl::base::Host;

/// Resolve applicable credentials for a target IP from credential mappings.
///
/// Returns credentials in specificity order: every matching IP override first
/// (in `ip_overrides` declaration order), then the network default as fallback.
/// Each entry includes the credential and its optional server-side ID (for
/// auto-assignment tracking). The caller is expected to try them in order and
/// stop at the first successful probe.
pub fn resolve_credentials_for_ip(
    mapping: &CredentialMapping<CredentialQueryPayload>,
    ip: IpAddr,
) -> Vec<(&CredentialQueryPayload, Option<Uuid>)> {
    let mut creds = Vec::new();

    // Every IP-specific override, in declaration order. A host assigned
    // multiple credentials for the same IP should have all of them tried.
    for o in mapping.ip_overrides.iter().filter(|o| o.ip == ip) {
        let cred_id = (o.credential_id != Uuid::nil()).then_some(o.credential_id);
        creds.push((&o.credential, cred_id));
    }

    // Network default as fallback — always tried after overrides when present.
    // The probe loop breaks on first success, so a working override short-circuits
    // the default automatically; the default only actually runs when every
    // override failed (wrong community, auth error, timeout, etc.).
    if let Some(default) = &mapping.default_credential {
        creds.push((default, None));
    }

    creds
}

/// Summarize credential assignments across discovered hosts, grouped by credential type.
///
/// Builds a credential_id → type lookup from the credential mappings, then groups
/// each host's assignments by type label. Returns type_label → list of "cred_id → ip".
/// Summarize the credential mappings the daemon actually used, grouped by credential type.
///
/// Driven by the mappings themselves (not discovered-host assignments): each mapping's
/// `ip_overrides` cover daemon-host (127.0.0.1) and per-host targets, and `default_credential`
/// covers network broadcast. `hosts` is used only to annotate an override IP with a discovered
/// host name when one matches.
pub fn summarize_credential_assignments(
    hosts: &[(IpAddr, Host)],
    credential_mappings: &[CredentialMapping<CredentialQueryPayload>],
) -> HashMap<String, Vec<String>> {
    let host_name_by_ip: HashMap<IpAddr, String> = hosts
        .iter()
        .map(|(ip, host)| (*ip, host.base.name.clone()))
        .collect();

    let mut by_type: HashMap<String, Vec<String>> = HashMap::new();
    for mapping in credential_mappings {
        // Per-IP overrides: daemon-host (127.0.0.1) and explicit per-host targets.
        for o in &mapping.ip_overrides {
            let label: String =
                Into::<CredentialQueryPayloadDiscriminants>::into(&o.credential).to_string();
            let target = match host_name_by_ip.get(&o.ip) {
                Some(name) => format!("{} ({})", o.ip, name),
                None => o.ip.to_string(),
            };
            by_type
                .entry(label)
                .or_default()
                .push(format!("{} → {}", o.credential_id, target));
        }
        // Network-broadcast default credential (applies to all hosts on the network).
        if let Some(default) = &mapping.default_credential {
            let label: String =
                Into::<CredentialQueryPayloadDiscriminants>::into(default).to_string();
            by_type
                .entry(label)
                .or_default()
                .push("network default".to_string());
        }
    }

    by_type
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::{
        IpOverride, ResolvableSecret, SnmpQueryCredential, SnmpVersion,
    };

    fn snmp(community: &str) -> CredentialQueryPayload {
        CredentialQueryPayload::Snmp(SnmpQueryCredential {
            version: SnmpVersion::V2c,
            community: ResolvableSecret::Value {
                value: community.to_string(),
            },
            v3: None,
        })
    }

    fn snmp_community(payload: &CredentialQueryPayload) -> &str {
        match payload {
            CredentialQueryPayload::Snmp(s) => match &s.community {
                ResolvableSecret::Value { value } => value,
                ResolvableSecret::FilePath { .. } => panic!("expected inline community"),
            },
            _ => panic!("expected SNMP payload"),
        }
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn resolve_credentials_for_ip_single_override_then_default() {
        let override_id = Uuid::new_v4();
        let mapping = CredentialMapping {
            default_credential: Some(snmp("netdefault")),
            ip_overrides: vec![IpOverride {
                ip: ip("10.0.0.5"),
                credential: snmp("secret42"),
                credential_id: override_id,
            }],
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert_eq!(creds.len(), 2, "expected override + default fallback");
        assert_eq!(snmp_community(creds[0].0), "secret42");
        assert_eq!(creds[0].1, Some(override_id));
        assert_eq!(snmp_community(creds[1].0), "netdefault");
        assert_eq!(creds[1].1, None);
    }

    #[test]
    fn resolve_credentials_for_ip_multiple_overrides_then_default() {
        // Same IP, two host-scoped overrides (e.g., two SNMP creds assigned to
        // the same host via host_credentials) — every one should be tried in
        // declaration order, then fall back to the network default.
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let target = ip("10.0.0.5");
        let mapping = CredentialMapping {
            default_credential: Some(snmp("netdefault")),
            ip_overrides: vec![
                IpOverride {
                    ip: target,
                    credential: snmp("override_a"),
                    credential_id: id_a,
                },
                IpOverride {
                    ip: target,
                    credential: snmp("override_b"),
                    credential_id: id_b,
                },
                // A different IP's override must NOT be returned here.
                IpOverride {
                    ip: ip("10.0.0.99"),
                    credential: snmp("other_host"),
                    credential_id: Uuid::new_v4(),
                },
            ],
        };

        let creds = resolve_credentials_for_ip(&mapping, target);

        assert_eq!(creds.len(), 3, "two overrides + default fallback");
        assert_eq!(snmp_community(creds[0].0), "override_a");
        assert_eq!(creds[0].1, Some(id_a));
        assert_eq!(snmp_community(creds[1].0), "override_b");
        assert_eq!(creds[1].1, Some(id_b));
        assert_eq!(snmp_community(creds[2].0), "netdefault");
        assert_eq!(creds[2].1, None);
    }

    #[test]
    fn resolve_credentials_for_ip_multiple_overrides_no_default() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let target = ip("10.0.0.5");
        let mapping = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![
                IpOverride {
                    ip: target,
                    credential: snmp("override_a"),
                    credential_id: id_a,
                },
                IpOverride {
                    ip: target,
                    credential: snmp("override_b"),
                    credential_id: id_b,
                },
            ],
        };

        let creds = resolve_credentials_for_ip(&mapping, target);

        assert_eq!(creds.len(), 2, "both overrides, no default");
        assert_eq!(snmp_community(creds[0].0), "override_a");
        assert_eq!(snmp_community(creds[1].0), "override_b");
    }

    #[test]
    fn resolve_credentials_for_ip_default_only_when_no_matching_override() {
        let mapping = CredentialMapping {
            default_credential: Some(snmp("netdefault")),
            ip_overrides: vec![IpOverride {
                ip: ip("10.0.0.99"),
                credential: snmp("other_host"),
                credential_id: Uuid::new_v4(),
            }],
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert_eq!(creds.len(), 1);
        assert_eq!(snmp_community(creds[0].0), "netdefault");
        assert_eq!(creds[0].1, None);
    }

    #[test]
    fn resolve_credentials_for_ip_returns_empty_when_both_absent() {
        let mapping: CredentialMapping<CredentialQueryPayload> = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![],
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert!(creds.is_empty());
    }

    #[test]
    fn resolve_credentials_for_ip_treats_nil_credential_id_as_none() {
        // Daemon-injected bootstrap creds have Uuid::nil() — they're not tied
        // to a server-side credential entity and shouldn't leak into
        // assignment tracking. The helper maps nil → None.
        let mapping = CredentialMapping {
            default_credential: None,
            ip_overrides: vec![IpOverride {
                ip: ip("10.0.0.5"),
                credential: snmp("bootstrap"),
                credential_id: Uuid::nil(),
            }],
        };

        let creds = resolve_credentials_for_ip(&mapping, ip("10.0.0.5"));

        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].1, None);
    }
}
