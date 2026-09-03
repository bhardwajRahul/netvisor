use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::ike::IkeProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

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

    /// Derived from the probe. IKE's opening exchange is unauthenticated by construction, so a
    /// responder answers it; a listener that does not is not claimed as IPsec.
    ///
    /// One port here rather than the two the pattern used to name: the probe itself tries 500 and
    /// then 4500, and a pattern arm for 4500 with no probe behind it would be a UDP port nothing
    /// ever reports open.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&IkeProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(IkeProbe)]
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<StrongSwan>));
