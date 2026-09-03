use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::mgcp::{MGCP_CALL_AGENT_PORT, MGCP_GATEWAY_PORT, MgcpProbe};
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Mgcp;

impl ServiceDefinition for Mgcp {
    fn name(&self) -> &'static str {
        "MGCP Endpoint"
    }
    fn description(&self) -> &'static str {
        "Media Gateway Control Protocol, tying a voice gateway to its call agent"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Telephony
    }

    /// Either end of the protocol. A gateway listens on 2427 and a call agent on 2727; finding
    /// either identifies the service, and the two are not usually on one host.
    ///
    /// `AnyOf` of two UDP ports, each only ever reported open when its probe answered an `AUEP`.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::Port(PortType::new_udp(MGCP_GATEWAY_PORT)),
            Pattern::Port(PortType::new_udp(MGCP_CALL_AGENT_PORT)),
        ])
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![
            Box::new(MgcpProbe::new(MGCP_GATEWAY_PORT)),
            Box::new(MgcpProbe::new(MGCP_CALL_AGENT_PORT)),
        ]
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Mgcp>));
