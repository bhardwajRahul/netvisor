use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{ClientProbe, Pattern};

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

    // Podman exposes a Docker-compatible API over a local unix socket (and,
    // when configured, a TCP proxy). Like Docker, there is no passive network
    // signature — detection is the credentialed socket/proxy probe, surfaced
    // here as a Podman client-probe response.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::ClientResponse(ClientProbe::Podman)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/podman.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Podman>));
