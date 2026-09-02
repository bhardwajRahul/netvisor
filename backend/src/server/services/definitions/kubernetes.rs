use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Kubernetes;

impl ServiceDefinition for Kubernetes {
    fn name(&self) -> &'static str {
        "Kubernetes"
    }
    fn description(&self) -> &'static str {
        "Container orchestration platform"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Orchestrator
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            // The API server refuses an unauthenticated request, and the refusal is the evidence:
            // it comes back as the API's own `Status` object rather than a bare 401, so it can only
            // have been produced by something running the Kubernetes API. Anonymous access to
            // `/livez` is upstream-default but most distributions disable it, which is why this
            // matches the error rather than a health check.
            Pattern::Endpoint(
                PortType::Kubernetes,
                "/version",
                "\"kind\": \"Status\"",
                Some(400..500),
            ),
            // The control-plane and node ports, which qualify the above rather than standing alone.
            Pattern::AnyOf(vec![
                Pattern::Port(PortType::new_tcp(10250)),
                Pattern::Port(PortType::new_tcp(10259)),
                Pattern::Port(PortType::new_tcp(10257)),
                Pattern::Port(PortType::new_tcp(10256)),
            ]),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/kubernetes.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Kubernetes>));
