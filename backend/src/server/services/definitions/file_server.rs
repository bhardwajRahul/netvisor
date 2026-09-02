use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::ftp::FtpProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct FileServer;

impl ServiceDefinition for FileServer {
    fn name(&self) -> &'static str {
        "FTP Server"
    }
    fn description(&self) -> &'static str {
        "Generic FTP file sharing service"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Storage
    }

    /// Derived from the probe, so a listener on this port that does not speak the protocol is
    /// not claimed as this service.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&FtpProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(FtpProbe)]
    }

    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<FileServer>));
