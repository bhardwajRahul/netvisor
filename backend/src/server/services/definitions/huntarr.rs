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

    /// No published container image resolves for Huntarr, so its web UI's response has not been
    /// seen and any match string would be a guess.
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
