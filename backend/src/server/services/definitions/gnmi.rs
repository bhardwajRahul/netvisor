use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{ClientProbe, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Gnmi;

impl ServiceDefinition for Gnmi {
    fn name(&self) -> &'static str {
        "gNMI"
    }
    fn description(&self) -> &'static str {
        "Network management interface over gRPC"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkCore
    }
    /// Credential-gated for the same reason as SNMP: a gRPC listener answers a TCP connect
    /// whatever it serves, so a port alone does not establish that this is gNMI. Only a
    /// completed `Get` against `/lldp` or `/interfaces` does, which needs the credential.
    ///
    /// The port stays on the credential rather than here. 9339 is the IANA registration, but
    /// Arista ships 6030 and Nokia and Juniper 57400, so a port fixed in the definition would be
    /// wrong on most fleets.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::ClientResponse(ClientProbe::Gnmi)
    }
    /// A management protocol many device types expose, not a product that identifies its host.
    /// Same standing as SNMP, and the specificity checks in `services/impl/tests.rs` exempt
    /// generic definitions from needing `Pattern::Port` coverage.
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<Gnmi>));
