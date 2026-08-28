use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::{ConnectOnly, ServiceDefinition};
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct HyperFile;

impl ServiceDefinition for HyperFile {
    fn name(&self) -> &'static str {
        "HyperFile Server"
    }
    fn description(&self) -> &'static str {
        "PC SOFT HFSQL Client-Server database (WinDev/WebDev)"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Database
    }

    /// HyperFileSQL's wire protocol is proprietary and undocumented, and there is no public build
    /// to check a match against.
    fn connect_only_rationale(&self) -> Option<ConnectOnly> {
        Some(ConnectOnly::NoVerifiableImplementation)
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Port(PortType::new_tcp(4900))
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<HyperFile>));
