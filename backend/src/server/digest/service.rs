use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
use crate::server::{
    bindings::{r#impl::base::Binding, service::BindingService},
    digest::payload::{
        AffectedHostCard, BindingSummary, DigestRecipient, DiscoveryDigestFlags,
        DiscoveryDigestOperation, DiscoveryDigestPayload, DiscoveryDigestScope, HostCardStatus,
        HostDeltas, HostSummary, InterfaceSummary, IpAddressSummary, PortSummary, ServiceSummary,
        SubnetSummary, VlanSummary,
    },
    discovery::r#impl::types::DiscoveryType,
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
    pub async fn compute_and_publish(&self, payload: &DiscoveryUpdatePayload) -> Result<()> {
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
        t_start: DateTime<Utc>,
        t_end: DateTime<Utc>,
        network_name: &str,
        organization_id: Uuid,
    ) -> Result<DiscoveryDigestPayload> {
        let network_id = payload.network_id;

        // Subnets scanned: hydrate from session's discovery type when it
        // carries an explicit subnet list. Host-scoped variants
        // (SelfReport / Docker) leave the list empty — those scans don't
        // sweep a subnet.
        let subnet_ids = match &payload.discovery_type {
            DiscoveryType::Network { subnet_ids, .. }
            | DiscoveryType::Unified { subnet_ids, .. } => subnet_ids.clone().unwrap_or_default(),
            DiscoveryType::SelfReport { .. } | DiscoveryType::Docker { .. } => Vec::new(),
        };
        let subnets_scanned = if subnet_ids.is_empty() {
            Vec::new()
        } else {
            let subnets = self
                .subnet_service
                .get_all(StorableFilter::<Subnet>::new_from_entity_ids(&subnet_ids))
                .await?;
            subnets.iter().map(subnet_summary).collect()
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
            self.compute_deltas(&scanned_ids, t_start, t_end).await?
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
    async fn compute_deltas(
        &self,
        host_ids: &[Uuid],
        t_start: DateTime<Utc>,
        t_end: DateTime<Utc>,
    ) -> Result<HashMap<Uuid, HostDeltas>> {
        let ports_added: Vec<Port> = self
            .port_service
            .storage()
            .get_all(
                StorableFilter::<Port>::new()
                    .live()
                    .uuids_column("host_id", host_ids)
                    .created_between(t_start, t_end),
            )
            .await?;
        let ports_removed: Vec<Port> = self
            .port_service
            .storage()
            .get_all(
                StorableFilter::<Port>::new()
                    .live()
                    .uuids_column("host_id", host_ids)
                    .created_before(t_start)
                    .last_seen_before(t_start),
            )
            .await?;

        let services_added: Vec<NetworkServiceEntity> = self
            .service_service
            .storage()
            .get_all(
                StorableFilter::<NetworkServiceEntity>::new()
                    .live()
                    .uuids_column("host_id", host_ids)
                    .created_between(t_start, t_end),
            )
            .await?;
        let services_removed: Vec<NetworkServiceEntity> = self
            .service_service
            .storage()
            .get_all(
                StorableFilter::<NetworkServiceEntity>::new()
                    .live()
                    .uuids_column("host_id", host_ids)
                    .created_before(t_start)
                    .last_seen_before(t_start),
            )
            .await?;

        let ips_added: Vec<IPAddress> = self
            .ip_address_service
            .storage()
            .get_all(
                StorableFilter::<IPAddress>::new()
                    .live()
                    .uuids_column("host_id", host_ids)
                    .created_between(t_start, t_end),
            )
            .await?;
        let ips_removed: Vec<IPAddress> = self
            .ip_address_service
            .storage()
            .get_all(
                StorableFilter::<IPAddress>::new()
                    .live()
                    .uuids_column("host_id", host_ids)
                    .created_before(t_start)
                    .last_seen_before(t_start),
            )
            .await?;

        let interfaces_added: Vec<Interface> = self
            .interface_service
            .storage()
            .get_all(
                StorableFilter::<Interface>::new()
                    .live()
                    .uuids_column("host_id", host_ids)
                    .created_between(t_start, t_end),
            )
            .await?;
        let interfaces_removed: Vec<Interface> = self
            .interface_service
            .storage()
            .get_all(
                StorableFilter::<Interface>::new()
                    .live()
                    .uuids_column("host_id", host_ids)
                    .created_before(t_start)
                    .last_seen_before(t_start),
            )
            .await?;

        // Bindings have no host_id FK — they hang off services. Resolve via
        // the services on the scanned hosts (live), then filter bindings by
        // service_id IN (...).
        let scanned_services: Vec<NetworkServiceEntity> = self
            .service_service
            .storage()
            .get_all(
                StorableFilter::<NetworkServiceEntity>::new()
                    .live()
                    .uuids_column("host_id", host_ids),
            )
            .await?;
        let service_to_host: HashMap<Uuid, Uuid> = scanned_services
            .iter()
            .map(|s| (s.id, s.base.host_id))
            .collect();
        let scanned_service_ids: Vec<Uuid> = scanned_services.iter().map(|s| s.id).collect();

        let (bindings_added, bindings_removed) = if scanned_service_ids.is_empty() {
            (Vec::new(), Vec::new())
        } else {
            let added = self
                .binding_service
                .storage()
                .get_all(
                    StorableFilter::<Binding>::new()
                        .live()
                        .uuids_column("service_id", &scanned_service_ids)
                        .created_between(t_start, t_end),
                )
                .await?;
            let removed = self
                .binding_service
                .storage()
                .get_all(
                    StorableFilter::<Binding>::new()
                        .live()
                        .uuids_column("service_id", &scanned_service_ids)
                        .created_before(t_start)
                        .last_seen_before(t_start),
                )
                .await?;
            (added, removed)
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
        label: p.to_string(),
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
    InterfaceSummary {
        id: i.id,
        host_id: i.base.host_id,
        label: i.to_string(),
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
