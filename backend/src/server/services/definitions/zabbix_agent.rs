use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::zabbix::ZabbixAgentProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

/// Zabbix Agent - Monitoring agent for Zabbix server
///
/// Port 10050 (TCP) is the IANA-assigned port for Zabbix agent.
/// The agent uses a binary protocol with "ZBXD\x01" header for communication.
/// Supports passive checks where the Zabbix server/proxy sends commands like
/// "agent.ping" and receives responses wrapped in the ZBXD protocol header.
///
/// Detection: Port-only (Medium confidence for IANA-assigned port).
/// Protocol-level detection could send "agent.ping" and check for ZBXD
/// response header, but this requires custom scanner support.
///
/// Note: This is for the Zabbix Agent (port 10050), not the Zabbix Server
/// which has an HTTP interface (see zabbix.rs).
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct ZabbixAgent;

impl ServiceDefinition for ZabbixAgent {
    fn name(&self) -> &'static str {
        "Zabbix Agent"
    }
    fn description(&self) -> &'static str {
        "Monitoring agent for Zabbix server"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Monitoring
    }
    /// Derived from the probe, so a listener on this port that does not speak the protocol is
    /// not claimed as this service.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&ZabbixAgentProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(ZabbixAgentProbe)]
    }
    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/zabbix.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<ZabbixAgent>));
