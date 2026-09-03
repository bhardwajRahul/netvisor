use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Neo4j;

impl ServiceDefinition for Neo4j {
    fn name(&self) -> &'static str {
        "Neo4j"
    }
    fn description(&self) -> &'static str {
        "Graph database"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Database
    }
    /// The HTTP discovery endpoint on 7474 answers unauthenticated with the server's own version
    /// and Bolt routing URIs.
    ///
    /// 7474 was not previously scanned at all — the pattern listed 7473 (the HTTPS browser) and
    /// 7687 (Bolt) — so this both validates the detection and reaches the port that identifies it.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(PortType::new_tcp(7474), "/", "neo4j_version", None)
    }
    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/png/neo4j.png"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Neo4j>));
