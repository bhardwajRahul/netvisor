//! Event subscriber implementation for DaemonService.
//!
//! Subscribes to Discovery entity events (DiscoveryStarted, DiscoveryCancelled)
//! and handles them for ServerPoll-mode daemons by sending HTTP requests.

use std::collections::HashMap;

use async_trait::async_trait;
use uuid::Uuid;

use crate::server::daemons::r#impl::api::DaemonDiscoveryRequest;
use crate::server::daemons::r#impl::base::DaemonMode;
use crate::server::daemons::service::DaemonService;
use crate::server::shared::entities::{Entity, EntityDiscriminants};
use crate::server::shared::events::bus::{EventFilter, EventSubscriber};
use crate::server::shared::events::types::{EntityOperation, Event};
use crate::server::shared::services::traits::CrudService;

#[async_trait]
impl EventSubscriber for DaemonService {
    fn event_filter(&self) -> EventFilter {
        // Subscribe to Discovery entity events with DiscoveryStarted/Cancelled operations
        EventFilter::entity_only(HashMap::from([(
            EntityDiscriminants::Discovery,
            Some(vec![
                EntityOperation::DiscoveryStarted,
                EntityOperation::DiscoveryCancelled,
            ]),
        )]))
    }

    async fn handle_events(&self, events: Vec<Event>) -> Result<(), anyhow::Error> {
        for event in events {
            if let Event::Entity(entity_event) = event {
                // Extract discovery from entity_type
                let Entity::Discovery(discovery) = &entity_event.entity_type else {
                    continue;
                };
                let daemon_id = discovery.base.daemon_id;
                let discovery_type = discovery.base.discovery_type.clone();

                // Extract session_id from metadata - skip if missing (graceful handling)
                let Some(session_id) = entity_event.metadata["session_id"]
                    .as_str()
                    .and_then(|s| Uuid::parse_str(s).ok())
                else {
                    tracing::warn!(
                        event_id = %entity_event.id,
                        operation = ?entity_event.operation,
                        "Discovery event missing session_id in metadata, skipping"
                    );
                    continue;
                };

                // Check if daemon is ServerPoll mode and reachable
                let Some(daemon) = self.get_by_id(&daemon_id).await? else {
                    tracing::debug!(
                        daemon_id = %daemon_id,
                        "Daemon not found for discovery event, skipping"
                    );
                    continue;
                };

                if daemon.base.mode != DaemonMode::ServerPoll || daemon.base.is_unreachable {
                    tracing::trace!(
                        daemon_id = %daemon_id,
                        mode = ?daemon.base.mode,
                        is_unreachable = daemon.base.is_unreachable,
                        "Daemon not eligible for discovery event handling, skipping"
                    );
                    continue;
                }

                // Get the API key for this daemon
                let api_key = match self.get_daemon_api_key(&daemon).await {
                    Ok(key) => key,
                    Err(e) => {
                        tracing::error!(
                            error = ?e,
                            daemon_id = %daemon_id,
                            "Failed to get API key for daemon, skipping event"
                        );
                        continue;
                    }
                };

                match entity_event.operation {
                    EntityOperation::DiscoveryStarted => {
                        tracing::info!(
                            daemon_id = %daemon_id,
                            session_id = %session_id,
                            "Handling DiscoveryStarted event for ServerPoll daemon"
                        );

                        let request = DaemonDiscoveryRequest {
                            session_id,
                            discovery_type,
                        };

                        if let Err(e) = self
                            .send_discovery_request_to_daemon(&daemon, &api_key, request)
                            .await
                        {
                            tracing::error!(
                                error = ?e,
                                daemon_id = %daemon_id,
                                session_id = %session_id,
                                "Failed to send discovery request to daemon"
                            );
                        }
                    }
                    EntityOperation::DiscoveryCancelled => {
                        tracing::info!(
                            daemon_id = %daemon_id,
                            session_id = %session_id,
                            "Handling DiscoveryCancelled event for ServerPoll daemon"
                        );

                        if let Err(e) = self
                            .send_discovery_cancellation_to_daemon(&daemon, &api_key, session_id)
                            .await
                        {
                            tracing::error!(
                                error = ?e,
                                daemon_id = %daemon_id,
                                session_id = %session_id,
                                "Failed to send cancellation to daemon"
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "daemon-discovery-events"
    }
}
