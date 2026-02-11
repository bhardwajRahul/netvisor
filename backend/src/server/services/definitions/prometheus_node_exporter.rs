use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct PrometheusNodeExporter;

impl ServiceDefinition for PrometheusNodeExporter {
    fn name(&self) -> &'static str {
        "Prometheus Node Exporter"
    }
    fn description(&self) -> &'static str {
        "Prometheus exporter for hardware and OS metrics"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Monitoring
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(PortType::JetDirect, "/metrics", "node_exporter", None)
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/prometheus.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<PrometheusNodeExporter>
));
