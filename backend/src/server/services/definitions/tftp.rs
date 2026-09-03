use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::tftp::TftpProbe;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Tftp;

impl ServiceDefinition for Tftp {
    fn name(&self) -> &'static str {
        "TFTP Server"
    }
    fn description(&self) -> &'static str {
        "Trivial file transfer, used to load device firmware and configuration"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkCore
    }

    /// A bare `Pattern::Port` on a **UDP** port, which is validated evidence rather than a bare
    /// connect: a UDP port is only ever reported open when its registered probe answered. The
    /// equivalent on TCP is what this whole line of work exists to remove.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Port(PortType::new_udp(69))
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(TftpProbe)]
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Tftp>));
