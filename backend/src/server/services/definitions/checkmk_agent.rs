use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

/// CheckMK Agent - Monitoring agent for CheckMK server
///
/// Port 6556 (TCP) is the IANA-assigned port for CheckMK agent.
/// The agent uses a text-based protocol where the server connects and
/// receives monitoring data in a custom format with sections like
/// <<<check_mk>>>, <<<cpu>>>, <<<mem>>>, etc.
///
/// Detection: Port-only (Medium confidence for IANA-assigned port).
///
/// Note: This is for the CheckMK Agent (port 6556), not the CheckMK Server
/// which has an HTTP interface.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct CheckmkAgent;

impl ServiceDefinition for CheckmkAgent {
    fn name(&self) -> &'static str {
        "CheckMK Agent"
    }
    fn description(&self) -> &'static str {
        "Monitoring agent for CheckMK server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Monitoring
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        // Port 6556 is the IANA-assigned port for CheckMK agent
        Pattern::Port(PortType::new_tcp(6556))
    }
    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/checkmk.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<CheckmkAgent>
));
