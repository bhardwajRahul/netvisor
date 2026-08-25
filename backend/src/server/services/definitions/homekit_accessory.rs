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
    /// Generic on purpose: the accessory's TXT `ci=` category would narrow it to a lock or a
    /// sensor, but a pattern cannot read TXT values yet, so one definition covering the protocol
    /// beats guessing. Excludes the Apple hubs, which advertise HAP as well but are their own
    /// devices.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::DnsSdService(DnsSdServiceType::HOMEKIT),
            Pattern::Not(Box::new(Pattern::DnsSdService(DnsSdServiceType::AIRPLAY))),
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
