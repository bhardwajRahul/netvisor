//! Shared discovery operations used by both the pipeline and integrations.
//!
//! `DiscoveryOps` provides entity creation, service matching, and progress reporting
//! without requiring `DiscoveryRunner` or its associated traits.

use std::{net::IpAddr, sync::Arc, time::Duration};

use anyhow::{Error, anyhow};
use backon::{ExponentialBuilder, Retryable};
use chrono::Utc;
use mac_address::MacAddress;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    daemon::{
        discovery::{
            buffer::EntityBuffer,
            types::base::{
                DiscoveryCriticalError, DiscoveryPhase, DiscoverySessionInfo,
                DiscoverySessionUpdate,
            },
        },
        shared::{api_client::DaemonApiClient, config::ConfigStore},
    },
    server::{
        credentials::r#impl::types::CredentialAssignment,
        daemons::r#impl::{
            api::{DaemonDiscoveryRequest, DiscoveryUpdatePayload},
            base::DaemonMode,
        },
        discovery::r#impl::types::{DiscoveryType, HostNamingFallback},
        hosts::r#impl::{
            api::{DiscoveryHostRequest, HostResponse},
            base::{Host, HostBase},
            virtualization::HostVirtualization,
        },
        interfaces::r#impl::base::Interface,
        ip_addresses::r#impl::base::IPAddress,
        ports::r#impl::base::{Port, PortType},
        services::{
            definitions::{ServiceDefinitionRegistry, gateway::Gateway},
            r#impl::{
                base::{
                    DiscoverySessionServiceMatchParams, Service, ServiceMatchBaselineParams,
                    ServiceMatchServiceParams,
                },
                definitions::{ServiceDefinition, ServiceDefinitionExt},
                patterns::MatchConfidence,
                virtualization::{DockerVirtualization, ServiceVirtualization},
            },
        },
        shared::{
            types::api::ApiErrorResponse,
            types::entities::EntitySource,
            types::metadata::HasId,
        },
        subnets::r#impl::base::Subnet,
    },
};

use super::base::DaemonDiscoveryService;

/// Default number of retries for entity creation during discovery.
const ENTITY_CREATION_MAX_RETRIES: usize = 5;

/// Timeout for waiting for server confirmation in ServerPoll mode.
const SERVER_POLL_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(120);

/// Mutable host state passed to integration execute() methods.
/// Integrations enrich the host via builder methods.
pub struct HostData {
    pub host: Host,
    pub services: Vec<Service>,
    pub ports: Vec<Port>,
    pub ip_addresses: Vec<IPAddress>,
    pub interfaces: Vec<Interface>,
    pub subnets: Vec<Subnet>,
}

impl HostData {
    pub fn new(
        host: Host,
        services: Vec<Service>,
        ports: Vec<Port>,
        ip_addresses: Vec<IPAddress>,
        interfaces: Vec<Interface>,
        subnets: Vec<Subnet>,
    ) -> Self {
        Self {
            host,
            services,
            ports,
            ip_addresses,
            interfaces,
            subnets,
        }
    }

    // --- Field builders: first-write-wins (only set if currently None) ---

    pub fn with_sys_descr(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_descr.is_none() {
            self.host.base.sys_descr = Some(v);
        }
        self
    }

    pub fn with_sys_name(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_name.is_none() {
            self.host.base.sys_name = Some(v);
        }
        self
    }

    pub fn with_sys_object_id(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_object_id.is_none() {
            self.host.base.sys_object_id = Some(v);
        }
        self
    }

    pub fn with_sys_location(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_location.is_none() {
            self.host.base.sys_location = Some(v);
        }
        self
    }

    pub fn with_sys_contact(&mut self, v: String) -> &mut Self {
        if self.host.base.sys_contact.is_none() {
            self.host.base.sys_contact = Some(v);
        }
        self
    }

    pub fn with_chassis_id(&mut self, v: String) -> &mut Self {
        if self.host.base.chassis_id.is_none() {
            self.host.base.chassis_id = Some(v);
        }
        self
    }

    pub fn with_manufacturer(&mut self, v: String) -> &mut Self {
        if self.host.base.manufacturer.is_none() {
            self.host.base.manufacturer = Some(v);
        }
        self
    }

    pub fn with_model(&mut self, v: String) -> &mut Self {
        if self.host.base.model.is_none() {
            self.host.base.model = Some(v);
        }
        self
    }

    pub fn with_serial_number(&mut self, v: String) -> &mut Self {
        if self.host.base.serial_number.is_none() {
            self.host.base.serial_number = Some(v);
        }
        self
    }

    pub fn with_management_url(&mut self, v: String) -> &mut Self {
        if self.host.base.management_url.is_none() {
            self.host.base.management_url = Some(v);
        }
        self
    }

    pub fn with_virtualization(&mut self, v: HostVirtualization) -> &mut Self {
        if self.host.base.virtualization.is_none() {
            self.host.base.virtualization = Some(v);
        }
        self
    }

    /// Set MAC on the interface matching the given IP address.
    /// Used by SNMP to enrich MAC from ipAddrTable when ARP didn't provide one.
    pub fn with_mac_for_ip(&mut self, ip: IpAddr, mac: MacAddress) -> &mut Self {
        if let Some(ip_address) = self
            .ip_addresses
            .iter_mut()
            .find(|i| i.base.ip_address == ip)
            && ip_address.base.mac_address.is_none()
        {
            ip_address.base.mac_address = Some(mac);
        }
        self
    }

    // --- Append methods: multiple integrations can contribute ---

    pub fn add_service(&mut self, s: Service) -> &mut Self {
        self.services.push(s);
        self
    }

    pub fn add_port(&mut self, p: Port) -> &mut Self {
        self.ports.push(p);
        self
    }

    pub fn add_ip_address(&mut self, i: IPAddress) -> &mut Self {
        self.ip_addresses.push(i);
        self
    }

    pub fn add_if_entry(&mut self, ie: Interface) -> &mut Self {
        self.interfaces.push(ie);
        self
    }

    pub fn add_subnet(&mut self, s: Subnet) -> &mut Self {
        self.subnets.push(s);
        self
    }

    pub fn add_credential_assignment(&mut self, ca: CredentialAssignment) -> &mut Self {
        self.host.base.credential_assignments.push(ca);
        self
    }

    /// Set hostname from SNMP sysName as a fallback if DNS didn't provide one.
    /// Also updates `host.base.name` when it was set to the IP address as a fallback.
    pub fn with_hostname_fallback(&mut self, hostname: String) -> &mut Self {
        if self.host.base.hostname.is_none() {
            // Check if host.name was set to IP as fallback — if so, override with hostname
            let ip_str = self
                .ip_addresses
                .first()
                .map(|i| i.base.ip_address.to_string());
            if ip_str.as_deref() == Some(&self.host.base.name) {
                self.host.base.name = hostname.clone();
            }
            self.host.base.hostname = Some(hostname);
        }
        self
    }
}

/// Shared discovery operations for both the pipeline and integrations.
///
/// Provides entity creation, service matching, and progress reporting
/// without requiring `DiscoveryRunner`.
pub struct DiscoveryOps {
    pub config_store: Arc<ConfigStore>,
    pub api_client: Arc<DaemonApiClient>,
    pub entity_buffer: Arc<EntityBuffer>,
    pub discovery_type: DiscoveryType,
    session: Arc<tokio::sync::RwLock<Option<super::base::DiscoverySession>>>,
    /// Stores the terminal state (Complete/Failed/Cancelled) for ServerPoll mode.
    /// In ServerPoll mode, the server polls for progress updates. If the session ends
    /// between polls, we need to retain the terminal state so the server can receive it.
    /// This is cleared when a new session starts.
    pub terminal_payload: Arc<RwLock<Option<DiscoveryUpdatePayload>>>,
}

impl DiscoveryOps {
    pub fn new(service: &DaemonDiscoveryService, discovery_type: DiscoveryType) -> Self {
        Self {
            config_store: service.config_store.clone(),
            api_client: service.api_client.clone(),
            entity_buffer: service.entity_buffer.clone(),
            discovery_type,
            session: service.current_session.clone(),
            terminal_payload: service.terminal_payload.clone(),
        }
    }

    pub async fn daemon_id(&self) -> Result<Uuid, Error> {
        self.config_store.get_id().await
    }

    pub async fn network_id(&self) -> Result<Uuid, Error> {
        self.config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow!("Network ID not set"))
    }

    pub async fn get_session(&self) -> Result<super::base::DiscoverySession, Error> {
        self.session
            .read()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow!("No active discovery session"))
    }

    // --- Session lifecycle methods ---

    /// Initialize a discovery session: set up session info, gateway IPs, clear terminal payload,
    /// create DiscoverySession.
    pub async fn initialize_session(
        &self,
        request: &DaemonDiscoveryRequest,
        daemon_id: Uuid,
        gateway_ips: Vec<IpAddr>,
    ) -> Result<(), Error> {
        tracing::debug!(
            "Setting session info for {} discovery session {}",
            request.discovery_type,
            request.session_id
        );
        let network_id = self
            .config_store
            .get_network_id()
            .await?
            .ok_or_else(|| anyhow!("Network ID not set, aborting discovery session"))?;

        let session_info = DiscoverySessionInfo {
            session_id: request.session_id,
            network_id,
            daemon_id,
            started_at: Some(Utc::now()),
            discovery_type: request.discovery_type.clone(),
            discovery_id: request.discovery_id,
        };

        let session = super::base::DiscoverySession::new(session_info, gateway_ips);

        // Clear terminal payload from previous session (if any) when starting new session
        let mut terminal_payload = self.terminal_payload.write().await;
        *terminal_payload = None;
        drop(terminal_payload);

        let mut current_session = self.session.write().await;
        *current_session = Some(session);

        Ok(())
    }

    /// Start a discovery session: initialize session, report "Started" update.
    pub async fn start_session(
        &self,
        request: &DaemonDiscoveryRequest,
        gateway_ips: Vec<IpAddr>,
    ) -> Result<(), Error> {
        let daemon_id = self.config_store.get_id().await?;

        tracing::info!(
            "Starting {} discovery session {}",
            request.discovery_type,
            request.session_id
        );

        self.initialize_session(request, daemon_id, gateway_ips)
            .await?;

        self.report_discovery_update(DiscoverySessionUpdate {
            phase: DiscoveryPhase::Started,
            progress: 0,
            error: None,
            finished_at: None,
        })
        .await?;

        let session = self.get_session().await?;

        tracing::info!(
            session_id = %session.info.session_id,
            discovery_type = ?self.discovery_type,
            "Discovery session started"
        );

        Ok(())
    }

    /// Finish a discovery session: report terminal state, store terminal payload for ServerPoll,
    /// clear entity buffer and session.
    pub async fn finish_session(
        &self,
        discovery_result: Result<(), Error>,
        cancel: CancellationToken,
    ) -> Result<(), Error> {
        let session = self.get_session().await?;
        let session_id = session.info.session_id;

        let final_progress = session
            .last_progress
            .load(std::sync::atomic::Ordering::Relaxed);

        // Build the terminal update based on result
        let terminal_update = match &discovery_result {
            Ok(_) => {
                tracing::info!(
                    session_id = %session_id,
                    progress = 100,
                    "Discovery session completed successfully"
                );
                DiscoverySessionUpdate {
                    phase: DiscoveryPhase::Complete,
                    progress: 100,
                    error: None,
                    finished_at: Some(Utc::now()),
                }
            }
            Err(_) if cancel.is_cancelled() => {
                tracing::warn!(
                    session_id = %session_id,
                    progress = %final_progress,
                    "Discovery session cancelled"
                );
                DiscoverySessionUpdate {
                    phase: DiscoveryPhase::Cancelled,
                    progress: final_progress,
                    error: None,
                    finished_at: Some(Utc::now()),
                }
            }
            Err(e) => {
                tracing::error!(
                    session_id = %session_id,
                    progress = %final_progress,
                    error = %e,
                    "Discovery session failed"
                );

                let error = DiscoveryCriticalError::from_error_string(e.to_string())
                    .map(|e| e.to_string())
                    .unwrap_or(format!("Critical error: {}", e));

                cancel.cancel();
                DiscoverySessionUpdate {
                    phase: DiscoveryPhase::Failed,
                    progress: final_progress,
                    error: Some(error),
                    finished_at: Some(Utc::now()),
                }
            }
        };

        // Report the terminal update (in DaemonPoll mode, this POSTs to server)
        self.report_discovery_update(terminal_update.clone())
            .await?;

        // Store terminal payload for ServerPoll mode - the server polls for progress
        // and needs to receive the terminal state even after current_session is cleared.
        // This payload persists until a new session starts.
        let terminal_payload = DiscoveryUpdatePayload::from_state_and_update(
            self.discovery_type.clone(),
            session.info.clone(),
            terminal_update,
        );
        let mut stored_terminal = self.terminal_payload.write().await;
        *stored_terminal = Some(terminal_payload);
        drop(stored_terminal);

        // Clear entity buffer - all await_*() calls have completed by now
        // (either successfully found Created entries or timed out)
        self.entity_buffer.clear_all().await;

        let mut current_session = self.session.write().await;
        if let Some(session) = current_session.as_ref()
            && session.info.session_id == session_id
        {
            *current_session = None;
        }

        if cancel.is_cancelled() {
            return Ok(());
        }

        Ok(())
    }

    /// Report a discovery update to the server. Non-critical: logs errors but doesn't fail.
    async fn report_discovery_update(&self, update: DiscoverySessionUpdate) -> Result<(), Error> {
        use std::sync::atomic::Ordering;

        let session = self.get_session().await?;

        // Map progress for scanning updates through the session's progress range
        let update = if update.phase == DiscoveryPhase::Scanning {
            let start = session.progress_range_start.load(Ordering::Relaxed);
            let end = session.progress_range_end.load(Ordering::Relaxed);
            DiscoverySessionUpdate {
                progress: map_progress(update.progress, start, end),
                ..update
            }
        } else {
            update
        };

        let mut payload = DiscoveryUpdatePayload::from_state_and_update(
            self.discovery_type.clone(),
            session.info.clone(),
            update,
        );

        // Populate estimation fields from session atomics
        let hosts = session.hosts_discovered.load(Ordering::Relaxed);
        if hosts > 0 {
            payload.hosts_discovered = Some(hosts);
        }
        let estimate = session.estimated_remaining_secs.load(Ordering::Relaxed);
        if estimate != u32::MAX {
            payload.estimated_remaining_secs = Some(estimate);
        }

        let path = format!("/api/v1/discovery/{}/update", session.info.session_id);

        // Progress updates are non-critical - log errors but don't fail discovery
        if let Err(e) = self
            .api_client
            .post_no_data(&path, &payload, "Failed to report discovery update")
            .await
        {
            tracing::warn!(
                session_id = %session.info.session_id,
                error = %e,
                "Failed to report discovery update"
            );
        } else {
            tracing::trace!(
                "Discovery update reported for session {}",
                session.info.session_id
            );
        }

        Ok(())
    }

    /// Report scanning progress. Mode-aware: DaemonPoll POSTs to server,
    /// ServerPoll updates session atomics only.
    pub async fn report_progress(&self, percent: u8) -> Result<(), Error> {
        use crate::daemon::discovery::types::base::DiscoverySessionUpdate;
        use std::sync::atomic::Ordering;

        let session = self.get_session().await?;
        let start = session.progress_range_start.load(Ordering::Relaxed);
        let end = session.progress_range_end.load(Ordering::Relaxed);
        let percent = map_progress(percent, start, end);

        let last_report_time = &session.last_progress_report_time;
        let last_progress = &session.last_progress;

        let prev_percent = last_progress.load(Ordering::Relaxed);
        let progress_changed = percent > prev_percent || percent == 100;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last_time = last_report_time.load(Ordering::Relaxed);

        let heartbeat_interval_secs = 30;
        let heartbeat_due = now >= last_time + heartbeat_interval_secs;

        if !progress_changed && !heartbeat_due && percent < 100 {
            return Ok(());
        }

        if percent < 100 && !heartbeat_due && now < last_time + 10 {
            return Ok(());
        }

        if last_report_time
            .compare_exchange(last_time, now, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            return Ok(());
        }

        last_progress.store(percent, Ordering::Relaxed);

        // Mode-aware: only POST in DaemonPoll mode
        let mode = self.config_store.get_mode().await?;
        if mode == DaemonMode::DaemonPoll {
            let update = DiscoverySessionUpdate::scanning(percent);
            let mut payload = DiscoveryUpdatePayload::from_state_and_update(
                self.discovery_type.clone(),
                session.info.clone(),
                update,
            );

            let hosts = session.hosts_discovered.load(Ordering::Relaxed);
            if hosts > 0 {
                payload.hosts_discovered = Some(hosts);
            }
            let estimate = session.estimated_remaining_secs.load(Ordering::Relaxed);
            if estimate != u32::MAX {
                payload.estimated_remaining_secs = Some(estimate);
            }

            let path = format!("/api/v1/discovery/{}/update", session.info.session_id);
            if let Err(e) = self
                .api_client
                .post_no_data(&path, &payload, "Failed to report discovery update")
                .await
            {
                tracing::warn!(
                    session_id = %session.info.session_id,
                    error = %e,
                    "Failed to report discovery update"
                );
            }
        }

        Ok(())
    }

    /// Create a host with its children.
    /// DaemonPoll: POSTs to server. ServerPoll: buffers for server to poll.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_host(
        &self,
        host: Host,
        ip_addresses: Vec<IPAddress>,
        ports: Vec<Port>,
        services: Vec<Service>,
        interfaces: Vec<Interface>,
        subnets: Vec<Subnet>,
        cancel: &CancellationToken,
    ) -> Result<HostResponse, Error> {
        let mode = self.config_store.get_mode().await?;
        let pending_id = host.id;

        let request = DiscoveryHostRequest {
            host,
            ip_addresses,
            ports,
            services,
            interfaces,
            subnets,
        };

        self.entity_buffer.push_host(request.clone()).await;

        match mode {
            DaemonMode::DaemonPoll => {
                let api_client = &self.api_client;
                let response: HostResponse = (|| async {
                    api_client
                        .post("/api/v1/hosts/discovery", &request, "Failed to create host")
                        .await
                })
                .retry(
                    ExponentialBuilder::default()
                        .with_min_delay(Duration::from_millis(500))
                        .with_max_delay(Duration::from_secs(30))
                        .with_max_times(ENTITY_CREATION_MAX_RETRIES),
                )
                .when(|e| e.downcast_ref::<ApiErrorResponse>().is_none())
                .notify(|e, dur| tracing::warn!("Retrying host creation after {:?}: {}", dur, e))
                .await?;

                self.entity_buffer
                    .mark_host_created(pending_id, response.clone())
                    .await;

                Ok(response)
            }
            DaemonMode::ServerPoll => {
                let actual_host = self
                    .entity_buffer
                    .await_host(&pending_id, SERVER_POLL_CONFIRMATION_TIMEOUT, cancel)
                    .await
                    .ok_or_else(|| {
                        if cancel.is_cancelled() {
                            anyhow!("Discovery cancelled while waiting for host creation")
                        } else {
                            anyhow!("Timeout waiting for host creation confirmation from server")
                        }
                    })?;

                Ok(HostResponse::from_host_with_children(
                    actual_host,
                    request.ip_addresses,
                    request.ports,
                    request.services,
                    request.interfaces,
                ))
            }
        }
    }

    /// Create a subnet.
    /// DaemonPoll: POSTs to server. ServerPoll: buffers for server to poll.
    pub async fn create_subnet(
        &self,
        subnet: &Subnet,
        cancel: &CancellationToken,
    ) -> Result<Subnet, Error> {
        let mode = self.config_store.get_mode().await?;
        let pending_id = subnet.id;

        self.entity_buffer.push_subnet(subnet.clone()).await;

        match mode {
            DaemonMode::DaemonPoll => {
                let api_client = &self.api_client;
                let actual: Subnet = (|| async {
                    api_client
                        .post("/api/v1/subnets", subnet, "Failed to create subnet")
                        .await
                })
                .retry(
                    ExponentialBuilder::default()
                        .with_min_delay(Duration::from_millis(500))
                        .with_max_delay(Duration::from_secs(30))
                        .with_max_times(ENTITY_CREATION_MAX_RETRIES),
                )
                .notify(|e, dur| tracing::warn!("Retrying subnet creation after {:?}: {}", dur, e))
                .await?;

                self.entity_buffer
                    .mark_subnet_created(pending_id, actual.clone())
                    .await;

                Ok(actual)
            }
            DaemonMode::ServerPoll => self
                .entity_buffer
                .await_subnet(&pending_id, SERVER_POLL_CONFIRMATION_TIMEOUT, cancel)
                .await
                .ok_or_else(|| {
                    if cancel.is_cancelled() {
                        anyhow!("Discovery cancelled while waiting for subnet creation")
                    } else {
                        anyhow!("Timeout waiting for subnet creation confirmation from server")
                    }
                }),
        }
    }

    /// Upsert VLANs discovered via SNMP. Returns mapping of vlan_number → entity UUID.
    /// Posts to the server's VLAN discovery endpoint.
    pub async fn upsert_vlans(
        &self,
        vlans: &[crate::daemon::discovery::integration::snmp::types::VlanInfo],
        network_id: Uuid,
    ) -> Result<std::collections::HashMap<u16, Uuid>, Error> {
        use crate::server::vlans::handlers::{
            VlanDiscoveryItem, VlanDiscoveryRequest, VlanDiscoveryResponse,
        };

        let request = VlanDiscoveryRequest {
            network_id,
            vlans: vlans
                .iter()
                .map(|v| VlanDiscoveryItem {
                    vlan_number: v.vlan_id,
                    name: v.name.clone(),
                })
                .collect(),
        };

        let api_client = &self.api_client;
        let response: crate::server::shared::types::api::ApiResponse<VlanDiscoveryResponse> =
            (|| async {
                api_client
                    .post(
                        "/api/v1/vlans/discovery",
                        &request,
                        "Failed to upsert VLANs",
                    )
                    .await
            })
            .retry(
                ExponentialBuilder::default()
                    .with_min_delay(Duration::from_millis(500))
                    .with_max_delay(Duration::from_secs(10))
                    .with_max_times(3),
            )
            .notify(|e, dur| tracing::warn!("Retrying VLAN upsert after {:?}: {}", dur, e))
            .await?;

        let mut mapping = std::collections::HashMap::new();
        if let Some(data) = response.data {
            for item in data.vlans {
                mapping.insert(item.vlan_number, item.id);
            }
        }
        Ok(mapping)
    }

    /// Run service matching against all registered service definitions.
    /// Returns matched services and ports. Pure logic — no side effects.
    pub fn match_services(
        &self,
        host: &Host,
        baseline_params: &ServiceMatchBaselineParams,
        gateway_ips: &[IpAddr],
        daemon_id: &Uuid,
        network_id: &Uuid,
    ) -> Result<(Vec<Service>, Vec<Port>), Error> {
        use crate::server::services::definitions::{
            docker_container::DockerContainer, open_ports::OpenPorts,
        };

        let ServiceMatchBaselineParams { all_ports, .. } = baseline_params;

        let mut services = Vec::new();
        let mut host_ports = Vec::new();
        let mut unbound_ports = all_ports.to_vec();
        let mut container_matched = false;

        let mut sorted_service_definitions: Vec<Box<dyn ServiceDefinition>> =
            ServiceDefinitionRegistry::all_service_definitions()
                .into_iter()
                .collect();

        sorted_service_definitions.sort_by_key(|s| {
            if !ServiceDefinitionExt::is_generic(s) {
                0
            } else if s.id() == OpenPorts.id() {
                3
            } else if s.id() == DockerContainer.id() || s.id() == Gateway.id() {
                2
            } else {
                1
            }
        });

        for service_definition in sorted_service_definitions {
            let service_params = ServiceMatchServiceParams {
                service_definition,
                matched_services: &services,
                unbound_ports: &unbound_ports,
            };

            let params: DiscoverySessionServiceMatchParams<'_> =
                DiscoverySessionServiceMatchParams {
                    service_params,
                    baseline_params,
                    daemon_id,
                    discovery_type: &self.discovery_type,
                    network_id,
                    gateway_ips,
                    host_id: &host.id,
                };

            if let Some((service, mut ports, _endpoint)) = Service::from_discovery(params)
                && !container_matched
            {
                if let Some(ServiceVirtualization::Docker(DockerVirtualization {
                    container_id: Some(_),
                    ..
                })) = &service.base.virtualization
                {
                    container_matched = true
                }

                let bound_port_types: Vec<PortType> =
                    ports.iter().map(|p| p.base.port_type).collect();

                host_ports.append(&mut ports);
                unbound_ports.retain(|p| !bound_port_types.contains(p));
                services.push(service);
            }
        }

        services.sort_by_key(|a| {
            -(match &a.base.source {
                EntitySource::DiscoveryWithMatch { details, .. } => {
                    (details.confidence as i32)
                        + if a.base.service_definition.has_logo() {
                            1
                        } else {
                            0
                        }
                }
                _ => MatchConfidence::NotApplicable as i32,
            })
        });

        host_ports.extend(unbound_ports.into_iter().map(Port::new_hostless));

        Ok((services, host_ports))
    }

    /// Build a HostData from scan results: creates host entity, runs service matching, names host.
    pub async fn build_host_from_scan(
        &self,
        params: ServiceMatchBaselineParams<'_>,
        hostname: Option<String>,
        host_naming_fallback: HostNamingFallback,
    ) -> Result<Option<HostData>, Error> {
        let ServiceMatchBaselineParams { ip_address, .. } = params;

        let daemon_id = self.daemon_id().await?;
        let network_id = self.network_id().await?;
        let session = self.get_session().await?;
        let gateway_ips = session.gateway_ips.clone();

        let mut host = Host::new(HostBase {
            name: "Unknown Device".to_string(),
            hostname: hostname.clone(),
            tags: Vec::new(),
            network_id,
            description: None,
            source: EntitySource::Discovery,
            virtualization: None,
            hidden: false,
            sys_descr: None,
            sys_object_id: None,
            sys_location: None,
            sys_contact: None,
            management_url: None,
            chassis_id: None,
            sys_name: None,
            manufacturer: None,
            model: None,
            serial_number: None,
            credential_assignments: vec![],
        });

        let ip_addresses = vec![ip_address.clone()];

        let (services, ports) =
            self.match_services(&host, &params, &gateway_ips, &daemon_id, &network_id)?;

        // Determine host name
        let best_service_name = services
            .iter()
            .find(|s| !ServiceDefinitionExt::is_generic(&s.base.service_definition))
            .map(|s| s.base.service_definition.name().to_string());

        if let Some(hostname) = hostname {
            host.base.name = hostname;
        } else if host_naming_fallback == HostNamingFallback::BestService
            && let Some(best_service_name) = best_service_name
        {
            host.base.name = best_service_name
        } else if host_naming_fallback == HostNamingFallback::Ip {
            host.base.name = ip_address.base.ip_address.to_string()
        } else if let Some(best_service_name) = best_service_name {
            host.base.name = best_service_name
        } else {
            host.base.name = ip_address.base.ip_address.to_string()
        }

        tracing::info!(
            ip = %ip_address.base.ip_address,
            host_name = %host.base.name,
            service_count = %services.len(),
            port_count = %ports.len(),
            "Processed host",
        );

        Ok(Some(HostData::new(
            host,
            services,
            ports,
            ip_addresses,
            vec![],
            vec![],
        )))
    }
}

fn map_progress(raw: u8, start: u8, end: u8) -> u8 {
    if start == 0 && end == 100 {
        return raw;
    }
    start + (raw as f64 * (end - start) as f64 / 100.0) as u8
}
