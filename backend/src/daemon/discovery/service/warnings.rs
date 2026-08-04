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

use strum::EnumCount;

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

/// Why a group of SNMP data came up short.
///
/// The renderer used to guess — its empty-walk line said the query "usually timed out" — because
/// the reason stopped at the walk and never travelled with the result. Each of these calls for a
/// different sentence, and two of them are not faults at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShortfallReason {
    /// Stopped at our own entry cap. The device is fine and larger than we read.
    ///
    /// Carries the limit so the renderer can name it without reaching into the SNMP module for
    /// a constant — the number is the whole point of the sentence.
    EntryCap { limit: usize },
    /// The agent stopped answering partway.
    NoAnswer,
    /// The agent answered out of step with what was asked — a stale or non-advancing response.
    Desynchronised,
    /// The agent does not implement this MIB. Not a fault, and no later scan will change it.
    Unsupported,
}

/// An SNMP data group a walk may come up short on.
///
/// An enum rather than a free string so the renderer below is exhaustive: every group has to
/// declare which consequence sentence describes it, and a new one cannot be added without
/// choosing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnmpWalkGroup {
    Lldp,
    Cdp,
    /// `dot1dBasePortIfIndex` — the bridge-port numbering both groups below are keyed by.
    BridgePortNumbering,
    BridgeForwarding,
    VlanMembership,
}

impl SnmpWalkGroup {
    /// Noun phrase, used as the object of a sentence.
    fn label(self) -> &'static str {
        match self {
            Self::Lldp => "LLDP neighbours",
            Self::Cdp => "CDP neighbours",
            Self::BridgePortNumbering => "SNMP bridge-port numbering",
            Self::BridgeForwarding => "bridge forwarding",
            Self::VlanMembership => "VLAN membership",
        }
    }

    /// Whether an empty result here means the device does not implement the table at all,
    /// rather than that a read of an implemented table fell short.
    ///
    /// Only true for the bridge-port numbering, because it is the *root* of the bridge MIB:
    /// a switch that serves none of it has no MAC-address table or VLAN membership to offer
    /// over SNMP at all, and telling its operator that "previously discovered values were
    /// kept" promises a refresh that will never come.
    fn absence_means_unsupported(self) -> bool {
        matches!(self, Self::BridgePortNumbering)
    }
}

/// LLDP neighbours whose local port could not be matched to an interface on the device.
///
/// Kept apart from [`IncompleteSnmpWalk`] because it is a different kind of problem and reads
/// nothing like one. The walk succeeded — the neighbours are there — but their `lldpLocPortNum`
/// could not be translated to an `ifIndex`, so each one is attached to whatever interface holds
/// that number, or to nothing. The result is a map that looks complete and is wrong in a
/// specific place, which no "data was incomplete" sentence describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedLldpPorts {
    pub ip: IpAddr,
    pub unresolved: usize,
    pub total: usize,
}

/// One line naming the devices, or empty if there were none.
pub fn render_unresolved_lldp_ports(records: &[UnresolvedLldpPorts]) -> Vec<String> {
    let affected: BTreeSet<IpAddr> = records
        .iter()
        .filter(|r| r.unresolved > 0)
        .map(|r| r.ip)
        .collect();
    if affected.is_empty() {
        return Vec::new();
    }
    let total: usize = records.iter().map(|r| r.unresolved).sum();
    vec![format!(
        "{} reported {total} LLDP neighbour{} whose local port does not match any interface on \
         the device, so those links may be drawn against the wrong port. This usually means the \
         switch numbers its LLDP ports separately from its interfaces.",
        list_addresses_prose(&affected),
        if total == 1 { "" } else { "s" }
    )]
}

/// Neighbour records a device served without the identifier that makes them usable.
///
/// A third kind of problem again, and it must not read like either neighbour above. The walk
/// finished and the rows arrived; they are missing a mandatory field — the LLDP chassis ID
/// (IEEE 802.1AB) or the CDP device id — which is what L2 resolution matches the far end on, so
/// they are discarded rather than written over good data.
///
/// The distinction that matters to an operator: **no rescan will fix this**. "Incomplete" invites
/// a retry, and a retry against a switch whose firmware serves malformed records produces exactly
/// the same result. It reached us as a bare `dropped=N` in the daemon log, which is how a customer
/// ended up hand-running snmpwalk to tell us what their switch had sent (GH #668).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedNeighbours {
    pub ip: IpAddr,
    pub group: SnmpWalkGroup,
    pub discarded: usize,
    /// Records that survived, so the line can say whether this cost the device some of its
    /// topology or all of it.
    pub kept: usize,
}

/// One line naming the devices, or empty if there were none.
pub fn render_malformed_neighbours(records: &[MalformedNeighbours]) -> Vec<String> {
    let affected: BTreeSet<IpAddr> = records
        .iter()
        .filter(|r| r.discarded > 0)
        .map(|r| r.ip)
        .collect();
    if affected.is_empty() {
        return Vec::new();
    }
    let discarded: usize = records.iter().map(|r| r.discarded).sum();
    // "None left" and "some left" are different situations for whoever reads this: the first
    // means the device is absent from L2 Physical, the second that it is there but incomplete.
    let kept: usize = records.iter().map(|r| r.kept).sum();
    let consequence = if kept == 0 {
        "so those devices contribute no physical links at all"
    } else {
        "so some of their physical links are missing"
    };
    vec![format!(
        "{} reported {discarded} neighbour record{} without the identifier needed to match the \
         far end, {consequence}. The devices answered in full — the records themselves are \
         incomplete, so rescanning will not change this.",
        list_addresses_prose(&affected),
        if discarded == 1 { "" } else { "s" }
    )]
}

/// A device whose VLAN table was read and could not be recorded.
///
/// Not a shortfall in a walk, and it must not read like one: the switch answered in full, and the
/// upsert to the server failed. Reporting it as "VLAN membership was incomplete" would send an
/// operator to inspect a switch that did nothing wrong, when the fault is on our side of the
/// wire. The consequence is real either way — every interface on that device loses its VLAN ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlanRecordingFailed {
    pub ip: IpAddr,
}

/// One line naming the devices, or empty if there were none.
pub fn render_vlan_recording_failures(records: &[VlanRecordingFailed]) -> Vec<String> {
    let affected: BTreeSet<IpAddr> = records.iter().map(|r| r.ip).collect();
    if affected.is_empty() {
        return Vec::new();
    }
    vec![format!(
        "The VLANs reported by {} could not be saved, so VLAN membership is missing from their \
         interfaces. The devices answered correctly — this is a failure recording the result, and \
         the daemon log has the underlying error.",
        list_addresses_prose(&affected)
    )]
}

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
    pub group: SnmpWalkGroup,
    pub returned_any: bool,
    /// Why it came up short, when the walk could establish that.
    pub reason: Option<ShortfallReason>,
}

/// What one host's SNMP collection managed to read, per group.
///
/// `complete` mirrors [`SnmpCollection::complete`] on the daemon side; `returned_any` is whether
/// that group produced any records at all.
#[derive(Debug, Clone, Copy, Default)]
pub struct SnmpGroupOutcome {
    pub complete: bool,
    pub returned_any: bool,
    pub reason: Option<ShortfallReason>,
}

/// Per-group outcomes for one host, as [`snmp_walk_shortfalls`] consumes them.
#[derive(Debug, Clone, Copy, Default)]
pub struct SnmpCollectionOutcome {
    pub lldp: SnmpGroupOutcome,
    pub cdp: SnmpGroupOutcome,
    pub bridge_port_numbering: SnmpGroupOutcome,
    pub bridge_forwarding: SnmpGroupOutcome,
    pub vlan_membership: SnmpGroupOutcome,
}

/// The groups worth reporting for one host.
///
/// Bridge forwarding and VLAN membership are both keyed by `dot1dBasePortIfIndex`, so when *that*
/// walk fails they are marked incomplete having attempted nothing of their own. Reporting all
/// three told an operator their switch had failed three ways when it had failed once, and pointed
/// at the two tables that are consequences rather than the one that is the cause. Report the
/// cause, and stay silent about the derived groups unless they failed on their own account.
pub fn snmp_walk_shortfalls(ip: IpAddr, outcome: SnmpCollectionOutcome) -> Vec<IncompleteSnmpWalk> {
    let root_failed = !outcome.bridge_port_numbering.complete;
    [
        (SnmpWalkGroup::Lldp, outcome.lldp, true),
        (SnmpWalkGroup::Cdp, outcome.cdp, true),
        (
            SnmpWalkGroup::BridgePortNumbering,
            outcome.bridge_port_numbering,
            true,
        ),
        (
            SnmpWalkGroup::BridgeForwarding,
            outcome.bridge_forwarding,
            !root_failed,
        ),
        (
            SnmpWalkGroup::VlanMembership,
            outcome.vlan_membership,
            !root_failed,
        ),
    ]
    .into_iter()
    .filter(|(_, group, report)| *report && !group.complete)
    .map(|(group, outcome, _)| IncompleteSnmpWalk {
        ip,
        group,
        returned_any: outcome.returned_any,
        reason: outcome.reason,
    })
    .collect()
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

    // (returned_any, reason, group) -> devices, then invert so identical device sets collapse.
    //
    // `returned_any` and `reason` are part of the key, not sentences appended afterwards. Both are
    // properties of one group's walk, so aggregating them per *device* produced lines that
    // contradicted themselves — "192.168.7.230 did not finish reporting VLAN membership or bridge
    // forwarding" immediately followed by "192.168.7.230 returned nothing at all", which reads as
    // two incompatible claims about the same walk. Keyed this way, each line makes one claim.
    type Key = (bool, Option<ShortfallReason>, SnmpWalkGroup);
    let mut devices_by_group: BTreeMap<Key, BTreeSet<IpAddr>> = BTreeMap::new();
    for r in records {
        devices_by_group
            .entry((r.returned_any, r.reason, r.group))
            .or_default()
            .insert(r.ip);
    }
    let mut groups_by_devices: BTreeMap<
        (bool, Option<ShortfallReason>, BTreeSet<IpAddr>),
        Vec<SnmpWalkGroup>,
    > = BTreeMap::new();
    for ((returned_any, reason, group), ips) in devices_by_group {
        groups_by_devices
            .entry((returned_any, reason, ips))
            .or_default()
            .push(group);
    }

    groups_by_devices
        .iter()
        .map(|((returned_any, reason, ips), groups)| {
            let who = list_addresses_prose(ips);
            let labels: Vec<&str> = groups.iter().map(|g| g.label()).collect();
            let what = join_prose(&labels);
            match reason {
                // Our limit, not their fault, and it will recur on every scan of a device this
                // size. "Did not finish reporting" invited an operator to go looking for a
                // problem on hardware that was answering perfectly.
                Some(ShortfallReason::EntryCap { limit }) => format!(
                    "{who} has more {what} than one scan reads — collection stops at {limit} \
                     entries per table, so the rest were not read. The data recorded is correct \
                     as far as it goes."
                ),
                // The device does not implement it. No refresh to promise, nothing to fix.
                Some(ShortfallReason::Unsupported) => format!(
                    "{who} does not implement {what} over SNMP, so it cannot be read from the \
                     device at all. Previously discovered values were kept."
                ),
                // Answering out of step is a transient worth naming: it is the signature of a
                // busy agent racing itself, and points somewhere completely different from a
                // device that has simply gone quiet. Ahead of the `returned_any` arm below,
                // because a desynchronised walk usually *does* return rows before it goes wrong
                // — putting it after meant it never matched.
                Some(ShortfallReason::Desynchronised) => format!(
                    "{who} answered out of step with what was asked for {what}, which usually \
                     means the agent is under load. Previously discovered values were kept and \
                     refresh on the next complete scan."
                ),
                _ if *returned_any => format!(
                    "{who} did not finish reporting {what}, so previously discovered values were \
                     kept rather than overwritten and refresh on the next complete scan."
                ),
                _ if groups.iter().all(|g| g.absence_means_unsupported()) => format!(
                    "{who} did not answer for {what}, which these switches commonly do not \
                     implement. Their MAC-address-table and VLAN membership cannot be read over \
                     SNMP; a UniFi controller integration reports the same data where one manages \
                     the device."
                ),
                _ => format!(
                    "{who} returned no {what} data at all — the device stopped answering rather \
                     than reporting that it has none. Previously discovered values were kept \
                     rather than overwritten and refresh on the next complete scan."
                ),
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

/// How a credential attempt that actually ran came out.
///
/// Deliberately a *subset* of [`CredentialIssueReason`]: an integration observes only what
/// happened on the wire, and has no way to know about the dispatch-level reasons ("never
/// scanned", "gate closed"). Splitting the two means an integration cannot report a reason it
/// cannot have established.
///
/// Every variant maps to a different thing for the operator to do, which is the test for whether
/// one belongs here. "The device refused my password" and "nothing was listening" arrive as the
/// same empty result and send an operator to opposite ends of the problem.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    strum_macros::EnumCount,
    strum_macros::EnumIter,
)]
pub enum AttemptOutcome {
    /// The endpoint answered and refused the credential. The credential is wrong.
    Rejected,
    /// Nothing was listening: connection refused, or no route.
    Unreachable,
    /// Connected, then never answered inside the timeout.
    TimedOut,
    /// Something answered, but it is not this service — usually the wrong port.
    NotThisService,
    /// TLS negotiation failed. Distinct from [`Self::Rejected`] because the fix is a trust
    /// setting, not a password.
    TlsFailed,
    /// The credential itself is incomplete or unusable — our configuration rather than their
    /// device, and the only outcome an operator fixes in Scanopy rather than on the network.
    Malformed,
    /// The credential worked and the collection after it did not. The host's data is missing
    /// rather than merely stale, which is why it is worth its own line.
    CollectionFailed,
    /// The credential worked, the collection started, and the time limit stopped it before it
    /// finished.
    ///
    /// Distinct from [`Self::TimedOut`], which is the address never answering, and from
    /// [`Self::CollectionFailed`], which is the collection erroring: here the service answered
    /// perfectly well and there was simply more to read than there was time to read it. Reporting
    /// this as [`Self::TimedOut`] told operators to check that the service was listening, on a
    /// host where it had just enumerated 77 containers (GH #650).
    CollectionTimedOut,
    /// The scan was cancelled. Never rendered — the user stopped it, so it is not a finding.
    Cancelled,
}

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
    /// The attempt ran and did not succeed.
    ///
    /// Replaces a `ProbeRejected` variant that named one outcome and was used for all of them —
    /// every integration flattened cancellation, a closed port, a TLS failure and a genuinely
    /// wrong password into the same "was rejected" line.
    Attempted {
        outcome: AttemptOutcome,
        /// The integration's own diagnostic, already phrased for an operator.
        message: String,
    },
}

/// One IP-targeted credential that did not work, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialIssue {
    /// Human label for the credential type, from `CredentialQueryPayload::discovery_label`.
    pub label: &'static str,
    pub ip: IpAddr,
    pub reason: CredentialIssueReason,
}

/// Whether a finished credential attempt is worth telling the operator about, and as what.
///
/// Pure, and separated from the dispatch that calls it for the same reason
/// [`snmp_walk_shortfalls`] is: the policy is the part worth testing, and testing it in place
/// would mean standing up a whole `DaemonDiscoveryService`. Both the probe path and the execute
/// path go through here, so the two cannot drift apart.
///
/// Two things are deliberately silent:
///
/// - **A network default that failed.** It is broadcast at every address in the subnet, so
///   failing is its normal condition — reporting it would bury the real findings under a line per
///   unresponsive host on any /24 sweep. Only a credential the user pinned to this host is news.
/// - **A cancelled attempt.** The operator stopped the scan; nothing about that is a finding, and
///   it used to be reported as a rejected credential.
pub fn issue_for_attempt(
    label: &'static str,
    ip: IpAddr,
    outcome: AttemptOutcome,
    message: String,
    user_assigned: bool,
) -> Option<CredentialIssue> {
    if !user_assigned || outcome == AttemptOutcome::Cancelled {
        return None;
    }
    Some(CredentialIssue {
        label,
        ip,
        reason: CredentialIssueReason::Attempted { outcome, message },
    })
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

    // One line per outcome, because the whole point of the outcome is that the fix differs.
    for outcome in ATTEMPT_ORDER {
        let Some(advice) = outcome.advice() else {
            continue;
        };
        let matching: Vec<&CredentialIssue> = issues
            .iter()
            .filter(|i| {
                attempt_outcome(&i.reason) == Some(outcome)
                    && !already_covered_by_address_line(issues, i)
            })
            .collect();
        if matching.is_empty() {
            continue;
        }
        // The first message is representative and already phrased for an operator; grouping by
        // outcome is what carries the rest, so the line stays one sentence.
        let first = matching
            .iter()
            .find_map(|i| match &i.reason {
                CredentialIssueReason::Attempted { message, .. } => Some(message.as_str()),
                _ => None,
            })
            .unwrap_or("no further detail");
        let ips: BTreeSet<IpAddr> = matching.iter().map(|i| i.ip).collect();
        parts.push(format!(
            "{} {} ({first})",
            describe_targets(issues, &ips),
            advice
        ));
    }

    parts.into_iter().map(|p| format!("{p}.")).collect()
}

/// The order the outcome lines appear in: most actionable first, so the line an operator can do
/// something about is not buried under the ones describing the network.
///
/// Fixed-length rather than a slice, so a new [`AttemptOutcome`] that is not listed here is an
/// array-length compile error. As a slice this compiled fine with a variant missing and simply
/// never rendered it — the same failure mode [`AttemptOutcome::advice`] was made a `match` to
/// avoid, and the one this constant still had.
const ATTEMPT_ORDER: [AttemptOutcome; AttemptOutcome::COUNT] = [
    AttemptOutcome::Rejected,
    AttemptOutcome::Malformed,
    AttemptOutcome::TlsFailed,
    AttemptOutcome::NotThisService,
    AttemptOutcome::CollectionFailed,
    AttemptOutcome::CollectionTimedOut,
    AttemptOutcome::Unreachable,
    AttemptOutcome::TimedOut,
    AttemptOutcome::Cancelled,
];

impl AttemptOutcome {
    /// What to tell the operator, or `None` for an outcome that is not a finding.
    ///
    /// A `match` rather than a lookup table, so a new outcome cannot be added without deciding
    /// how it reads. As a table this compiled fine with a variant missing and simply never
    /// mentioned it — the failure mode being that an operator hits a problem the product has a
    /// name for and is told nothing at all.
    fn advice(self) -> Option<&'static str> {
        match self {
            Self::Rejected => {
                Some("was refused — check the username, password or community string")
            }
            Self::Malformed => Some("is incomplete and could not be used — re-enter it"),
            Self::TlsFailed => Some(
                "could not negotiate TLS — if the appliance serves a self-signed certificate, \
                 turn on \"accept invalid certificates\" in the daemon's scan settings",
            ),
            Self::NotThisService => Some(
                "reached something that is not the expected service — check the port on the \
                 credential",
            ),
            Self::CollectionFailed => Some(
                "authenticated and then failed while collecting, so this host's data is missing \
                 rather than out of date",
            ),
            // Deliberately outside the `already_covered_by_address_line` suppression below: no
            // address-level line says this, because the address answered fine. Suppressing it
            // the way `TimedOut` is suppressed is what made a 300s container scan silent.
            Self::CollectionTimedOut => Some(
                "authenticated and then ran out of time before it finished collecting — rescan \
                 this host on its own, or narrow what the scan covers",
            ),
            // The user stopped the scan. Not a finding.
            Self::Cancelled => None,
            // Rendered only when no address-level line already said it — see
            // `already_covered_by_address_line`. Suppressing these unconditionally was wrong: it
            // assumed a sweep, where "nothing answered there" is reported per address. The
            // daemon-host phase has no such line, because 127.0.0.1 is always up, so a Docker
            // socket credential pointing at a path that does not exist stayed silent even after
            // its issue was correctly built and delivered.
            Self::Unreachable | Self::TimedOut => Some(
                "could not be reached at that address — check the address, port and that the \
                 service is listening",
            ),
        }
    }
}

/// Whether an address-level line in the same batch already says what this one would.
///
/// "Nothing answered at 10.0.0.5" and "the SNMP credential for 10.0.0.5 could not be reached" are
/// the same fact, and on a sweep the first is reported per address already. So the second is
/// dropped — but *only* when the first is actually present. Deciding this by outcome alone, as
/// this used to, silently swallowed every unreachable-credential report from the daemon-host
/// phase, where no address-level line exists because 127.0.0.1 is always up.
fn already_covered_by_address_line(issues: &[CredentialIssue], issue: &CredentialIssue) -> bool {
    if !matches!(
        attempt_outcome(&issue.reason),
        Some(AttemptOutcome::Unreachable | AttemptOutcome::TimedOut)
    ) {
        return false;
    }
    issues.iter().any(|other| {
        other.ip == issue.ip
            && matches!(
                other.reason,
                CredentialIssueReason::TargetNotScanned
                    | CredentialIssueReason::TargetNotResponding
            )
    })
}

fn attempt_outcome(reason: &CredentialIssueReason) -> Option<AttemptOutcome> {
    match reason {
        CredentialIssueReason::Attempted { outcome, .. } => Some(*outcome),
        _ => None,
    }
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

    /// GH #650. A container scan that authenticated, enumerated 77 containers and then ran out of
    /// time was reported with the `TimedOut` wording — "could not be reached at that address,
    /// check the service is listening" — and on a sweep was suppressed entirely, because
    /// `already_covered_by_address_line` treats `TimedOut` as saying the same thing as an
    /// address-level line. Neither may happen to a collection that ran out of time.
    #[test]
    fn a_collection_that_ran_out_of_time_is_not_reported_as_an_unreachable_host() {
        let addr = ip("10.1.1.99");
        let issues = vec![
            CredentialIssue {
                label: "Podman socket connection",
                ip: addr,
                reason: CredentialIssueReason::Attempted {
                    outcome: AttemptOutcome::CollectionTimedOut,
                    message: "Integration timed out after 300s".to_string(),
                },
            },
            // An address-level line for the same address, which is what suppresses `TimedOut`.
            CredentialIssue {
                label: "SNMP queries",
                ip: addr,
                reason: CredentialIssueReason::TargetNotResponding,
            },
        ];

        let rendered = render_credential_issues(&issues);
        let text = joined(&rendered);

        assert!(
            text.contains("ran out of time"),
            "a collection that hit its time limit must still be reported: {text}"
        );
        assert!(
            !text.contains("check the address, port and that the service is listening"),
            "the service answered fine; sending the operator to check it is listening is the \
             misdiagnosis this outcome exists to prevent: {text}"
        );
    }

    /// `ATTEMPT_ORDER`'s length is compiler-checked against the variant count, but an entry that
    /// is present and unreachable would still render nothing. Pair the compile-time check with a
    /// behavioural one so both halves of "every outcome the product names gets said out loud"
    /// hold.
    #[test]
    fn every_reportable_outcome_produces_a_line() {
        use strum::IntoEnumIterator;

        for outcome in AttemptOutcome::iter() {
            let issues = vec![CredentialIssue {
                label: "SNMP queries",
                ip: ip("10.0.0.5"),
                reason: CredentialIssueReason::Attempted {
                    outcome,
                    message: "diagnostic".to_string(),
                },
            }];
            let rendered = render_credential_issues(&issues);

            match outcome.advice() {
                Some(_) => assert_eq!(
                    rendered.len(),
                    1,
                    "{outcome:?} has advice but rendered {rendered:?}"
                ),
                None => assert!(
                    rendered.is_empty(),
                    "{outcome:?} has no advice but rendered {rendered:?}"
                ),
            }
        }
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
                        group: SnmpWalkGroup::BridgeForwarding,
                        returned_any: false,
                        reason: None,
                    },
                    IncompleteSnmpWalk {
                        ip: addr,
                        group: SnmpWalkGroup::VlanMembership,
                        returned_any: false,
                        reason: None,
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
            group: SnmpWalkGroup::BridgeForwarding,
            returned_any: true,
            reason: None,
        }]));
        let empty = joined(&render_incomplete_snmp_walks(&[IncompleteSnmpWalk {
            ip: ip("10.0.0.1"),
            group: SnmpWalkGroup::BridgeForwarding,
            returned_any: false,
            reason: None,
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

    /// A switch that serves no bridge MIB at all (the Ubiquiti USW-Pro-Max) hits this every
    /// scan, for ever. The generic empty-walk sentence promised a refresh on the next complete
    /// scan that could never arrive, and blamed a timeout on hardware that answered everything
    /// else promptly.
    #[test]
    fn an_unimplemented_table_does_not_promise_a_later_refresh() {
        let msg = joined(&render_incomplete_snmp_walks(&[IncompleteSnmpWalk {
            ip: ip("192.168.210.217"),
            group: SnmpWalkGroup::BridgePortNumbering,
            returned_any: false,
            reason: None,
        }]));

        assert!(msg.contains("192.168.210.217 did not answer for"), "{msg}");
        assert!(msg.contains("commonly do not implement"), "{msg}");
        // The two claims that were wrong for this device.
        assert!(!msg.contains("refresh on the next complete scan"), "{msg}");
        assert!(!msg.contains("timed out"), "{msg}");
    }

    /// A switch with no bridge MIB fails three groups at once, but only one of them is a
    /// finding: the other two never ran. Reporting all three read as three separate faults.
    #[test]
    fn a_failed_bridge_port_walk_suppresses_the_groups_keyed_by_it() {
        let shortfalls = snmp_walk_shortfalls(
            ip("192.168.210.217"),
            SnmpCollectionOutcome {
                lldp: SnmpGroupOutcome {
                    complete: true,
                    returned_any: false,
                    reason: None,
                },
                cdp: SnmpGroupOutcome {
                    complete: true,
                    returned_any: false,
                    reason: None,
                },
                // Everything below is a consequence of this one failure.
                bridge_port_numbering: SnmpGroupOutcome::default(),
                bridge_forwarding: SnmpGroupOutcome::default(),
                vlan_membership: SnmpGroupOutcome::default(),
            },
        );

        let groups: Vec<SnmpWalkGroup> = shortfalls.iter().map(|s| s.group).collect();
        assert_eq!(groups, vec![SnmpWalkGroup::BridgePortNumbering]);
    }

    /// The suppression is scoped to the root failing. When the bridge-port walk succeeds, a
    /// genuinely short FDB walk is the device's own finding and must still be reported.
    #[test]
    fn a_short_fdb_walk_is_reported_when_the_bridge_port_walk_succeeded() {
        let shortfalls = snmp_walk_shortfalls(
            ip("192.168.210.217"),
            SnmpCollectionOutcome {
                lldp: SnmpGroupOutcome {
                    complete: true,
                    returned_any: true,
                    reason: None,
                },
                cdp: SnmpGroupOutcome {
                    complete: true,
                    returned_any: false,
                    reason: None,
                },
                bridge_port_numbering: SnmpGroupOutcome {
                    complete: true,
                    returned_any: true,
                    reason: None,
                },
                bridge_forwarding: SnmpGroupOutcome {
                    complete: false,
                    returned_any: true,
                    reason: None,
                },
                vlan_membership: SnmpGroupOutcome {
                    complete: true,
                    returned_any: true,
                    reason: None,
                },
            },
        );

        let groups: Vec<SnmpWalkGroup> = shortfalls.iter().map(|s| s.group).collect();
        assert_eq!(groups, vec![SnmpWalkGroup::BridgeForwarding]);
        assert!(shortfalls[0].returned_any);
    }

    /// The same device, truncated rather than absent, is a different problem and must keep the
    /// truncation wording — `absence_means_unsupported` gates on the empty case alone.
    #[test]
    fn a_truncated_bridge_port_walk_still_reads_as_truncation() {
        let msg = joined(&render_incomplete_snmp_walks(&[IncompleteSnmpWalk {
            ip: ip("192.168.210.217"),
            group: SnmpWalkGroup::BridgePortNumbering,
            returned_any: true,
            reason: None,
        }]));

        assert!(msg.contains("did not finish reporting"), "{msg}");
        assert!(!msg.contains("commonly do not implement"), "{msg}");
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

    fn shortfall(ip_str: &str, reason: Option<ShortfallReason>) -> IncompleteSnmpWalk {
        IncompleteSnmpWalk {
            ip: ip(ip_str),
            group: SnmpWalkGroup::BridgeForwarding,
            // Rows arrived and then stopped, which is the case that previously swallowed every
            // one of these distinctions into "did not finish reporting".
            returned_any: true,
            reason,
        }
    }

    /// Our own limit, hit every scan on a device this size. Reporting it as a shortfall sent an
    /// operator looking for a fault on hardware that was answering perfectly, and no amount of
    /// re-scanning would have changed it.
    #[test]
    fn hitting_the_entry_cap_names_the_limit_and_does_not_imply_a_fault() {
        let msg = joined(&render_incomplete_snmp_walks(&[shortfall(
            "10.0.0.1",
            Some(ShortfallReason::EntryCap { limit: 10_000 }),
        )]));

        assert!(
            msg.contains("more bridge forwarding than one scan reads"),
            "{msg}"
        );
        assert!(
            msg.contains("10000"),
            "the limit is the point of the sentence: {msg}"
        );
        assert!(!msg.contains("did not finish reporting"), "{msg}");
        assert!(!msg.contains("stopped answering"), "{msg}");
    }

    /// The simulator's signature, and the one the sim-vs-customer comparison turns on: an agent
    /// racing itself answers with the wrong OID. That points somewhere completely different from
    /// a device that has gone quiet, so it cannot share a line with one.
    #[test]
    fn an_out_of_step_agent_reads_differently_from_a_silent_one() {
        let desynced = joined(&render_incomplete_snmp_walks(&[shortfall(
            "10.0.0.1",
            Some(ShortfallReason::Desynchronised),
        )]));
        let silent = joined(&render_incomplete_snmp_walks(&[IncompleteSnmpWalk {
            ip: ip("10.0.0.2"),
            group: SnmpWalkGroup::BridgeForwarding,
            returned_any: false,
            reason: Some(ShortfallReason::NoAnswer),
        }]));

        assert!(desynced.contains("answered out of step"), "{desynced}");
        assert!(desynced.contains("under load"), "{desynced}");
        assert!(silent.contains("stopped answering"), "{silent}");
        assert!(!silent.contains("out of step"), "{silent}");
    }

    /// A device that does not implement the MIB will never implement it, so promising a refresh
    /// on the next complete scan is a promise that cannot be kept.
    #[test]
    fn an_unsupported_table_promises_no_refresh() {
        let msg = joined(&render_incomplete_snmp_walks(&[shortfall(
            "10.0.0.1",
            Some(ShortfallReason::Unsupported),
        )]));

        assert!(msg.contains("does not implement"), "{msg}");
        assert!(!msg.contains("refresh on the next complete scan"), "{msg}");
    }

    /// This one produces a map that is *wrong* rather than incomplete — the link is drawn, against
    /// the wrong port — so it must not be phrased as missing data.
    #[test]
    fn unresolved_lldp_ports_say_the_links_may_be_wrong_not_missing() {
        let msg = joined(&render_unresolved_lldp_ports(&[UnresolvedLldpPorts {
            ip: ip("192.168.7.238"),
            unresolved: 3,
            total: 4,
        }]));

        assert!(msg.contains("192.168.7.238"), "{msg}");
        assert!(msg.contains("3 LLDP neighbours"), "{msg}");
        assert!(msg.contains("wrong port"), "{msg}");
    }

    /// Everything resolved is the normal case on most vendors, and must be silent.
    #[test]
    fn fully_resolved_lldp_ports_are_not_reported() {
        assert!(
            render_unresolved_lldp_ports(&[UnresolvedLldpPorts {
                ip: ip("10.0.0.1"),
                unresolved: 0,
                total: 6,
            }])
            .is_empty()
        );
    }

    /// GH #668. The one thing this line has to get across is that retrying is pointless — the
    /// device answered in full and the records themselves are unusable. "Incomplete" invites a
    /// rescan that produces the identical result.
    #[test]
    fn malformed_neighbours_promise_no_refresh() {
        let msg = joined(&render_malformed_neighbours(&[MalformedNeighbours {
            ip: ip("192.168.7.244"),
            group: SnmpWalkGroup::Lldp,
            discarded: 14,
            kept: 0,
        }]));

        assert!(msg.contains("192.168.7.244"), "{msg}");
        assert!(msg.contains("14 neighbour records"), "{msg}");
        assert!(msg.contains("rescanning will not change this"), "{msg}");
        assert!(msg.contains("no physical links at all"), "{msg}");
    }

    /// Losing some neighbours and losing all of them put the device in different places on the
    /// map, so they must not read the same.
    #[test]
    fn a_partial_loss_does_not_claim_the_device_has_no_links() {
        let msg = joined(&render_malformed_neighbours(&[MalformedNeighbours {
            ip: ip("10.0.0.1"),
            group: SnmpWalkGroup::Lldp,
            discarded: 2,
            kept: 5,
        }]));

        assert!(
            msg.contains("some of their physical links are missing"),
            "{msg}"
        );
    }

    /// The common case by far — every device reports this group on every scan, and a device that
    /// discarded nothing must stay silent.
    #[test]
    fn discarding_nothing_is_not_reported() {
        assert!(
            render_malformed_neighbours(&[MalformedNeighbours {
                ip: ip("10.0.0.1"),
                group: SnmpWalkGroup::Cdp,
                discarded: 0,
                kept: 3,
            }])
            .is_empty()
        );
    }

    /// The switch did nothing wrong. Blaming it would send an operator to the wrong end of the
    /// problem entirely.
    #[test]
    fn a_failed_vlan_save_does_not_blame_the_device() {
        let msg = joined(&render_vlan_recording_failures(&[VlanRecordingFailed {
            ip: ip("10.0.0.1"),
        }]));

        assert!(msg.contains("could not be saved"), "{msg}");
        assert!(msg.contains("answered correctly"), "{msg}");
    }

    fn attempted(ip_str: &str, outcome: AttemptOutcome) -> CredentialIssue {
        CredentialIssue {
            label: "UniFi controller connection",
            ip: ip(ip_str),
            reason: CredentialIssueReason::Attempted {
                outcome,
                message: "detail from the integration".to_string(),
            },
        }
    }

    fn decide(outcome: AttemptOutcome, user_assigned: bool) -> Option<CredentialIssue> {
        issue_for_attempt(
            "SNMP queries",
            ip("10.0.0.1"),
            outcome,
            "detail from the integration".to_string(),
            user_assigned,
        )
    }

    /// A network default is tried at every address in the subnet, so failing is its normal
    /// condition. Reporting it would put a line per unresponsive host into the notification and
    /// bury the credentials the user actually configured.
    #[test]
    fn a_failing_network_default_is_not_a_finding() {
        assert!(decide(AttemptOutcome::Rejected, false).is_none());
        assert!(decide(AttemptOutcome::TlsFailed, false).is_none());
    }

    /// The operator stopped the scan. This used to be reported as a rejected credential — the
    /// container probe returned the same failure for cancellation as for a refused socket.
    #[test]
    fn a_cancelled_attempt_is_not_a_finding_even_when_configured() {
        assert!(decide(AttemptOutcome::Cancelled, true).is_none());
    }

    /// The case the whole mechanism exists for: a credential the user pinned to this host was
    /// refused, and both the outcome and the integration's own wording survive to the renderer.
    #[test]
    fn a_configured_credential_that_was_refused_is_reported_in_full() {
        let issue = decide(AttemptOutcome::Rejected, true).expect("should be reported");
        assert_eq!(issue.ip, ip("10.0.0.1"));
        assert_eq!(issue.label, "SNMP queries");
        match issue.reason {
            CredentialIssueReason::Attempted { outcome, message } => {
                assert_eq!(outcome, AttemptOutcome::Rejected);
                assert_eq!(message, "detail from the integration");
            }
            other => panic!("expected an Attempted reason, got {other:?}"),
        }
    }

    /// `execute` only runs after a credential has already worked against this host, so dispatch
    /// passes `user_assigned: true` there unconditionally — there is no broadcast-noise case to
    /// suppress. This pins that a collection failure survives the decision.
    #[test]
    fn a_collection_failure_after_a_working_credential_is_reported() {
        assert!(decide(AttemptOutcome::CollectionFailed, true).is_some());
    }

    /// The reason the outcome exists at all. Every one of these was a single "was rejected" line
    /// before, so an operator with a self-signed certificate, a wrong port and a wrong password
    /// was told the same thing three times and given no way to tell them apart.
    #[test]
    fn each_attempt_outcome_gets_its_own_line_and_its_own_fix() {
        let lines = render_credential_issues(&[
            attempted("10.0.0.1", AttemptOutcome::Rejected),
            attempted("10.0.0.2", AttemptOutcome::TlsFailed),
            attempted("10.0.0.3", AttemptOutcome::NotThisService),
            attempted("10.0.0.4", AttemptOutcome::Malformed),
            attempted("10.0.0.5", AttemptOutcome::CollectionFailed),
        ]);

        assert_eq!(lines.len(), 5, "{lines:?}");
        let msg = joined(&lines);
        assert!(msg.contains("10.0.0.1") && msg.contains("username, password or community"));
        assert!(msg.contains("10.0.0.2") && msg.contains("accept invalid certificates"));
        assert!(msg.contains("10.0.0.3") && msg.contains("check the port on the credential"));
        assert!(msg.contains("10.0.0.4") && msg.contains("re-enter it"));
        assert!(msg.contains("10.0.0.5") && msg.contains("missing rather than out of date"));
    }

    /// The operator stopped the scan. Reporting anything about it — and this used to report the
    /// credential as rejected — is telling them their configuration is broken because they
    /// pressed cancel.
    #[test]
    fn a_cancelled_attempt_is_never_reported() {
        assert!(
            render_credential_issues(&[attempted("10.0.0.1", AttemptOutcome::Cancelled)])
                .is_empty()
        );
    }

    /// Suppressed only when the address-level line is actually there. Repeating "nothing answered
    /// at 10.0.0.1" per credential is the same fact twice.
    #[test]
    fn an_unreachable_attempt_does_not_repeat_an_address_line_that_exists() {
        let lines = render_credential_issues(&[
            CredentialIssue {
                label: "SNMP queries",
                ip: ip("10.0.0.1"),
                reason: CredentialIssueReason::TargetNotResponding,
            },
            attempted("10.0.0.1", AttemptOutcome::Unreachable),
        ]);

        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].contains("nothing answered at that address"),
            "{lines:?}"
        );
    }

    /// …and reported when it is not. The daemon-host phase has no address-level line, because
    /// 127.0.0.1 is always up — so deciding this by outcome alone swallowed every unreachable
    /// report from it, which is how a Docker socket credential pointing at a path that does not
    /// exist stayed silent even after its issue was built and delivered correctly.
    #[test]
    fn an_unreachable_attempt_is_reported_when_no_address_line_covers_it() {
        let lines =
            render_credential_issues(&[attempted("127.0.0.1", AttemptOutcome::Unreachable)]);

        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("127.0.0.1"), "{lines:?}");
        assert!(lines[0].contains("could not be reached"), "{lines:?}");
    }

    /// Hosts sharing an outcome collapse onto one line, the same way they do for every other
    /// reason — fifteen switches refusing the same community is one problem, not fifteen.
    #[test]
    fn hosts_sharing_an_outcome_collapse_onto_one_line() {
        let issues: Vec<CredentialIssue> = (1..=4)
            .map(|n| attempted(&format!("10.0.0.{n}"), AttemptOutcome::Rejected))
            .collect();

        let lines = render_credential_issues(&issues);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].contains("10.0.0.1"));
        assert!(lines[0].contains("10.0.0.4"));
    }
}
