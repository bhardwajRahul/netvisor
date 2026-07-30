//! Deferred, aggregated scan warnings.
//!
//! The session's `warnings` field is a flat `Vec<String>` rendered verbatim into one
//! notification, so anything pushed per host multiplies by the host count. A customer scanning
//! ~15 switches received fifteen full paragraphs in a single notification, which is unreadable
//! and buries the one line that matters.
//!
//! Producers that fire per host record a typed value here instead, and the session renders one
//! summary line per kind at finalize. Two rules hold for every renderer below:
//!
//! - **Say what it means for the user, not what the code saw.** "Previously discovered values
//!   were kept" is the actionable part; the internal completeness flag is not.
//! - **Never truncate silently.** A capped list says how many were elided, because a list that
//!   simply stops reads as "that was all of them".

use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;

use crate::server::ports::r#impl::base::PortType;

/// How many addresses a summary lists before eliding the rest.
const MAX_LISTED: usize = 10;

/// Render `ips` as an English list — "A", "A and B", "A, B, and C" — so it can be the subject
/// of a sentence, which is where every warning here puts the addresses. Capped at
/// [`MAX_LISTED`], with the remainder as a final list item ("…, and 5 more") rather than a
/// parenthetical, so a long list still reads as prose and never silently stops.
fn list_addresses_prose(ips: &BTreeSet<IpAddr>) -> String {
    let mut parts: Vec<String> = ips
        .iter()
        .take(MAX_LISTED)
        .map(|ip| ip.to_string())
        .collect();
    let elided = ips.len().saturating_sub(parts.len());
    if elided > 0 {
        parts.push(format!("{elided} more"));
    }
    match parts.len() {
        0 => String::new(),
        1 => parts.remove(0),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let last = parts.pop().unwrap_or_default();
            format!("{}, and {}", parts.join(", "), last)
        }
    }
}

// ============================================================================
// Incomplete SNMP walks
// ============================================================================

/// One SNMP data group that a walk could not read in full, for one device.
///
/// `returned_any` distinguishes the two cases the old single phrasing conflated. A walk that
/// returns rows and stops was genuinely truncated; a walk that returns nothing timed out or
/// errored outright — `Default` for these result types is `complete: false` with no records, so
/// "the device stopped responding partway through" was simply wrong for the second case and sent
/// operators to inspect hardware that was fine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncompleteSnmpWalk {
    pub ip: IpAddr,
    pub group: &'static str,
    pub returned_any: bool,
}

/// One line per distinct failure, or empty if there were none.
///
/// Lines are keyed by the set of devices rather than by data type, so a device short on both
/// LLDP and CDP is named once ("… did not finish reporting LLDP neighbours or CDP neighbours")
/// instead of appearing in a separate line per type. Addresses lead each line; a run that
/// buries them behind a count is one the reader has to re-parse to act on.
pub fn render_incomplete_snmp_walks(records: &[IncompleteSnmpWalk]) -> Vec<String> {
    if records.is_empty() {
        return Vec::new();
    }

    // (returned_any, group) -> devices, then invert so identical device sets collapse.
    //
    // `returned_any` is part of the key, not a sentence appended afterwards. It is a property of
    // one group's walk, so aggregating it per *device* produced lines that contradicted
    // themselves — "192.168.7.230 did not finish reporting VLAN membership or bridge forwarding"
    // immediately followed by "192.168.7.230 returned nothing at all", which reads as two
    // incompatible claims about the same walk. Keyed this way, each line makes one claim.
    let mut devices_by_group: BTreeMap<(bool, &str), BTreeSet<IpAddr>> = BTreeMap::new();
    for r in records {
        devices_by_group
            .entry((r.returned_any, r.group))
            .or_default()
            .insert(r.ip);
    }
    let mut groups_by_devices: BTreeMap<(bool, BTreeSet<IpAddr>), Vec<&str>> = BTreeMap::new();
    for ((returned_any, group), ips) in devices_by_group {
        groups_by_devices
            .entry((returned_any, ips))
            .or_default()
            .push(group);
    }

    groups_by_devices
        .iter()
        .map(|((returned_any, ips), groups)| {
            let who = list_addresses_prose(ips);
            let what = join_prose(groups);
            if *returned_any {
                format!(
                    "{who} did not finish reporting {what}, so previously discovered values were \
                     kept rather than overwritten and refresh on the next complete scan."
                )
            } else {
                format!(
                    "{who} returned no {what} data at all, which usually means the query timed \
                     out rather than that the device is faulty. Previously discovered values were \
                     kept rather than overwritten and refresh on the next complete scan."
                )
            }
        })
        .collect()
}

/// Join labels as an English list, matching [`list_addresses_prose`].
fn join_prose(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_string(),
        [a, b] => format!("{a} or {b}"),
        _ => {
            let (last, rest) = items.split_last().expect("non-empty");
            format!("{}, or {}", rest.join(", "), last)
        }
    }
}

// ============================================================================
// Incomplete interface (ifTable) walks
// ============================================================================

/// One device whose ifTable walk fell short, and in which of the two ways.
///
/// Kept separate from [`IncompleteSnmpWalk`] because the two failures mean different things to
/// an operator and must not be merged into one sentence: a truncated interface *set* means
/// interfaces are genuinely missing, while a truncated attribute column only means some
/// descriptions or speeds are blank. Reporting the second as possible data loss sends people
/// hunting for interfaces that were never absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncompleteInterfaceWalk {
    pub ip: IpAddr,
    /// Interfaces read before the walk stopped.
    pub collected: usize,
    /// `true` when the whole interface set was read and only attribute columns fell short.
    pub set_complete: bool,
}

/// One line per distinct failure, or empty if there were none.
///
/// Returns a list rather than a paragraph so the UI can render each as its own bullet: a run
/// that hits several unrelated problems produces several short statements, not one wall of
/// prose the reader has to parse apart.
pub fn render_incomplete_interface_walks(records: &[IncompleteInterfaceWalk]) -> Vec<String> {
    let missing: BTreeSet<IpAddr> = records
        .iter()
        .filter(|r| !r.set_complete)
        .map(|r| r.ip)
        .collect();
    let blank: BTreeSet<IpAddr> = records
        .iter()
        .filter(|r| r.set_complete)
        .map(|r| r.ip)
        .collect();

    let mut lines = Vec::new();
    if !missing.is_empty() {
        lines.push(format!(
            "{} stopped responding partway through the SNMP interface list, so some interfaces \
             are missing.",
            list_addresses_prose(&missing)
        ));
    }
    if !blank.is_empty() {
        lines.push(format!(
            "{} returned every SNMP interface but stopped while reading their details, so some \
             descriptions or speeds may be blank.",
            list_addresses_prose(&blank)
        ));
    }
    lines
}

// ============================================================================
// Credential issues
// ============================================================================

/// Why an IP-targeted credential produced nothing.
///
/// Only credentials the user deliberately assigned to a host are reported. A network-default
/// credential failing is routine — it is broadcast at every address in the subnet — and
/// reporting those would flood the notification on any sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialIssueReason {
    /// The address is not inside any subnet this scan enumerated, so it was never contacted.
    TargetNotScanned,
    /// The address *was* in scope, but nothing answered there, so no host was ever deep-scanned
    /// and the credential was never applied. Distinct from [`Self::TargetNotScanned`] because
    /// the fix is different: there the discovery's subnets are wrong, here the address is wrong
    /// or the host is down.
    TargetNotResponding,
    /// The host was scanned but the credential's port was not open, so no probe was attempted.
    GateClosed { ports: Vec<PortType> },
    /// The probe ran and the endpoint refused it.
    ProbeRejected { message: String },
}

/// One IP-targeted credential that did not work, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialIssue {
    /// Human label for the credential type, from `CredentialQueryPayload::discovery_label`.
    pub label: &'static str,
    pub ip: IpAddr,
    pub reason: CredentialIssueReason,
}

/// One line per reason, or empty if there were none.
///
/// Grouped by reason rather than by credential, because the fix differs per reason: a target on
/// no scanned subnet is a discovery-scope problem, an unanswered address is a wrong-address or
/// host-down problem, a closed gate is a port problem, and a rejection is a credential problem.
/// Each gets its own line so the reader can act on one without disentangling it from the rest.
pub fn render_credential_issues(issues: &[CredentialIssue]) -> Vec<String> {
    if issues.is_empty() {
        return Vec::new();
    }

    let mut parts: Vec<String> = Vec::new();

    let not_scanned: BTreeSet<IpAddr> = issues
        .iter()
        .filter(|i| i.reason == CredentialIssueReason::TargetNotScanned)
        .map(|i| i.ip)
        .collect();
    if !not_scanned.is_empty() {
        parts.push(format!(
            "{} was never contacted because its address is not on any subnet this scan covers — \
             add the subnet to the discovery, or move the credential to a host inside it",
            describe_targets(issues, &not_scanned)
        ));
    }

    let not_responding: BTreeSet<IpAddr> = issues
        .iter()
        .filter(|i| i.reason == CredentialIssueReason::TargetNotResponding)
        .map(|i| i.ip)
        .collect();
    if !not_responding.is_empty() {
        parts.push(format!(
            "{} was not tried because nothing answered at that address during the scan — check \
             the address is right and the host is online",
            describe_targets(issues, &not_responding)
        ));
    }

    let gated: BTreeSet<IpAddr> = issues
        .iter()
        .filter(|i| matches!(i.reason, CredentialIssueReason::GateClosed { .. }))
        .map(|i| i.ip)
        .collect();
    if !gated.is_empty() {
        let ports: BTreeSet<u16> = issues
            .iter()
            .filter_map(|i| match &i.reason {
                CredentialIssueReason::GateClosed { ports } => Some(ports),
                _ => None,
            })
            .flatten()
            .map(|p| p.number())
            .collect();
        parts.push(format!(
            "{} was not tried because port {} was not open on it — check the port configured on \
             the credential",
            describe_targets(issues, &gated),
            ports
                .into_iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(" / ")
        ));
    }

    let rejected: Vec<&CredentialIssue> = issues
        .iter()
        .filter(|i| matches!(i.reason, CredentialIssueReason::ProbeRejected { .. }))
        .collect();
    if !rejected.is_empty() {
        // The first message is representative and already phrased for an operator; the count
        // carries the rest so the line stays one sentence.
        let first = rejected
            .iter()
            .find_map(|i| match &i.reason {
                CredentialIssueReason::ProbeRejected { message } => Some(message.as_str()),
                _ => None,
            })
            .unwrap_or("the endpoint refused it");
        let ips: BTreeSet<IpAddr> = rejected.iter().map(|i| i.ip).collect();
        parts.push(format!(
            "{} was rejected: {}",
            describe_targets(issues, &ips),
            first
        ));
    }

    parts.into_iter().map(|p| format!("{p}.")).collect()
}

/// "The SNMP queries credential for 10.0.0.5" / "2 credentials for 10.0.0.5, 10.0.0.6".
fn describe_targets(issues: &[CredentialIssue], ips: &BTreeSet<IpAddr>) -> String {
    let labels: BTreeSet<&str> = issues
        .iter()
        .filter(|i| ips.contains(&i.ip))
        .map(|i| i.label)
        .collect();
    let label = if labels.len() == 1 {
        format!("The {} credential", labels.iter().next().unwrap())
    } else {
        format!(
            "{} credentials ({})",
            labels.len(),
            labels.into_iter().collect::<Vec<_>>().join(", ")
        )
    };
    format!("{} for {}", label, list_addresses_prose(ips))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    /// Join a renderer's lines for assertions about wording. Structure — how many lines, and
    /// what is on each — is asserted against the list itself.
    fn joined(lines: &[String]) -> String {
        lines.join(" ")
    }

    #[test]
    fn no_records_produces_no_warning() {
        assert!(render_incomplete_snmp_walks(&[]).is_empty());
        assert!(render_incomplete_interface_walks(&[]).is_empty());
        assert!(render_credential_issues(&[]).is_empty());
    }

    /// The reported problem: fifteen hosts produced fifteen paragraphs. One line, always.
    #[test]
    fn many_hosts_sharing_a_failure_collapse_onto_one_line() {
        let records: Vec<IncompleteSnmpWalk> = (1..=15)
            .flat_map(|n| {
                let addr = ip(&format!("192.168.210.{n}"));
                [
                    IncompleteSnmpWalk {
                        ip: addr,
                        group: "bridge forwarding",
                        returned_any: false,
                    },
                    IncompleteSnmpWalk {
                        ip: addr,
                        group: "VLAN membership",
                        returned_any: false,
                    },
                ]
            })
            .collect();

        let lines = render_incomplete_snmp_walks(&records);
        let msg = joined(&lines);
        // All 15 share the same two groups, so they collapse onto one line rather than
        // producing a paragraph each.
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(msg.contains("bridge forwarding"));
        assert!(msg.contains("VLAN membership"));
        // Capped, but says so rather than stopping silently. Prose form, since the addresses
        // are the subject of the sentence.
        assert!(msg.contains(", and 5 more returned no "), "{msg}");
    }

    /// A walk that returned nothing and one that was truncated are different problems, and the
    /// old single phrasing ("stopped responding partway through") described only the second.
    #[test]
    fn an_empty_walk_reads_differently_from_a_truncated_one() {
        let truncated = joined(&render_incomplete_snmp_walks(&[IncompleteSnmpWalk {
            ip: ip("10.0.0.1"),
            group: "bridge forwarding",
            returned_any: true,
        }]));
        let empty = joined(&render_incomplete_snmp_walks(&[IncompleteSnmpWalk {
            ip: ip("10.0.0.1"),
            group: "bridge forwarding",
            returned_any: false,
        }]));

        // Each line makes exactly one claim about the walk, never both.
        assert!(
            truncated.contains("did not finish reporting"),
            "{truncated}"
        );
        assert!(!truncated.contains("returned no "), "{truncated}");
        assert!(
            empty.contains("returned no bridge forwarding data at all"),
            "{empty}"
        );
        assert!(!empty.contains("did not finish reporting"), "{empty}");
    }

    /// Pins the exact copy for the customer's reported scenario, because this string is the
    /// entire user-visible output of the feature — a refactor that quietly degrades it into
    /// something unreadable would otherwise pass every other test here.
    #[test]
    fn the_unreachable_controller_message_reads_as_intended() {
        let lines = render_credential_issues(&[CredentialIssue {
            label: "UniFi controller connection",
            ip: ip("192.168.50.2"),
            reason: CredentialIssueReason::TargetNotScanned,
        }]);

        assert_eq!(
            lines,
            vec![
                "The UniFi controller connection credential for 192.168.50.2 was never \
                 contacted because its address is not on any subnet this scan covers — add the \
                 subnet to the discovery, or move the credential to a host inside it."
                    .to_string()
            ]
        );
    }

    /// Reproduces the three separate paragraphs a real scan emitted before the ifTable warning
    /// was aggregated, and pins that they now collapse to one line while still distinguishing
    /// "interfaces are missing" from "some fields are blank".
    #[test]
    fn interface_walks_split_by_meaning_one_line_each() {
        let lines = render_incomplete_interface_walks(&[
            IncompleteInterfaceWalk {
                ip: ip("192.168.7.233"),
                collected: 3,
                set_complete: true,
            },
            IncompleteInterfaceWalk {
                ip: ip("192.168.7.242"),
                collected: 17,
                set_complete: false,
            },
            IncompleteInterfaceWalk {
                ip: ip("192.168.7.235"),
                collected: 3,
                set_complete: false,
            },
        ]);
        let msg = joined(&lines);

        // One line per distinct failure, so the UI can bullet them.
        assert_eq!(lines.len(), 2, "{lines:?}");
        // The addresses lead the sentence rather than trailing it after a colon.
        assert!(msg.contains("192.168.7.235 and 192.168.7.242 stopped responding partway through"));
        // ...and the attribute-only device is reported separately, which is the distinction
        // that matters — it has all its interfaces, just not all their fields.
        assert!(
            msg.contains("192.168.7.233 returned every SNMP interface but"),
            "{msg}"
        );
    }

    #[test]
    fn address_lists_read_as_english() {
        let render = |addrs: &[&str]| {
            joined(&render_incomplete_interface_walks(
                &addrs
                    .iter()
                    .map(|a| IncompleteInterfaceWalk {
                        ip: ip(a),
                        collected: 1,
                        set_complete: false,
                    })
                    .collect::<Vec<_>>(),
            ))
        };

        assert!(render(&["10.0.0.1"]).contains("10.0.0.1 stopped"));
        assert!(render(&["10.0.0.1", "10.0.0.2"]).contains("10.0.0.1 and 10.0.0.2 stopped"));
        assert!(
            render(&["10.0.0.1", "10.0.0.2", "10.0.0.3"])
                .contains("10.0.0.1, 10.0.0.2, and 10.0.0.3 stopped")
        );

        // Past the cap the remainder becomes the final list item, so it still reads as prose
        // instead of stopping mid-sentence or trailing a parenthetical.
        let many: Vec<String> = (1..=13).map(|n| format!("10.0.0.{n}")).collect();
        let msg = render(&many.iter().map(String::as_str).collect::<Vec<_>>());
        assert!(msg.contains(", and 3 more stopped"), "{msg}");
    }

    #[test]
    fn interface_walks_of_one_kind_omit_the_other_clause() {
        let lines = render_incomplete_interface_walks(&[IncompleteInterfaceWalk {
            ip: ip("10.0.0.1"),
            collected: 5,
            set_complete: true,
        }]);
        assert_eq!(lines.len(), 1);
        let msg = joined(&lines);
        assert!(!msg.contains("some interfaces are missing"));
        assert!(msg.contains("descriptions or speeds may be blank"));
    }

    #[test]
    fn each_credential_reason_names_its_own_fix() {
        let issues = vec![
            CredentialIssue {
                label: "UniFi controller connection",
                ip: ip("10.9.0.1"),
                reason: CredentialIssueReason::TargetNotScanned,
            },
            CredentialIssue {
                label: "UniFi controller connection",
                ip: ip("10.0.0.7"),
                reason: CredentialIssueReason::GateClosed {
                    ports: vec![PortType::new_tcp(443)],
                },
            },
        ];

        let lines = render_credential_issues(&issues);
        // One line per reason: the two here have different fixes.
        assert_eq!(lines.len(), 2, "{lines:?}");
        let msg = joined(&lines);
        assert!(msg.contains("10.9.0.1"));
        assert!(msg.contains("not on any subnet"));
        assert!(msg.contains("10.0.0.7"));
        assert!(msg.contains("port 443 was not open"));
    }
}
