use crate::server::ports::r#impl::base::PortType;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct FireflyIii;

impl ServiceDefinition for FireflyIii {
    fn name(&self) -> &'static str {
        "Firefly III"
    }
    fn description(&self) -> &'static str {
        "Open-source personal finance manager"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Office
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            // Laravel session cookie, set on the unauthenticated login response.
            // Port-agnostic so it still matches behind a reverse proxy.
            Pattern::Header(None, "set-cookie", "firefly_iii_session", None),
            Pattern::Endpoint(PortType::Http8080, "/", "Login to Firefly III", None),
            Pattern::Endpoint(PortType::Http, "/", "Login to Firefly III", None),
            Pattern::Endpoint(PortType::Https, "/", "Login to Firefly III", None),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/firefly-iii.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<FireflyIii>));
