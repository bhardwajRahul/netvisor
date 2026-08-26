//! Inferring the address space behind far ends this network cannot place.
//!
//! **LLDP carries no prefix.** There is no netmask TLV in IEEE 802.1AB — `lldpRemManAddrTable`
//! gives an address, its family, and the interface it sits on, and nothing about the range around
//! it. So every CIDR here is a guess, which is why the rows it produces carry
//! [`SubnetCidrSource::Inferred`] and ask an operator to confirm them.
//!
//! What *is* evidence:
//!
//! 1. **The L2 segment.** Neighbours seen on ports carrying the same VLAN share a broadcast domain
//!    and therefore a subnet. That is what makes grouping their addresses structural rather than a
//!    heuristic over unrelated numbers.
//! 2. **Convention.** With nothing else, a LAN is a `/24` (IPv4) or a `/64` (IPv6). The `/64` is
//!    barely a guess — it is what SLAAC requires. The `/24` is a real one, and it is the reason the
//!    confidence rung exists. There is precedent: `Subnet::from_discovery` already widens a `/32`
//!    on a VPN tunnel to a `/24` by taking the first three octets.
//!
//! **Guess narrow, never wide.** A `/24` that turns out to sit inside a real `/22` is a subset:
//! longest-prefix matching keeps working and reconciling it is a merge of one row. A `/16` guessed
//! over several real `/24`s swallows addresses from segments discovered later and every one of them
//! needs re-homing. Narrow errors are local; wide errors are global. Hence [`WIDEST_INFERRED_V4`],
//! and hence a VLAN group whose addresses are further apart than that is not treated as one segment
//! at all.
//!
//! This module is deliberately free of the database: it takes what was observed and what the network
//! already holds, and returns what it believes. Everything it decides is therefore testable without
//! a Postgres.
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use cidr::{IpCidr, Ipv4Cidr, Ipv6Cidr};
use uuid::Uuid;

use crate::daemon::discovery::types::warnings::ProvisionalSubnet;
use crate::server::subnets::r#impl::{
    base::SubnetBase,
    types::{SubnetCidrSource, SubnetType},
};

/// The widest range an inference may produce for IPv4.
///
/// A VLAN whose neighbour addresses span more than this is not one segment being reported from
/// several ports — it is several segments, or an address that does not belong with the others. Both
/// are better served by per-`/24` ranges than by one row claiming a `/20`.
const WIDEST_INFERRED_V4: u8 = 22;

/// The prefix a lone IPv4 address is assumed to sit in.
const CONVENTIONAL_V4: u8 = 24;

/// The prefix a lone IPv6 address is assumed to sit in. Required by SLAAC, so barely a convention.
const CONVENTIONAL_V6: u8 = 64;

/// A far end that resolved to no host, and the address it published for itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UnplacedFarEnd {
    /// The local device that saw it, for the warning — not the far end, which is what is unknown.
    pub host_id: Uuid,
    /// The local port that saw it.
    pub if_descr: String,
    /// The far end's own `sysName`, where it sent one. The label an operator recognises.
    pub sys_name: Option<String>,
    /// The address it published. Already filtered by `is_usable_identity_address`.
    pub address: IpAddr,
    /// The VLAN the seeing port carries, where the bridge tables gave one.
    pub vlan_id: Option<Uuid>,
}

/// A range this network probably has, and the evidence that says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InferredRange {
    pub cidr: IpCidr,
    /// Every far end that placed itself inside it, in the order observed.
    pub far_ends: Vec<UnplacedFarEnd>,
    /// Whether a shared VLAN widened this past the conventional prefix.
    pub widened_by_vlan: bool,
}

/// Whether an address is in space a private network may legitimately number itself from.
///
/// A public address is not a segment of yours to invent — that is what `SubnetType::Internet`
/// exists for — and minting a subnet around one would put a range you do not own into a customer's
/// subnet list. Deliberately narrower than `daemons::ssrf::is_private_ip`, which folds loopback and
/// link-local *into* "private" because it is answering a different question (may this be fetched?).
/// Those are excluded upstream by `is_usable_identity_address`, and would be wrong to build a
/// subnet around in any case.
fn is_inferrable_space(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            // RFC 1918, plus RFC 6598 shared address space, which real networks do number from.
            v4.is_private() || (a == 100 && (64..128).contains(&b))
        }
        // Unique local addresses, `fc00::/7`. Global unicast is somebody else's.
        IpAddr::V6(v6) => (v6.segments()[0] & 0xfe00) == 0xfc00,
    }
}

/// The network address of `addr` masked to `prefix` bits.
fn network_of(addr: IpAddr, prefix: u8) -> IpCidr {
    match addr {
        IpAddr::V4(v4) => {
            let mask = if prefix == 0 {
                0
            } else {
                u32::MAX << (32 - prefix)
            };
            let network = Ipv4Addr::from(u32::from(v4) & mask);
            IpCidr::V4(Ipv4Cidr::new(network, prefix).expect("masked address aligns with prefix"))
        }
        IpAddr::V6(v6) => {
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            let network = Ipv6Addr::from(u128::from(v6) & mask);
            IpCidr::V6(Ipv6Cidr::new(network, prefix).expect("masked address aligns with prefix"))
        }
    }
}

/// The longest prefix that still contains every one of `addresses`.
///
/// Only meaningful within one address family; callers group by family first.
fn common_prefix(addresses: &[IpAddr]) -> Option<u8> {
    let mut iter = addresses.iter();
    let first = iter.next()?;
    match first {
        IpAddr::V4(a) => {
            let mut differing = 0u32;
            for other in addresses.iter() {
                let IpAddr::V4(o) = other else { return None };
                differing |= u32::from(*a) ^ u32::from(*o);
            }
            Some(differing.leading_zeros().min(32) as u8)
        }
        IpAddr::V6(a) => {
            let mut differing = 0u128;
            for other in addresses.iter() {
                let IpAddr::V6(o) = other else { return None };
                differing |= u128::from(*a) ^ u128::from(*o);
            }
            Some(differing.leading_zeros().min(128) as u8)
        }
    }
}

/// The prefix a single address is assumed to sit in, by family.
fn conventional_prefix(addr: &IpAddr) -> u8 {
    match addr {
        IpAddr::V4(_) => CONVENTIONAL_V4,
        IpAddr::V6(_) => CONVENTIONAL_V6,
    }
}

/// Which bucket an address falls in before any VLAN widening.
fn conventional_bucket(addr: IpAddr) -> IpCidr {
    network_of(addr, conventional_prefix(&addr))
}

/// The ranges these far ends imply, given what the network already holds.
///
/// `live_cidrs` is every subnet on the network, whatever its confidence. Two separate jobs:
/// an address already inside one is not evidence of anything missing — the range is known, the
/// *host* at that address simply is not — and a range that would overlap an existing subnet is a
/// symptom rather than an opportunity, so it is dropped rather than created.
///
/// Grouping is **network-wide, not per device**. Two switches naming far ends in the same range
/// must produce one subnet, and only the server sees both: a network can have several daemons, each
/// scanning a slice, and the pair may never appear in one daemon's report.
pub(super) fn infer_ranges(
    far_ends: Vec<UnplacedFarEnd>,
    live_cidrs: &[IpCidr],
) -> Vec<InferredRange> {
    let candidates: Vec<UnplacedFarEnd> = far_ends
        .into_iter()
        .filter(|f| is_inferrable_space(&f.address))
        .filter(|f| !live_cidrs.iter().any(|c| c.contains(&f.address)))
        .collect();

    // VLAN first: a shared broadcast domain is the only evidence that can widen a range past the
    // conventional prefix. Everything else falls back to its own bucket.
    let mut by_vlan: BTreeMap<Uuid, Vec<UnplacedFarEnd>> = BTreeMap::new();
    let mut loose: Vec<UnplacedFarEnd> = Vec::new();
    for far_end in candidates {
        match far_end.vlan_id {
            Some(vlan) => by_vlan.entry(vlan).or_default().push(far_end),
            None => loose.push(far_end),
        }
    }

    // `BTreeMap` keyed by the CIDR's string form: `IpCidr` is not `Ord`, and the ordering only has
    // to be *stable* so the pass produces the same rows for the same evidence on every scan.
    let mut ranges: BTreeMap<String, InferredRange> = BTreeMap::new();

    for (_, group) in by_vlan {
        for range in widen_within_vlan(group) {
            merge_into(&mut ranges, range);
        }
    }
    for far_end in loose {
        merge_into(
            &mut ranges,
            InferredRange {
                cidr: conventional_bucket(far_end.address),
                far_ends: vec![far_end],
                widened_by_vlan: false,
            },
        );
    }

    ranges
        .into_values()
        .filter(|range| !live_cidrs.iter().any(|c| overlaps(c, &range.cidr)))
        .collect()
}

/// One VLAN's far ends, widened to their common prefix where that is still a plausible segment.
///
/// A group spanning more than [`WIDEST_INFERRED_V4`] is not one segment reported from several
/// ports, so it degrades to conventional buckets rather than claiming a range nobody has.
fn widen_within_vlan(group: Vec<UnplacedFarEnd>) -> Vec<InferredRange> {
    let addresses: Vec<IpAddr> = group.iter().map(|f| f.address).collect();
    let conventional = addresses.first().map(conventional_prefix);

    // IPv6 is never widened: `/64` is what the addressing plan requires, not a convention to
    // improve on, and a shorter prefix would claim somebody's whole allocation.
    let widen = match (common_prefix(&addresses), conventional) {
        (Some(common), Some(conventional)) if matches!(addresses[0], IpAddr::V4(_)) => {
            (common < conventional && common >= WIDEST_INFERRED_V4).then_some(common)
        }
        _ => None,
    };

    match widen {
        Some(prefix) => vec![InferredRange {
            cidr: network_of(addresses[0], prefix),
            far_ends: group,
            widened_by_vlan: true,
        }],
        None => {
            let mut buckets: BTreeMap<String, InferredRange> = BTreeMap::new();
            for far_end in group {
                merge_into(
                    &mut buckets,
                    InferredRange {
                        cidr: conventional_bucket(far_end.address),
                        far_ends: vec![far_end],
                        widened_by_vlan: false,
                    },
                );
            }
            buckets.into_values().collect()
        }
    }
}

/// Fold a range into the accumulator, joining the evidence when the CIDR is already there.
fn merge_into(ranges: &mut BTreeMap<String, InferredRange>, range: InferredRange) {
    match ranges.get_mut(&range.cidr.to_string()) {
        Some(existing) => {
            existing.far_ends.extend(range.far_ends);
            existing.widened_by_vlan |= range.widened_by_vlan;
        }
        None => {
            ranges.insert(range.cidr.to_string(), range);
        }
    }
}

/// Whether two ranges share any address. Either containing the other is enough.
fn overlaps(a: &IpCidr, b: &IpCidr) -> bool {
    a.contains(&b.first_address()) || b.contains(&a.first_address())
}

use super::*;

impl HostService {
    /// Create a subnet for every range these far ends imply, and report each one.
    ///
    /// Runs after the resolution pass rather than inside it: the ranges are a property of the whole
    /// network's unplaced far ends, not of any one interface, and pooling them is the entire reason
    /// this lives on the server (see the module docs).
    ///
    /// A failure to create one range never fails the pass. Link resolution is the caller's actual
    /// job, and losing every resolved link because a subnet insert raced another session would be a
    /// far worse outcome than one missing range that the next scan re-proposes anyway.
    pub(super) async fn infer_far_end_subnets(
        &self,
        network_id: Uuid,
        far_ends: Vec<UnplacedFarEnd>,
    ) -> Result<Vec<DiscoveryWarning>> {
        if far_ends.is_empty() {
            return Ok(Vec::new());
        }

        let live = self
            .subnet_service
            .get_all(StorableFilter::<Subnet>::new_from_network_ids(&[network_id]).live())
            .await?;
        let live_cidrs: Vec<IpCidr> = live.iter().map(|s| s.base.cidr).collect();

        let mut warnings = Vec::new();
        for range in infer_ranges(far_ends, &live_cidrs) {
            let mut subnet = Subnet::new(SubnetBase {
                cidr: range.cidr,
                // The whole point: a range nothing read, only inferred, so the row asks to be
                // confirmed rather than asserting itself.
                cidr_source: SubnetCidrSource::Inferred,
                network_id,
                name: range.cidr.to_string(),
                description: None,
                // Not `Management` even though a management address is what usually produces this:
                // on a flat network that address is simply the device's LAN address, and typing the
                // subnet would be a second guess stacked on the first.
                subnet_type: SubnetType::Unknown,
                virtualization_service_id: None,
                source: EntitySource::Discovery,
                tags: Vec::new(),
            });
            subnet.last_seen_at = Utc::now();

            let created = match self
                .subnet_service
                .create(subnet, AuthenticatedEntity::System)
                .await
            {
                Ok(created) => created,
                Err(e) => {
                    tracing::warn!(
                        network_id = %network_id,
                        cidr = %range.cidr,
                        error = %e,
                        "Could not create inferred subnet; leaving its far ends unplaced"
                    );
                    continue;
                }
            };

            tracing::info!(
                network_id = %network_id,
                cidr = %range.cidr,
                subnet_id = %created.id,
                addresses = range.far_ends.len(),
                widened_by_vlan = range.widened_by_vlan,
                "Inferred a subnet from far-end addresses"
            );

            warnings.push(DiscoveryWarning::ProvisionalSubnetInferred(
                ProvisionalSubnet {
                    cidr: range.cidr.to_string(),
                    subnet_id: created.id,
                    addresses: range
                        .far_ends
                        .iter()
                        .map(|f| f.address.to_string())
                        .collect(),
                    sys_names: range
                        .far_ends
                        .iter()
                        .filter_map(|f| f.sys_name.clone())
                        .collect(),
                    seen_by_host_ids: range.far_ends.iter().map(|f| f.host_id).collect(),
                    widened_by_vlan: range.widened_by_vlan,
                },
            ));
        }

        Ok(warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn far_end(address: &str, vlan_id: Option<Uuid>) -> UnplacedFarEnd {
        UnplacedFarEnd {
            host_id: Uuid::new_v4(),
            if_descr: "Gi0/1".to_string(),
            sys_name: Some("far-end".to_string()),
            address: address.parse().unwrap(),
            vlan_id,
        }
    }

    fn cidr(s: &str) -> IpCidr {
        s.parse().unwrap()
    }

    fn cidrs(ranges: &[InferredRange]) -> Vec<String> {
        ranges.iter().map(|r| r.cidr.to_string()).collect()
    }

    /// The convention rung: one address, nothing else, a `/24`.
    #[test]
    fn a_lone_address_becomes_the_conventional_prefix() {
        let ranges = infer_ranges(vec![far_end("10.20.30.11", None)], &[]);
        assert_eq!(cidrs(&ranges), vec!["10.20.30.0/24"]);
    }

    /// The reason this is server-side. Two switches naming far ends in one range must produce one
    /// subnet — a network can have several daemons and the pair may never appear in one report.
    #[test]
    fn far_ends_in_one_range_seen_by_different_devices_produce_one_subnet() {
        let ranges = infer_ranges(
            vec![
                far_end("10.20.30.11", None),
                far_end("10.20.30.240", None),
                far_end("10.20.30.24", None),
            ],
            &[],
        );

        assert_eq!(cidrs(&ranges), vec!["10.20.30.0/24"]);
        assert_eq!(ranges[0].far_ends.len(), 3, "all three are its evidence");
    }

    /// Adjacent `/24`s with nothing tying them together are two segments, not a `/23`. Widening
    /// needs evidence, and proximity alone is not evidence.
    #[test]
    fn adjacent_ranges_without_vlan_evidence_stay_separate() {
        let ranges = infer_ranges(
            vec![far_end("10.20.30.11", None), far_end("10.20.31.5", None)],
            &[],
        );
        assert_eq!(cidrs(&ranges), vec!["10.20.30.0/24", "10.20.31.0/24"]);
    }

    /// A shared VLAN *is* evidence: the ports are one broadcast domain, so the addresses are one
    /// subnet even though they straddle a `/24` boundary.
    #[test]
    fn a_shared_vlan_widens_past_the_conventional_prefix() {
        let vlan = Some(Uuid::new_v4());
        let ranges = infer_ranges(
            vec![far_end("10.20.30.11", vlan), far_end("10.20.31.5", vlan)],
            &[],
        );

        assert_eq!(cidrs(&ranges), vec!["10.20.30.0/23"]);
        assert!(ranges[0].widened_by_vlan);
    }

    /// Guess narrow, never wide. Addresses this far apart on one VLAN are not one segment being
    /// reported from several ports, and claiming the range that spans them would swallow
    /// everything discovered between them later.
    #[test]
    fn a_vlan_spanning_more_than_the_cap_degrades_to_conventional_buckets() {
        let vlan = Some(Uuid::new_v4());
        let ranges = infer_ranges(
            vec![far_end("10.20.30.11", vlan), far_end("10.20.99.5", vlan)],
            &[],
        );

        assert_eq!(cidrs(&ranges), vec!["10.20.30.0/24", "10.20.99.0/24"]);
        assert!(ranges.iter().all(|r| !r.widened_by_vlan));
    }

    /// A public address is not a segment of yours to invent. `SubnetType::Internet` is where those
    /// belong, and minting a range around one would put address space a customer does not own into
    /// their subnet list.
    #[test]
    fn a_public_address_infers_nothing() {
        let ranges = infer_ranges(vec![far_end("8.8.8.8", None)], &[]);
        assert!(ranges.is_empty());
    }

    /// The range is already known; only the *host* at that address is missing. Inferring here
    /// would create a duplicate of a subnet the network already holds.
    #[test]
    fn an_address_inside_a_known_subnet_infers_nothing() {
        let ranges = infer_ranges(
            vec![far_end("192.168.7.99", None)],
            &[cidr("192.168.7.0/24")],
        );
        assert!(ranges.is_empty());
    }

    /// A widened range that would straddle a subnet already held is dropped rather than created.
    ///
    /// The shape has to reach the guard to test it: an address *inside* a known subnet is removed
    /// before any widening happens, so the case that survives that filter is a known subnet sitting
    /// in the gap between two addresses — part of the VLAN's range already carved out and scanned.
    /// Creating the `/23` over it would put two overlapping rows in the subnet list.
    #[test]
    fn a_widened_range_overlapping_a_known_subnet_is_dropped() {
        let vlan = Some(Uuid::new_v4());
        let far_ends = vec![far_end("10.20.30.11", vlan), far_end("10.20.31.5", vlan)];

        assert_eq!(
            cidrs(&infer_ranges(far_ends.clone(), &[])),
            vec!["10.20.30.0/23"],
            "without the known subnet the pair widens, or this proves nothing"
        );

        // Contains neither address, and sits inside the `/23` they span.
        assert!(infer_ranges(far_ends, &[cidr("10.20.30.128/25")]).is_empty());
    }

    /// The narrower half of the same case: an address already inside a known subnet is not evidence
    /// of a missing range at all, so it drops out before bucketing and takes no range with it.
    #[test]
    fn a_known_address_drops_out_without_suppressing_its_neighbours() {
        let vlan = Some(Uuid::new_v4());
        let ranges = infer_ranges(
            vec![far_end("10.20.30.11", vlan), far_end("10.20.31.5", vlan)],
            &[cidr("10.20.31.0/24")],
        );

        // 10.20.31.5 is accounted for; 10.20.30.11 still is not.
        assert_eq!(cidrs(&ranges), vec!["10.20.30.0/24"]);
    }

    /// IPv6 is bucketed and never widened: `/64` is what the addressing plan requires rather than a
    /// convention to improve on, and a shorter prefix would claim a whole allocation.
    #[test]
    fn ipv6_far_ends_bucket_at_the_slaac_prefix() {
        let vlan = Some(Uuid::new_v4());
        let ranges = infer_ranges(
            vec![
                far_end("fd00:1234:5678:9abc::11", vlan),
                far_end("fd00:1234:5678:9abc::24", vlan),
            ],
            &[],
        );
        assert_eq!(cidrs(&ranges), vec!["fd00:1234:5678:9abc::/64"]);
    }

    /// Same evidence, same rows, every scan — the pass writes through a CIDR dedup, so a bucketing
    /// that varied with input order would churn subnets rather than converge on them.
    #[test]
    fn the_same_evidence_in_a_different_order_produces_the_same_ranges() {
        let one = infer_ranges(
            vec![
                far_end("10.20.30.11", None),
                far_end("10.20.40.5", None),
                far_end("10.20.30.99", None),
            ],
            &[],
        );
        let other = infer_ranges(
            vec![
                far_end("10.20.30.99", None),
                far_end("10.20.30.11", None),
                far_end("10.20.40.5", None),
            ],
            &[],
        );
        assert_eq!(cidrs(&one), cidrs(&other));
    }
}
