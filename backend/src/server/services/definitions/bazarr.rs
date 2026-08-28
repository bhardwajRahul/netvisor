use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Bazarr;

impl ServiceDefinition for Bazarr {
    fn name(&self) -> &'static str {
        "Bazarr"
    }
    fn description(&self) -> &'static str {
        "A companion application to Sonarr and Radarr that manages and downloads subtitles"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Media
    }

    /// The login page is served unauthenticated and carries the application name in its title.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(PortType::new_tcp(6767), "/", "<title>Bazarr</title>", None)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/bazarr.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Bazarr>));
