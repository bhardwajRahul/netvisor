use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::dns_tcp::DnsTcpProbe;
use crate::daemon::utils::app_probe::udp::DnsProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

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

    /// Both transports, each behind its own probe: the UDP side resolves a real name through a
    /// library client, the TCP side sends a length-prefixed query and matches the response. Either
    /// alone identifies a DNS server, which is why this is `AnyOf` — a resolver reachable only over
    /// TCP is a deliberate configuration, not an edge case.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![probe_pattern(&DnsProbe), probe_pattern(&DnsTcpProbe)])
    }

    fn is_generic(&self) -> bool {
        true
    }

    /// One probe per transport. Both have to be declared here or the port each covers is scanned
    /// and never validated.
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(DnsProbe), Box::new(DnsTcpProbe)]
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<DnsServer>));
