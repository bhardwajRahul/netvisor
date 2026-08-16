use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{InstantOnDeviceType, Pattern};

/// An Instant On wireless access point, as reported by the site inventory. APs return no port
/// table, so this is usually the only structural thing known about them beyond model and address.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct InstantOnAccessPoint;

impl ServiceDefinition for InstantOnAccessPoint {
    fn name(&self) -> &'static str {
        "Instant On Access Point"
    }
    fn description(&self) -> &'static str {
        "HPE Networking Instant On wireless access point"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkAccess
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::ManagedDeviceType(InstantOnDeviceType::ACCESS_POINT)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/hpe.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<InstantOnAccessPoint>
));
