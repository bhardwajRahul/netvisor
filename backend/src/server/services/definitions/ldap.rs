use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::ldap::LdapProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct LDAP;

impl ServiceDefinition for LDAP {
    fn name(&self) -> &'static str {
        "Open LDAP"
    }
    fn description(&self) -> &'static str {
        "Generic LDAP directory service"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::IdentityAndAccess
    }
    /// Derived from the probe, so a listener on this port that does not speak the protocol is
    /// not claimed as this service.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&LdapProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(LdapProbe)]
    }
    fn is_generic(&self) -> bool {
        true
    }
    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/openldap.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<LDAP>));
