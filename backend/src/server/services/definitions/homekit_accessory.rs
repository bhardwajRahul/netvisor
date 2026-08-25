use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{DnsSdServiceType, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct HomeKitAccessory;

impl ServiceDefinition for HomeKitAccessory {
    fn name(&self) -> &'static str {
        "HomeKit Accessory"
    }
    fn description(&self) -> &'static str {
        "A device speaking the HomeKit Accessory Protocol"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::IoT
    }

    /// Sensors, plugs, locks and bulbs that speak HAP. Most expose no scannable TCP port at all —
    /// HAP runs on an ephemeral port the accessory picks and only announces over mDNS — so this
    /// whole population was previously invisible to a port-scan-driven discovery.
    ///
    /// Generic on purpose: the accessory's TXT `ci=` category would narrow this to a lock or a
    /// sensor, and `Pattern::DnsSd` can now read it, but the category is a number whose meaning is
    /// defined by Apple's accessory profiles — worth splitting only against real devices to check
    /// against, which is how the Apple TV and HomePod definitions had to be corrected. Excludes
    /// the Apple hubs, which advertise HAP as well but are their own devices.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::DnsSd(DnsSdServiceType::HOMEKIT, None),
            Pattern::Not(Box::new(Pattern::DnsSd(DnsSdServiceType::AIRPLAY, None))),
        ])
    }

    fn is_generic(&self) -> bool {
        true
    }

    fn logo_url(&self) -> &'static str {
        // The icon set has no HomeKit mark of its own; Apple's is the closest true thing.
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/apple.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<HomeKitAccessory>
));
