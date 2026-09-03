use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct PowerDNS;

impl ServiceDefinition for PowerDNS {
    fn name(&self) -> &'static str {
        "PowerDNS"
    }
    fn description(&self) -> &'static str {
        "Authoritative DNS server with API"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::DNS
    }

    /// The built-in webserver's root page is served unauthenticated and titles itself, which is
    /// what separates PowerDNS from any other DNS server answering on 53. Only the `/api/` paths
    /// are gated behind `X-API-Key`.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::Port(PortType::DnsUdp),
            Pattern::Endpoint(
                PortType::Http8081,
                "/",
                "PowerDNS Authoritative Server Monitor",
                None,
            ),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/powerdns.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<PowerDNS>));
