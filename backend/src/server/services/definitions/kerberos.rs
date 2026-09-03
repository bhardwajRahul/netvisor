use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::kerberos::KerberosProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Kerberos;

impl ServiceDefinition for Kerberos {
    fn name(&self) -> &'static str {
        "Kerberos"
    }
    fn description(&self) -> &'static str {
        "Kerberos authentication service"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::IdentityAndAccess
    }
    /// Derived from the probe. A KDC answers an `AS-REQ` from anyone, because that is the message
    /// a client sends before it has any ticket; a listener on 88 that does not is not claimed as
    /// Kerberos.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&KerberosProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(KerberosProbe)]
    }
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Kerberos>));
