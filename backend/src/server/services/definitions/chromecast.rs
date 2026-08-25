use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{DnsSdServiceType, Pattern, Vendor};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct ChromecastDevice;

impl ServiceDefinition for ChromecastDevice {
    fn name(&self) -> &'static str {
        "Chromecast"
    }

    fn description(&self) -> &'static str {
        "Google Chromecast streaming device"
    }

    fn category(&self) -> ServiceCategory {
        ServiceCategory::IoT
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        // The OUI is shared with every other Google device, and the port pair with Google Home,
        // so the mDNS arm is what actually separates a Chromecast from a speaker — it is the
        // service type only a Cast receiver advertises. Kept as an alternative rather than a
        // requirement so a Chromecast on a routed subnet, where multicast never reaches, is still
        // found by its ports.
        Pattern::AllOf(vec![
            Pattern::MacVendor(Vendor::GOOGLE),
            Pattern::AnyOf(vec![
                Pattern::DnsSdService(DnsSdServiceType::GOOGLE_CAST),
                Pattern::AllOf(vec![
                    Pattern::Port(PortType::new_tcp(8008)),
                    Pattern::Port(PortType::new_tcp(8009)),
                ]),
            ]),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://simpleicons.org/icons/googlecast.svg"
    }
    fn logo_needs_white_background(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<ChromecastDevice>
));
