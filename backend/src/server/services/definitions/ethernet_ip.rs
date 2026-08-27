use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::ethernet_ip::EtherNetIpProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct EtherNetIp;

impl ServiceDefinition for EtherNetIp {
    fn name(&self) -> &'static str {
        "EtherNet/IP"
    }
    fn description(&self) -> &'static str {
        "CIP-based industrial protocol used by Rockwell and Allen-Bradley automation equipment"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Industrial
    }
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&EtherNetIpProbe)
    }
    fn app_probe(&self) -> Option<Box<dyn AppProbe>> {
        Some(Box::new(EtherNetIpProbe))
    }
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<EtherNetIp>));
