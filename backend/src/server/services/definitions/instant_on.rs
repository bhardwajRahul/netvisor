use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{ClientProbe, InstantOnDeviceType, Pattern};

/// The management relationship, not a listening service: this host is administered through the
/// Instant On cloud portal. It is the credential's `associated_service()`, so matching it is what
/// allows the integration's `execute` to run.
///
/// Two ways to match, and both are true statements about the host. The anchor — the switch the
/// credential is assigned to — matches on the probe, which is the only evidence available before
/// the inventory is fetched. Every other device the site reports matches on its own device class.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct InstantOn;

impl ServiceDefinition for InstantOn {
    fn name(&self) -> &'static str {
        "Instant On"
    }
    fn description(&self) -> &'static str {
        "Managed through the HPE Networking Instant On cloud portal"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::NetworkAccess
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        // No endpoint arm: there is nothing on the LAN to fingerprint, because the portal is not
        // served by any of these devices. And no `MacVendor` guard, unlike the UniFi definitions —
        // they need one because their endpoint arm could false-positive, whereas
        // `ManagedDeviceType` evidence can only come from our own authenticated fetch. ANDing a
        // vendor string here would buy nothing and would silently stop matching if HPE's OUI
        // registration ever renders differently from the constant.
        Pattern::AnyOf(vec![
            Pattern::ClientResponse(ClientProbe::InstantOn),
            Pattern::ManagedDeviceType(InstantOnDeviceType::SWITCH),
            Pattern::ManagedDeviceType(InstantOnDeviceType::STACK),
            Pattern::ManagedDeviceType(InstantOnDeviceType::ACCESS_POINT),
            Pattern::ManagedDeviceType(InstantOnDeviceType::GATEWAY),
        ])
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/hpe.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<InstantOn>));
