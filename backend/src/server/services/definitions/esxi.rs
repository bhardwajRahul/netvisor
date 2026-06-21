use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Esxi;

impl ServiceDefinition for Esxi {
    fn name(&self) -> &'static str {
        "ESXi"
    }
    fn description(&self) -> &'static str {
        "VMware ESXi bare-metal hypervisor"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Hypervisor
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(PortType::Https, "/", "VMware ESXi", None)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/vmware-esxi.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Esxi>));
