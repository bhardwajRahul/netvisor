//! Hosts subscriber for `DiscoveryPhase::Complete` events.
//!
//! Reconciles host state after a discovery session finishes: marks unseen
//! hosts and surfaces freshness so the UI can flag stale records.

use async_trait::async_trait;

use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryPhaseDiscriminants};
use crate::server::hosts::service::HostService;
use crate::server::shared::events::registry::SubscriberRegistration;
use crate::server::shared::events::traits::{Event, EventFilter, Subscriber};

#[async_trait]
impl Subscriber<DiscoveryPhase> for HostService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::ops(vec![DiscoveryPhaseDiscriminants::Complete])
    }

    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), anyhow::Error> {
        for event in events {
            if event.operation != DiscoveryPhase::Complete {
                continue;
            }
            let session_id = event.scope.session_id;
            let network_id = event.scope.network_id;
            // Resolve LLDP/CDP neighbor links — purely server-side DB operation,
            // works for all daemon modes (DaemonPoll and ServerPoll).
            if let Err(e) = self.resolve_lldp_links(network_id).await {
                tracing::warn!(
                    session_id = %session_id,
                    network_id = %network_id,
                    error = %e,
                    "Failed to resolve LLDP links after discovery completion"
                );
            }
            // Resolve FDB single-MAC ports after LLDP/CDP (lower priority)
            if let Err(e) = self.resolve_fdb_links(network_id).await {
                tracing::warn!(
                    session_id = %session_id,
                    network_id = %network_id,
                    error = %e,
                    "Failed to resolve FDB links after discovery completion"
                );
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<HostService, DiscoveryPhase>());
