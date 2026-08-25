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
    /// Matched on the AirPlay TXT `model`, not on service types. An earlier version required
    /// `_airplay._tcp` plus `_companion-link._tcp` on the theory that the pairing channel
    /// separated a TV from a speaker; a live scan promptly labelled a MacBook Pro an Apple TV,
    /// because a Mac advertises both. The service type states a capability — "can receive
    /// AirPlay", true of Macs, speakers and televisions — while `model` states what the device is.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::DnsSd(DnsSdServiceType::AIRPLAY, Some(("model", "AppleTV")))
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/apple.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<AppleTv>));
