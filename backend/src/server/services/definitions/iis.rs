use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Iis;

impl ServiceDefinition for Iis {
    fn name(&self) -> &'static str {
        "Microsoft IIS"
    }
    fn description(&self) -> &'static str {
        "Microsoft Internet Information Services web server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::ReverseProxy
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Header(None, "Server", "Microsoft-IIS", None)
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Iis>));
