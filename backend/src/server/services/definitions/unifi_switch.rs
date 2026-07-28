use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, UnifiDeviceType, Vendor};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct UnifiSwitch;

impl ServiceDefinition for UnifiSwitch {
    fn name(&self) -> &'static str {
        "UniFi Switch"
    }
    fn description(&self) -> &'static str {
        "Ubiquiti UniFi managed switch"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkCore
    }

    /// UniFi switches expose no distinguishing network banner — that is the whole reason this
    /// integration exists — so the controller's own inventory is the only evidence. Narrower
    /// than the generic `Switch` definition, and it carries the UniFi logo.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::MacVendor(Vendor::UBIQUITI),
            Pattern::ManagedDeviceType(UnifiDeviceType::SWITCH),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/unifi.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<UnifiSwitch>));
