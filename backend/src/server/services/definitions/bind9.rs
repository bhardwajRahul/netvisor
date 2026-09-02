use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Bind9;

impl ServiceDefinition for Bind9 {
    fn name(&self) -> &'static str {
        "Bind9"
    }
    fn description(&self) -> &'static str {
        "Berkeley Internet Name Domain DNS server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::DNS
    }

    /// The statistics channel serves XML referencing BIND's own stylesheet, which is what
    /// separates BIND from any other DNS server answering on 53.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::Port(PortType::DnsUdp),
            Pattern::Endpoint(PortType::new_tcp(8053), "/", "/bind9.xsl", None),
        ])
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Bind9>));
