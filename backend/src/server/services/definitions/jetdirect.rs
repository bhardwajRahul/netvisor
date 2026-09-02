use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{ConnectOnly, ServiceDefinition};
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct JetDirect;

impl ServiceDefinition for JetDirect {
    fn name(&self) -> &'static str {
        "JetDirect"
    }
    fn description(&self) -> &'static str {
        "HP JetDirect RAW printing protocol"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Printer
    }

    /// A raw-socket printer port prints whatever it is sent. There is no query that is not also a
    /// print job, so this one is detected by the port and stays that way.
    fn connect_only_rationale(&self) -> Option<ConnectOnly> {
        Some(ConnectOnly::ProbeUnsafe)
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Port(PortType::JetDirect)
    }

    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<JetDirect>));
