use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Termix;

impl ServiceDefinition for Termix {
    fn name(&self) -> &'static str {
        "Termix"
    }
    fn description(&self) -> &'static str {
        "Web-based server management with SSH terminal, tunneling, and file editing"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::RemoteAccess
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Endpoint(PortType::Http, "/", "<title>Termix</title>", None),
            Pattern::Endpoint(PortType::Https, "/", "<title>Termix</title>", None),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/termix.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Termix>));
