use crate::{
    daemon::discovery::{
        service::base::{
            CreatesDiscoveredEntities, DiscoveryRunner, DiscoverySession, RunsDiscovery,
        },
        types::base::{DiscoveryPhase, DiscoverySessionInfo, DiscoverySessionUpdate},
    },
    server::{
        bindings::r#impl::base::Binding,
        daemons::r#impl::api::{DaemonCapabilities, DaemonDiscoveryRequest},
        discovery::r#impl::types::DiscoveryType,
        interfaces::r#impl::base::{ALL_INTERFACES_IP, Interface},
        ports::r#impl::base::{Port, PortType},
        services::{
            definitions::scanopy_daemon::ScanopyDaemon,
            r#impl::{base::ServiceBase, definitions::ServiceDefinition, patterns::MatchDetails},
        },
        shared::{
            storage::traits::Storable,
            types::entities::{DiscoveryMetadata, EntitySource},
        },
        subnets::r#impl::{base::Subnet, types::SubnetTypeDiscriminants},
    },
};
use crate::{
    daemon::utils::base::DaemonUtils,
    server::{
        hosts::r#impl::base::{Host, HostBase},
        services::r#impl::base::Service,
    },
};
use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::Utc;
use cidr::IpCidr;
use futures::future::join_all;
use std::net::{IpAddr, Ipv4Addr};
use strum::IntoDiscriminant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Default)]
pub struct SelfReportDiscovery {
    host_id: Uuid,
}

impl SelfReportDiscovery {
    pub fn new(host_id: Uuid) -> Self {
        Self { host_id }
    }
}

impl CreatesDiscoveredEntities for DiscoveryRunner<SelfReportDiscovery> {}

#[async_trait]
impl RunsDiscovery for DiscoveryRunner<SelfReportDiscovery> {
    fn discovery_type(&self) -> DiscoveryType {
        DiscoveryType::SelfReport {
            host_id: self.domain.host_id,
        }
    }

    async fn discover(
        &self,
        request: DaemonDiscoveryRequest,
        _cancel: CancellationToken,
    ) -> Result<(), Error> {
        let daemon_id = self.as_ref().config_store.get_id().await?;
        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set, aborting discovery session"))?;

        let session_info = DiscoverySessionInfo {
            session_id: request.session_id,
            network_id,
            daemon_id,
            started_at: Some(Utc::now()),
        };

        let session = DiscoverySession::new(session_info, Vec::new());
        let mut current_session = self.as_ref().current_session.write().await;
        *current_session = Some(session);
        drop(current_session);

        let utils = &self.as_ref().utils;

        let host_id = self.domain.host_id;

        let network_id = self
            .as_ref()
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Network ID not set"))?;

        let binding_address = self.as_ref().config_store.get_bind_address().await?;
        let binding_ip = IpAddr::V4(binding_address.parse::<Ipv4Addr>()?);

        let interface_start = std::time::Instant::now();
        let (interfaces, subnets, _) = utils
            .get_own_interfaces(self.discovery_type(), daemon_id, network_id)
            .await?;
        tracing::debug!(
            elapsed_ms = interface_start.elapsed().as_millis(),
            interface_count = interfaces.len(),
            subnet_count = subnets.len(),
            "Network interfaces gathered"
        );

        // Get docker subnets to double verify that subnet interface string matching filtered them correctly
        let docker_proxy = self.as_ref().config_store.get_docker_proxy().await;
        let docker_proxy_ssl_info = self.as_ref().config_store.get_docker_proxy_ssl_info().await;

        let docker_client = self
            .as_ref()
            .utils
            .new_local_docker_client(docker_proxy, docker_proxy_ssl_info)
            .await;

        let (docker_cidrs, has_docker_socket) = if let Ok(docker_client) = docker_client {
            tracing::debug!("Docker client available, fetching Docker networks");
            match self
                .as_ref()
                .utils
                .get_subnets_from_docker_networks(
                    daemon_id,
                    network_id,
                    &docker_client,
                    self.discovery_type(),
                )
                .await
            {
                Ok(docker_subnets) => {
                    let docker_cidrs: Vec<IpCidr> =
                        docker_subnets.iter().map(|s| s.base.cidr).collect();
                    tracing::debug!(
                        docker_subnet_count = docker_cidrs.len(),
                        cidrs = ?docker_cidrs.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
                        "Docker subnets detected (will be excluded from self-report)"
                    );
                    (docker_cidrs, true)
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "Failed to get Docker networks - proceeding without Docker subnet filtering"
                    );
                    (Vec::new(), true) // Still has Docker socket, just couldn't list networks
                }
            }
        } else {
            tracing::debug!("Docker socket not available - skipping Docker subnet detection");
            (Vec::new(), false)
        };

        // Filter out docker bridge subnets, those are handled in docker discovery
        let subnets_to_create: Vec<Subnet> = subnets
            .into_iter()
            .filter(|s| {
                let is_docker_bridge =
                    s.base.subnet_type.discriminant() == SubnetTypeDiscriminants::DockerBridge;
                let is_in_docker_cidrs = docker_cidrs.contains(&s.base.cidr);
                let keep = !is_docker_bridge && !is_in_docker_cidrs;
                if !keep {
                    tracing::debug!(
                        cidr = %s.base.cidr,
                        is_docker_bridge,
                        is_in_docker_cidrs,
                        "Filtering out subnet (Docker-related)"
                    );
                }
                keep
            })
            .collect();

        tracing::info!(
            subnet_count = subnets_to_create.len(),
            cidrs = ?subnets_to_create.iter().map(|s| s.base.cidr.to_string()).collect::<Vec<_>>(),
            "Creating subnets from discovered interfaces"
        );

        // Create subnets individually, collecting successes and logging failures
        // This prevents one subnet failure from blocking all interfaces
        let subnet_futures = subnets_to_create.iter().map(|subnet| async move {
            let cidr = subnet.base.cidr;
            match self.create_subnet(subnet).await {
                Ok(created) => {
                    tracing::debug!(
                        cidr = %cidr,
                        subnet_id = %created.id,
                        "Subnet created successfully"
                    );
                    Some(created)
                }
                Err(e) => {
                    tracing::warn!(
                        cidr = %cidr,
                        error = %e,
                        "Failed to create subnet - interfaces in this CIDR will be skipped"
                    );
                    None
                }
            }
        });
        let created_subnets: Vec<Subnet> = join_all(subnet_futures)
            .await
            .into_iter()
            .flatten()
            .collect();

        tracing::debug!(
            created_count = created_subnets.len(),
            requested_count = subnets_to_create.len(),
            "Subnet creation complete"
        );

        // Update capabilities
        let interfaced_subnet_ids: Vec<Uuid> = created_subnets.iter().map(|s| s.id).collect();

        tracing::debug!(
            "Updating capabilities with {} interfaced subnets: {:?}",
            interfaced_subnet_ids.len(),
            interfaced_subnet_ids
        );

        self.update_capabilities(has_docker_socket, interfaced_subnet_ids)
            .await?;

        // Created subnets may differ from discovered if there are existing subnets with the same CIDR, so we need to update interface subnet_id references
        // Also filter out interfaces where subnet creation didn't happen for any reason
        let original_interface_count = interfaces.len();
        let interfaces: Vec<Interface> = interfaces
            .into_iter()
            .filter_map(|mut i| {
                if let Some(subnet) = created_subnets
                    .iter()
                    .find(|s| s.base.cidr.contains(&i.base.ip_address))
                {
                    i.base.subnet_id = subnet.id;
                    return Some(i);
                }
                tracing::warn!(
                    interface_name = ?i.base.name,
                    ip_address = %i.base.ip_address,
                    "Dropping interface - no matching subnet was created (subnet creation may have failed)"
                );
                None
            })
            .collect();

        if interfaces.len() < original_interface_count {
            tracing::warn!(
                original_count = original_interface_count,
                kept_count = interfaces.len(),
                dropped_count = original_interface_count - interfaces.len(),
                "Some interfaces were dropped due to missing subnets"
            );
        } else {
            tracing::debug!(
                interface_count = interfaces.len(),
                "All interfaces have matching subnets"
            );
        }

        let daemon_bound_subnet_ids: Vec<Uuid> = if binding_address == ALL_INTERFACES_IP.to_string()
        {
            created_subnets.iter().map(|s| s.id).collect()
        } else {
            created_subnets
                .iter()
                .filter(|s| s.base.cidr.contains(&binding_ip))
                .map(|s| s.id)
                .collect()
        };

        let own_port = Port::new_hostless(PortType::new_tcp(
            self.as_ref().config_store.get_port().await?,
        ));
        let own_port_id = own_port.id;
        let local_ip = utils.get_own_ip_address()?;
        let hostname = utils.get_own_hostname();

        // Create host base - children (interfaces, ports, services) are passed separately
        let host_base = HostBase {
            name: hostname.clone().unwrap_or(format!("{}", local_ip)),
            hostname,
            network_id,
            description: Some("Scanopy daemon".to_string()),
            tags: Vec::new(),
            source: EntitySource::Discovery {
                metadata: vec![DiscoveryMetadata::new(self.discovery_type(), daemon_id)],
            },
            hidden: false,
            virtualization: None,
        };

        // Ports to create with the host
        let ports = vec![own_port];

        let mut host = Host::new(host_base);

        host.id = host_id;

        let mut services = Vec::new();
        let daemon_service_definition = ScanopyDaemon;

        let daemon_service_bound_interfaces: Vec<&Interface> = interfaces
            .iter()
            .filter(|i| daemon_bound_subnet_ids.contains(&i.base.subnet_id))
            .collect();

        let daemon_service = Service::new(ServiceBase {
            name: ServiceDefinition::name(&daemon_service_definition).to_string(),
            service_definition: Box::new(daemon_service_definition),
            tags: Vec::new(),
            network_id,
            bindings: daemon_service_bound_interfaces
                .iter()
                .map(|i| Binding::new_port_serviceless(own_port_id, Some(i.id)))
                .collect(),
            host_id: host.id,
            virtualization: None,
            source: EntitySource::DiscoveryWithMatch {
                metadata: vec![DiscoveryMetadata::new(self.discovery_type(), daemon_id)],
                details: MatchDetails::new_certain("Scanopy Daemon self-report"),
            },
            position: 0,
        });

        services.push(daemon_service);

        tracing::debug!(
            "Collected information about own host with local IP: {}, Hostname: {:?}",
            local_ip,
            host.base.hostname
        );

        // Pass interfaces and ports separately - server will create them with the correct host_id
        tracing::debug!("Creating host with interfaces, ports, and services");
        self.create_host(host, interfaces.clone(), ports, services)
            .await?;

        self.report_discovery_update(DiscoverySessionUpdate {
            phase: DiscoveryPhase::Complete,
            progress: 100,
            error: None,
            finished_at: Some(Utc::now()),
        })
        .await?;

        Ok(())
    }
}

impl DiscoveryRunner<SelfReportDiscovery> {
    async fn update_capabilities(
        &self,
        has_docker_socket: bool,
        interfaced_subnet_ids: Vec<Uuid>,
    ) -> Result<(), Error> {
        tracing::debug!(
            has_docker_socket,
            subnet_count = interfaced_subnet_ids.len(),
            subnet_ids = ?interfaced_subnet_ids,
            "Updating daemon capabilities"
        );

        let capabilities = DaemonCapabilities {
            has_docker_socket,
            interfaced_subnet_ids: interfaced_subnet_ids.clone(),
        };

        let daemon_id = self.as_ref().api_client.config().get_id().await?;
        let path = format!("/api/daemons/{}/update-capabilities", daemon_id);

        match self
            .as_ref()
            .api_client
            .post_no_data(&path, &capabilities, "Failed to update capabilities")
            .await
        {
            Ok(()) => {
                tracing::info!(
                    has_docker_socket,
                    subnet_count = interfaced_subnet_ids.len(),
                    "Daemon capabilities updated successfully"
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    has_docker_socket,
                    subnet_count = interfaced_subnet_ids.len(),
                    error = %e,
                    "Failed to update daemon capabilities"
                );
                Err(e)
            }
        }
    }
}
