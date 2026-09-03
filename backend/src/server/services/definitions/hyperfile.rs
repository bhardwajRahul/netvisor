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

    /// The exception stands, but the reason recorded here was wrong and is now checked rather than
    /// assumed.
    ///
    /// There *is* a public build — `windev/hfsql`, PC SOFT's own image — so the claim that none
    /// exists was false. Running it (amd64 only, under emulation, with `HFSQL_ALLOW_EMPTY_PASSWORD`)
    /// shows the real obstacle: the server accepts the connection, volunteers nothing, and answers
    /// nothing to any opener that can be constructed without the proprietary protocol. That is
    /// `NoDistinguishingHandshake` — the same shape a middlebox produces, which is exactly why no
    /// match string is invented for it here.
    fn connect_only_rationale(&self) -> Option<ConnectOnly> {
        Some(ConnectOnly::NoDistinguishingHandshake)
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Port(PortType::new_tcp(4900))
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<HyperFile>));
