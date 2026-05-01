//! Metrics subscriber for every operation type.
//!
//! Increments a Prometheus counter per event. Entity events go to
//! `scanopy_entity_events_total{entity_type, operation}`; non-entity events
//! go to `scanopy_events_total{category, operation}`. Two metrics, two
//! cardinalities — entity dimensions stay separate from event categories.

use anyhow::Error;
use async_trait::async_trait;
use strum::IntoDiscriminant;

use crate::{
    daemon::discovery::types::base::DiscoveryPhase,
    server::{
        metrics::service::MetricsService,
        shared::events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, EventFilter, Subscriber},
            types::{
                AnalyticsOperation, AuthOperation, BillingOperation, EntityOperation,
                OnboardingOperation,
            },
        },
    },
};

/// Per-entity-type counter (Host/Service/Subnet/etc. × Created/Updated/Deleted/...).
fn record_entity(entity_type: &str, operation: impl std::fmt::Display) {
    metrics::counter!(
        "scanopy_entity_events_total",
        "entity_type" => entity_type.to_string(),
        "operation" => operation.to_string(),
    )
    .increment(1);
}

/// Per-category counter for non-entity events (billing/onboarding/analytics/...).
/// Kept distinct from entity events so per-host metrics aren't muddled with
/// per-login metrics.
fn record_event(category: &str, operation: impl std::fmt::Display) {
    metrics::counter!(
        "scanopy_events_total",
        "category" => category.to_string(),
        "operation" => operation.to_string(),
    )
    .increment(1);
}

#[async_trait]
impl Subscriber<BillingOperation> for MetricsService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            record_event("billing", event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    MetricsService,
    BillingOperation,
>());

#[async_trait]
impl Subscriber<OnboardingOperation> for MetricsService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        for event in events {
            record_event("onboarding", event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    MetricsService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<AnalyticsOperation> for MetricsService {
    fn filter(&self) -> EventFilter<AnalyticsOperation> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<AnalyticsOperation>>) -> Result<(), Error> {
        for event in events {
            record_event("analytics", event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    MetricsService,
    AnalyticsOperation,
>());

#[async_trait]
impl Subscriber<AuthOperation> for MetricsService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        for event in events {
            record_event("auth", event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<MetricsService, AuthOperation>());

#[async_trait]
impl Subscriber<EntityOperation> for MetricsService {
    fn filter(&self) -> EntityEventFilter {
        EntityEventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        for event in events {
            let entity_type = event.scope.entity_type().discriminant().to_string();
            record_entity(&entity_type, event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<MetricsService, EntityOperation>());

#[async_trait]
impl Subscriber<DiscoveryPhase> for MetricsService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), Error> {
        for event in events {
            record_event("discovery", event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<MetricsService, DiscoveryPhase>());
