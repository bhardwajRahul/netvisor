//! Logging subscriber for every operation type.
//!
//! Renders each event as JSON via `Display for Event<Op>` and emits at the
//! event's declared `log_level`. Honours `flags.suppress_logs` to keep noisy
//! emissions out of structured logs.

use anyhow::Error;
use async_trait::async_trait;

use crate::{
    daemon::discovery::types::base::DiscoveryPhase,
    server::{
        logging::service::LoggingService,
        shared::events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, EventFilter, Operation, Subscriber},
            types::{
                AnalyticsOperation, AuthOperation, BillingOperation, EntityOperation,
                EventLogLevel, OnboardingOperation,
            },
        },
    },
};

fn log_at_level(level: EventLogLevel, label: &str, message: impl std::fmt::Display) {
    match level {
        EventLogLevel::Error => tracing::error!(label = label, "{}", message),
        EventLogLevel::Warn => tracing::warn!(label = label, "{}", message),
        EventLogLevel::Info => tracing::info!(label = label, "{}", message),
        EventLogLevel::Debug => tracing::debug!(label = label, "{}", message),
        EventLogLevel::Trace => tracing::trace!(label = label, "{}", message),
    }
}

fn log_event<Op: Operation>(event: &Event<Op>, suppress: bool) {
    if suppress {
        return;
    }
    log_at_level(event.operation.log_level(), &event.log_label(), event);
}

#[async_trait]
impl Subscriber<BillingOperation> for LoggingService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    BillingOperation,
>());

#[async_trait]
impl Subscriber<OnboardingOperation> for LoggingService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<AnalyticsOperation> for LoggingService {
    fn filter(&self) -> EventFilter<AnalyticsOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<AnalyticsOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    AnalyticsOperation,
>());

#[async_trait]
impl Subscriber<AuthOperation> for LoggingService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<LoggingService, AuthOperation>());

#[async_trait]
impl Subscriber<EntityOperation> for LoggingService {
    fn filter(&self) -> EntityEventFilter {
        EntityEventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<LoggingService, EntityOperation>());

#[async_trait]
impl Subscriber<DiscoveryPhase> for LoggingService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<LoggingService, DiscoveryPhase>());
