//! Event subscriber implementation for DaemonService.
//!
//! Subscribes to discovery events with `Started` / `Cancelled` phases and
//! handles them for ServerPoll-mode daemons.

use async_trait::async_trait;

use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryPhaseDiscriminants};
use crate::server::daemons::r#impl::base::DaemonMode;
use crate::server::daemons::service::DaemonService;
use crate::server::shared::events::registry::SubscriberRegistration;
use crate::server::shared::events::traits::{Event, EventFilter, Subscriber};
use crate::server::shared::services::traits::CrudService;

#[async_trait]
impl Subscriber<DiscoveryPhase> for DaemonService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::ops(vec![
            DiscoveryPhaseDiscriminants::Started,
            DiscoveryPhaseDiscriminants::Cancelled,
        ])
    }

    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), anyhow::Error> {
        for event in events {
            let daemon_id = event.scope.daemon_id;
            let session_id = event.scope.session_id;
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

            // Get the API key - optional for legacy daemons (< v0.14.0)
            let api_key = if daemon.supports_full_server_poll() {
                match self.get_daemon_api_key(&daemon).await {
                    Ok(key) => Some(key),
                    Err(e) => {
                        tracing::error!(
                            error = ?e,
                            daemon_id = %daemon_id,
                            "Failed to get API key for daemon, skipping event"
                        );
                        continue;
                    }
                }
            } else {
                tracing::debug!(
                    daemon_id = %daemon_id,
                    version = ?daemon.base.version,
                    "Legacy daemon (< v0.14.0) - sending discovery command without auth"
                );
                None
            };

            match event.operation {
                DiscoveryPhase::Started => {
                    tracing::debug!(
                        daemon_id = %daemon_id,
                        session_id = %session_id,
                        "Discovery session queued — will dispatch when daemon reports ready"
                    );
                }
                DiscoveryPhase::Cancelled => {
                    tracing::info!(
                        daemon_id = %daemon_id,
                        session_id = %session_id,
                        "Handling DiscoveryCancelled event for ServerPoll daemon"
                    );

                    if let Err(e) = self
                        .send_discovery_cancellation_to_daemon(
                            &daemon,
                            api_key.as_deref(),
                            session_id,
                        )
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
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<DaemonService, DiscoveryPhase>());
