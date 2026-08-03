use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::base::DiscoverySessionServiceMatchParams;
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::{MatchConfidence, Pattern};
use crate::server::services::r#impl::virtualization::{
    PodmanVirtualization, ServiceVirtualization,
};

#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct PodmanContainer;

impl ServiceDefinition for PodmanContainer {
    fn name(&self) -> &'static str {
        "Podman Container"
    }
    fn description(&self) -> &'static str {
        "A generic podman container"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Container
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::AllOf(vec![
            Pattern::ContainerVirtualization,
            Pattern::Custom(
                |p: &DiscoverySessionServiceMatchParams| {
                    // If there's a matched service with the id of the container, the container was already detected as a non-generic service
                    let c_id = match p.baseline_params.virtualization_metadata {
                        Some(ServiceVirtualization::Podman(PodmanVirtualization {
                            container_id: Some(id),
                            ..
                        })) => id,
                        _ => return false, // No podman container_id -> not a podman container
                    };

                    p.service_params.matched_services.iter().all(|s| {
                        match &s.base.virtualization_metadata {
                            Some(ServiceVirtualization::Podman(PodmanVirtualization {
                                container_id,
                                ..
                            })) if container_id.is_some() => *container_id != Some(c_id.clone()),
                            _ => true,
                        }
                    })
                },
                // The generic container owns all of its own ports, so bind them here. Otherwise
                // they stay in unbound_ports and "Unclaimed Open Ports" re-claims them.
                |p| p.baseline_params.all_ports.to_vec(),
                "No other services with this container's ID have been matched",
                "A service with this container's ID has already been matched",
                MatchConfidence::Low,
            ),
        ])
    }

    fn is_generic(&self) -> bool {
        true
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<PodmanContainer>
));
