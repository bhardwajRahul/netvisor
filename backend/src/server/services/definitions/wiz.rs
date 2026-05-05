use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, Vendor};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct WiZ;

impl ServiceDefinition for WiZ {
    fn name(&self) -> &'static str {
        "Wiz"
    }
    fn description(&self) -> &'static str {
        "Wiz IoT Devices"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::IoT
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::MacVendor(Vendor::WIZ)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/wiz.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<WiZ>));