use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::smb::SmbProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Samba;

impl ServiceDefinition for Samba {
    fn name(&self) -> &'static str {
        "Samba"
    }
    fn description(&self) -> &'static str {
        "Generic SMB file server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Storage
    }
    /// Derived from the probe, so a listener on this port that does not speak the protocol is
    /// not claimed as this service.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&SmbProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(SmbProbe)]
    }
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Samba>));
