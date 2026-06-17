//! Brevo subscriber for billing, onboarding, auth, discovery, and entity events.
//!
//! Syncs CRM state: company attributes (plan, status, lifecycle markers,
//! engagement counters) and contact attributes (email, role, marketing opt-in,
//! signup metadata). Routes auth `Register` to contact creation + DOI flow;
//! routes org/billing/discovery/entity events to company updates.

use crate::{
    daemon::discovery::types::base::{DiscoveryPhase, DiscoveryPhaseDiscriminants},
    server::{
        auth::middleware::auth::AuthenticatedEntity,
        brevo::service::BrevoService,
        shared::{
            entities::EntityDiscriminants,
            events::{
                registry::SubscriberRegistration,
                traits::{EntityEventFilter, Event, EventFilter, Subscriber},
                types::{
                    AuthOperation, AuthOperationDiscriminants, BillingOperation, EntityOperation,
                    EntityOperationDiscriminants, OnboardingOperation,
                },
            },
        },
    },
};
use anyhow::Error;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[async_trait]
impl Subscriber<BillingOperation> for BrevoService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in &events {
            if let Err(e) = self.handle_billing_event(event).await {
                tracing::warn!(
                    error = %e,
                    operation = %event.operation,
                    "Failed to sync billing event to Brevo"
                );
            }
        }
        Ok(())
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<BrevoService, BillingOperation>());

#[async_trait]
impl Subscriber<OnboardingOperation> for BrevoService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        for event in &events {
            if let Err(e) = self.handle_onboarding_event(event).await {
                tracing::warn!(
                    error = %e,
                    operation = %event.operation,
                    "Failed to sync onboarding event to Brevo"
                );
            }
        }
        Ok(())
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<
    BrevoService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<AuthOperation> for BrevoService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::ops(vec![
            AuthOperationDiscriminants::LoginSuccess,
            AuthOperationDiscriminants::Register,
        ])
    }

    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        for event in &events {
            match &event.operation {
                AuthOperation::LoginSuccess { .. } => {
                    if let AuthenticatedEntity::User { email, user_id, .. } = &event.authentication
                        && let Err(e) = self
                            .update_contact_last_login(email.to_string(), *user_id)
                            .await
                    {
                        tracing::warn!(error = %e, "Failed to sync auth login to Brevo");
                    }
                }
                AuthOperation::Register { .. } => {
                    if let Err(e) = self.handle_register(event).await {
                        tracing::warn!(error = %e, "Failed to handle register in Brevo");
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<BrevoService, AuthOperation>());

#[async_trait]
impl Subscriber<DiscoveryPhase> for BrevoService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::ops(vec![DiscoveryPhaseDiscriminants::Scanning])
    }

    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), Error> {
        for event in &events {
            if event.operation == DiscoveryPhase::Scanning
                && let Some(org_id) = self.get_org_id_from_network(&event.scope.network_id).await
                && let Err(e) = self.update_company_last_discovery(org_id).await
            {
                tracing::warn!(error = %e, "Failed to sync discovery to Brevo");
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<BrevoService, DiscoveryPhase>());

#[async_trait]
impl Subscriber<EntityOperation> for BrevoService {
    fn filter(&self) -> EntityEventFilter {
        let create_or_delete = Some(vec![
            EntityOperationDiscriminants::Created,
            EntityOperationDiscriminants::Deleted,
        ]);
        EntityEventFilter::by_entity(HashMap::from([
            (EntityDiscriminants::Network, create_or_delete.clone()),
            (EntityDiscriminants::Host, create_or_delete.clone()),
            (EntityDiscriminants::User, create_or_delete),
        ]))
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        // Aggregate org IDs whose entity counts changed; sync once per org.
        let mut org_ids_for_metrics: HashSet<Uuid> = HashSet::new();
        for event in &events {
            if let Some(org_id) = event.scope.organization_id() {
                org_ids_for_metrics.insert(org_id);
            } else if let Some(network_id) = event.scope.network_id()
                && let Some(org_id) = self.get_org_id_from_network(&network_id).await
            {
                org_ids_for_metrics.insert(org_id);
            }
        }

        for org_id in org_ids_for_metrics {
            if let Err(e) = self.sync_org_entity_metrics(org_id).await {
                tracing::warn!(
                    error = %e,
                    organization_id = %org_id,
                    "Failed to sync organization metrics to Brevo"
                );
            }
        }

        Ok(())
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<BrevoService, EntityOperation>());
