use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::bindings::service::BindingService;
use crate::server::daemons::r#impl::api::{DiscoveryUpdatePayload, ScannedEntityIds};
use crate::server::services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt};
use crate::server::services::r#impl::virtualization::ServiceVirtualization;
use crate::server::{
    digest::payload::{
        AffectedHostCard, DigestRecipient, DiscoveryDigestFlags, DiscoveryDigestOperation,
        DiscoveryDigestPayload, DiscoveryDigestScope, HostCardStatus, HostSummary,
        InterfaceSummary, IpAddressSummary, PortSummary, ServiceSummary, SubnetSummary, TagStatus,
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

        // Subnets scanned: trust the daemon's reported set. Empty for
        // host-scoped discoveries (SelfReport / Docker) is the correct
        // answer — those don't sweep a subnet.
        let subnets_scanned: Vec<SubnetSummary> = if scanned.subnet_ids.is_empty() {
            Vec::new()
        } else {
            self.subnet_service
                .get_all(StorableFilter::<Subnet>::new_from_entity_ids(
                    &scanned.subnet_ids,
                ))
                .await?
                .iter()
                .map(subnet_summary)
                .collect()
        };

        // Three buckets of hosts the digest covers.
        let hosts_added_records: Vec<Host> = self
            .host_service
            .get_all(
                StorableFilter::<Host>::new_from_network_ids(&[network_id])
                    .live()
                    .created_between(t_start, t_end),
            )
            .await?;
        let hosts_vanished_records: Vec<Host> = self
            .host_service
            .get_all(
                StorableFilter::<Host>::new_from_network_ids(&[network_id])
                    .live()
                    .created_before(t_start)
                    .last_seen_before(t_start),
            )
            .await?;
        let hosts_scanned_records: Vec<Host> = self
            .host_service
            .get_all(
                StorableFilter::<Host>::new_from_network_ids(&[network_id])
                    .live()
                    .created_before(t_start)
                    .last_seen_between(t_start, t_end),
            )
            .await?;

        // Union of all hosts that might appear in the digest. We hydrate
        // children for all of them in one pass; the per-tag `status` on each
        // child summary tells the renderer what changed.
        let mut affected_ids: Vec<Uuid> = Vec::with_capacity(
            hosts_added_records.len() + hosts_vanished_records.len() + hosts_scanned_records.len(),
        );
        affected_ids.extend(hosts_added_records.iter().map(|h| h.id));
        affected_ids.extend(hosts_vanished_records.iter().map(|h| h.id));
        affected_ids.extend(hosts_scanned_records.iter().map(|h| h.id));

        let vanished_host_ids: HashSet<Uuid> =
            hosts_vanished_records.iter().map(|h| h.id).collect();
        let current_children = self
            .fetch_current_children(&affected_ids, scanned, &vanished_host_ids, t_start, t_end)
            .await?;

        let hosts_added: Vec<AffectedHostCard> = hosts_added_records
            .iter()
            .map(|h| build_card(h, HostCardStatus::New, &current_children))
            .collect();
        let hosts_vanished: Vec<AffectedHostCard> = hosts_vanished_records
            .iter()
            .map(|h| build_card(h, HostCardStatus::Vanished, &current_children))
            .collect();

        // A scanned host is "Changed" only if at least one of its children
        // has a non-Unchanged status. Hosts with no real deltas are dropped
        // from the digest entirely.
        let mut hosts_changed: Vec<AffectedHostCard> = hosts_scanned_records
            .iter()
            .map(|h| build_card(h, HostCardStatus::Changed, &current_children))
            .filter(card_has_changes)
            .collect();
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
    /// host_id, with each child summary carrying its `TagStatus`:
    /// - `New` when `created_at` falls inside the scan window.
    /// - `Removed` when the daemon did not include the id in its scan set.
    /// - `Unchanged` otherwise.
    ///
    /// Children of vanished hosts always get `Unchanged` (we just enumerate
    /// what we know; the host card itself communicates the vanished state).
    /// Filters Unclaimed Open Ports out of the service list.
    async fn fetch_current_children(
        &self,
        host_ids: &[Uuid],
        scanned: &ScannedEntityIds,
        vanished_host_ids: &HashSet<Uuid>,
        t_start: DateTime<Utc>,
        t_end: DateTime<Utc>,
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
            let status = tag_status(
                s.base.host_id,
                s.id,
                s.created_at,
                &scanned_service_ids,
                vanished_host_ids,
                t_start,
                t_end,
            );
            out.entry(s.base.host_id)
                .or_default()
                .services
                .push(service_summary(s, status));
        }
        for ip in &ips {
            let status = tag_status(
                ip.base.host_id,
                ip.id,
                ip.created_at,
                &scanned_ip_ids,
                vanished_host_ids,
                t_start,
                t_end,
            );
            out.entry(ip.base.host_id)
                .or_default()
                .ip_addresses
                .push(ip_summary(ip, status));
        }
        for i in &interfaces {
            let status = tag_status(
                i.base.host_id,
                i.id,
                i.created_at,
                &scanned_interface_ids,
                vanished_host_ids,
                t_start,
                t_end,
            );
            out.entry(i.base.host_id)
                .or_default()
                .interfaces
                .push(interface_summary(i, status));
        }
        for p in &ports {
            let status = tag_status(
                p.base.host_id,
                p.id,
                p.created_at,
                &scanned_port_ids,
                vanished_host_ids,
                t_start,
                t_end,
            );
            out.entry(p.base.host_id)
                .or_default()
                .ports
                .push(port_summary(p, status));
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

/// Decide a child's `TagStatus`. Children of vanished hosts are flat
/// `Unchanged` — the host card itself communicates the vanished state, so
/// per-child markers would be redundant. For non-vanished hosts:
/// `created_at` inside the scan window ⇒ New; not in the daemon's scan
/// set ⇒ Removed; otherwise Unchanged.
fn tag_status(
    host_id: Uuid,
    child_id: Uuid,
    created_at: DateTime<Utc>,
    scanned_ids: &HashSet<Uuid>,
    vanished_host_ids: &HashSet<Uuid>,
    t_start: DateTime<Utc>,
    t_end: DateTime<Utc>,
) -> TagStatus {
    if vanished_host_ids.contains(&host_id) {
        return TagStatus::Unchanged;
    }
    if created_at >= t_start && created_at <= t_end {
        TagStatus::New
    } else if !scanned_ids.contains(&child_id) {
        TagStatus::Removed
    } else {
        TagStatus::Unchanged
    }
}

/// True when at least one child carries a non-`Unchanged` status — used to
/// drop noisy "Changed" host cards where nothing actually changed.
fn card_has_changes(card: &AffectedHostCard) -> bool {
    card.services
        .iter()
        .any(|s| s.status != TagStatus::Unchanged)
        || card
            .ip_addresses
            .iter()
            .any(|x| x.status != TagStatus::Unchanged)
        || card
            .interfaces
            .iter()
            .any(|i| i.status != TagStatus::Unchanged)
        || card.ports.iter().any(|p| p.status != TagStatus::Unchanged)
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
    status: HostCardStatus,
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

fn port_summary(p: &Port, status: TagStatus) -> PortSummary {
    PortSummary {
        id: p.id,
        host_id: p.base.host_id,
        // Port's Display impl is `"{port_type} (ID: {id})"`. Drop the ID
        // suffix — recipients only want the human-readable port type.
        label: p.base.port_type.to_string(),
        status,
    }
}

fn service_summary(s: &NetworkServiceEntity, status: TagStatus) -> ServiceSummary {
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
    }
}

fn ip_summary(ip: &IPAddress, status: TagStatus) -> IpAddressSummary {
    IpAddressSummary {
        id: ip.id,
        host_id: ip.base.host_id,
        address: ip.base.ip_address.to_string(),
        status,
    }
}

fn interface_summary(i: &Interface, status: TagStatus) -> InterfaceSummary {
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
