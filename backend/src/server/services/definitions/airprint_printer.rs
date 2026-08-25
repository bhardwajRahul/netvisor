use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{DnsSdServiceType, Pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct AirPrintPrinter;

impl ServiceDefinition for AirPrintPrinter {
    fn name(&self) -> &'static str {
        "AirPrint Printer"
    }
    fn description(&self) -> &'static str {
        "A network printer advertising AirPrint"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Printer
    }

    /// Identifies a printer without touching it. That matters more here than elsewhere: the
    /// raw-socket ports a printer answers on are exactly the ones `probe_raw_socket_ports`
    /// defaults to *off* for, because probing them makes some JetDirect models emit a page. An
    /// mDNS announcement is read-only and costs the device nothing.
    ///
    /// Vendor-agnostic on purpose — the vendor-specific definitions match on their own web UIs and
    /// keep their higher-confidence claim; this covers everything else that advertises IPP.
    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AnyOf(vec![
            Pattern::DnsSd(DnsSdServiceType::IPP, None),
            Pattern::DnsSd(DnsSdServiceType::PDL_DATASTREAM, None),
        ])
    }

    fn is_generic(&self) -> bool {
        true
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/printer.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<AirPrintPrinter>
));
