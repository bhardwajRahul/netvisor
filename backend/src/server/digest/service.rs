use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::daemons::r#impl::api::{DiscoveryUpdatePayload, ScannedEntityIds};
use crate::server::{
    bindings::{r#impl::base::Binding, service::BindingService},
    digest::payload::{
        AffectedHostCard, BindingSummary, DigestRecipient, DiscoveryDigestFlags,
        DiscoveryDigestOperation, DiscoveryDigestPayload, DiscoveryDigestScope, HostCardStatus,
        HostDeltas, HostSummary, InterfaceSummary, IpAddressSummary, PortSummary, ServiceSummary,
        SubnetSummary, VlanSummary,
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

        // Per-host deltas for the scanned set (only changed hosts will be
        // surfaced — empty deltas are filtered downstream).
        let scanned_ids: Vec<Uuid> = hosts_scanned_records.iter().map(|h| h.id).collect();
        let scanned_deltas: HashMap<Uuid, HostDeltas> = if scanned_ids.is_empty() {
            HashMap::new()
        } else {
            self.compute_deltas(&scanned_ids, scanned, t_start, t_end)
                .await?
        };
        let changed_records: Vec<&Host> = hosts_scanned_records
            .iter()
            .filter(|h| scanned_deltas.get(&h.id).is_some_and(|d| !d.is_empty()))
            .collect();

        // Union of all affected host ids; hydrate current children once for
        // the whole set.
        let mut affected_ids: Vec<Uuid> = Vec::with_capacity(
            hosts_added_records.len() + hosts_vanished_records.len() + changed_records.len(),
        );
        affected_ids.extend(hosts_added_records.iter().map(|h| h.id));
        affected_ids.extend(hosts_vanished_records.iter().map(|h| h.id));
        affected_ids.extend(changed_records.iter().map(|h| h.id));

        let current_children = self.fetch_current_children(&affected_ids).await?;

        let hosts_added: Vec<AffectedHostCard> = hosts_added_records
            .iter()
            .map(|h| build_card(h, HostCardStatus::New, &current_children, None))
            .collect();
        let hosts_vanished: Vec<AffectedHostCard> = hosts_vanished_records
            .iter()
            .map(|h| build_card(h, HostCardStatus::Vanished, &current_children, None))
            .collect();
        let mut hosts_changed: Vec<AffectedHostCard> = changed_records
            .iter()
            .map(|h| {
                let deltas = scanned_deltas.get(&h.id).cloned();
                build_card(h, HostCardStatus::Changed, &current_children, deltas)
            })
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

    /// Compute per-host added/removed children for the scanned host set.
    /// Result is keyed by host_id; absent keys had zero deltas.
    ///
    /// "Removed" is detected via set membership against `ScannedEntityIds`
    /// — a child is removed iff it's currently live on a scanned host but
    /// its id is NOT in the daemon's reported scan set. This sidesteps the
    /// foundation-worker reconciliation bug where child `last_seen_at` is
    /// not refreshed on natural-key match (so timestamp-based detection
    /// would mark every pre-existing child as removed).
    async fn compute_deltas(
        &self,
        host_ids: &[Uuid],
        scanned: &ScannedEntityIds,
        t_start: DateTime<Utc>,
        t_end: DateTime<Utc>,
    ) -> Result<HashMap<Uuid, HostDeltas>> {
        // All live children on the scanned hosts — one query per kind.
        let all_ports: Vec<Port> = self
            .port_service
            .storage()
            .get_all(
                StorableFilter::<Port>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;
        let all_services: Vec<NetworkServiceEntity> = self
            .service_service
            .storage()
            .get_all(
                StorableFilter::<NetworkServiceEntity>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;
        let all_ips: Vec<IPAddress> = self
            .ip_address_service
            .storage()
            .get_all(
                StorableFilter::<IPAddress>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;
        let all_interfaces: Vec<Interface> = self
            .interface_service
            .storage()
            .get_all(
                StorableFilter::<Interface>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;

        // Bindings hang off services (no host_id FK). Resolve host via the
        // live services already loaded above — capture before moving the
        // service vec into the partition.
        let service_to_host: HashMap<Uuid, Uuid> = all_services
            .iter()
            .map(|s| (s.id, s.base.host_id))
            .collect();
        let scanned_service_uuids: Vec<Uuid> = all_services.iter().map(|s| s.id).collect();

        // Partition each set into "added in this window" vs "removed from a
        // scanned host" (live, present on scanned host, not in scan).
        let scanned_port_ids: HashSet<Uuid> = scanned.port_ids.iter().copied().collect();
        let scanned_service_ids: HashSet<Uuid> = scanned.service_ids.iter().copied().collect();
        let scanned_ip_ids: HashSet<Uuid> = scanned.ip_address_ids.iter().copied().collect();
        let scanned_interface_ids: HashSet<Uuid> = scanned.interface_ids.iter().copied().collect();
        let scanned_binding_ids: HashSet<Uuid> = scanned.binding_ids.iter().copied().collect();

        let (ports_added, ports_removed) =
            partition_by_scan(all_ports, &scanned_port_ids, t_start, t_end, |p| {
                p.created_at
            });
        let (services_added, services_removed) =
            partition_by_scan(all_services, &scanned_service_ids, t_start, t_end, |s| {
                s.created_at
            });
        let (ips_added, ips_removed) =
            partition_by_scan(all_ips, &scanned_ip_ids, t_start, t_end, |ip| ip.created_at);
        let (interfaces_added, interfaces_removed) = partition_by_scan(
            all_interfaces,
            &scanned_interface_ids,
            t_start,
            t_end,
            |i| i.created_at,
        );

        let (bindings_added, bindings_removed) = if scanned_service_uuids.is_empty() {
            (Vec::new(), Vec::new())
        } else {
            let all_bindings: Vec<Binding> = self
                .binding_service
                .storage()
                .get_all(
                    StorableFilter::<Binding>::new()
                        .live()
                        .uuids_column("service_id", &scanned_service_uuids),
                )
                .await?;
            partition_by_scan(all_bindings, &scanned_binding_ids, t_start, t_end, |b| {
                b.created_at
            })
        };

        let mut by_host: HashMap<Uuid, HostDeltas> = HashMap::new();
        for p in &ports_added {
            by_host
                .entry(p.base.host_id)
                .or_default()
                .ports_added
                .push(port_summary(p));
        }
        for p in &ports_removed {
            by_host
                .entry(p.base.host_id)
                .or_default()
                .ports_removed
                .push(port_summary(p));
        }
        for s in &services_added {
            by_host
                .entry(s.base.host_id)
                .or_default()
                .services_added
                .push(service_summary(s));
        }
        for s in &services_removed {
            by_host
                .entry(s.base.host_id)
                .or_default()
                .services_removed
                .push(service_summary(s));
        }
        for ip in &ips_added {
            by_host
                .entry(ip.base.host_id)
                .or_default()
                .ip_addresses_added
                .push(ip_summary(ip));
        }
        for ip in &ips_removed {
            by_host
                .entry(ip.base.host_id)
                .or_default()
                .ip_addresses_removed
                .push(ip_summary(ip));
        }
        for i in &interfaces_added {
            by_host
                .entry(i.base.host_id)
                .or_default()
                .interfaces_added
                .push(interface_summary(i));
        }
        for i in &interfaces_removed {
            by_host
                .entry(i.base.host_id)
                .or_default()
                .interfaces_removed
                .push(interface_summary(i));
        }
        for b in &bindings_added {
            if let Some(host_id) = service_to_host.get(&b.base.service_id) {
                by_host
                    .entry(*host_id)
                    .or_default()
                    .bindings_added
                    .push(binding_summary(b, *host_id));
            }
        }
        for b in &bindings_removed {
            if let Some(host_id) = service_to_host.get(&b.base.service_id) {
                by_host
                    .entry(*host_id)
                    .or_default()
                    .bindings_removed
                    .push(binding_summary(b, *host_id));
            }
        }

        Ok(by_host)
    }

    /// Fetch the live current children for each affected host id and bucket
    /// them by host. Powers the per-host card rendering — recipients see the
    /// host's full footprint, not just deltas.
    async fn fetch_current_children(
        &self,
        host_ids: &[Uuid],
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

        let mut out: HashMap<Uuid, HostChildren> = HashMap::new();
        for id in host_ids {
            out.entry(*id).or_default();
        }
        for s in &services {
            out.entry(s.base.host_id)
                .or_default()
                .services
                .push(service_summary(s));
        }
        for ip in &ips {
            out.entry(ip.base.host_id)
                .or_default()
                .ip_addresses
                .push(ip_summary(ip));
        }
        for i in &interfaces {
            out.entry(i.base.host_id)
                .or_default()
                .interfaces
                .push(interface_summary(i));
        }
        for p in &ports {
            out.entry(p.base.host_id)
                .or_default()
                .ports
                .push(port_summary(p));
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

/// Partition a vec of entities into `(added, removed)` for digest deltas:
/// - **added**: created during `[start, end]`. Authoritative signal.
/// - **removed**: id NOT in the scan's reported set. Sidesteps the
///   foundation-worker reconciliation bug where existing children don't
///   get `last_seen_at` refreshed on natural-key match.
fn partition_by_scan<T>(
    items: Vec<T>,
    scanned_ids: &HashSet<Uuid>,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    created_at: impl Fn(&T) -> DateTime<Utc>,
) -> (Vec<T>, Vec<T>)
where
    T: Identified,
{
    let mut added = Vec::new();
    let mut removed = Vec::new();
    for item in items {
        let ts = created_at(&item);
        if ts >= start && ts <= end {
            added.push(item);
        } else if !scanned_ids.contains(&item.id()) {
            removed.push(item);
        }
    }
    (added, removed)
}

trait Identified {
    fn id(&self) -> Uuid;
}

impl Identified for Port {
    fn id(&self) -> Uuid {
        self.id
    }
}
impl Identified for NetworkServiceEntity {
    fn id(&self) -> Uuid {
        self.id
    }
}
impl Identified for IPAddress {
    fn id(&self) -> Uuid {
        self.id
    }
}
impl Identified for Interface {
    fn id(&self) -> Uuid {
        self.id
    }
}
impl Identified for Binding {
    fn id(&self) -> Uuid {
        self.id
    }
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
    deltas: Option<HostDeltas>,
) -> AffectedHostCard {
    let kids = children.get(&host.id);
    AffectedHostCard {
        host: host_summary(host),
        status,
        services: kids.map(|c| c.services.clone()).unwrap_or_default(),
        ip_addresses: kids.map(|c| c.ip_addresses.clone()).unwrap_or_default(),
        interfaces: kids.map(|c| c.interfaces.clone()).unwrap_or_default(),
        ports: kids.map(|c| c.ports.clone()).unwrap_or_default(),
        deltas,
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

fn port_summary(p: &Port) -> PortSummary {
    PortSummary {
        id: p.id,
        host_id: p.base.host_id,
        // Port's Display impl is `"{port_type} (ID: {id})"`. Drop the ID
        // suffix — recipients only want the human-readable port type.
        label: p.base.port_type.to_string(),
    }
}

fn service_summary(s: &NetworkServiceEntity) -> ServiceSummary {
    ServiceSummary {
        id: s.id,
        host_id: s.base.host_id,
        name: s.base.name.clone(),
    }
}

fn ip_summary(ip: &IPAddress) -> IpAddressSummary {
    IpAddressSummary {
        id: ip.id,
        host_id: ip.base.host_id,
        address: ip.base.ip_address.to_string(),
    }
}

fn interface_summary(i: &Interface) -> InterfaceSummary {
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
    }
}

fn binding_summary(b: &Binding, host_id: Uuid) -> BindingSummary {
    BindingSummary {
        id: b.id,
        host_id,
        label: b.to_string(),
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
