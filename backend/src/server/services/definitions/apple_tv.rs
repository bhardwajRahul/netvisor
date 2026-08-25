use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{DnsSdServiceType, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct AppleTv;

impl ServiceDefinition for AppleTv {
    fn name(&self) -> &'static str {
        "Apple TV"
    }
    fn description(&self) -> &'static str {
        "Apple set-top box and HomeKit hub"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Media
    }

    /// mDNS is the only signal that reaches this device. It exposes no TCP port worth scanning —
    /// AirPlay's own port is not in any discovery set and answers nothing useful to a port scan —
    /// so before DNS-SD an Apple TV appeared as a bare address with no service on it.
    ///
    /// `_companion-link._tcp` is what separates it from a HomePod, which advertises AirPlay too:
    /// it is Apple's device-to-device pairing channel, present on the TV and absent on speakers.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::DnsSdService(DnsSdServiceType::AIRPLAY),
            Pattern::DnsSdService(DnsSdServiceType::COMPANION_LINK),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/apple.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<AppleTv>));
