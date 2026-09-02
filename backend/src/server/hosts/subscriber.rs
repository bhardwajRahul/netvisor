//! Hosts subscriber for the `EntityOperation::Created` event on the historical Discovery row:
//! pulls `ScannedEntityIds` from the in-memory event scope and backfills `last_discovery_id` /
//! `first_discovery_id` on the host rows that session touched.
//!
//! Neighbour resolution used to live here too, on `DiscoveryPhase::Complete`. It now runs before
//! the scan record is written (`DaemonService::process_discovery_progress`), because a host it
//! mints is a host that session found — and reaching it from here meant arriving after this very
//! subscriber had already stamped everything, so a minted far end got no discovery FKs and no
//! digest entry.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::server::hosts::service::HostService;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::events::registry::SubscriberRegistration;
use crate::server::shared::events::traits::{EntityEventFilter, Event, Subscriber};
use crate::server::shared::events::types::{EntityOperation, EntityOperationDiscriminants};
use crate::server::shared::services::traits::{
    DiscoveryFkUpdater, extract_scanned_from_discovery_event,
};

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
