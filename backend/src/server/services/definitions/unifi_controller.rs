use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{ClientProbe, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct UnifiController;

impl ServiceDefinition for UnifiController {
    fn name(&self) -> &'static str {
        "UniFi Controller"
    }
    fn description(&self) -> &'static str {
        "Ubiquiti UniFi network controller"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkAccess
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        // The endpoint arm only ever caught a legacy self-hosted Network Application on 8443.
        // A UniFi OS console (443) or UniFi OS Server (11443) serves no `/manage` page, so a
        // credentialed probe is the only way to recognise those — and `execute_integrations`
        // requires this service to be matched before the UniFi integration is allowed to run.
        Pattern::AnyOf(vec![
            Pattern::Endpoint(PortType::Https8443, "/manage", "UniFi", None),
            Pattern::ClientResponse(ClientProbe::UnifiController),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/unifi.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<UnifiController>
));
