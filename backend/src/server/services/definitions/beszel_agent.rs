use crate::daemon::utils::app_probe::AppProbe;
use crate::daemon::utils::app_probe::beszel_agent::BeszelAgentProbe;
use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{Pattern, probe_pattern};

/// Beszel Agent — lightweight server monitoring agent.
///
/// The agent runs an SSH server on 45876 (via `gliderlabs/ssh`) that the Beszel hub connects to and
/// pulls metrics from.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct BeszelAgent;

impl ServiceDefinition for BeszelAgent {
    fn name(&self) -> &'static str {
        "Beszel Agent"
    }
    fn description(&self) -> &'static str {
        "Lightweight server monitoring agent"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Monitoring
    }

    /// The agent's port, qualified by the software name in its SSH identification string.
    ///
    /// This was declared `NoVerifiableImplementation`, which was wrong on both counts: the agent is
    /// published as `henrygd/beszel-agent` with an arm64 image, and no exchange has to be completed
    /// because an SSH server speaks first. Its banner reads `SSH-2.0-beszel_<version>`; see
    /// [`crate::daemon::utils::app_probe::beszel_agent`].
    fn discovery_pattern(&self) -> Pattern<'_> {
        probe_pattern(&BeszelAgentProbe)
    }
    fn app_probes(&self) -> Vec<Box<dyn AppProbe>> {
        vec![Box::new(BeszelAgentProbe)]
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/beszel.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(create_service::<BeszelAgent>));
