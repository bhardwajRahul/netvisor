use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::bacula::BaculaProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Bacula;

impl ServiceDefinition for Bacula {
    fn name(&self) -> &'static str {
        "Bacula"
    }
    fn description(&self) -> &'static str {
        "Network backup solution"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Backup
    }

    /// The Director's port, qualified by the CRAM-MD5 challenge it issues to an unauthenticated
    /// `Hello`.
    ///
    /// This was declared `NoDistinguishingHandshake` on the reading that the Director authenticates
    /// before identifying itself. It sends the challenge first, in plaintext, and the challenge
    /// names the algorithm; see [`crate::daemon::utils::app_probe::bacula`].
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&BaculaProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(BaculaProbe)]
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/bacula.png"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Bacula>));
