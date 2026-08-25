use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{DnsSdServiceType, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct HomePod;

impl ServiceDefinition for HomePod {
    fn name(&self) -> &'static str {
        "HomePod"
    }
    fn description(&self) -> &'static str {
        "Apple smart speaker and HomeKit hub"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Media
    }

    /// Matched on the AirPlay TXT `model`, which reads `AudioAccessory5,1` and similar.
    ///
    /// An earlier version asked for `_airplay._tcp` and `_raop._tcp` with no
    /// `_companion-link._tcp`, reasoning only about how to tell a HomePod from an Apple TV. A live
    /// scan then labelled two Sonos speakers HomePods: every AirPlay 2 speaker advertises exactly
    /// that combination, and the definition had no way to say *whose* speaker it was. The Sonos
    /// announces `model=Five` and `manufacturer=Sonos` in the same TXT record that a HomePod uses
    /// to announce itself, so matching the model is both narrower and simpler than the three
    /// service-type arms it replaces.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::DnsSd(DnsSdServiceType::AIRPLAY, Some(("model", "AudioAccessory")))
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/apple.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<HomePod>));
