use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Serviio;

impl ServiceDefinition for Serviio {
    fn name(&self) -> &'static str {
        "Serviio"
    }
    fn description(&self) -> &'static str {
        "DLNA media streaming server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Media
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(
            PortType::new_tcp(23423),
            "/rest/application",
            "<serviioId>",
            None,
        )
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/serviio.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Serviio>));
