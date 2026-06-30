use anyhow::{Error, Result, anyhow};
use bollard::{
    Docker,
    models::{ContainerInspectResponse, ContainerSummary, PortSummaryTypeEnum},
    query_parameters::{InspectContainerOptions, ListContainersOptions},
};
use futures::future::try_join_all;
use futures::stream::{self, StreamExt};
use mac_address::MacAddress;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
};
use strum::IntoDiscriminant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::daemon::discovery::service::ops::DiscoveryOps;
use crate::daemon::utils::base::{DaemonUtils, PlatformDaemonUtils};
use crate::daemon::utils::scanner::scan_endpoints;
use crate::server::bindings::r#impl::base::{Binding, BindingDiscriminants};
use crate::server::discovery::r#impl::types::HostNamingFallback;
use crate::server::ip_addresses::r#impl::base::{ALL_IP_ADDRESSES_IP, IPAddress, IPAddressBase};
use crate::server::ports::r#impl::base::{Port, PortType};
use crate::server::services::r#impl::base::{Service, ServiceMatchBaselineParams};
use crate::server::services::r#impl::endpoints::{Endpoint, EndpointResponse};
use crate::server::subnets::r#impl::base::Subnet;

use super::ContainerRuntime;

type IpPortHashMap = HashMap<IpAddr, Vec<PortType>>;

/// Result of scanning a single container — services, ports, and ip_addresses
/// to be merged into the parent host's HostData rather than creating separate host entities.
pub struct ContainerScanResult {
    pub services: Vec<Service>,
    pub ports: Vec<Port>,
    pub ip_addresses: Vec<IPAddress>,
}

pub struct ProcessContainerParams<'a> {
    pub containers_interfaces_and_subnets: &'a HashMap<String, Vec<(IPAddress, Subnet)>>,
    pub container: &'a ContainerInspectResponse,
    pub container_summary: &'a ContainerSummary,
    pub runtime_service_id: &'a Uuid,
    pub cancel: CancellationToken,
}

/// Scans a container runtime (Docker or Podman) over its Docker-compatible API.
/// `runtime` selects the virtualization variants stamped onto discovered
/// services and subnets.
pub struct ContainerScanner<'a> {
    pub runtime: ContainerRuntime,
    pub client: &'a Docker,
    pub runtime_service_id: Uuid,
    pub host_ip: IpAddr,
    pub host_naming_fallback: HostNamingFallback,
    pub ops: &'a DiscoveryOps,
    pub cancel: &'a CancellationToken,
    pub accept_invalid_certs: bool,
    pub utils: &'a PlatformDaemonUtils,
}

impl<'a> ContainerScanner<'a> {
    /// Create bridge subnets from the runtime's networks.
    /// Returns the bridge subnets locally for use in container interface resolution.
    pub async fn create_bridge_subnets(&self) -> Result<Vec<Subnet>, Error> {
        let network_id = self.ops.network_id().await?;

        let subnets = self
            .utils
            .get_subnets_from_docker_networks(
                network_id,
                self.client,
                self.runtime,
                self.runtime_service_id,
            )
            .await
            .unwrap_or_else(|e| {
                // A failed/mis-deserialized networks listing silently empties bridge
                // subnets, degrading container→subnet mapping. Log it rather than swallow.
                tracing::warn!(
                    runtime = self.runtime.label(),
                    error = %e,
                    "Failed to list container networks; bridge subnets will be empty"
                );
                Vec::new()
            });

        // Return bridge subnets locally — they'll be created on the server
        // during create_host after service dedup (so service_id can be patched)
        Ok(subnets
            .into_iter()
            .filter(|s| s.is_container_bridge_subnet())
            .collect())
    }

    pub async fn scan_and_process_containers(
        &self,
        containers: Vec<(ContainerInspectResponse, ContainerSummary)>,
        containers_interfaces_and_subnets: &HashMap<String, Vec<(IPAddress, Subnet)>>,
        progress: Arc<AtomicU8>,
    ) -> Result<Vec<ContainerScanResult>> {
        let concurrent_scans = 15usize;
        let total = containers.len().max(1);

        // Process containers concurrently using streams
        let results = stream::iter(containers.into_iter())
            .map(|(container, container_summary)| {
                let cancel = self.cancel.clone();

                async move {
                    self.process_single_container(&ProcessContainerParams {
                        containers_interfaces_and_subnets,
                        container: &container,
                        container_summary: &container_summary,
                        runtime_service_id: &self.runtime_service_id,
                        cancel,
                    })
                    .await
                }
            })
            .buffer_unordered(concurrent_scans);

        let mut stream_pin = Box::pin(results);
        let mut all_container_data = Vec::new();
        let mut completed = 0usize;

        while let Some(result) = stream_pin.next().await {
            if self.cancel.is_cancelled() {
                tracing::warn!("Docker discovery session was cancelled");
                return Err(Error::msg("Docker discovery session was cancelled"));
            }

            completed += 1;
            progress.store(
                ((completed as f64 / total as f64) * 99.0) as u8,
                Ordering::Relaxed,
            );

            match result {
                Ok(Some(container_result)) => all_container_data.push(container_result),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        phase = "container_processing",
                        "Container processing error"
                    );
                }
            }
        }

        Ok(all_container_data)
    }

    async fn process_single_container(
        &self,
        params: &ProcessContainerParams<'_>,
    ) -> Result<Option<ContainerScanResult>> {
        let ProcessContainerParams {
            container,
            container_summary,
            cancel,
            ..
        } = params;

        if let Some(container_id) = container.id.clone() {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            if container_id != container_summary.id.clone().unwrap_or_default() {
                tracing::warn!(
                    "Container inspection failure; inspected container does not match container summary"
                );
                return Ok(None);
            }

            let host_networking_mode = container
                .host_config
                .as_ref()
                .and_then(|c| c.network_mode.clone())
                .unwrap_or_default()
                == "host";

            if host_networking_mode {
                return self
                    .process_host_mode_container(params, &container_id)
                    .await;
            } else {
                return self
                    .process_bridge_mode_container(params, &container_id)
                    .await;
            }
        }

        Ok(None)
    }

    async fn process_host_mode_container(
        &self,
        params: &ProcessContainerParams<'_>,
        container_id: &String,
    ) -> Result<Option<ContainerScanResult>> {
        let ProcessContainerParams {
            containers_interfaces_and_subnets,
            container,
            cancel,
            runtime_service_id,
            ..
        } = params;

        tracing::info!(
            "Processing host mode container {}",
            container
                .name
                .as_ref()
                .unwrap_or(&"Unknown Container Name".to_string())
        );

        let host_ip = self.host_ip;

        let open_ports: Vec<PortType> = container
            .config
            .as_ref()
            .and_then(|c| c.exposed_ports.as_ref())
            .map(|p| {
                p.iter()
                    .filter_map(|v| PortType::from_str(v).ok())
                    .collect()
            })
            .unwrap_or_default();

        // Scan endpoints for exposed ports if any are declared
        let endpoint_responses = if !open_ports.is_empty() {
            let port_scan_batch_size = 200usize.clamp(16, 1000);
            let accept_invalid_certs = self.accept_invalid_certs;
            tokio::spawn(scan_endpoints(
                host_ip,
                cancel.clone(),
                Some(open_ports.clone()),
                None,
                port_scan_batch_size,
                true,
                accept_invalid_certs,
            ))
            .await
            .map_err(|e| anyhow!("Scan task panicked: {}", e))?
            .map_err(|e| anyhow!("Endpoint scanning error: {}", e))?
        } else {
            vec![]
        };

        let empty_vec_ref = &vec![];

        let container_interfaces_and_subnets = containers_interfaces_and_subnets
            .get(container_id)
            .unwrap_or(empty_vec_ref);

        for (ip_address, subnet) in container_interfaces_and_subnets {
            let empty_client_responses = std::collections::HashMap::new();
            let params = ServiceMatchBaselineParams {
                subnet,
                ip_address,
                all_ports: &open_ports,
                endpoint_responses: &endpoint_responses,
                virtualization: &Some(
                    self.runtime.service_virtualization(
                        container
                            .name
                            .clone()
                            .map(|n| n.trim_start_matches("/").to_string()),
                        container.id.clone(),
                        **runtime_service_id,
                        Self::extract_compose_project(container),
                    ),
                ),
                client_responses: &empty_client_responses,
            };

            if let Ok(Some(host_data)) = self
                .ops
                .build_host_from_scan(params, None, self.host_naming_fallback)
                .await
            {
                return Ok(Some(ContainerScanResult {
                    services: host_data.services,
                    ports: host_data.ports,
                    ip_addresses: host_data.ip_addresses,
                }));
            }
        }
        Ok(None)
    }

    async fn process_bridge_mode_container(
        &self,
        params: &ProcessContainerParams<'_>,
        container_id: &String,
    ) -> Result<Option<ContainerScanResult>> {
        let ProcessContainerParams {
            containers_interfaces_and_subnets,
            container,
            container_summary,
            cancel,
            runtime_service_id,
            ..
        } = params;

        tracing::info!(
            "Processing bridge mode container {}",
            container
                .name
                .as_ref()
                .unwrap_or(&"Unknown Container Name".to_string())
        );

        let empty_vec_ref = &vec![];

        let container_interfaces_and_subnets = containers_interfaces_and_subnets
            .get(container_id)
            .unwrap_or(empty_vec_ref);

        // Pod members / infra containers share a netns and report the pod's full published ports;
        // scope them to their own image-declared exposed ports so each container is attributed only
        // its own services (the infra/pause container, exposing nothing, becomes a generic container).
        let exposed_port_filter: Option<HashSet<u16>> = if Self::scopes_to_exposed_ports(container)
        {
            Some(Self::exposed_port_numbers(container))
        } else {
            None
        };

        let (host_ip_to_host_ports, container_ips_to_container_ports, host_to_container_port_map) =
            self.get_ports_from_container(
                container_summary,
                container_interfaces_and_subnets,
                exposed_port_filter.as_ref(),
            );

        if container_interfaces_and_subnets.is_empty() {
            tracing::warn!(
                container = ?container.name,
                "No ip_addresses found for bridge container - Docker bridge subnets may not have been created"
            );
            return Ok(None);
        }

        for (ip_address, subnet) in container_interfaces_and_subnets {
            if cancel.is_cancelled() {
                return Err(Error::msg("Discovery was cancelled"));
            }

            let mut endpoint_responses = if let Some(name) = &container.name {
                self.scan_container_endpoints(
                    ip_address,
                    &host_to_container_port_map,
                    name.trim_start_matches("/"),
                    cancel.clone(),
                    exposed_port_filter.as_ref(),
                )
                .await?
            } else {
                vec![]
            };

            // Always try external probing when published ports exist. Exec-based results
            // may contain partial responses (e.g., from bash /dev/tcp raw sockets) that
            // don't match specific service patterns but would prevent a fallback-only
            // approach from firing. Merging both sets gives the pattern matcher the best
            // chance of identifying the specific service.
            if !host_to_container_port_map.is_empty() {
                let accept_invalid_certs = self.accept_invalid_certs;
                let external_responses = self
                    .scan_container_endpoints_external(
                        ip_address,
                        &host_to_container_port_map,
                        cancel.clone(),
                        accept_invalid_certs,
                    )
                    .await?;
                if !external_responses.is_empty() {
                    tracing::debug!(
                        "External endpoint probing found {} responses for container at {}",
                        external_responses.len(),
                        ip_address.base.ip_address
                    );
                    endpoint_responses.extend(external_responses);
                }
            }

            if !endpoint_responses.is_empty() {
                tracing::debug!(
                    "Found {} endpoint responses for container at {}",
                    endpoint_responses.len(),
                    ip_address.base.ip_address
                );
            }

            let empty_vec_ref: &Vec<_> = &Vec::new();
            let container_ports_on_ip_address = container_ips_to_container_ports
                .get(&ip_address.base.ip_address)
                .unwrap_or(empty_vec_ref);

            let empty_client_responses = std::collections::HashMap::new();
            if let Ok(Some(mut host_data)) = self
                .ops
                .build_host_from_scan(
                    ServiceMatchBaselineParams {
                        subnet,
                        ip_address,
                        all_ports: container_ports_on_ip_address,
                        endpoint_responses: &endpoint_responses,
                        virtualization: &Some(
                            self.runtime.service_virtualization(
                                container
                                    .name
                                    .clone()
                                    .map(|n| n.trim_start_matches("/").to_string()),
                                container.id.clone(),
                                **runtime_service_id,
                                Self::extract_compose_project(container),
                            ),
                        ),
                        client_responses: &empty_client_responses,
                    },
                    None,
                    self.host_naming_fallback,
                )
                .await
            {
                // Add all ip_addresses relevant to container to the ip_addresses vec
                container_interfaces_and_subnets.iter().for_each(|(i, _)| {
                    if !host_data.ip_addresses.contains(i) {
                        host_data.ip_addresses.push(i.clone())
                    }
                });

                // Container-runtime bridge subnets (Docker OR Podman) — used to exclude
                // container-internal bindings from host-port placement below.
                let container_bridge_subnet_ids: Vec<Uuid> = container_interfaces_and_subnets
                    .iter()
                    .filter(|(_, subnet)| subnet.is_container_bridge_subnet())
                    .map(|(_, subnet)| subnet.id)
                    .collect();

                host_data.services.iter_mut().for_each(|s| {
                    // Add all host port + IPs and any container ports which weren't matched
                    // We know they are open on this host even if no services matched them
                    container_ports_on_ip_address
                        .iter()
                        .for_each(|container_port| {
                            // Add bindings for container ports which weren't matched
                            match host_data
                                .ports
                                .iter()
                                .find(|p| p.base.port_type == *container_port)
                            {
                                Some(unmatched_container_port)
                                    if !s
                                        .base
                                        .bindings
                                        .iter()
                                        .filter_map(|b| b.port_id())
                                        .any(|port_id| port_id == unmatched_container_port.id) =>
                                {
                                    s.base.bindings.push(Binding::new_port_serviceless(
                                        unmatched_container_port.id,
                                        Some(ip_address.id),
                                    ))
                                }
                                _ => (),
                            }
                        });

                    // Add bindings for all host ports, provided there's an interface
                    host_ip_to_host_ports.iter().for_each(|(ip, pbs)| {
                        pbs.iter().for_each(|pb| {
                            // If there's an existing port and existing non-docker bindings, they'll need to be replaced if listener is on all ip_addresses otherwise there'll be duplicate bindings
                            let (port, existing_non_docker_bindings) =
                                match host_data.ports.iter().find(|p| p.base.port_type == *pb) {
                                    // Port exists on host, so get IDs of existing non-Docker bridge service bindings
                                    Some(existing_port) => (
                                        *existing_port,
                                        s.base
                                            .bindings
                                            .iter()
                                            .filter_map(|b| {
                                                if let Some(port_id) = b.port_id()
                                                    && port_id == existing_port.id
                                                {
                                                    // Only include if it's NOT on a Docker bridge
                                                    // Look up interface in the ip_addresses vec
                                                    if let Some(ip_address_id) = b.ip_address_id()
                                                        && let Some(ip_address) = host_data
                                                            .ip_addresses
                                                            .iter()
                                                            .find(|i| i.id == ip_address_id)
                                                        && !container_bridge_subnet_ids
                                                            .contains(&ip_address.base.subnet_id)
                                                    {
                                                        return Some(b.id());
                                                    }
                                                }
                                                None
                                            })
                                            .collect(),
                                    ),
                                    // Port doesn't exist on host yet, so it can't have been bound by service
                                    None => (Port::new_hostless(*pb), vec![]),
                                };

                            // Get host interface from the ip_addresses vec
                            let host_interface = host_data
                                .ip_addresses
                                .iter()
                                .find(|i| i.base.ip_address == *ip);

                            // Add binding to specific ip_address, or all ip_addresses if it's on ALL_IP_ADDRESSES_IP
                            match host_interface {
                                Some(host_ip_address) => {
                                    s.base.bindings.push(Binding::new_port_serviceless(
                                        port.id,
                                        Some(host_ip_address.id),
                                    ));
                                    host_data.ports.push(port);
                                }
                                None if *ip == ALL_IP_ADDRESSES_IP => {
                                    // Remove existing non-Docker bridge bindings for this port
                                    s.base.bindings = s
                                        .base
                                        .bindings
                                        .iter()
                                        .filter(|b| !existing_non_docker_bindings.contains(&b.id()))
                                        .cloned()
                                        .collect();

                                    // Add bindings for all non-Docker bridge ip_addresses
                                    // Use the interface ID from the `interfaces` list (not container_interfaces_and_subnets)
                                    // because Interface::eq deduplication at lines 617-621 may have matched
                                    // different interface objects with different UUIDs
                                    for (ip_address, subnet) in container_interfaces_and_subnets {
                                        if !subnet.is_container_bridge_subnet() {
                                            // Find the matching interface in the ip_addresses list
                                            if let Some(matched_ip_address) = host_data
                                                .ip_addresses
                                                .iter()
                                                .find(|i| *i == ip_address)
                                            {
                                                s.base.bindings.push(
                                                    Binding::new_port_serviceless(
                                                        port.id,
                                                        Some(matched_ip_address.id),
                                                    ),
                                                );
                                            }
                                        }
                                    }

                                    host_data.ports.push(port);
                                }
                                _ => {}
                            }
                        });
                    });

                    // Remove any interface bindings which are now superceded by port bindings
                    // (interface binding is implicit in port binding)
                    let ip_address_ids_with_port_binding: Vec<Uuid> = s
                        .base
                        .bindings
                        .clone()
                        .into_iter()
                        .filter_map(|b| {
                            if b.base.binding_type.discriminant() == BindingDiscriminants::Port
                                && let Some(ip_address_id) = b.ip_address_id()
                            {
                                return Some(ip_address_id);
                            }
                            None
                        })
                        .collect();

                    s.base.bindings.retain(|b| {
                        b.base.binding_type.discriminant() == BindingDiscriminants::Port
                            || !ip_address_ids_with_port_binding
                                .contains(&b.ip_address_id().unwrap_or_default())
                    });
                });

                return Ok(Some(ContainerScanResult {
                    services: host_data.services,
                    ports: host_data.ports,
                    ip_addresses: host_data.ip_addresses,
                }));
            }
        }

        Ok(None)
    }

    /// Image-declared exposed port numbers for a container (`config.exposed_ports` keys like
    /// "80/tcp"). Empty when the image declares none (e.g. a pod infra/pause container).
    fn exposed_port_numbers(container: &ContainerInspectResponse) -> HashSet<u16> {
        container
            .config
            .as_ref()
            .and_then(|c| c.exposed_ports.as_ref())
            .map(|p| {
                p.iter()
                    .filter_map(|k| PortType::from_str(k).ok().map(|pt| pt.number()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether a container's ports must be scoped to its own image-declared exposed ports rather
    /// than the published ports it reports. True for pod members (`NetworkMode "container:<id>"`)
    /// and pod infra/pause containers (name "<id>-infra"): both share one netns and report the
    /// pod's full published port set, so without scoping every member matches every co-pod service.
    fn scopes_to_exposed_ports(container: &ContainerInspectResponse) -> bool {
        let shared_netns = container
            .host_config
            .as_ref()
            .and_then(|c| c.network_mode.as_deref())
            .is_some_and(|m| m.starts_with("container:"));
        let is_infra = container
            .name
            .as_deref()
            .is_some_and(|n| n.ends_with("-infra"));
        shared_netns || is_infra
    }

    async fn scan_container_endpoints(
        &self,
        ip_address: &IPAddress,
        host_to_container_port_map: &HashMap<(IpAddr, u16), u16>,
        container_name: &str,
        cancel: CancellationToken,
        exposed_port_filter: Option<&HashSet<u16>>,
    ) -> Result<Vec<EndpointResponse>, Error> {
        use std::collections::HashMap;

        // Build inverse map: (container_port) -> Vec<(host_ip, host_port)>
        let mut container_to_host_port_map: HashMap<u16, Vec<(IpAddr, u16)>> = HashMap::new();
        for ((host_ip, host_port), container_port) in host_to_container_port_map {
            container_to_host_port_map
                .entry(*container_port)
                .or_default()
                .push((*host_ip, *host_port));
        }

        let docker = self.client;

        // Scope the loopback exec probe to the container's own ports when required (shared-netns
        // members), so a member doesn't probe a co-pod service over the shared namespace.
        let all_endpoints: Vec<_> = Service::all_discovery_endpoints()
            .into_iter()
            .filter(|e| {
                exposed_port_filter.is_none_or(|allowed| allowed.contains(&e.port_type.number()))
            })
            .collect();

        let mut endpoint_responses = Vec::new();

        for endpoint in all_endpoints {
            if cancel.is_cancelled() {
                tracing::debug!(
                    "Container endpoint scanning cancelled for {}",
                    container_name
                );
                break;
            }

            // Build command with multiple fallback options
            // Test both HTTP and HTTPS
            let requests = [
                // curl - HTTP
                format!(
                    "curl -i -s -m 1 -L --max-redirs 2 http://127.0.0.1:{}{}",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // curl - HTTPS (with -k for self-signed certs)
                format!(
                    "curl -k -i -s -m 1 -L --max-redirs 2 https://127.0.0.1:{}{}",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // wget - HTTP
                format!(
                    "wget -S -q -O- -T 1 http://127.0.0.1:{}{}",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // wget - HTTPS (with --no-check-certificate)
                format!(
                    "wget --no-check-certificate -S -q -O- -T 1 https://127.0.0.1:{}{}",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // Python - HTTP
                format!(
                    "python3 -c \"import urllib.request; req = urllib.request.Request('http://127.0.0.1:{}{}'); \
                    exec(\\\"try:\\\\n resp = urllib.request.urlopen(req)\\\\n print('HTTP/1.1', resp.status, resp.msg)\\\\n \
                    for h in resp.headers: print(h + ':', resp.headers[h])\\\\n print()\\\\n \
                    print(resp.read().decode('utf-8'))\\\\nexcept urllib.error.HTTPError as e:\\\\n \
                    print('HTTP/1.1', e.code, e.msg)\\\\n for h in e.headers: print(h + ':', e.headers[h])\\\\n \
                    print()\\\\n print(e.read().decode('utf-8'))\\\")\"",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // Python - HTTPS (with unverified SSL context)
                format!(
                    "python3 -c \"import urllib.request, ssl; \
                    ctx = ssl._create_unverified_context(); \
                    req = urllib.request.Request('https://127.0.0.1:{}{}'); \
                    exec(\\\"try:\\\\n resp = urllib.request.urlopen(req, context=ctx)\\\\n print('HTTP/1.1', resp.status, resp.msg)\\\\n \
                    for h in resp.headers: print(h + ':', resp.headers[h])\\\\n print()\\\\n \
                    print(resp.read().decode('utf-8'))\\\\nexcept urllib.error.HTTPError as e:\\\\n \
                    print('HTTP/1.1', e.code, e.msg)\\\\n for h in e.headers: print(h + ':', e.headers[h])\\\\n \
                    print()\\\\n print(e.read().decode('utf-8'))\\\")\"",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
                // bash /dev/tcp - only supports HTTP (no TLS)
                format!(
                    "bash -c \"exec 3<>/dev/tcp/127.0.0.1/{} && echo -e 'GET {} HTTP/1.0\\r\\nHost: 127.0.0.1\\r\\n\\r\\n' >&3 && cat <&3\"",
                    endpoint.port_type.number(),
                    endpoint.path
                ),
            ];

            // Join with || to try each in order, fallback to empty string
            let command = format!("{} || echo ''", requests.join(" 2>/dev/null || "));

            // Execute curl with command that works for environment
            let exec = docker
                .create_exec(
                    container_name,
                    bollard::exec::CreateExecOptions {
                        cmd: Some(vec!["sh", "-c", &command]),
                        attach_stdout: Some(true),
                        attach_stderr: Some(true),
                        ..Default::default()
                    },
                )
                .await;

            let Ok(exec_result) = exec else {
                continue;
            };

            if let Ok(bollard::exec::StartExecResults::Attached { mut output, .. }) =
                docker.start_exec(&exec_result.id, None).await
            {
                use futures::StreamExt;
                let mut full_response = String::new();

                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => {
                            tracing::debug!("Endpoint scan cancelled for container {}", container_name);
                            break;
                        }
                        msg = output.next() => {
                            match msg {
                                Some(Ok(bollard::container::LogOutput::StdOut { message })) => {
                                    full_response.push_str(&String::from_utf8_lossy(&message));
                                }
                                Some(Ok(bollard::container::LogOutput::StdErr { message })) => {
                                    // wget outputs headers to stderr with -S flag
                                    full_response.push_str(&String::from_utf8_lossy(&message));
                                }
                                Some(Ok(_)) => {}
                                Some(Err(e)) => {
                                    tracing::warn!("Error reading docker exec output: {}", e);
                                    break;
                                }
                                None => break,
                            }
                        }
                    }
                }

                let full_response = full_response.trim();

                // Parse response to check status code and extract body
                if let Some((status, body, headers)) = Self::parse_http_response(full_response) {
                    // Map back to the host-visible endpoint
                    if let Some(host_mappings) =
                        container_to_host_port_map.get(&endpoint.port_type.number())
                    {
                        for (host_ip, host_port) in host_mappings {
                            let host_endpoint = Endpoint {
                                ip: Some(*host_ip),
                                port_type: PortType::new_tcp(*host_port),
                                protocol: endpoint.protocol,
                                path: endpoint.path.clone(),
                            };

                            endpoint_responses.push(EndpointResponse {
                                endpoint: host_endpoint,
                                body: body.clone(),
                                status,
                                headers: headers.clone(),
                            });
                        }
                    }

                    // Also add the container-internal endpoint
                    let container_endpoint = Endpoint {
                        ip: Some(ip_address.base.ip_address), // Container's IP on the bridge network
                        port_type: PortType::new_tcp(endpoint.port_type.number()), // Container port, not host port
                        protocol: endpoint.protocol,
                        path: endpoint.path.clone(),
                    };

                    endpoint_responses.push(EndpointResponse {
                        endpoint: container_endpoint,
                        body: body.clone(),
                        status,
                        headers: headers.clone(),
                    });
                }
            }
        }

        Ok(endpoint_responses)
    }

    /// Fallback endpoint scanning for bridge-mode containers that lack HTTP tools.
    /// Probes host-published ports externally via reqwest, then remaps responses
    /// back to container ports for the pattern matcher.
    async fn scan_container_endpoints_external(
        &self,
        ip_address: &IPAddress,
        host_to_container_port_map: &HashMap<(IpAddr, u16), u16>,
        cancel: CancellationToken,
        accept_invalid_certs: bool,
    ) -> Result<Vec<EndpointResponse>, Error> {
        // Build inverse map: container_port -> Vec<(host_ip, host_port)>
        let mut container_to_host_port_map: HashMap<u16, Vec<(IpAddr, u16)>> = HashMap::new();
        for ((host_ip, host_port), container_port) in host_to_container_port_map {
            container_to_host_port_map
                .entry(*container_port)
                .or_default()
                .push((*host_ip, *host_port));
        }

        let all_endpoints = Service::all_discovery_endpoints();

        // Filter to endpoints whose port matches a container port with a host mapping
        let probeable_endpoints: Vec<_> = all_endpoints
            .into_iter()
            .filter(|e| container_to_host_port_map.contains_key(&e.port_type.number()))
            .collect();

        if probeable_endpoints.is_empty() {
            return Ok(vec![]);
        }

        let client = reqwest::Client::builder()
            .connect_timeout(crate::daemon::utils::scanner::SCAN_TIMEOUT)
            .danger_accept_invalid_certs(accept_invalid_certs)
            .build()
            .map_err(|e| anyhow!("Could not build client: {}", e))?;

        let mut endpoint_responses = Vec::new();

        for endpoint in &probeable_endpoints {
            if cancel.is_cancelled() {
                break;
            }

            let Some(host_mappings) = container_to_host_port_map.get(&endpoint.port_type.number())
            else {
                continue;
            };

            for (host_ip, host_port) in host_mappings {
                if cancel.is_cancelled() {
                    break;
                }

                // Resolve 0.0.0.0 to the Docker host's IP
                let probe_ip = if *host_ip == ALL_IP_ADDRESSES_IP {
                    self.host_ip
                } else {
                    *host_ip
                };

                // Try HTTP and HTTPS, same pattern as scan_endpoints in scanner.rs
                let http_url = format!("http://{}:{}{}", probe_ip, host_port, endpoint.path);
                let https_url = format!("https://{}:{}{}", probe_ip, host_port, endpoint.path);

                let urls = [http_url, https_url];

                for url in &urls {
                    tracing::trace!("Docker external probe: {}", url);

                    // Timeout covers connect + headers; body has its own deadline.
                    match tokio::time::timeout(
                        crate::daemon::utils::scanner::SCAN_TIMEOUT,
                        client.get(url).send(),
                    )
                    .await
                    {
                        Ok(Ok(response)) => {
                            let status = response.status().as_u16();
                            let headers: HashMap<String, String> = response
                                .headers()
                                .iter()
                                .filter_map(|(name, value)| {
                                    value
                                        .to_str()
                                        .ok()
                                        .map(|v| (name.as_str().to_lowercase(), v.to_string()))
                                })
                                .collect();

                            let deadline = tokio::time::Instant::now()
                                + crate::daemon::utils::scanner::SCAN_TIMEOUT;
                            let body =
                                crate::daemon::utils::scanner::read_response_body_until_deadline(
                                    response, deadline,
                                )
                                .await;

                            tracing::debug!(
                                "Docker external probe {} returned {} (length: {})",
                                url,
                                status,
                                body.len()
                            );

                            // Container-port response for pattern matching
                            endpoint_responses.push(EndpointResponse {
                                endpoint: Endpoint {
                                    ip: Some(ip_address.base.ip_address),
                                    port_type: endpoint.port_type,
                                    protocol: endpoint.protocol,
                                    path: endpoint.path.clone(),
                                },
                                body: body.clone(),
                                status,
                                headers: headers.clone(),
                            });

                            // Host-port response for downstream binding logic
                            endpoint_responses.push(EndpointResponse {
                                endpoint: Endpoint {
                                    ip: Some(*host_ip),
                                    port_type: PortType::new_tcp(*host_port),
                                    protocol: endpoint.protocol,
                                    path: endpoint.path.clone(),
                                },
                                body,
                                status,
                                headers,
                            });

                            // Got a response, no need to try HTTPS
                            break;
                        }
                        Ok(Err(e)) => {
                            tracing::trace!("Docker external probe {} failed: {}", url, e);
                            continue;
                        }
                        Err(_) => {
                            tracing::trace!(
                                "Docker external probe {} timed out waiting for headers",
                                url
                            );
                            continue;
                        }
                    }
                }
            }
        }

        Ok(endpoint_responses)
    }

    fn extract_compose_project(container: &ContainerInspectResponse) -> Option<String> {
        container
            .config
            .as_ref()
            .and_then(|c| c.labels.as_ref())
            .and_then(|l| l.get("com.docker.compose.project").cloned())
    }

    /// Parse HTTP response to extract status code and body
    /// Returns (status_code, body) if successful
    fn parse_http_response(response: &str) -> Option<(u16, String, HashMap<String, String>)> {
        if response.is_empty() {
            return None;
        }

        let response_bytes = response.as_bytes();

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut parsed_response = httparse::Response::new(&mut headers);

        match parsed_response.parse(response_bytes) {
            Ok(httparse::Status::Complete(headers_len)) => {
                let status = parsed_response.code?;
                let body = &response_bytes[headers_len..];
                let body = String::from_utf8_lossy(body).to_string();
                let headers: HashMap<String, String> = parsed_response
                    .headers
                    .iter()
                    .filter_map(|header| {
                        // Convert header value bytes to string
                        std::str::from_utf8(header.value).ok().map(|value| {
                            (
                                header.name.to_lowercase(), // Normalize to lowercase
                                value.to_string(),
                            )
                        })
                    })
                    .collect();

                Some((status, body, headers))
            }
            Ok(httparse::Status::Partial) => {
                // Not enough data, might be incomplete response
                tracing::debug!("Partial HTTP response received");
                None
            }
            Err(_) => None,
        }
    }

    fn get_ports_from_container(
        &self,
        container_summary: &ContainerSummary,
        container_interfaces_and_subnets: &[(IPAddress, Subnet)],
        exposed_port_filter: Option<&HashSet<u16>>,
    ) -> (IpPortHashMap, IpPortHashMap, HashMap<(IpAddr, u16), u16>) {
        let mut host_ip_to_host_ports: IpPortHashMap = HashMap::new();
        let mut container_ips_to_container_ports: IpPortHashMap = HashMap::new();
        let mut host_to_container_port_map: HashMap<(IpAddr, u16), u16> = HashMap::new();

        let container_ips: Vec<IpAddr> = container_interfaces_and_subnets
            .iter()
            .map(|(i, _)| i.base.ip_address)
            .collect();

        if let Some(ports) = &container_summary.ports {
            ports.iter().for_each(|p| {
                // Shared-netns members (and pod infra containers) report the pod's full published
                // port set; scope to the container's own image-declared exposed ports so each is
                // attributed only its own services.
                if let Some(allowed) = exposed_port_filter
                    && !allowed.contains(&p.private_port)
                {
                    return;
                }
                // Handle ports regardless of whether ip is set
                if let Some(port_type @ (PortSummaryTypeEnum::TCP | PortSummaryTypeEnum::UDP)) =
                    p.typ
                {
                    let private_port = match port_type {
                        PortSummaryTypeEnum::TCP => PortType::new_tcp(p.private_port),
                        PortSummaryTypeEnum::UDP => PortType::new_udp(p.private_port),
                        _ => unreachable!("Already matched TCP/UDP in outer pattern"),
                    };

                    // Always add the private port to all container IPs
                    container_ips.iter().for_each(|ip| {
                        container_ips_to_container_ports
                            .entry(*ip)
                            .or_default()
                            .push(private_port);
                    });

                    // Only handle host port mapping if we have both ip and public_port
                    if let (Some(ip_str), Some(public)) = (&p.ip, p.public_port)
                        && let Ok(ip) = ip_str.parse::<IpAddr>()
                    {
                        let public_port = match port_type {
                            PortSummaryTypeEnum::TCP => PortType::new_tcp(public),
                            PortSummaryTypeEnum::UDP => PortType::new_udp(public),
                            _ => unreachable!("Already matched TCP/UDP in outer pattern"),
                        };

                        host_ip_to_host_ports
                            .entry(ip)
                            .or_default()
                            .push(public_port);

                        host_to_container_port_map.insert((ip, public), p.private_port);
                    }
                }
            });
        }

        (
            host_ip_to_host_ports,
            container_ips_to_container_ports,
            host_to_container_port_map,
        )
    }

    pub fn get_container_interfaces(
        &self,
        containers: &[(ContainerInspectResponse, ContainerSummary)],
        subnets: &[Subnet],
        host_interfaces: &mut [IPAddress],
    ) -> HashMap<String, Vec<(IPAddress, Subnet)>> {
        // Created subnets may differ from discovered if there are existing subnets with the same CIDR, so we need to update interface subnet_id references
        let host_interfaces_and_subnets = host_interfaces
            .iter_mut()
            .filter_map(|i| {
                if let Some(subnet) = subnets
                    .iter()
                    .find(|s| s.base.cidr.contains(&i.base.ip_address))
                {
                    i.base.subnet_id = subnet.id;

                    return Some((i.clone(), subnet.clone()));
                }

                None
            })
            .collect::<Vec<(IPAddress, Subnet)>>();

        // Collect ip_addresses from containers
        let mut interfaces_by_id: HashMap<String, Vec<(IPAddress, Subnet)>> = containers
            .iter()
            .filter_map(|(container, _)| {
                let host_networking_mode = container
                    .host_config
                    .as_ref()
                    .and_then(|c| c.network_mode.clone())
                    .unwrap_or_default()
                    == "host";

                let mut ip_addresses_and_subnets: Vec<(IPAddress, Subnet)> = if host_networking_mode
                {
                    host_interfaces_and_subnets.clone()
                }
                // Containers not in host networking mode
                else if let Some(network_settings) = &container.network_settings {
                    if let Some(networks) = &network_settings.networks {
                        networks
                            .iter()
                            .filter_map(|(network_name, endpoint)| {
                                // Parse interface if IP
                                if let Some(ip_string) = &endpoint.ip_address {
                                    let ip_address = ip_string.parse::<IpAddr>().ok();

                                    if let Some(ip_address) = ip_address
                                        && let Some(subnet) = subnets
                                            .iter()
                                            .find(|s| s.base.cidr.contains(&ip_address))
                                    {
                                        // Parse MAC address from Docker network endpoint
                                        let mac_address = endpoint
                                            .mac_address
                                            .as_ref()
                                            .and_then(|mac_str| mac_str.parse::<MacAddress>().ok());

                                        return Some((
                                            IPAddress::new(IPAddressBase {
                                                network_id: subnet.base.network_id,
                                                host_id: Uuid::nil(), // Placeholder - server will set correct host_id
                                                subnet_id: subnet.id,
                                                ip_address,
                                                mac_address,
                                                name: Some(network_name.to_owned()),
                                                position: 0,
                                            }),
                                            subnet.clone(),
                                        ));
                                    }
                                }
                                tracing::warn!(
                                    "No matching subnet found for container {:?} on network '{}'",
                                    container.name,
                                    network_name
                                );

                                None
                            })
                            .collect::<Vec<(IPAddress, Subnet)>>()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Merge in host ip_addresses
                ip_addresses_and_subnets.extend(host_interfaces_and_subnets.clone());

                container
                    .id
                    .as_ref()
                    .map(|id| (id.clone(), ip_addresses_and_subnets))
            })
            .collect();

        // Pod / shared-netns members run with NetworkMode "container:<id>" — they share the
        // referenced container's network namespace and report no networks of their own (so the
        // pass above leaves them with empty interfaces and they'd be dropped). Inherit the
        // referenced container's interfaces so the member is still discovered (e.g. a pod's
        // nginx member sharing the infra container's IP). The reference may be a short or full id.
        let shared_netns_members: Vec<(String, String)> = containers
            .iter()
            .filter_map(|(container, _)| {
                let mode = container
                    .host_config
                    .as_ref()
                    .and_then(|c| c.network_mode.clone())
                    .unwrap_or_default();
                let reference = mode.strip_prefix("container:")?.to_string();
                Some((container.id.clone()?, reference))
            })
            .collect();

        for (member_id, reference) in shared_netns_members {
            if let Some(parent_id) = interfaces_by_id
                .keys()
                .find(|k| k.starts_with(&reference))
                .cloned()
                && let Some(parent_ifaces) = interfaces_by_id.get(&parent_id).cloned()
                && !parent_ifaces.is_empty()
            {
                interfaces_by_id.insert(member_id, parent_ifaces);
            }
        }

        interfaces_by_id
    }

    pub async fn get_containers_and_summaries(
        &self,
    ) -> Result<Vec<(ContainerInspectResponse, ContainerSummary)>, Error> {
        let container_summaries = self
            .client
            .list_containers(None::<ListContainersOptions>)
            .await
            .map_err(|e| anyhow!(e))?;

        let containers_to_inspect: Vec<_> = container_summaries
            .iter()
            .filter_map(|c| {
                if let Some(id) = &c.id {
                    return Some(
                        self.client
                            .inspect_container(id, None::<InspectContainerOptions>),
                    );
                }
                None
            })
            .collect();

        let inspected_containers: Vec<ContainerInspectResponse> =
            try_join_all(containers_to_inspect).await?;

        Ok(inspected_containers
            .into_iter()
            .zip(container_summaries)
            .collect())
    }
}
