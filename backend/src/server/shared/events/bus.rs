use std::sync::Arc;

use anyhow::Result;

use crate::{
    daemon::discovery::types::base::DiscoveryPhase,
    server::shared::events::{
        traits::{Event, Operation, Subscriber, TypedChannel},
        types::{
            AnalyticsOperation, AuthOperation, BillingOperation, EntityOperation,
            OnboardingOperation,
        },
    },
};

pub struct EventBus {
    pub billing_channel: TypedChannel<BillingOperation>,
    pub onboarding_channel: TypedChannel<OnboardingOperation>,
    pub analytics_channel: TypedChannel<AnalyticsOperation>,
    pub auth_channel: TypedChannel<AuthOperation>,
    pub entity_channel: TypedChannel<EntityOperation>,
    pub discovery_channel: TypedChannel<DiscoveryPhase>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            billing_channel: TypedChannel::new(),
            onboarding_channel: TypedChannel::new(),
            analytics_channel: TypedChannel::new(),
            auth_channel: TypedChannel::new(),
            entity_channel: TypedChannel::new(),
            discovery_channel: TypedChannel::new(),
        }
    }

    /// Register a typed subscriber. Op is inferred from the subscriber's
    /// `Subscriber<Op>` impl; the bus routes to the right channel via
    /// `BusChannel<Op>`. `name` is the auto-generated identifier from the
    /// `SubscriberRegistration` entry — used for diagnostic logging.
    pub async fn register<Op>(&self, subscriber: Arc<dyn Subscriber<Op>>, name: &'static str)
    where
        Op: Operation,
        Self: BusChannel<Op>,
    {
        self.channel().register(subscriber, name).await
    }

    /// Publish a typed event. Op is inferred from the event's type; the bus
    /// routes to the right channel via `BusChannel<Op>`.
    pub async fn publish<Op>(&self, event: Event<Op>) -> Result<()>
    where
        Op: Operation,
        Self: BusChannel<Op>,
    {
        self.channel().publish(event).await
    }
}

/// Maps an `Operation` type to its corresponding channel on the bus. One impl
/// per Op type — the only place the per-op-type plumbing knows about specific
/// channel fields.
pub trait BusChannel<Op: Operation> {
    fn channel(&self) -> &TypedChannel<Op>;
}

impl BusChannel<BillingOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<BillingOperation> {
        &self.billing_channel
    }
}

impl BusChannel<OnboardingOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<OnboardingOperation> {
        &self.onboarding_channel
    }
}

impl BusChannel<AnalyticsOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<AnalyticsOperation> {
        &self.analytics_channel
    }
}

impl BusChannel<AuthOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<AuthOperation> {
        &self.auth_channel
    }
}

impl BusChannel<EntityOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<EntityOperation> {
        &self.entity_channel
    }
}

impl BusChannel<DiscoveryPhase> for EventBus {
    fn channel(&self) -> &TypedChannel<DiscoveryPhase> {
        &self.discovery_channel
    }
}
