use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    email::traits::EmailService,
    shared::events::{
        bus::{EventFilter, EventSubscriber},
        types::{Event, TelemetryOperation},
    },
};
use anyhow::Error;
use async_trait::async_trait;
use std::collections::HashMap;

/// Billing lifecycle operations that should include metadata for Plunk segmentation
const BILLING_LIFECYCLE_OPS: &[TelemetryOperation] = &[
    TelemetryOperation::CheckoutStarted,
    TelemetryOperation::CheckoutCompleted,
    TelemetryOperation::TrialStarted,
    TelemetryOperation::TrialEnded,
    TelemetryOperation::SubscriptionCancelled,
];

#[async_trait]
impl EventSubscriber for EmailService {
    fn event_filter(&self) -> EventFilter {
        EventFilter {
            entity_operations: Some(HashMap::new()),
            auth_operations: Some(vec![]),
            telemetry_operations: None, // Subscribe to all telemetry events
            network_ids: Some(vec![]),
        }
    }

    async fn handle_events(&self, events: Vec<Event>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        for event in events {
            if let AuthenticatedEntity::User { email, .. } = event.authentication() {
                let operation = event.operation();

                // For billing lifecycle events, include metadata for Plunk segmentation
                let data = if let Event::Telemetry(ref telemetry) = event {
                    if BILLING_LIFECYCLE_OPS.contains(&telemetry.operation) {
                        Some(event.metadata())
                    } else {
                        None
                    }
                } else {
                    None
                };

                self.track_event(operation.to_string().to_lowercase(), email, data)
                    .await?;
            };
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "email_triggers"
    }
}
