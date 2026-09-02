use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{ConnectOnly, ServiceDefinition};
use crate::server::services::r#impl::patterns::Pattern;

/// Beszel Agent - Lightweight server monitoring agent
///
/// Port 45876 (TCP) is the default port for the Beszel agent's SSH server.
/// The agent uses a custom SSH-based protocol (via gliderlabs/ssh) for secure
/// metric transmission to the Beszel hub. The hub pulls metrics from agents
/// through this SSH connection.
///
/// Detection: Port-only (Medium confidence for unique port).
/// Protocol-level detection would require sending SSH handshake and checking
/// for identifiable characteristics, which is not currently supported.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct BeszelAgent;

impl ServiceDefinition for BeszelAgent {
    fn name(&self) -> &'static str {
        "Beszel Agent"
    }
    fn description(&self) -> &'static str {
        "Lightweight server monitoring agent"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Monitoring
    }
    /// The agent speaks an SSH-framed protocol but expects the hub's key, and there is no published
    /// exchange an unauthenticated peer can complete to confirm it.
    fn connect_only_rationale(&self) -> Option<ConnectOnly> {
        Some(ConnectOnly::NoVerifiableImplementation)
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        // Port 45876 is the default SSH server port for Beszel agents
        Pattern::Port(PortType::new_tcp(45876))
    }
    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/beszel.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<BeszelAgent>));
