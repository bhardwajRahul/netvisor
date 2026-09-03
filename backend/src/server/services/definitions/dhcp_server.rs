use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::udp::DhcpProbe;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct DhcpServer;

impl ServiceDefinition for DhcpServer {
    fn name(&self) -> &'static str {
        "DHCP Server"
    }
    fn description(&self) -> &'static str {
        "A generic DHCP server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkCore
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Port(PortType::Dhcp)
    }

    fn is_generic(&self) -> bool {
        true
    }

    /// `discovery_pattern` is deliberately left as it was. Routing the probe through
    /// `probe_pattern` would be equivalent here, but the four migrated probes contribute no
    /// `ClientProbe`, so the only thing that would change is the shape of the pattern — and this
    /// migration is supposed to change nothing that a scan can observe.
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(DhcpProbe)]
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<DhcpServer>));
