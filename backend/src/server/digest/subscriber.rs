//! Subscriber on the historical-Discovery `EntityOperation::Created` event.
//! Filters to terminal-Complete sessions (the foundation worker publishes the
//! same event for `Failed` and `Cancelled`; v1 of the digest only emails on
//! successful completion). Calls `DiscoveryDigestService::compute_and_publish`
//! which fans out to the email subscriber via a separate event.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::daemons::r#impl::api::ScannedEntityIds;
use crate::server::digest::service::DiscoveryDigestService;
use crate::server::discovery::r#impl::types::RunType;
use crate::server::shared::entities::{Entity, EntityDiscriminants};
use crate::server::shared::events::registry::SubscriberRegistration;
use crate::server::shared::events::traits::{EntityEventFilter, Event, Subscriber};
use crate::server::shared::events::types::{EntityOperation, EntityOperationDiscriminants};

#[async_trait]
impl Subscriber<EntityOperation> for DiscoveryDigestService {
    fn filter(&self) -> EntityEventFilter {
        EntityEventFilter::by_entity(HashMap::from([(
            EntityDiscriminants::Discovery,
            Some(vec![EntityOperationDiscriminants::Created]),
        )]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> anyhow::Result<()> {
        for event in events {
            let Entity::Discovery(discovery) = event.scope.entity_type() else {
                continue;
            };
            let RunType::Historical { results } = &discovery.base.run_type else {
                continue;
            };
            if results.phase != DiscoveryPhase::Complete {
                continue;
            }
            // ScannedEntityIds carries the daemon-reported, server-resolved
            // canonical IDs of every entity touched this session. We use it
            // to detect "removed" children (live on a scanned host but absent
            // from the scan response). It's `Some` only while the in-memory
            // event scope is live — by the time the historical Discovery row
            // is read back from the DB, `scanned` has been stripped.
            let empty = ScannedEntityIds::default();
            let scanned = results.scanned.as_ref().unwrap_or(&empty);
            if let Err(e) = self.compute_and_publish(results, scanned).await {
                tracing::warn!(
                    session_id = %results.session_id,
                    error = %e,
                    "Failed to compute discovery digest",
                );
            }
        }
        Ok(())
    }
}

inventory::submit!(SubscriberRegistration::new::<
    DiscoveryDigestService,
    EntityOperation,
>());
