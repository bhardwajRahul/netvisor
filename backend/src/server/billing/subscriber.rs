use std::collections::HashMap;

use anyhow::Error;
use async_trait::async_trait;

use crate::server::{
    billing::service::BillingService,
    networks::r#impl::Network,
    shared::{
        entities::{Entity, EntityDiscriminants},
        events::{
            bus::{EventFilter, EventSubscriber},
            types::{EntityOperation, Event},
        },
        services::traits::CrudService,
        storage::filter::StorableFilter,
    },
    users::r#impl::base::User,
};

#[async_trait]
impl EventSubscriber for BillingService {
    fn event_filter(&self) -> EventFilter {
        EventFilter::entity_only(HashMap::from([
            (
                EntityDiscriminants::Network,
                Some(vec![EntityOperation::Created, EntityOperation::Deleted]),
            ),
            (
                EntityDiscriminants::User,
                Some(vec![EntityOperation::Created, EntityOperation::Deleted]),
            ),
        ]))
    }

    async fn handle_events(&self, events: Vec<Event>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        for event in events {
            if let Event::Entity(e) = event
                && let Some(org_id) = if let Some(org_id) = e.organization_id {
                    Some(org_id)
                } else if let Some(network_id) = e.network_id {
                    self.network_service
                        .get_by_id(&network_id)
                        .await?
                        .map(|n| n.base.organization_id)
                } else {
                    None
                }
                && let Some(org) = self.organization_service.get_by_id(&org_id).await?
            {
                match e.entity_type {
                    Entity::Network(_) | Entity::User(_) => {
                        let network_filter = StorableFilter::<Network>::new_from_org_id(&org_id);
                        let user_filter = StorableFilter::<User>::new_from_org_id(&org_id);

                        let network_count =
                            self.network_service.get_all(network_filter).await?.len();

                        let seat_count = self.user_service.get_all(user_filter).await?.len();

                        // Skip if org has no plan, or plan has no metered addons
                        if let Some(ref plan) = org.base.plan {
                            if plan.config().seat_cents.is_none()
                                && plan.config().network_cents.is_none()
                            {
                                continue;
                            }
                        } else {
                            continue;
                        }

                        self.update_addon_prices(org, network_count as u64, seat_count as u64)
                            .await?;
                    }
                    _ => (),
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "billing_quota_update"
    }
}
