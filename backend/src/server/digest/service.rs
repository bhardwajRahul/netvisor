use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::bindings::service::BindingService;
use crate::server::daemons::r#impl::api::{DiscoveryUpdatePayload, ScannedEntityIds};
use crate::server::discovery::r#impl::types::DiscoveryType;
use crate::server::discovery::service::DiscoveryService;
use crate::server::services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt};
use crate::server::services::r#impl::virtualization::ServiceVirtualization;
use crate::server::shared::storage::snapshot::DiscoveryTracked;
use crate::server::{
    digest::payload::{
        AffectedHostCard, DigestRecipient, DiscoveryDigestFlags, DiscoveryDigestOperation,
        DiscoveryDigestPayload, DiscoveryDigestScope, EntityDigestStatus, HostSummary,
        InterfaceSummary, IpAddressSummary, PortSummary, ServiceSummary, SubnetSummary,
        VlanSummary,
    },
    hosts::{r#impl::base::Host, service::HostService},
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    ip_addresses::{r#impl::base::IPAddress, service::IPAddressService},
    networks::service::NetworkService,
    ports::{r#impl::base::Port, service::PortService},
    services::{r#impl::base::Service as NetworkServiceEntity, service::ServiceService},
    shared::{
        events::{bus::EventBus, traits::Event},
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, traits::Storage},
    },
    subnets::{r#impl::base::Subnet, service::SubnetService},
    users::service::UserService,
    vlans::{r#impl::base::Vlan, service::VlanService},
};

/// Read-only aggregator that answers "what changed in this network during
/// session [T_start, T_end]" by composing SCD2 timestamp filters across the
/// per-entity-tracked tables. Mirrors `TopologyService`'s shape: holds Arcs
/// to the entity services it queries, no storage-layer deps.
pub struct DiscoveryDigestService {
    pub host_service: Arc<HostService>,
    pub service_service: Arc<ServiceService>,
    pub port_service: Arc<PortService>,
    pub ip_address_service: Arc<IPAddressService>,
    pub interface_service: Arc<InterfaceService>,
    pub binding_service: Arc<BindingService>,
    pub subnet_service: Arc<SubnetService>,
    pub vlan_service: Arc<VlanService>,
    pub user_service: Arc<UserService>,
    pub network_service: Arc<NetworkService>,
    pub discovery_service: Arc<DiscoveryService>,
    pub event_bus: Arc<EventBus>,
}

impl DiscoveryDigestService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host_service: Arc<HostService>,
        service_service: Arc<ServiceService>,
        port_service: Arc<PortService>,
        ip_address_service: Arc<IPAddressService>,
        interface_service: Arc<InterfaceService>,
        binding_service: Arc<BindingService>,
        subnet_service: Arc<SubnetService>,
        vlan_service: Arc<VlanService>,
        user_service: Arc<UserService>,
        network_service: Arc<NetworkService>,
        discovery_service: Arc<DiscoveryService>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            host_service,
            service_service,
            port_service,
            ip_address_service,
            interface_service,
            binding_service,
            subnet_service,
            vlan_service,
            user_service,
            network_service,
            discovery_service,
            event_bus,
        }
    }

    /// Compute the digest for `payload` and publish a
    /// `DiscoveryDigestOperation::Computed` event. Skips publishing entirely
    /// when timestamps are missing — those would yield meaningless filter
    /// windows.
    pub async fn compute_and_publish(
        &self,
        payload: &DiscoveryUpdatePayload,
        scanned: &ScannedEntityIds,
    ) -> Result<()> {
        let (Some(t_start), Some(t_end)) = (payload.started_at, payload.finished_at) else {
            tracing::warn!(
                session_id = %payload.session_id,
                "Discovery session terminal payload missing started_at or finished_at; skipping digest",
            );
            return Ok(());
        };

        let network = match self.network_service.get_by_id(&payload.network_id).await? {
            Some(n) => n,
            None => {
                tracing::warn!(
                    network_id = %payload.network_id,
                    "Network missing for digest computation; skipping",
                );
                return Ok(());
            }
        };

        let digest = self
            .compute(
                payload,
                scanned,
                t_start,
                t_end,
                &network.base.name,
                network.base.organization_id,
            )
            .await?;

        let scope = DiscoveryDigestScope {
            organization_id: network.base.organization_id,
            network_id: payload.network_id,
        };
        let event = Event::new(
            scope,
            DiscoveryDigestOperation::Computed {
                payload: Box::new(digest),
            },
            AuthenticatedEntity::System,
        )
        .with_flags(DiscoveryDigestFlags::default());
        self.event_bus.publish(event).await?;
        Ok(())
    }

    async fn compute(
        &self,
        payload: &DiscoveryUpdatePayload,
        scanned: &ScannedEntityIds,
        t_start: DateTime<Utc>,
        t_end: DateTime<Utc>,
        network_name: &str,
        organization_id: Uuid,
    ) -> Result<DiscoveryDigestPayload> {
        let network_id = payload.network_id;

        // Subnets scanned: prefer the discovery config's explicit subnet
        // list when set (the user targeted a specific subset). Fall back
        // to the daemon's reported set. Either way, drop loopback subnets
        // (127.0.0.0/8 + ::1) — they're not user-meaningful.
        let targeted: Option<&[Uuid]> = match &payload.discovery_type {
            DiscoveryType::Network {
                subnet_ids: Some(ids),
                ..
            }
            | DiscoveryType::Unified {
                subnet_ids: Some(ids),
                ..
            } if !ids.is_empty() => Some(ids.as_slice()),
            _ => None,
        };
        let subnet_ids: &[Uuid] = targeted.unwrap_or(scanned.subnet_ids.as_slice());

        let subnets_scanned: Vec<SubnetSummary> = if subnet_ids.is_empty() {
            Vec::new()
        } else {
            self.subnet_service
                .get_all(StorableFilter::<Subnet>::new_from_entity_ids(subnet_ids))
                .await?
                .iter()
                .filter(|s| !s.base.cidr.first_address().is_loopback())
                .map(subnet_summary)
                .collect()
        };

        // Recent-history grace: a child not in this scan that was last
        // touched by one of the top-N most recent historical discoveries
        // on this network gets `PossiblyMissing` rather than `Missing`.
        // Tolerates transient scan-to-scan noise. Top-(N+1) gives us the
        // discovery that just fell off the grace window this scan (used to
        // detect fresh `Missing` transitions).
        const REMOVAL_GRACE_SCANS: usize = 3;
        let recent_window: Vec<Uuid> = self
            .discovery_service
            .get_recent_historical_ids(network_id, REMOVAL_GRACE_SCANS + 1)
            .await?;
        let window = DigestWindow {
            t_start,
            t_end,
            recent: recent_window
                .iter()
                .take(REMOVAL_GRACE_SCANS)
                .copied()
                .collect(),
            d1: recent_window.get(1).copied(),
            d_dropped: recent_window.get(REMOVAL_GRACE_SCANS).copied(),
        };

        // One query for all live hosts on the network — the generic helper
        // buckets them by status. Per-entity-type queries for children are
        // batched the same way inside fetch_current_children.
        let all_hosts: Vec<Host> = self
            .host_service
            .get_all(StorableFilter::<Host>::new_from_network_ids(&[network_id]).live())
            .await?;
        let scanned_host_ids: HashSet<Uuid> = scanned.host_ids.iter().copied().collect();

        // First pass on hosts: bucket each one by status + fresh.
        struct HostBucket {
            host: Host,
            status: EntityDigestStatus,
            is_fresh: bool,
        }
        let host_buckets: Vec<HostBucket> = all_hosts
            .into_iter()
            .map(|h| {
                let (status, is_fresh) = compute_digest_status(&h, &scanned_host_ids, &window);
                HostBucket {
                    host: h,
                    status,
                    is_fresh,
                }
            })
            .collect();

        // Affected = added + every host that's seen-this-scan (their
        // children might have fresh deltas) + every host that's
        // (Possibly)Missing with fresh transition (we want to render
        // their last-known children too).
        let affected_ids: Vec<Uuid> = host_buckets
            .iter()
            .filter(|b| match b.status {
                EntityDigestStatus::New | EntityDigestStatus::Unchanged => true,
                EntityDigestStatus::PossiblyMissing | EntityDigestStatus::Missing => b.is_fresh,
            })
            .map(|b| b.host.id)
            .collect();

        let current_children = self
            .fetch_current_children(&affected_ids, scanned, &window)
            .await?;

        let mut hosts_added: Vec<AffectedHostCard> = Vec::new();
        let mut hosts_vanished: Vec<AffectedHostCard> = Vec::new();
        let mut hosts_changed: Vec<AffectedHostCard> = Vec::new();

        for HostBucket {
            host,
            status,
            is_fresh,
        } in host_buckets
        {
            match status {
                EntityDigestStatus::New => {
                    hosts_added.push(build_card(&host, status, &current_children));
                }
                EntityDigestStatus::PossiblyMissing | EntityDigestStatus::Missing => {
                    if is_fresh {
                        hosts_vanished.push(build_card(&host, status, &current_children));
                    }
                }
                EntityDigestStatus::Unchanged => {
                    let card = build_card(&host, status, &current_children);
                    if card_has_fresh_children(&card) {
                        hosts_changed.push(card);
                    }
                }
            }
        }
        hosts_changed.sort_by(|a, b| a.host.label.cmp(&b.host.label));

        // VLANs added / removed mirror the host added/vanished logic but on
        // the network scope.
        let vlans_added_records: Vec<Vlan> = self
            .vlan_service
            .get_all(
                StorableFilter::<Vlan>::new_from_network_ids(&[network_id])
                    .live()
                    .created_between(t_start, t_end),
            )
            .await?;
        let vlans_added: Vec<VlanSummary> = vlans_added_records.iter().map(vlan_summary).collect();

        let vlans_removed_records: Vec<Vlan> = self
            .vlan_service
            .get_all(
                StorableFilter::<Vlan>::new_from_network_ids(&[network_id])
                    .live()
                    .created_before(t_start)
                    .last_seen_before(t_start),
            )
            .await?;
        let vlans_removed: Vec<VlanSummary> =
            vlans_removed_records.iter().map(vlan_summary).collect();

        let recipients = self.resolve_recipients(network_id, organization_id).await?;

        Ok(DiscoveryDigestPayload {
            session_id: payload.session_id,
            network_id,
            network_name: network_name.to_string(),
            started_at: t_start,
            finished_at: t_end,
            subnets_scanned,
            hosts_added,
            hosts_vanished,
            hosts_changed,
            vlans_added,
            vlans_removed,
            recipients,
        })
    }

    /// Fetch live children for the affected-host set and bucket them by
    /// host_id. Each child summary carries its `EntityDigestStatus` and
    /// an `is_fresh` flag (computed via the generic
    /// [`compute_digest_status`] helper). Filters Unclaimed Open Ports and
    /// loopback IPs.
    async fn fetch_current_children(
        &self,
        host_ids: &[Uuid],
        scanned: &ScannedEntityIds,
        window: &DigestWindow,
    ) -> Result<HashMap<Uuid, HostChildren>> {
        if host_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let services: Vec<NetworkServiceEntity> = self
            .service_service
            .storage()
            .get_all(
                StorableFilter::<NetworkServiceEntity>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;
        let ips: Vec<IPAddress> = self
            .ip_address_service
            .storage()
            .get_all(
                StorableFilter::<IPAddress>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;
        let interfaces: Vec<Interface> = self
            .interface_service
            .storage()
            .get_all(
                StorableFilter::<Interface>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;
        let ports: Vec<Port> = self
            .port_service
            .storage()
            .get_all(
                StorableFilter::<Port>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;

        let scanned_service_ids: HashSet<Uuid> = scanned.service_ids.iter().copied().collect();
        let scanned_port_ids: HashSet<Uuid> = scanned.port_ids.iter().copied().collect();
        let scanned_ip_ids: HashSet<Uuid> = scanned.ip_address_ids.iter().copied().collect();
        let scanned_interface_ids: HashSet<Uuid> = scanned.interface_ids.iter().copied().collect();

        let mut out: HashMap<Uuid, HostChildren> = HashMap::new();
        for id in host_ids {
            out.entry(*id).or_default();
        }
        for s in &services {
            // Skip the synthetic "Unclaimed Open Ports" service — it's
            // useful in the UI's open-ports panel but noise in the digest.
            if s.base.service_definition.is_open_ports() {
                continue;
            }
            let (status, is_fresh) = compute_digest_status(s, &scanned_service_ids, window);
            out.entry(s.base.host_id)
                .or_default()
                .services
                .push(service_summary(s, status, is_fresh));
        }
        for ip in &ips {
            // Skip loopback (127.0.0.0/8, ::1) — typically the daemon's own
            // local address, set once at daemon registration and never
            // re-included in subsequent scan sets. Would graduate straight
            // to Missing and falsely mark the daemon host as Changed.
            if ip.base.ip_address.is_loopback() {
                continue;
            }
            let (status, is_fresh) = compute_digest_status(ip, &scanned_ip_ids, window);
            out.entry(ip.base.host_id)
                .or_default()
                .ip_addresses
                .push(ip_summary(ip, status, is_fresh));
        }
        for i in &interfaces {
            let (status, is_fresh) = compute_digest_status(i, &scanned_interface_ids, window);
            out.entry(i.base.host_id)
                .or_default()
                .interfaces
                .push(interface_summary(i, status, is_fresh));
        }
        for p in &ports {
            let (status, is_fresh) = compute_digest_status(p, &scanned_port_ids, window);
            out.entry(p.base.host_id)
                .or_default()
                .ports
                .push(port_summary(p, status, is_fresh));
        }
        Ok(out)
    }

    async fn resolve_recipients(
        &self,
        network_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Vec<DigestRecipient>> {
        let users = self
            .user_service
            .get_users_with_network_access(&network_id, &organization_id)
            .await?;
        Ok(users
            .into_iter()
            .map(|u| DigestRecipient {
                user_id: u.id,
                email: u.base.email,
                discovery_digest_enabled: u.base.email_settings.discovery_digest,
            })
            .collect())
    }
}

impl EventBusService<Host> for DiscoveryDigestService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &Host) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, _entity: &Host) -> Option<Uuid> {
        None
    }
}

/// Per-scan window context for `compute_digest_status`. Built once per
/// digest from the network's recent-discovery history.
struct DigestWindow {
    t_start: DateTime<Utc>,
    t_end: DateTime<Utc>,
    /// Top-N most-recent historical discoveries (grace window).
    recent: HashSet<Uuid>,
    /// Previous scan: `last_discovery_id == this` ⇒ entity was seen last
    /// time and isn't now → fresh PossiblyMissing transition.
    d1: Option<Uuid>,
    /// The discovery that just dropped off the top-N this scan:
    /// `last_discovery_id == this` ⇒ entity just graduated to Missing.
    d_dropped: Option<Uuid>,
}

/// Compute the per-entity digest status. Generic over any
/// `DiscoveryTracked` type — hosts and their children all flow through
/// this single helper.
///
/// Returns `(status, is_fresh)` where `is_fresh` means "this status was
/// acquired in THIS scan" (a transition just happened). Stably-stale
/// entities have `is_fresh == false` and don't trigger card inclusion.
///
/// `scanned_ids` is the daemon-reported set for whichever entity kind
/// `T` is — `scanned.host_ids` for hosts, `scanned.port_ids` for ports,
/// etc.
fn compute_digest_status<T: DiscoveryTracked>(
    entity: &T,
    scanned_ids: &HashSet<Uuid>,
    window: &DigestWindow,
) -> (EntityDigestStatus, bool) {
    use EntityDigestStatus::*;
    let id = entity.id();
    let created_at = entity.created_at();
    let last_disc = entity.last_discovery_id();

    if scanned_ids.contains(&id) {
        let is_new = created_at >= window.t_start && created_at <= window.t_end;
        return (if is_new { New } else { Unchanged }, is_new);
    }
    // Not in this scan. Recent-history grace check.
    let is_fresh_pm = matches!((last_disc, window.d1), (Some(a), Some(b)) if a == b);
    let is_fresh_m = matches!((last_disc, window.d_dropped), (Some(a), Some(b)) if a == b);
    match last_disc {
        Some(lid) if window.recent.contains(&lid) => (PossiblyMissing, is_fresh_pm),
        _ => (Missing, is_fresh_m),
    }
}

/// True when at least one child has `is_fresh == true` — i.e. a real
/// transition happened on this host this scan. Used to drop noisy host
/// cards whose only "non-Unchanged" children have been in that state for
/// multiple scans already.
fn card_has_fresh_children(card: &AffectedHostCard) -> bool {
    card.services.iter().any(|s| s.is_fresh)
        || card.ip_addresses.iter().any(|x| x.is_fresh)
        || card.interfaces.iter().any(|i| i.is_fresh)
        || card.ports.iter().any(|p| p.is_fresh)
}

#[derive(Default)]
struct HostChildren {
    services: Vec<ServiceSummary>,
    ip_addresses: Vec<IpAddressSummary>,
    interfaces: Vec<InterfaceSummary>,
    ports: Vec<PortSummary>,
}

fn build_card(
    host: &Host,
    status: EntityDigestStatus,
    children: &HashMap<Uuid, HostChildren>,
) -> AffectedHostCard {
    let kids = children.get(&host.id);
    AffectedHostCard {
        host: host_summary(host),
        status,
        services: kids.map(|c| c.services.clone()).unwrap_or_default(),
        ip_addresses: kids.map(|c| c.ip_addresses.clone()).unwrap_or_default(),
        interfaces: kids.map(|c| c.interfaces.clone()).unwrap_or_default(),
        ports: kids.map(|c| c.ports.clone()).unwrap_or_default(),
    }
}

fn host_summary(h: &Host) -> HostSummary {
    let label = h
        .base
        .hostname
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| h.base.name.clone());
    HostSummary { id: h.id, label }
}

fn port_summary(p: &Port, status: EntityDigestStatus, is_fresh: bool) -> PortSummary {
    PortSummary {
        id: p.id,
        host_id: p.base.host_id,
        // Port's Display impl is `"{port_type} (ID: {id})"`. Drop the ID
        // suffix — recipients only want the human-readable port type.
        label: p.base.port_type.to_string(),
        status,
        is_fresh,
    }
}

fn service_summary(
    s: &NetworkServiceEntity,
    status: EntityDigestStatus,
    is_fresh: bool,
) -> ServiceSummary {
    let logo_url = {
        let url = s.base.service_definition.logo_url();
        if url.is_empty() {
            None
        } else {
            Some(url.to_string())
        }
    };
    let is_container = matches!(
        s.base.virtualization,
        Some(ServiceVirtualization::Docker(_))
    );
    ServiceSummary {
        id: s.id,
        host_id: s.base.host_id,
        name: s.base.name.clone(),
        is_container,
        logo_url,
        status,
        is_fresh,
    }
}

fn ip_summary(ip: &IPAddress, status: EntityDigestStatus, is_fresh: bool) -> IpAddressSummary {
    IpAddressSummary {
        id: ip.id,
        host_id: ip.base.host_id,
        address: ip.base.ip_address.to_string(),
        status,
        is_fresh,
    }
}

fn interface_summary(
    i: &Interface,
    status: EntityDigestStatus,
    is_fresh: bool,
) -> InterfaceSummary {
    // Interface's Display includes its UUID. For the digest we want only the
    // human-readable bits: the description if discovery provided one, else
    // the ifIndex.
    let label = if i.base.if_descr.is_empty() {
        format!("ifIndex {}", i.base.if_index)
    } else {
        i.base.if_descr.clone()
    };
    InterfaceSummary {
        id: i.id,
        host_id: i.base.host_id,
        label,
        status,
        is_fresh,
    }
}

fn subnet_summary(s: &Subnet) -> SubnetSummary {
    SubnetSummary {
        id: s.id,
        label: s.base.name.clone(),
    }
}

fn vlan_summary(v: &Vlan) -> VlanSummary {
    VlanSummary {
        id: v.id,
        vlan_number: v.base.vlan_number,
        name: v.base.name.clone(),
    }
}
