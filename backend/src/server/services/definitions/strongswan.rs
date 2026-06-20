use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct StrongSwan;

impl ServiceDefinition for StrongSwan {
    fn name(&self) -> &'static str {
        "StrongSwan"
    }
    fn description(&self) -> &'static str {
        "Open-source IPsec VPN daemon (IKE / NAT-T)"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::VPN
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::Port(PortType::new_udp(500)),
            Pattern::Port(PortType::new_udp(4500)),
        ])
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<StrongSwan>));
