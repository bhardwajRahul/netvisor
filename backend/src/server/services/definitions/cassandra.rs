use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::cassandra::CassandraProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Cassandra;

impl ServiceDefinition for Cassandra {
    fn name(&self) -> &'static str {
        "Cassandra"
    }
    fn description(&self) -> &'static str {
        "Distributed NoSQL database"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Database
    }
    /// Derived from the probe, so a listener on this port that does not speak the protocol is
    /// not claimed as this service.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&CassandraProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(CassandraProbe)]
    }
    fn is_generic(&self) -> bool {
        true
    }
    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/apache-cassandra.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Cassandra>));
