//! Hosts subscriber for `DiscoveryPhase::Complete` events and the
//! `EntityOperation::Created` event for the historical Discovery row.
//!
//! - `DiscoveryPhase::Complete`: reconciles host state after a session
//!   finishes (LLDP/FDB neighbor resolution).
//! - `EntityOperation::Created` (Discovery): pulls `ScannedEntityIds` from
//!   the in-memory event scope and backfills `last_discovery_id` /
//!   `first_discovery_id` on the host rows the daemon scanned.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryPhaseDiscriminants};
use crate::server::hosts::service::HostService;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::events::registry::SubscriberRegistration;
use crate::server::shared::events::traits::{EntityEventFilter, Event, EventFilter, Subscriber};
use crate::server::shared::events::types::{EntityOperation, EntityOperationDiscriminants};
use crate::server::shared::services::traits::{
    DiscoveryFkUpdater, extract_scanned_from_discovery_event,
};

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
            match self.resolve_lldp_links(network_id).await {
                // Resolution necessarily runs after the historical Discovery row is written, so
                // its findings are appended to that row rather than carried in the daemon's own
                // warning list. Without this they exist only in server logs, which a self-hosted
                // operator has no way to read from the UI.
                Ok(outcome) => {
                    if let Err(e) = self
                        .append_resolution_warnings(session_id, outcome.warnings)
                        .await
                    {
                        tracing::warn!(
                            session_id = %session_id,
                            network_id = %network_id,
                            error = %e,
                            "Failed to record LLDP resolution warnings on the scan record"
                        );
                    }
                }
                Err(e) => tracing::warn!(
                    session_id = %session_id,
                    network_id = %network_id,
                    error = %e,
                    "Failed to resolve LLDP links after discovery completion"
                ),
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

#[async_trait]
impl Subscriber<EntityOperation> for HostService {
    fn filter(&self) -> EntityEventFilter {
        // Narrow to Created events on Discovery rows. The historical
        // Discovery insert in DiscoveryService::update_session is the only
        // publisher we care about here; Created events for other entity
        // types are filtered out at the registry level.
        EntityEventFilter::by_entity(HashMap::from([(
            EntityDiscriminants::Discovery,
            Some(vec![EntityOperationDiscriminants::Created]),
        )]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> anyhow::Result<()> {
        for event in events {
            if let Some((scanned, discovery_id)) = extract_scanned_from_discovery_event(&event) {
                <Self as DiscoveryFkUpdater<crate::server::hosts::r#impl::base::Host>>::update_discovery_fks(
                    self,
                    scanned,
                    discovery_id,
                )
                .await?;
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<HostService, EntityOperation>());
