use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::saltmaster::{
    SALT_PUBLISH_PORT, SALT_REQUEST_PORT, SALT_SSH_PORT,
};
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{ClientProbe, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct SaltProxy;

impl ServiceDefinition for SaltProxy {
    fn name(&self) -> &'static str {
        "Salt Proxy"
    }
    fn description(&self) -> &'static str {
        "A Salt Proxy server acts as a proxy between the Salt Master and the minions."
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Development
    }

    /// The same two ZeroMQ ports as a master, plus Salt SSH on 8022 — the port that distinguishes
    /// the two.
    ///
    /// `ClientResponse` rather than [`probe_pattern`]: the ZMTP probes are registered by
    /// [`SaltMaster`], since a port may only be claimed once, and the evidence they produce is keyed
    /// by [`ClientProbe`] rather than by which definition asked for it.
    ///
    /// [`probe_pattern`]: crate::server::services::r#impl::patterns::probe_pattern
    /// [`SaltMaster`]: crate::server::services::definitions::saltmaster::SaltMaster
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::Port(PortType::new_tcp(SALT_PUBLISH_PORT)),
            Pattern::Port(PortType::new_tcp(SALT_REQUEST_PORT)),
            Pattern::ClientResponse(ClientProbe::Zmtp),
            Pattern::Port(PortType::new_tcp(SALT_SSH_PORT)),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/salt-project.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<SaltProxy>));
