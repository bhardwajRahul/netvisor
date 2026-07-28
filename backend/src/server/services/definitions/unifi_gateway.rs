use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, UnifiDeviceType, Vendor};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct UnifiGateway;

impl ServiceDefinition for UnifiGateway {
    fn name(&self) -> &'static str {
        "UniFi Gateway"
    }
    fn description(&self) -> &'static str {
        "Ubiquiti UniFi security gateway or Dream Machine"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkCore
    }

    /// Covers both the standalone security gateway (`ugw`) and the Dream Machine family
    /// (`udm`), which is a gateway that also hosts the controller. A UDM therefore matches
    /// this *and* `UnifiController`, which is correct — it is both things.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::MacVendor(Vendor::UBIQUITI),
            Pattern::AnyOf(vec![
                Pattern::ManagedDeviceType(UnifiDeviceType::GATEWAY),
                Pattern::ManagedDeviceType(UnifiDeviceType::DREAM_MACHINE),
            ]),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/unifi.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<UnifiGateway>
));
