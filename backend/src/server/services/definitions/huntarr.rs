use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{ConnectOnly, ServiceDefinition};
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Huntarr;

impl ServiceDefinition for Huntarr {
    fn name(&self) -> &'static str {
        "Huntarr"
    }
    fn description(&self) -> &'static str {
        "Finds missing media and upgrades your existing content."
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Media
    }

    /// The exception stands, and the earlier note that "no published container image resolves" was
    /// right by accident — the images do resolve, they are simply not this service.
    ///
    /// Everything published under `huntarr/` is either a headless `*arr` worker (`4sonarr`,
    /// `4radarr`, `4lidarr`, …), which connects out to a Sonarr API and listens on nothing, or
    /// `huntarr/plexguide`, which is a different product serving 9700. Nothing under that namespace,
    /// `huntarr/huntarr`, or `ghcr.io/plexguide/huntarr` publishes the web UI this port belongs to,
    /// so its response has still never been seen and any match string would be a guess.
    fn connect_only_rationale(&self) -> Option<ConnectOnly> {
        Some(ConnectOnly::NoVerifiableImplementation)
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Port(PortType::new_tcp(9705))
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/huntarr.png"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Huntarr>));
