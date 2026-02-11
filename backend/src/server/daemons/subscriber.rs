//! Event subscriber implementation for DaemonService.
//!
//! Subscribes to Discovery entity events (DiscoveryStarted, DiscoveryCancelled)
//! and handles them for ServerPoll-mode daemons by sending HTTP requests.

use async_trait::async_trait;

use crate::daemon::discovery::types::base::DiscoveryPhase;
use crate::server::daemons::r#impl::api::DaemonDiscoveryRequest;
use crate::server::daemons::r#impl::base::DaemonMode;
use crate::server::daemons::service::DaemonService;
use crate::server::shared::events::bus::{EventFilter, EventSubscriber};
use crate::server::shared::events::types::Event;
use crate::server::shared::services::traits::CrudService;

#[async_trait]
impl EventSubscriber for DaemonService {
    fn event_filter(&self) -> EventFilter {
        // Subscribe to Discovery entity events with DiscoveryStarted/Cancelled operations
        EventFilter::discovery_only(Some(vec![
            DiscoveryPhase::Started,
            DiscoveryPhase::Cancelled,
        ]))
    }

    async fn handle_events(&self, events: Vec<Event>) -> Result<(), anyhow::Error> {
        for event in events {
            if let Event::Discovery(discovery_event) = event {
                // Check if daemon is ServerPoll mode and reachable
                let Some(daemon) = self.get_by_id(&discovery_event.daemon_id).await? else {
                    tracing::debug!(
                        daemon_id = %discovery_event.daemon_id,
                        "Daemon not found for discovery event, skipping"
                    );
                    continue;
                };

                if daemon.base.mode != DaemonMode::ServerPoll || daemon.base.is_unreachable {
                    tracing::trace!(
                        daemon_id = %discovery_event.daemon_id,
                        mode = ?daemon.base.mode,
                        is_unreachable = daemon.base.is_unreachable,
                        "Daemon not eligible for discovery event handling, skipping"
                    );
                    continue;
                }

                // Get the API key - optional for legacy daemons (< v0.14.0)
                // Legacy daemons only have /api/discovery/initiate and /api/discovery/cancel
                // and they don't require authentication
                let api_key = if daemon.supports_full_server_poll() {
                    match self.get_daemon_api_key(&daemon).await {
                        Ok(key) => Some(key),
                        Err(e) => {
                            tracing::error!(
                                error = ?e,
                                daemon_id = %discovery_event.daemon_id,
                                "Failed to get API key for daemon, skipping event"
                            );
                            continue;
                        }
                    }
                } else {
                    tracing::debug!(
                        daemon_id = %discovery_event.daemon_id,
                        version = ?daemon.base.version,
                        "Legacy daemon (< v0.14.0) - sending discovery command without auth"
                    );
                    None
                };

                match discovery_event.phase {
                    DiscoveryPhase::Started => {
                        tracing::info!(
                            daemon_id = %discovery_event.daemon_id,
                            session_id = %discovery_event.session_id,
                            "Handling DiscoveryStarted event for ServerPoll daemon"
                        );

                        let request = DaemonDiscoveryRequest {
                            session_id: discovery_event.session_id,
                            discovery_type: discovery_event.discovery_type,
                        };

                        if let Err(e) = self
                            .send_discovery_request_to_daemon(&daemon, api_key.as_deref(), request)
                            .await
                        {
                            tracing::error!(
                                error = ?e,
                                daemon_id = %discovery_event.daemon_id,
                                session_id = %discovery_event.session_id,
                                "Failed to send discovery request to daemon"
                            );
                        }
                    }
                    DiscoveryPhase::Cancelled => {
                        tracing::info!(
                            daemon_id = %discovery_event.daemon_id,
                            session_id = %discovery_event.session_id,
                            "Handling DiscoveryCancelled event for ServerPoll daemon"
                        );

                        if let Err(e) = self
                            .send_discovery_cancellation_to_daemon(
                                &daemon,
                                api_key.as_deref(),
                                discovery_event.session_id,
                            )
                            .await
                        {
                            tracing::error!(
                                error = ?e,
                                daemon_id = %discovery_event.daemon_id,
                                session_id = %discovery_event.session_id,
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
