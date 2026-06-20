use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct MendelsonAs2;

impl ServiceDefinition for MendelsonAs2 {
    fn name(&self) -> &'static str {
        "Mendelson AS2"
    }
    fn description(&self) -> &'static str {
        "Open-source AS2 server for B2B/EDI file exchange"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Unknown
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::Endpoint(
            PortType::Https8443,
            "/as2/HttpReceiver",
            "mendelson opensource",
            None,
        )
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<MendelsonAs2>
));
