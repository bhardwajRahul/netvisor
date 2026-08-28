use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{ConnectOnly, ServiceDefinition};
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct DockerSwarm;

impl ServiceDefinition for DockerSwarm {
    fn name(&self) -> &'static str {
        "Docker Swarm"
    }
    fn description(&self) -> &'static str {
        "Docker native clustering and orchestration"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Orchestrator
    }

    /// 2377 is the raft port and speaks mutual TLS; 7946 is encrypted gossip. Neither answers an
    /// unauthenticated peer with anything that names Swarm.
    fn connect_only_rationale(&self) -> Option<ConnectOnly> {
        Some(ConnectOnly::NoDistinguishingHandshake)
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::Port(PortType::new_tcp(2377)),
            Pattern::Port(PortType::new_tcp(7946)),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/docker.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<DockerSwarm>));
