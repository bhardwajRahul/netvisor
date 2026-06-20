use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Podman;

impl ServiceDefinition for Podman {
    fn name(&self) -> &'static str {
        "Podman"
    }
    fn description(&self) -> &'static str {
        "Daemonless container runtime"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::ContainerRuntime
    }

    // Podman exposes only a local unix socket by default (no network port), so
    // there is no reliable network signature to auto-detect. Like Docker, real
    // detection requires a credentialed socket probe (deferred). Until then it
    // is manually addable and participates as a container-runtime virtualizer.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::None
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/podman.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Podman>));
