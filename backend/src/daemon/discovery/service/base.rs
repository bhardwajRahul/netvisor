use std::{
    net::IpAddr,
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU32, AtomicU64},
    },
};

use crate::daemon::discovery::service::warnings::{
    CredentialIssue, IncompleteInterfaceWalk, IncompleteSnmpWalk,
};
use crate::daemon::discovery::types::base::DiscoverySessionInfo;
use crate::daemon::{
    discovery::{buffer::EntityBuffer, manager::DaemonDiscoverySessionManager},
    shared::{api_client::DaemonApiClient, config::ConfigStore},
    utils::base::{PlatformDaemonUtils, create_system_utils},
};
use crate::server::credentials::r#impl::mapping::{CredentialMapping, CredentialQueryPayload};
use crate::server::discovery::r#impl::scan_settings::ScanSettings;
use crate::server::discovery::r#impl::types::{DiscoveryType, HostNamingFallback};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;

pub struct DiscoveryRunner {
    pub service: Arc<DaemonDiscoveryService>,
    pub manager: Arc<DaemonDiscoverySessionManager>,
    /// The type this run was dispatched as, echoed back verbatim on every
    /// progress update. Kept whole rather than reconstructed, so the server's
    /// stored session keeps the kind it started with.
    pub discovery_type: DiscoveryType,
    pub host_id: Uuid,
    pub subnet_ids: Option<Vec<Uuid>>,
    /// Specific addresses to scan (a rescan). `None` sweeps the subnets.
    pub target_ips: Option<std::collections::HashSet<std::net::IpAddr>>,
    /// TCP ports already known on the rescan target, scanned in addition to the
    /// standard discovery set.
    pub extra_ports: Vec<u16>,
    pub host_naming_fallback: HostNamingFallback,
    pub scan_settings: ScanSettings,
    pub credential_mappings: Vec<CredentialMapping<CredentialQueryPayload>>,
}

impl DiscoveryRunner {
    /// Build a runner from the dispatched discovery type.
    ///
    /// Returns `None` for the frozen legacy types, which the manager stubs out
    /// rather than running.
    pub fn new(
        service: Arc<DaemonDiscoveryService>,
        manager: Arc<DaemonDiscoverySessionManager>,
        discovery_type: DiscoveryType,
        credential_mappings: Vec<CredentialMapping<CredentialQueryPayload>>,
    ) -> Option<Self> {
        let (host_id, subnet_ids, target_ips, extra_ports, host_naming_fallback, scan_settings) =
            match &discovery_type {
                DiscoveryType::Unified {
                    host_id,
                    subnet_ids,
                    host_naming_fallback,
                    scan_settings,
                } => (
                    *host_id,
                    subnet_ids.clone(),
                    None,
                    Vec::new(),
                    *host_naming_fallback,
                    scan_settings.clone(),
                ),
                DiscoveryType::Rescan {
                    host_id,
                    ips,
                    ports,
                    settings,
                    ..
                } => (
                    *host_id,
                    None,
                    Some(ips.iter().copied().collect()),
                    ports
                        .iter()
                        .filter(|p| p.is_tcp())
                        .map(|p| p.number())
                        .collect(),
                    HostNamingFallback::default(),
                    ScanSettings::from(settings),
                ),
                DiscoveryType::SelfReport { .. }
                | DiscoveryType::Network { .. }
                | DiscoveryType::Docker { .. } => return None,
            };

        Some(Self {
            service,
            manager,
            discovery_type,
            host_id,
            subnet_ids,
            target_ips,
            extra_ports,
            host_naming_fallback,
            scan_settings,
            credential_mappings,
        })
    }
}

impl From<&DiscoveryRunner> for DiscoveryType {
    fn from(runner: &DiscoveryRunner) -> Self {
        runner.discovery_type.clone()
    }
}

#[derive(Clone)]
pub struct DiscoverySession {
    pub info: DiscoverySessionInfo,
    pub gateway_ips: Vec<IpAddr>,
    pub last_progress: Arc<AtomicU8>,
    pub last_progress_report_time: Arc<AtomicU64>,
    pub hosts_discovered: Arc<AtomicU32>,
    pub estimated_remaining_secs: Arc<AtomicU32>,
    pub progress_range_start: Arc<AtomicU8>,
    pub progress_range_end: Arc<AtomicU8>,
    /// Non-fatal warnings accumulated during the run (e.g. the discovery hit its
    /// time limit and left hosts un-scanned). Surfaced in the terminal session
    /// update so the user sees them without the run being marked as failed.
    ///
    /// Push here only for things that happen at most once per run. Anything that
    /// fires per host belongs in one of the typed accumulators below, which are
    /// rendered to a single line at finalize — a `Vec<String>` written per host
    /// multiplies by the host count and drowns the notification.
    pub warnings: Arc<std::sync::Mutex<Vec<String>>>,
    /// Per-host SNMP walks that could not be read in full. See
    /// [`crate::daemon::discovery::service::warnings`].
    pub incomplete_snmp_walks: Arc<std::sync::Mutex<Vec<IncompleteSnmpWalk>>>,
    /// Per-host ifTable walks that could not be read in full. Separate from the above because
    /// a truncated interface set and a truncated attribute column mean different things.
    pub incomplete_interface_walks: Arc<std::sync::Mutex<Vec<IncompleteInterfaceWalk>>>,
    /// IP-targeted credentials that produced nothing, and why.
    pub credential_issues: Arc<std::sync::Mutex<Vec<CredentialIssue>>>,
}

impl DiscoverySession {
    pub fn new(info: DiscoverySessionInfo, gateway_ips: Vec<IpAddr>) -> Self {
        Self {
            info,
            gateway_ips,
            last_progress: Arc::new(AtomicU8::new(0)),
            last_progress_report_time: Arc::new(AtomicU64::new(0)),
            hosts_discovered: Arc::new(AtomicU32::new(0)),
            estimated_remaining_secs: Arc::new(AtomicU32::new(u32::MAX)),
            progress_range_start: Arc::new(AtomicU8::new(0)),
            progress_range_end: Arc::new(AtomicU8::new(100)),
            warnings: Arc::new(std::sync::Mutex::new(Vec::new())),
            incomplete_snmp_walks: Arc::new(std::sync::Mutex::new(Vec::new())),
            incomplete_interface_walks: Arc::new(std::sync::Mutex::new(Vec::new())),
            credential_issues: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn set_progress_range(&self, start: u8, end: u8) {
        use std::sync::atomic::Ordering;
        self.progress_range_start.store(start, Ordering::Relaxed);
        self.progress_range_end.store(end, Ordering::Relaxed);
    }
}

pub struct DaemonDiscoveryService {
    pub config_store: Arc<ConfigStore>,
    pub api_client: Arc<DaemonApiClient>,
    pub utils: PlatformDaemonUtils,
    pub current_session: Arc<RwLock<Option<DiscoverySession>>>,
    pub entity_buffer: Arc<EntityBuffer>,
    /// Stores the terminal state (Complete/Failed/Cancelled) for ServerPoll mode.
    /// In ServerPoll mode, the server polls for progress updates. If the session ends
    /// between polls, we need to retain the terminal state so the server can receive it.
    /// This is cleared when a new session starts.
    pub terminal_payload: Arc<RwLock<Option<DiscoveryUpdatePayload>>>,
    /// Shared gate that staggers the start of DaemonPoll host-create requests so
    /// a burst of near-simultaneous deep-scan completions doesn't hammer the
    /// server's host-create endpoint (where they'd otherwise queue on the
    /// per-network `HostDedup` advisory lock). Holds the earliest instant the
    /// next host submission may start; each submission reserves and advances it.
    pub host_submit_gate: Arc<tokio::sync::Mutex<tokio::time::Instant>>,
}

impl DaemonDiscoveryService {
    pub fn new(config_store: Arc<ConfigStore>, entity_buffer: Arc<EntityBuffer>) -> Self {
        Self {
            api_client: Arc::new(DaemonApiClient::new(config_store.clone())),
            config_store,
            utils: create_system_utils(),
            current_session: Arc::new(RwLock::new(None)),
            entity_buffer,
            terminal_payload: Arc::new(RwLock::new(None)),
            host_submit_gate: Arc::new(tokio::sync::Mutex::new(tokio::time::Instant::now())),
        }
    }

    pub async fn get_session(&self) -> Result<DiscoverySession, anyhow::Error> {
        self.current_session
            .read()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No active discovery session"))
    }
}
