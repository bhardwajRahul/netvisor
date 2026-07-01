//! Shared container-runtime discovery machinery (Docker + Podman).
//!
//! Docker and Podman both expose a Docker-compatible REST API, so a single
//! scanner and two transports (HTTP(S) proxy + local Unix socket) serve both.
//! The [`ContainerRuntime`] kind selects the runtime-specific bits — which
//! daemon service the discovered containers belong to, which `ClientProbe`
//! feeds service matching, and which virtualization variants get stamped onto
//! services and subnets. The `docker` and `podman` integration modules are thin
//! wrappers that call into here with the appropriate runtime.

pub mod scanner;

use std::net::IpAddr;
use std::time::Duration;

use anyhow::{Error, Result};
use bollard::Docker;
use uuid::Uuid;

use crate::daemon::utils::base::DaemonUtils;
use crate::server::credentials::r#impl::mapping::ContainerProxyQueryCredential;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::services::r#impl::virtualization::{
    DockerVirtualization, PodmanVirtualization, ServiceVirtualization,
};
use crate::server::subnets::r#impl::types::SubnetType;
use crate::server::subnets::r#impl::virtualization::{
    DockerSubnetVirtualization, PodmanSubnetVirtualization, SubnetVirtualization,
};

use super::{ProbeContext, ProbeFailure, ProbeSuccess};

const CONTAINER_PROBE_MAX_ATTEMPTS: u32 = 3;

/// Which container runtime an integration targets. Selects every
/// runtime-specific decision in the otherwise-shared scanning machinery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

impl ContainerRuntime {
    /// The daemon service definition these containers belong to. `execute`
    /// matches it in `host_data` by id to recover its `service_id`.
    pub fn service_def(&self) -> Box<dyn ServiceDefinition> {
        match self {
            Self::Docker => Box::new(crate::server::services::definitions::docker_daemon::Docker),
            Self::Podman => Box::new(crate::server::services::definitions::podman::Podman),
        }
    }

    /// The client-probe value fed into service matching for this runtime.
    pub fn client_probe(&self) -> ClientProbe {
        match self {
            Self::Docker => ClientProbe::Docker,
            Self::Podman => ClientProbe::Podman,
        }
    }

    /// The `SubnetType` stamped on this runtime's bridge networks.
    pub fn bridge_subnet_type(&self) -> SubnetType {
        match self {
            Self::Docker => SubnetType::DockerBridge,
            Self::Podman => SubnetType::PodmanBridge,
        }
    }

    /// Build the subnet virtualization for a bridge network owned by `service_id`.
    pub fn subnet_virtualization(&self, service_id: Uuid) -> SubnetVirtualization {
        match self {
            Self::Docker => SubnetVirtualization::Docker(DockerSubnetVirtualization { service_id }),
            Self::Podman => SubnetVirtualization::Podman(PodmanSubnetVirtualization { service_id }),
        }
    }

    /// Build the service virtualization stamped onto a discovered container.
    pub fn service_virtualization(
        &self,
        container_name: Option<String>,
        container_id: Option<String>,
        service_id: Uuid,
        compose_project: Option<String>,
    ) -> ServiceVirtualization {
        match self {
            Self::Docker => ServiceVirtualization::Docker(DockerVirtualization {
                container_name,
                container_id,
                service_id,
                compose_project,
            }),
            Self::Podman => ServiceVirtualization::Podman(PodmanVirtualization {
                container_name,
                container_id,
                service_id,
                compose_project,
            }),
        }
    }

    /// Human-facing label for logs/diagnostics.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Docker => "Docker",
            Self::Podman => "Podman",
        }
    }
}

/// Handle returned by a successful container-runtime probe (Docker or Podman).
pub struct ContainerProbeHandle {
    pub client: Docker,
    pub port: u16,
    /// Must stay alive until `client` is dropped — bollard reads certs lazily.
    pub _ssl_temp_handles: Vec<tempfile::NamedTempFile>,
}

pub type SslPaths = Option<(String, String, String)>;

/// Build a proxy URL from a container-runtime proxy credential and target IP.
pub fn build_container_proxy_url(ip: IpAddr, cred: &ContainerProxyQueryCredential) -> String {
    let proxy_path = cred.path.as_deref().unwrap_or("").trim_start_matches('/');
    let has_ssl = cred.ssl_cert.is_some();
    let scheme = if has_ssl { "https" } else { "http" };
    let host_str = match ip {
        IpAddr::V6(v6) => format!("[{}]", v6),
        _ => ip.to_string(),
    };
    if proxy_path.is_empty() {
        format!("{}://{}:{}", scheme, host_str, cred.port)
    } else {
        format!("{}://{}:{}/{}", scheme, host_str, cred.port, proxy_path)
    }
}

/// Resolve SSL paths from a proxy credential, returning (cert, key, chain) paths
/// and temp file handles that must outlive the client.
pub fn resolve_container_ssl(
    cred: &ContainerProxyQueryCredential,
    label: &str,
) -> Result<(SslPaths, Vec<tempfile::NamedTempFile>), Error> {
    let mut temp_handles = Vec::new();

    let ssl_info = if let (Some(cert_rv), Some(key_rv), Some(chain_rv)) =
        (&cred.ssl_cert, &cred.ssl_key, &cred.ssl_chain)
    {
        let (cert_path, cert_handle) = cert_rv.resolve_to_path("ssl_cert", label)?;
        let (key_path, key_handle) = key_rv.resolve_to_path("ssl_key", label)?;
        let (chain_path, chain_handle) = chain_rv.resolve_to_path("ssl_chain", label)?;
        temp_handles.extend(cert_handle);
        temp_handles.extend(key_handle);
        temp_handles.extend(chain_handle);
        Some((
            cert_path.to_string_lossy().into_owned(),
            key_path.to_string_lossy().into_owned(),
            chain_path.to_string_lossy().into_owned(),
        ))
    } else {
        None
    };

    Ok((ssl_info, temp_handles))
}

/// Probe a container-runtime proxy endpoint (Docker or Podman).
pub async fn probe_proxy(
    ctx: &ProbeContext<'_>,
    runtime: ContainerRuntime,
) -> Result<ProbeSuccess, ProbeFailure> {
    let cred = ctx
        .credential
        .as_container_proxy()
        .ok_or_else(|| ProbeFailure {
            message: format!("Expected {} proxy credential", runtime.label()),
        })?;

    let proxy_url = build_container_proxy_url(ctx.ip, cred);
    let label = ctx.credential.discovery_label();
    let (ssl_paths, ssl_temp_handles) =
        resolve_container_ssl(cred, label).map_err(|e| ProbeFailure {
            message: format!("Failed to resolve {} SSL: {}", runtime.label(), e),
        })?;

    tracing::info!(ip = %ctx.ip, proxy_url = %proxy_url, runtime = runtime.label(), "Attempting container proxy probe");

    for attempt in 1..=CONTAINER_PROBE_MAX_ATTEMPTS {
        if ctx.cancel.is_cancelled() {
            return Err(ProbeFailure {
                message: "Cancelled".to_string(),
            });
        }

        match ctx
            .utils
            .new_docker_client(Ok(Some(proxy_url.clone())), Ok(ssl_paths.clone()))
            .await
        {
            Ok(client) => {
                tracing::info!(ip = %ctx.ip, proxy_url = %proxy_url, runtime = runtime.label(), "Container client probe succeeded");
                return Ok(ProbeSuccess {
                    client_probe: runtime.client_probe(),
                    ports: vec![PortType::new_tcp(cred.port)],
                    handle: Some(Box::new(ContainerProbeHandle {
                        client,
                        port: cred.port,
                        _ssl_temp_handles: ssl_temp_handles,
                    })),
                });
            }
            Err(e) => {
                if attempt < CONTAINER_PROBE_MAX_ATTEMPTS {
                    tracing::debug!(ip = %ctx.ip, attempt, error = %e, "Container client probe failed, retrying");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                } else {
                    return Err(ProbeFailure {
                        message: format!(
                            "{} probe failed after {} attempts: {}",
                            runtime.label(),
                            CONTAINER_PROBE_MAX_ATTEMPTS,
                            e
                        ),
                    });
                }
            }
        }
    }

    Err(ProbeFailure {
        message: format!("{} probe exhausted all attempts", runtime.label()),
    })
}

/// Probe a container-runtime local Unix socket. `socket_path` of `None` uses the
/// Docker defaults (DOCKER_HOST / `/var/run/docker.sock`); `Some(path)` connects
/// to an explicit socket (e.g. the Podman socket).
pub async fn probe_socket(
    ctx: &ProbeContext<'_>,
    runtime: ContainerRuntime,
    socket_path: Option<String>,
) -> Result<ProbeSuccess, ProbeFailure> {
    match ctx
        .utils
        .new_container_socket_client(runtime, socket_path.clone())
        .await
    {
        Ok(client) => {
            tracing::debug!(runtime = runtime.label(), socket = ?socket_path, "Container socket probe succeeded");
            Ok(ProbeSuccess {
                client_probe: runtime.client_probe(),
                ports: vec![],
                handle: Some(Box::new(ContainerProbeHandle {
                    client,
                    port: 0,
                    _ssl_temp_handles: vec![],
                })),
            })
        }
        Err(e) => Err(ProbeFailure {
            message: format!("{} socket connection failed: {}", runtime.label(), e),
        }),
    }
}

/// Execute container scanning after a successful probe, enriching `host_data`
/// with discovered container services/ports/ip_addresses and bridge subnets.
pub async fn execute(
    ctx: &super::IntegrationContext<'_>,
    host_data: &mut crate::daemon::discovery::service::ops::HostData,
    runtime: ContainerRuntime,
) -> Result<(), Error> {
    use std::sync::Arc;
    use std::sync::atomic::AtomicU8;

    use scanner::ContainerScanner;

    let handle = ctx
        .probe_handle
        .and_then(|h| h.downcast_ref::<ContainerProbeHandle>())
        .ok_or_else(|| anyhow::anyhow!("Missing ContainerProbeHandle"))?;

    // Find the runtime daemon service from host_data by id (matched via the
    // runtime's client-probe). Its service_id stamps every container/subnet.
    let runtime_def_id = runtime.service_def().id();
    let runtime_service_id = host_data
        .services
        .iter()
        .find(|s| s.base.service_definition.id() == runtime_def_id)
        .map(|s| s.id)
        .ok_or_else(|| {
            anyhow::anyhow!("{} daemon service not found in host_data", runtime.label())
        })?;

    let scanner = ContainerScanner {
        runtime,
        client: &handle.client,
        runtime_service_id,
        host_ip: ctx.ip,
        host_naming_fallback: ctx.host_naming_fallback,
        ops: ctx.ops,
        cancel: ctx.cancel,
        accept_invalid_certs: ctx.accept_invalid_certs,
        utils: ctx.utils,
    };

    // Create bridge subnets locally (sent to server via create_host after service dedup)
    let bridge_subnets = scanner.create_bridge_subnets().await?;
    for subnet in &bridge_subnets {
        host_data.add_subnet(subnet.clone());
    }
    ctx.ops.report_progress(10).await.ok();

    let all_subnets: Vec<_> = ctx
        .created_subnets
        .iter()
        .cloned()
        .chain(bridge_subnets)
        .collect();

    let containers = scanner.get_containers_and_summaries().await?;
    let container_count = containers.len();
    ctx.ops.report_progress(20).await.ok();

    let mut host_interfaces = host_data.ip_addresses.clone();
    let containers_interfaces_and_subnets =
        scanner.get_container_interfaces(&containers, &all_subnets, &mut host_interfaces);

    let container_results = scanner
        .scan_and_process_containers(
            containers,
            &containers_interfaces_and_subnets,
            Arc::new(AtomicU8::new(0)),
        )
        .await?;
    ctx.ops.report_progress(90).await.ok();

    tracing::info!(
        discovered = %container_results.len(),
        total_containers = container_count,
        runtime = runtime.label(),
        "Container scanning complete"
    );

    for result in container_results {
        for service in result.services {
            host_data.add_service(service);
        }
        for port in result.ports {
            host_data.add_port(port);
        }
        for ip_address in result.ip_addresses {
            host_data.add_ip_address(ip_address);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::services::r#impl::definitions::ServiceDefinition;

    #[test]
    fn runtime_selects_distinct_service_definitions() {
        assert_eq!(ContainerRuntime::Docker.service_def().name(), "Docker");
        assert_eq!(ContainerRuntime::Podman.service_def().name(), "Podman");
        assert_ne!(
            ContainerRuntime::Docker.service_def().id(),
            ContainerRuntime::Podman.service_def().id()
        );
    }

    #[test]
    fn runtime_selects_distinct_client_probes() {
        assert_eq!(ContainerRuntime::Docker.client_probe(), ClientProbe::Docker);
        assert_eq!(ContainerRuntime::Podman.client_probe(), ClientProbe::Podman);
    }

    #[test]
    fn runtime_selects_distinct_bridge_subnet_types() {
        assert_eq!(
            ContainerRuntime::Docker.bridge_subnet_type(),
            SubnetType::DockerBridge
        );
        assert_eq!(
            ContainerRuntime::Podman.bridge_subnet_type(),
            SubnetType::PodmanBridge
        );
    }

    #[test]
    fn runtime_stamps_matching_subnet_virtualization() {
        let id = Uuid::new_v4();
        assert!(matches!(
            ContainerRuntime::Docker.subnet_virtualization(id),
            SubnetVirtualization::Docker(d) if d.service_id == id
        ));
        assert!(matches!(
            ContainerRuntime::Podman.subnet_virtualization(id),
            SubnetVirtualization::Podman(p) if p.service_id == id
        ));
    }

    #[test]
    fn runtime_stamps_matching_service_virtualization() {
        let id = Uuid::new_v4();
        let docker = ContainerRuntime::Docker.service_virtualization(
            Some("web".into()),
            Some("abc".into()),
            id,
            Some("proj".into()),
        );
        assert!(matches!(docker, ServiceVirtualization::Docker(_)));
        assert_eq!(docker.service_id(), Some(id));
        assert_eq!(docker.container_name(), Some("web"));

        let podman = ContainerRuntime::Podman.service_virtualization(
            Some("web".into()),
            Some("abc".into()),
            id,
            None,
        );
        assert!(matches!(podman, ServiceVirtualization::Podman(_)));
        assert_eq!(podman.service_id(), Some(id));
    }
}
