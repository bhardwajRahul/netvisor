use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{InstantOnDeviceType, Pattern};

/// An Instant On gateway/router, as reported by the site inventory.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct InstantOnGateway;

impl ServiceDefinition for InstantOnGateway {
    fn name(&self) -> &'static str {
        "Instant On Gateway"
    }
    fn description(&self) -> &'static str {
        "HPE Networking Instant On gateway"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkAccess
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::ManagedDeviceType(InstantOnDeviceType::GATEWAY)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/hpe.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<InstantOnGateway>
));
