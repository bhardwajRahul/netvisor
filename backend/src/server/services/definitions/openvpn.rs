use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::openvpn::OpenVpnProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct OpenVpn;

impl ServiceDefinition for OpenVpn {
    fn name(&self) -> &'static str {
        "OpenVPN"
    }
    fn description(&self) -> &'static str {
        "OpenVPN server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::VPN
    }
    /// Derived from the probe, so a listener on this port that does not speak the protocol is
    /// not claimed as this service.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&OpenVpnProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(OpenVpnProbe)]
    }
    fn is_generic(&self) -> bool {
        true
    }
    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/openvpn.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<OpenVpn>));
