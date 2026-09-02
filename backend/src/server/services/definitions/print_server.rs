use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct PrintServer;

impl ServiceDefinition for PrintServer {
    fn name(&self) -> &'static str {
        "Print Server"
    }
    fn description(&self) -> &'static str {
        "A generic printing service"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Printer
    }

    /// IPP is HTTP, and a CUPS server names itself in the `Server` header on every response
    /// (`CUPS/2.4 IPP/2.1`).
    ///
    /// The LPD arms are gone. 515/udp was never reachable — nothing reports a UDP port open
    /// without a probe behind it — and 515/tcp rested on a bare connect, which any middlebox in
    /// the path satisfies. RFC 1179 gives LPD no greeting and no capability query to replace it
    /// with: a status request needs a queue name we do not have.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Header(Some(PortType::Ipp), "server", "CUPS", None)
    }

    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<PrintServer>));
