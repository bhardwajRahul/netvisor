//! InviteService subscriber for BillingOperation events.
//!
//! Revokes an org's outstanding invites when its subscription is cancelled.
//! Decoupled from BillingService so a billing-side state transition doesn't
//! need to hold a direct `Arc<InviteService>`.

use anyhow::Error;
use async_trait::async_trait;

use crate::server::{
    invites::service::InviteService,
    shared::events::{
        registry::SubscriberRegistration,
        traits::{Event, EventFilter, Subscriber},
        types::{BillingOperation, BillingOperationDiscriminants},
    },
};

#[async_trait]
impl Subscriber<BillingOperation> for InviteService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::ops(vec![BillingOperationDiscriminants::SubscriptionCancelled])
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            if matches!(
                event.operation,
                BillingOperation::SubscriptionCancelled { .. }
            ) {
                let org_id = event.scope.organization_id;
                if let Err(e) = self.revoke_org_invites(&org_id).await {
                    tracing::warn!(
                        organization_id = %org_id,
                        error = %e,
                        "Failed to revoke org invites on subscription cancellation",
                    );
                }
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<InviteService, BillingOperation>());
