use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, Vendor};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct UbiquitiDiscovery;

impl ServiceDefinition for UbiquitiDiscovery {
    fn name(&self) -> &'static str {
        "Ubiquiti Discovery"
    }
    fn description(&self) -> &'static str {
        "Ubiquiti device discovery service"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkAccess
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        // The port arm is gone: 10001/udp has no probe, so nothing ever reported it open and this
        // definition could never match. The OUI is the evidence that remains, and it is real — a
        // Ubiquiti MAC is read from an ARP reply on the host's own segment.
        Pattern::MacVendor(Vendor::UBIQUITI)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/ubiquiti.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<UbiquitiDiscovery>
));
