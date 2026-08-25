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

    /// AirPlay plus RAOP — AirPlay's audio half — and deliberately *not* `_companion-link._tcp`,
    /// which is how this stays distinct from [`super::apple_tv::AppleTv`]. An Apple TV advertises
    /// AirPlay and RAOP as well, so without the negative arm the two definitions would both match
    /// every Apple media device on the link.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::DnsSdService(DnsSdServiceType::AIRPLAY),
            Pattern::DnsSdService(DnsSdServiceType::RAOP),
            Pattern::Not(Box::new(Pattern::DnsSdService(
                DnsSdServiceType::COMPANION_LINK,
            ))),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/apple.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<HomePod>));
