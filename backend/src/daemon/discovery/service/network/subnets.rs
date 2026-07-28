use std::collections::HashSet;
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
    /// Fails the session if a target has no containing interfaced subnet. The
    /// server only accepts a rescan when the chosen daemon is interfaced on a
    /// subnet holding the host's IP, so reaching this means the daemon's live
    /// interfaces have diverged from the server's junction. Scanning anyway
    /// would skip ARP and could report a live but TCP-silent host as
    /// unresponsive — the exact answer this feature exists to avoid giving.
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

        let mut subnets: Vec<Subnet> = Vec::new();

        for ip in target_ips {
            let containing = longest_prefix_containing(
                subnet_cidr_to_mac
                    .iter()
                    .filter(|(_, mac)| mac.is_some())
                    .map(|(cidr, _)| *cidr),
                *ip,
            );

            let Some(containing_cidr) = containing else {
                return Err(anyhow::anyhow!(
                    "Cannot rescan {ip}: this daemon has no interface on a subnet containing it"
                ));
            };

            let Some(parent) = all_subnets
                .iter()
                .find(|s| s.base.cidr == containing_cidr)
                .cloned()
            else {
                return Err(anyhow::anyhow!(
                    "Cannot rescan {ip}: no subnet record exists for {containing_cidr}"
                ));
            };

            tracing::info!(
                target = %ip,
                subnet = %containing_cidr,
                "Resolved rescan target to its containing interfaced subnet"
            );

            if !subnets.iter().any(|s| s.id == parent.id) {
                subnets.push(parent);
            }
        }

        Ok(ResolvedScanTargets {
            subnets,
            target_ips: Some(target_ips.clone()),
        })
    }
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
        // The server refuses a rescan in this case; the daemon must agree rather
        // than silently scanning the address without ARP.
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
}
