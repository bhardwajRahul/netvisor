use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::zmtp::ZmtpProbe;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

/// Salt's publish port, a ZeroMQ PUB socket.
pub(crate) const SALT_PUBLISH_PORT: u16 = 4505;
/// Salt's request port, a ZeroMQ REP socket.
pub(crate) const SALT_REQUEST_PORT: u16 = 4506;
/// Salt SSH's port, which is what separates a proxy from a master.
pub(crate) const SALT_SSH_PORT: u16 = 8022;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct SaltMaster;

impl ServiceDefinition for SaltMaster {
    fn name(&self) -> &'static str {
        "Salt Master"
    }
    fn description(&self) -> &'static str {
        "A Salt master server acts as a central control bus for the clients, which are called minions."
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Development
    }

    /// Both ZeroMQ ports have to answer with a ZMTP greeting, and 8022 has to be absent — that last
    /// clause is what separates a master from a proxy.
    ///
    /// This used to be three bare ports, declared `NoDistinguishingHandshake` because CurveZMQ
    /// encrypts the payload. The greeting that *sets up* CurveZMQ is cleartext and comes first; see
    /// [`crate::daemon::utils::app_probe::zmtp`].
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            probe_pattern(&ZmtpProbe::new(SALT_PUBLISH_PORT)),
            probe_pattern(&ZmtpProbe::new(SALT_REQUEST_PORT)),
            Pattern::Not(Box::new(Pattern::Port(PortType::new_tcp(SALT_SSH_PORT)))),
        ])
    }

    /// The two ZeroMQ ports are registered here rather than on both Salt definitions, because a port
    /// may only be claimed by one probe. `Salt Proxy` matches on the same `ClientProbe::Zmtp`
    /// evidence without registering a second probe for the same port.
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![
            Box::new(ZmtpProbe::new(SALT_PUBLISH_PORT)),
            Box::new(ZmtpProbe::new(SALT_REQUEST_PORT)),
        ]
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/salt-project.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<SaltMaster>));
