use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::kafka::KafkaProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Kafka;

impl ServiceDefinition for Kafka {
    fn name(&self) -> &'static str {
        "Kafka"
    }
    fn description(&self) -> &'static str {
        "Event streaming platform"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::MessageQueue
    }
    /// Derived from the probe, so a listener on this port that does not speak the protocol is
    /// not claimed as this service.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&KafkaProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(KafkaProbe)]
    }
    fn logo_url(&self) -> &'static str {
        "https://simpleicons.org/icons/apachekafka.svg"
    }
    fn is_generic(&self) -> bool {
        true
    }
    fn logo_needs_white_background(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Kafka>));
