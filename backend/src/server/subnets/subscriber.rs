//! Subnet subscriber for the historical Discovery row's
//! `EntityOperation::Created` event. Does two things:
//!
//! 1. Pulls `ScannedEntityIds` off the in-memory event scope and backfills
//!    `last_discovery_id` / `first_discovery_id` on the subnet rows the daemon
//!    scanned.
//! 2. Reaps the transient `ScanTarget` subnets a rescan targeted. This event
//!    fires for every terminal phase, so a cancelled or failed rescan is cleaned
//!    up too, and it fires *after* the historical row exists, so the run's
//!    history outlives the rows it used.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::shared::entities::{Entity, EntityDiscriminants};
use crate::server::shared::events::registry::SubscriberRegistration;
use crate::server::shared::events::traits::{EntityEventFilter, Event, Subscriber};
use crate::server::shared::events::types::{EntityOperation, EntityOperationDiscriminants};
use crate::server::shared::services::traits::{
    DiscoveryFkUpdater, extract_scanned_from_discovery_event,
};
use crate::server::subnets::service::SubnetService;

#[async_trait]
impl Subscriber<EntityOperation> for SubnetService {
    fn filter(&self) -> EntityEventFilter {
        EntityEventFilter::by_entity(HashMap::from([(
            EntityDiscriminants::Discovery,
            Some(vec![EntityOperationDiscriminants::Created]),
        )]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> anyhow::Result<()> {
        for event in events {
            if let Some((scanned, discovery_id)) = extract_scanned_from_discovery_event(&event) {
                <Self as DiscoveryFkUpdater<
                    crate::server::subnets::r#impl::base::Subnet,
                >>::update_discovery_fks(self, scanned, discovery_id)
                .await?;
            }

            // Reap independently of the block above: that one requires
            // `scanned`, which a cancelled or failed run never carries — and
            // those runs still leave scan-target rows behind.
            if let Entity::Discovery(discovery) = event.scope.entity_type()
                && discovery
                    .base
                    .run_type
                    .historical_results()
                    .is_some_and(|r| r.targeted)
                && let DiscoveryType::Unified {
                    subnet_ids: Some(ids),
                    ..
                } = &discovery.base.discovery_type
            {
                self.reap_scan_targets(ids).await;
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<SubnetService, EntityOperation>());
