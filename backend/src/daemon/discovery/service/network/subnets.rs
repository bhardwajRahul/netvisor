use anyhow::Error;
use futures::future::try_join_all;
use strum::IntoDiscriminant;
use tokio_util::sync::CancellationToken;

use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{DaemonUtils, PlatformDaemonUtils};
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::subnets::r#impl::types::SubnetTypeDiscriminants;

use super::NetworkScan;

impl NetworkScan {
    /// Network-phase subnet resolution. Supports optional subnet_id filtering for
    /// targeted scans. Does not include Docker subnet merging (handled by
    /// create_initial_subnets before session start).
    pub async fn resolve_scan_subnets(
        &self,
        ops: &DiscoveryOps,
        utils: &PlatformDaemonUtils,
        cancel: &CancellationToken,
    ) -> Result<Vec<Subnet>, Error> {
        let network_id = ops
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

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
                    if s.base.subnet_type.discriminant() == SubnetTypeDiscriminants::DockerBridge {
                        tracing::warn!("Skipping {} with CIDR {}, docker bridge subnets are scanned in docker discovery", s.base.name, s.base.cidr);
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

        Ok(subnets)
    }
}
