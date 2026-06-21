use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Unimus;

impl ServiceDefinition for Unimus {
    fn name(&self) -> &'static str {
        "Unimus"
    }
    fn description(&self) -> &'static str {
        "Network device configuration backup and change management"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Backup
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(PortType::new_tcp(8085), "/", "<title>Unimus</title>", None)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/unimus.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Unimus>));
