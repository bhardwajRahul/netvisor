use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::modbus::ModbusProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct ModbusTcp;

impl ServiceDefinition for ModbusTcp {
    fn name(&self) -> &'static str {
        "Modbus TCP"
    }
    fn description(&self) -> &'static str {
        "Industrial fieldbus protocol used by PLCs, drives, and serial gateways"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Industrial
    }
    /// Derived from the probe, so the port that gets scanned and the port that gets probed cannot
    /// disagree. A listener on 502 that does not answer an MBAP frame is not claimed as Modbus.
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&ModbusProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(ModbusProbe)]
    }
    /// A protocol many vendors implement, not a product. A device answering Modbus is identified
    /// by what `0x2B` returns, not by the service definition.
    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<ModbusTcp>));
