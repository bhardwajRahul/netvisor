use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use anyhow::Error;
use futures::future::try_join_all;
use tokio_util::sync::CancellationToken;

use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{DaemonUtils, PlatformDaemonUtils};
use crate::server::subnets::r#impl::base::Subnet;

use super::NetworkScan;

/// Outcome of network-phase target resolution.
///
/// `target_ips` is `Some` only for a rescan, which names its addresses directly
/// rather than sweeping subnets. The subnets returned alongside are the real
/// ones containing those addresses, so everything downstream — the
/// interfaced/non-interfaced partition, the ARP source lookups, IP attribution —
/// works exactly as it does for a normal scan.
pub struct ResolvedScanTargets {
    pub subnets: Vec<Subnet>,
    pub target_ips: Option<HashSet<IpAddr>>,
}

impl NetworkScan {
    /// Network-phase target resolution: either the subnets to sweep, or the
    /// specific addresses a rescan is verifying.
    pub async fn resolve_scan_subnets(
        &self,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
        cancel: &CancellationToken,
    ) -> Result<ResolvedScanTargets, Error> {
        let network_id = ops
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        // A rescan names its targets, so resolve each to the subnet that holds it.
        if let Some(target_ips) = &self.target_ips {
            return self
                .resolve_rescan_targets(target_ips, ops, utils, network_id)
                .await;
        }

        // Target specific subnets if provided in discovery type
        let subnets = if let Some(subnet_ids) = &self.subnet_ids {
            let all_subnets: Vec<Subnet> = ops
                .api_client
                .get("/api/v1/subnets", "Failed to get subnets")
                .await?;
            all_subnets
                .into_iter()
                .filter(|s| subnet_ids.contains(&s.id))
                .collect()

        // Target all interfaced subnets if not
        } else {
            let interface_filter = ops.config_store.get_interfaces().await?;
            let (_, subnets, _) = utils
                .get_own_interfaces(network_id, &interface_filter)
                .await?;

            // Filter out docker bridge subnets (handled in docker discovery).
            // Size filtering for non-interfaced subnets is done later in
            // scan_and_process_hosts() where subnet_cidr_to_mac is available.
            let subnets: Vec<Subnet> = subnets
                .into_iter()
                .filter(|s| {
                    if s.is_container_bridge_subnet() {
                        tracing::warn!("Skipping {} with CIDR {}, container bridge subnets are scanned in container discovery", s.base.name, s.base.cidr);
                        return false
                    }

                    true
                })
                .collect();
            let subnet_futures = subnets
                .iter()
                .map(|subnet| ops.create_subnet(subnet, cancel));
            try_join_all(subnet_futures).await?
        };

        Ok(ResolvedScanTargets {
            subnets,
            target_ips: None,
        })
    }

    /// Map each rescan target to the interfaced subnet containing it.
    ///
    /// Targets that can't be resolved are dropped with a warning rather than
    /// failing the session — one address a daemon can't ARP must not cost a host
    /// its rescan when the rest are perfectly scannable. Failing only when
    /// *nothing* resolves keeps the fidelity guarantee that motivates the
    /// server-side precondition: every address actually scanned is ARP'd, so a
    /// live but TCP-silent host is never reported unresponsive.
    async fn resolve_rescan_targets(
        &self,
        target_ips: &HashSet<IpAddr>,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
        network_id: uuid::Uuid,
    ) -> Result<ResolvedScanTargets, Error> {
        let interface_filter = ops.config_store.get_interfaces().await?;
        let (_, _, subnet_cidr_to_mac) = utils
            .get_own_interfaces(network_id, &interface_filter)
            .await?;

        let all_subnets: Vec<Subnet> = ops
            .api_client
            .get("/api/v1/subnets", "Failed to get subnets")
            .await?;

        let resolution = resolve_rescan_subnets(target_ips, &subnet_cidr_to_mac, &all_subnets);

        for (ip, subnet) in &resolution.attributions {
            tracing::info!(
                target = %ip,
                subnet = %subnet,
                "Resolved rescan target to its containing interfaced subnet"
            );
        }
        for (ip, skip) in &resolution.skipped {
            tracing::warn!(target = %ip, reason = %skip, "Skipping rescan target");
        }

        if resolution.subnets.is_empty() {
            let reasons = resolution
                .skipped
                .iter()
                .map(|(ip, skip)| format!("{ip} ({skip})"))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(anyhow::anyhow!(
                "Cannot rescan this host: none of its addresses are scannable by this daemon — {reasons}"
            ));
        }

        Ok(ResolvedScanTargets {
            subnets: resolution.subnets,
            target_ips: Some(resolution.resolved),
        })
    }
}

/// Why a rescan target was left out of the scan.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RescanSkip {
    /// A loopback address is reachable without ARP and has no interfaced subnet
    /// to be scanned from — and loopback subnets are dropped from every scan
    /// anyway (see `scan_and_process_hosts`), so resolving one buys nothing.
    Loopback,
    /// No interface of this daemon sits on a subnet containing the address, so
    /// it can't be ARP'd. Normally the server's precondition has already
    /// filtered these out; reaching here means its junction is stale.
    NoInterfacedSubnet,
    /// An interface covers the address but the server has no subnet record for
    /// that CIDR, so there is nothing to attribute discovered hosts to.
    NoSubnetRecord(cidr::IpCidr),
}

impl std::fmt::Display for RescanSkip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loopback => write!(f, "loopback addresses are reached locally, not scanned"),
            Self::NoInterfacedSubnet => {
                write!(f, "this daemon has no interface on a subnet containing it")
            }
            Self::NoSubnetRecord(cidr) => write!(f, "no subnet record exists for {cidr}"),
        }
    }
}

/// The scannable half of a rescan's target set, plus what was dropped and why.
pub(crate) struct RescanResolution {
    /// The interfaced subnets the surviving targets will be scanned from.
    pub subnets: Vec<Subnet>,
    /// The targets that resolved — narrower than the requested set, so progress
    /// budgeting doesn't count addresses that will never be scanned.
    pub resolved: HashSet<IpAddr>,
    /// `(target, containing CIDR)` for each resolved target.
    pub attributions: Vec<(IpAddr, cidr::IpCidr)>,
    pub skipped: Vec<(IpAddr, RescanSkip)>,
}

/// Resolve rescan targets to the interfaced subnets they'll be scanned from.
///
/// Pure: the caller supplies the daemon's live interfaces and the server's
/// subnet records. Unresolvable targets are reported in `skipped` instead of
/// aborting, so a mixed host (a real address plus a loopback) still rescans.
pub(crate) fn resolve_rescan_subnets(
    target_ips: &HashSet<IpAddr>,
    subnet_cidr_to_mac: &HashMap<cidr::IpCidr, Option<mac_address::MacAddress>>,
    all_subnets: &[Subnet],
) -> RescanResolution {
    let mut resolution = RescanResolution {
        subnets: Vec::new(),
        resolved: HashSet::new(),
        attributions: Vec::new(),
        skipped: Vec::new(),
    };

    // Iterate in a stable order so logs and errors don't reshuffle per run.
    let mut ordered: Vec<IpAddr> = target_ips.iter().copied().collect();
    ordered.sort();

    for ip in ordered {
        if ip.is_loopback() {
            resolution.skipped.push((ip, RescanSkip::Loopback));
            continue;
        }

        // Only an interface with a MAC can ARP; a MAC-less interface (loopback,
        // some tunnels) would silently downgrade the scan to a TCP probe.
        let containing = longest_prefix_containing(
            subnet_cidr_to_mac
                .iter()
                .filter(|(_, mac)| mac.is_some())
                .map(|(cidr, _)| *cidr),
            ip,
        );

        let Some(containing_cidr) = containing else {
            resolution
                .skipped
                .push((ip, RescanSkip::NoInterfacedSubnet));
            continue;
        };

        let Some(parent) = all_subnets
            .iter()
            .find(|s| s.base.cidr == containing_cidr)
            .cloned()
        else {
            resolution
                .skipped
                .push((ip, RescanSkip::NoSubnetRecord(containing_cidr)));
            continue;
        };

        resolution.resolved.insert(ip);
        resolution.attributions.push((ip, containing_cidr));
        if !resolution.subnets.iter().any(|s| s.id == parent.id) {
            resolution.subnets.push(parent);
        }
    }

    resolution
}

/// The most specific of `cidrs` that contains `ip`.
///
/// Longest prefix wins, matching the server's dangling-subnet repair
/// (`resolve_dangling_subnet_id`): where a daemon has overlapping interfaces,
/// the narrower one is the interface that will actually ARP the address.
pub(crate) fn longest_prefix_containing(
    cidrs: impl Iterator<Item = cidr::IpCidr>,
    ip: IpAddr,
) -> Option<cidr::IpCidr> {
    cidrs
        .filter(|cidr| cidr.contains(&ip))
        .max_by_key(|cidr| cidr.network_length())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn cidr(s: &str) -> cidr::IpCidr {
        cidr::IpCidr::from_str(s).unwrap()
    }

    fn ip(s: &str) -> IpAddr {
        IpAddr::from_str(s).unwrap()
    }

    #[test]
    fn picks_the_narrowest_containing_interface() {
        // A daemon interfaced on both a /16 and a /24 covering the target must
        // scan from the /24 — that is the interface on the target's link.
        let candidates = [
            cidr("10.0.0.0/16"),
            cidr("10.0.5.0/24"),
            cidr("10.0.9.0/24"),
        ];
        assert_eq!(
            longest_prefix_containing(candidates.into_iter(), ip("10.0.5.7")),
            Some(cidr("10.0.5.0/24"))
        );
    }

    #[test]
    fn no_containing_interface_is_not_a_match() {
        // No match means the address can't be ARP'd, so the rescan skips it
        // rather than silently scanning it without ARP.
        let candidates = [cidr("10.0.5.0/24"), cidr("192.168.1.0/24")];
        assert_eq!(
            longest_prefix_containing(candidates.into_iter(), ip("172.16.0.4")),
            None
        );
    }

    #[test]
    fn a_host_route_can_be_the_match() {
        let candidates = [cidr("10.0.5.0/24"), cidr("10.0.5.7/32")];
        assert_eq!(
            longest_prefix_containing(candidates.into_iter(), ip("10.0.5.7")),
            Some(cidr("10.0.5.7/32"))
        );
    }

    fn a_mac() -> Option<mac_address::MacAddress> {
        Some(mac_address::MacAddress::new([0, 1, 2, 3, 4, 5]))
    }

    fn subnet(cidr_str: &str) -> Subnet {
        Subnet {
            id: uuid::Uuid::new_v4(),
            base: crate::server::subnets::r#impl::base::SubnetBase {
                cidr: cidr(cidr_str),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn targets(ips: &[&str]) -> HashSet<IpAddr> {
        ips.iter().map(|s| ip(s)).collect()
    }

    /// The reported bug: a daemon host carries 127.0.0.1 alongside its real address, and the
    /// loopback — which has no MAC and so no interfaced subnet — used to fail the whole rescan.
    /// The real address must still be scanned.
    #[test]
    fn a_loopback_target_is_skipped_without_costing_the_rescan() {
        let interfaces =
            HashMap::from([(cidr("10.0.5.0/24"), a_mac()), (cidr("127.0.0.0/8"), None)]);
        let records = vec![subnet("10.0.5.0/24"), subnet("127.0.0.0/8")];

        let resolution =
            resolve_rescan_subnets(&targets(&["127.0.0.1", "10.0.5.7"]), &interfaces, &records);

        assert_eq!(resolution.resolved, targets(&["10.0.5.7"]));
        assert_eq!(
            resolution
                .subnets
                .iter()
                .map(|s| s.base.cidr)
                .collect::<Vec<_>>(),
            vec![cidr("10.0.5.0/24")]
        );
        assert_eq!(
            resolution.skipped,
            vec![(ip("127.0.0.1"), RescanSkip::Loopback)]
        );
    }

    /// Nothing scannable is left, so the caller has an empty subnet set to fail on — a clear
    /// refusal rather than a scan that quietly covers no address.
    #[test]
    fn a_loopback_only_target_set_resolves_to_nothing() {
        let interfaces = HashMap::from([(cidr("127.0.0.0/8"), None)]);

        let resolution = resolve_rescan_subnets(
            &targets(&["127.0.0.1"]),
            &interfaces,
            &[subnet("127.0.0.0/8")],
        );

        assert!(resolution.subnets.is_empty());
        assert!(resolution.resolved.is_empty());
    }

    /// A target the daemon can't ARP, or one whose subnet the server has no record of, is
    /// dropped on its own — a stale interfaced-subnet junction must not cost the other targets.
    #[test]
    fn one_unresolvable_target_does_not_drop_its_siblings() {
        let interfaces = HashMap::from([
            (cidr("10.0.5.0/24"), a_mac()),
            (cidr("192.168.1.0/24"), a_mac()),
        ]);
        // No server record for 192.168.1.0/24, and nothing at all covering 172.16.0.4.
        let records = vec![subnet("10.0.5.0/24")];

        let resolution = resolve_rescan_subnets(
            &targets(&["10.0.5.7", "172.16.0.4", "192.168.1.9"]),
            &interfaces,
            &records,
        );

        assert_eq!(resolution.resolved, targets(&["10.0.5.7"]));
        assert_eq!(
            resolution.skipped,
            vec![
                (ip("172.16.0.4"), RescanSkip::NoInterfacedSubnet),
                (
                    ip("192.168.1.9"),
                    RescanSkip::NoSubnetRecord(cidr("192.168.1.0/24"))
                ),
            ]
        );
    }

    /// Several targets on one link share the subnet they're scanned from.
    #[test]
    fn targets_on_one_link_resolve_to_a_single_subnet() {
        let interfaces = HashMap::from([(cidr("10.0.5.0/24"), a_mac())]);

        let resolution = resolve_rescan_subnets(
            &targets(&["10.0.5.7", "10.0.5.8"]),
            &interfaces,
            &[subnet("10.0.5.0/24")],
        );

        assert_eq!(resolution.subnets.len(), 1);
        assert_eq!(resolution.resolved.len(), 2);
    }
}
