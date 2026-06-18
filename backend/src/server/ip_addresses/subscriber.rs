//! IPAddress subscriber for the historical Discovery row's
//! `EntityOperation::Created` event. Pulls `ScannedEntityIds` off the
//! in-memory event scope and backfills `last_discovery_id` /
//! `first_discovery_id` on the IP-address rows the daemon scanned.

use std::collections::HashMap;

use async_trait::async_trait;

use crate::server::ip_addresses::service::IPAddressService;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::events::registry::SubscriberRegistration;
use crate::server::shared::events::traits::{EntityEventFilter, Event, Subscriber};
use crate::server::shared::events::types::{EntityOperation, EntityOperationDiscriminants};
use crate::server::shared::services::traits::{
    DiscoveryFkUpdater, extract_scanned_from_discovery_event,
};

#[async_trait]
impl Subscriber<EntityOperation> for IPAddressService {
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
                    crate::server::ip_addresses::r#impl::base::IPAddress,
                >>::update_discovery_fks(self, scanned, discovery_id)
                .await?;
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    IPAddressService,
    EntityOperation,
>());
