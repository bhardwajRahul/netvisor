use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Proxmox;

impl ServiceDefinition for Proxmox {
    fn name(&self) -> &'static str {
        "Proxmox VE"
    }
    fn description(&self) -> &'static str {
        "Open-source virtualization management platform"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Hypervisor
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        // The bare-port arm is gone. As an `AnyOf` alternative it meant any listener on 8006 was
        // claimed as Proxmox whether or not the page said so, which made the endpoint match beside
        // it decorative.
        Pattern::Endpoint(PortType::new_tcp(8006), "/", "proxmox", None)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/proxmox.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Proxmox>));
