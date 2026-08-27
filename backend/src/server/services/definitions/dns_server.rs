use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::udp::DnsProbe;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct DnsServer;

impl ServiceDefinition for DnsServer {
    fn name(&self) -> &'static str {
        "DNS Server"
    }
    fn description(&self) -> &'static str {
        "A generic DNS server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::DNS
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Port(PortType::DnsTcp),
            Pattern::Port(PortType::DnsUdp),
        ])
    }

    fn is_generic(&self) -> bool {
        true
    }

    /// `discovery_pattern` is deliberately left as it was. Routing the probe through
    /// `probe_pattern` would be equivalent here, but the four migrated probes contribute no
    /// `ClientProbe`, so the only thing that would change is the shape of the pattern — and this
    /// migration is supposed to change nothing that a scan can observe.
    fn app_probe(&self) -> Option<Box<dyn AppProbe>> {
        Some(Box::new(DnsProbe))
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<DnsServer>));
