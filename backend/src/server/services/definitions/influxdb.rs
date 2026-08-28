use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct InfluxDB;

impl ServiceDefinition for InfluxDB {
    fn name(&self) -> &'static str {
        "InfluxDB"
    }
    fn description(&self) -> &'static str {
        "Time series database"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Database
    }
    /// `X-Influxdb-Version` is added by middleware to *every* response, so this needs no
    /// particular path and holds for both the OSS and enterprise builds.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Header(Some(PortType::InfluxDb), "x-influxdb-version", "", None)
    }
    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/influxdb.svg"
    }
    fn logo_needs_white_background(&self) -> bool {
        true
    }
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<InfluxDB>));
