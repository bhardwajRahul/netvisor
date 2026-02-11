use crate::{
    daemon::discovery::types::base::DiscoveryPhase,
    server::{
        brevo::service::BrevoService,
        shared::{
            entities::EntityDiscriminants,
            events::{
                bus::{EventFilter, EventSubscriber},
                types::{AuthOperation, EntityOperation, Event},
            },
        },
    },
};
use anyhow::Error;
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[async_trait]
impl EventSubscriber for BrevoService {
    fn event_filter(&self) -> EventFilter {
        let mut entity_ops = HashMap::new();
        entity_ops.insert(
            EntityDiscriminants::Network,
            Some(vec![EntityOperation::Created, EntityOperation::Deleted]),
        );
        entity_ops.insert(
            EntityDiscriminants::Host,
            Some(vec![EntityOperation::Created, EntityOperation::Deleted]),
        );
        entity_ops.insert(
            EntityDiscriminants::User,
            Some(vec![EntityOperation::Created, EntityOperation::Deleted]),
        );

        EventFilter {
            entity_operations: Some(entity_ops),
            auth_operations: Some(vec![AuthOperation::LoginSuccess]),
            telemetry_operations: None,
            discovery_phases: Some(vec![DiscoveryPhase::Scanning]),
            network_ids: None,
        }
    }

    async fn handle_events(&self, events: Vec<Event>) -> Result<(), Error> {
        let mut org_ids_for_metrics: HashSet<Uuid> = HashSet::new();

        for event in &events {
            match event {
                Event::Entity(entity_event) => {
                    if let Some(org_id) = entity_event.organization_id {
                        org_ids_for_metrics.insert(org_id);
                    } else if let Some(network_id) = entity_event.network_id
                        && let Some(org_id) = self.get_org_id_from_network(&network_id).await
                    {
                        org_ids_for_metrics.insert(org_id);
                    }
                }
                _ => {
                    if let Err(e) = self.handle_event(event).await {
                        tracing::warn!(
                            error = %e,
                            event_type = ?event.operation(),
                            "Failed to sync event to Brevo"
                        );
                    }
                }
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

    fn name(&self) -> &str {
        "brevo_crm"
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
