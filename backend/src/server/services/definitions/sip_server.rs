use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::sip::SipProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct SIPServer;

impl ServiceDefinition for SIPServer {
    fn name(&self) -> &'static str {
        "SIP Server"
    }
    fn description(&self) -> &'static str {
        "Session initiation protocol"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Telephony
    }
    /// Derived from the probe, so a listener on 5060 that does not answer an `OPTIONS` is not
    /// claimed as SIP. This used to be `AnyOf([Port(Sip), Port(SipTls)])`, which a FortiGate
    /// session helper satisfied on behalf of every empty address on every routed VLAN.
    ///
    /// 5061 is gone with it rather than being carried over: SIP-TLS is a TLS handshake before any
    /// SIP is spoken, so an `OPTIONS` cannot reach it and the port alone would be the same bare
    /// connect this change exists to remove.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&SipProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(SipProbe)]
    }
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<SIPServer>));
