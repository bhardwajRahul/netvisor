use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    bindings::r#impl::base::{Binding, BindingType},
    credentials::service::CredentialService,
    daemons::{r#impl::base::Daemon, service::DaemonService},
    hosts::r#impl::{
        api::{
            BindingInput, ConflictBehavior, CreateHostRequest, HostResponse, IPAddressInput,
            PortInput, ServiceInput, UpdateHostRequest,
        },
        base::{Host, HostBase},
    },
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    ip_addresses::{r#impl::base::IPAddress, service::IPAddressService},
    ports::{r#impl::base::Port, service::PortService},
    services::{
        r#impl::{base::Service, definitions::ServiceDefinitionExt},
        service::ServiceService,
    },
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        events::{bus::EventBus, types::EntityOperation},
        position::resolve_and_validate_input_positions,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            lock::{CONSOLIDATE_LOCK_TIMEOUT, DEFAULT_LOCK_TIMEOUT, LockKey},
            traits::{Entity, PaginatedResult, Storable, Storage},
        },
        types::{
            api::ValidationError,
            entities::{EntitySource, EntitySourceDiscriminants},
        },
    },
    snmp::resolution::{lldp::LldpResolver, resolver::LldpResolverImpl},
    subnets::{r#impl::base::Subnet, service::SubnetService},
    tags::entity_tags::EntityTagService,
    vlans::service::VlanService,
};
use anyhow::{Error, Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    sync::Arc,
};
use strum::IntoDiscriminant;
use uuid::Uuid;

pub struct HostLimitContext {
    pub limit: u64,
    pub org_id: Uuid,
    pub org_network_ids: Vec<Uuid>,
    pub plan: crate::server::billing::types::base::BillingPlan,
}

pub struct HostService {
    storage: Arc<GenericPostgresStorage<Host>>,
    ip_address_service: Arc<IPAddressService>,
    port_service: Arc<PortService>,
    service_service: Arc<ServiceService>,
    interface_service: Arc<InterfaceService>,
    pub daemon_service: Arc<DaemonService>,
    credential_service: Arc<CredentialService>,
    subnet_service: Arc<SubnetService>,
    vlan_service: Arc<VlanService>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<Host> for HostService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Host) -> Option<Uuid> {
        Some(entity.base.network_id)
    }
    fn get_organization_id(&self, _entity: &Host) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Host> for HostService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Host>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }

    /// Create a new host, or upsert if a matching host exists.
    ///
    /// This method uses `Host::eq` (ID comparison) to find existing hosts.
    /// For discovery workflows, `create_with_children` sets the incoming host's ID
    /// to match an existing host found via IP-address comparison, so this method
    /// will find the match and trigger `upsert_host()`.
    ///
    /// Upsert conditions:
    /// - Both hosts are from discovery (merges discovery metadata)
    /// - OR the IDs already match (handles re-discovery of known hosts)
    async fn create(&self, host: Host, authentication: AuthenticatedEntity) -> Result<Host> {
        // DB-level lock, scoped to the network: serializes the dedup in
        // `create_unlocked` across all backend instances. Keyed by network
        // (not host id) because two concurrent submissions of the same NEW
        // device carry distinct fresh UUIDs. Error paths release via Drop.
        //
        // NOTE: this ID-based dedup alone cannot catch two fresh-UUID
        // submissions of the same physical device — the IP/MAC natural-key
        // match lives in `create_with_children`, which holds this same lock
        // across match + create + IP insertion and therefore calls
        // `create_unlocked` directly.
        let dedup_guard = self
            .storage()
            .session_lock(
                LockKey::HostDedup {
                    network_id: host.base.network_id,
                },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?;
        let created = self.create_unlocked(host, authentication).await?;
        dedup_guard.release().await?;
        Ok(created)
    }

    async fn update(
        &self,
        updates: &mut Host,
        authentication: AuthenticatedEntity,
    ) -> Result<Host, Error> {
        let lock_guard = self
            .storage()
            .session_lock(LockKey::Host(updates.id), DEFAULT_LOCK_TIMEOUT)
            .await?;

        let current_host = self
            .get_by_id(&updates.id)
            .await?
            .ok_or_else(|| anyhow!("Host '{}' not found", updates.id))?;

        let updated = self.storage().update(updates).await?;
        let trigger_stale = updated.triggers_staleness(Some(current_host));

        if let Some(scope) = EntityScope::from_ids(
            updated.id(),
            updated.clone().into(),
            self.get_network_id(&updated),
            self.get_organization_id(&updated),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Updated, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        lock_guard.release().await?;
        Ok(updated)
    }
}

mod consolidate;
mod create;
mod delete;
mod discovery;
mod lifecycle;
mod topology;
mod update;

/// Statistics from LLDP link resolution.
#[derive(Default, Debug)]
pub struct LldpResolutionStats {
    /// Total number of interfaces with unresolved LLDP data
    pub total: usize,
    /// Number of interfaces where remote host was resolved
    pub hosts_resolved: usize,
    /// Number of interfaces where remote port (interface) was resolved
    pub ports_resolved: usize,
}

/// Check whether a claimer's `(port_id, ip_address_id)` overlaps with an
/// Open Ports binding's `(port_id, ip_address_id)`.
/// Uses the same semantics as `partition_conflicting_bindings`:
/// None (all ip_addresses) overlaps with anything, Some(a) overlaps Some(a).
fn bindings_overlap(claim_iface: &Option<Uuid>, op_iface: &Option<Uuid>) -> bool {
    match (claim_iface, op_iface) {
        (None, _) | (_, None) => true,
        (Some(a), Some(b)) => a == b,
    }
}

/// Detect VRRP/HSRP virtual router MAC addresses by their well-known prefixes.
///
/// Virtual router protocols assign deterministic MACs shared across physical router peers.
/// These must be excluded from host identity matching to prevent different physical routers
/// in the same redundancy group from being deduped into a single host.
///
/// The VRRP/HSRP group ID is encoded in the last byte(s) of the MAC itself, so detection
/// requires only the MAC prefix — no SNMP MIB query needed.
fn is_virtual_router_mac(mac: &MacAddress) -> bool {
    let bytes = mac.bytes();
    // VRRP (RFC 5798): 00:00:5e:00:01:XX where XX = VRRP group ID (0-255)
    (bytes[0..5] == [0x00, 0x00, 0x5e, 0x00, 0x01])
    // HSRP v1 (Cisco): 00:00:0c:07:ac:XX where XX = HSRP group ID (0-255)
    || (bytes[0..5] == [0x00, 0x00, 0x0c, 0x07, 0xac])
    // HSRP v2 (Cisco): 00:00:0c:9f:fX:XX where X:XX = HSRP group ID (0-4095)
    || (bytes[0..4] == [0x00, 0x00, 0x0c, 0x9f] && (bytes[4] & 0xf0) == 0xf0)
}

/// Compare two ip_addresses for host dedup matching.
///
/// Three match branches, checked in order:
/// 1. **IP+subnet** (primary): same IP on the same subnet = same logical interface
/// 2. **ID** (secondary): same non-nil database UUID = known same record
/// 3. **MAC** (tertiary, conditional): same MAC address, but only when the MAC is unique
///    among incoming ip_addresses (count == 1). Shared MACs (count > 1) indicate VLAN
///    sub-interfaces, bridge members, or bond members — distinct ip_addresses that must
///    not be collapsed. Unique MACs indicate a standalone interface (e.g., a Docker
///    container whose IP changed via DHCP) where MAC is a valid identity anchor.
fn ip_addresses_match(
    incoming: &IPAddress,
    existing: &IPAddress,
    incoming_mac_counts: &HashMap<MacAddress, usize>,
) -> bool {
    // Primary: same IP on same subnet
    (incoming.base.ip_address == existing.base.ip_address
        && incoming.base.subnet_id == existing.base.subnet_id)
    // Secondary: same non-nil ID
    || (incoming.id == existing.id
        && incoming.id != Uuid::nil()
        && existing.id != Uuid::nil())
    // Tertiary: MAC match, gated on incoming MAC uniqueness
    || (incoming.base.mac_address.is_some()
        && incoming.base.mac_address == existing.base.mac_address
        && incoming
            .base
            .mac_address
            .map(|mac| incoming_mac_counts.get(&mac).copied().unwrap_or(0) == 1)
            .unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::IPAddressBase;

    fn make_interface(ip: IpAddr, subnet_id: Uuid, mac: Option<MacAddress>) -> IPAddress {
        IPAddress {
            id: Uuid::new_v4(),
            base: IPAddressBase {
                ip_address: ip,
                subnet_id,
                mac_address: mac,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    // --- is_virtual_router_mac tests ---

    #[test]
    fn vrrp_mac_detected() {
        // VRRP (RFC 5798): 00:00:5e:00:01:XX
        let mac = MacAddress::new([0x00, 0x00, 0x5e, 0x00, 0x01, 0x01]);
        assert!(is_virtual_router_mac(&mac), "VRRP MAC should be detected");
    }

    #[test]
    fn hsrp_v1_mac_detected() {
        // HSRP v1: 00:00:0c:07:ac:XX
        let mac = MacAddress::new([0x00, 0x00, 0x0c, 0x07, 0xac, 0x0a]);
        assert!(
            is_virtual_router_mac(&mac),
            "HSRP v1 MAC should be detected"
        );
    }

    #[test]
    fn hsrp_v2_mac_detected() {
        // HSRP v2: 00:00:0c:9f:fX:XX
        let mac = MacAddress::new([0x00, 0x00, 0x0c, 0x9f, 0xf0, 0x0a]);
        assert!(
            is_virtual_router_mac(&mac),
            "HSRP v2 MAC should be detected"
        );
    }

    #[test]
    fn normal_mac_not_virtual_router() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        assert!(
            !is_virtual_router_mac(&mac),
            "Regular MAC should not be detected as virtual router"
        );
    }

    // --- ip_addresses_match tests ---

    #[test]
    fn match_by_ip_subnet() {
        let subnet = Uuid::new_v4();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let a = make_interface(ip, subnet, None);
        let b = make_interface(ip, subnet, None);
        let counts = HashMap::new();
        assert!(ip_addresses_match(&a, &b, &counts));
    }

    #[test]
    fn no_match_different_ip_subnet() {
        let a = make_interface("10.0.0.1".parse().unwrap(), Uuid::new_v4(), None);
        let b = make_interface("20.0.0.1".parse().unwrap(), Uuid::new_v4(), None);
        let counts = HashMap::new();
        assert!(!ip_addresses_match(&a, &b, &counts));
    }

    #[test]
    fn mac_match_when_unique_in_batch() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let a = make_interface("10.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let b = make_interface("20.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        // MAC appears only once in the incoming batch — standalone ip_address, safe to match
        let counts = HashMap::from([(mac, 1)]);
        assert!(
            ip_addresses_match(&a, &b, &counts),
            "Unique MAC in batch should allow MAC matching (Docker/DHCP case)"
        );
    }

    #[test]
    fn mac_no_match_when_shared_in_batch() {
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let a = make_interface("10.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let b = make_interface("20.0.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        // MAC appears 3 times in the incoming batch — VLAN sub-interfaces, must not match
        let counts = HashMap::from([(mac, 3)]);
        assert!(
            !ip_addresses_match(&a, &b, &counts),
            "Shared MAC in batch (VLANs) must not match"
        );
    }

    // --- #600 characterization: multi-IP host on a single MAC ---
    //
    // These document the *actual* dedup decision for a host that responds on multiple
    // IP addresses behind one MAC (multi-homed server, IP aliases). They are evidence,
    // not a fix: they pin down what the current heuristic does so we can tell whether
    // the reported "tied to lowest IP" behavior is a real defect or expected dedup.

    /// Mirror how `find_matching_host_by_ip_addresses` builds `incoming_mac_counts`:
    /// the count is computed from the IP addresses of a *single incoming payload*.
    fn mac_counts_for_payload(payload: &[IPAddress]) -> HashMap<MacAddress, usize> {
        payload
            .iter()
            .filter_map(|i| i.base.mac_address)
            .fold(HashMap::new(), |mut acc, mac| {
                *acc.entry(mac).or_insert(0) += 1;
                acc
            })
    }

    #[test]
    fn mac_match_same_subnet_when_unique_in_batch() {
        // Multi-homed / IP-alias case: two IPs on the SAME subnet sharing one MAC.
        // Primary (ip+subnet) branch fails (different IP); MAC branch (count==1) must
        // carry the match so both IPs collapse onto one host.
        let subnet = Uuid::new_v4();
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x01]);
        let a = make_interface("192.168.1.10".parse().unwrap(), subnet, Some(mac));
        let b = make_interface("192.168.1.11".parse().unwrap(), subnet, Some(mac));
        let counts = HashMap::from([(mac, 1)]);
        assert!(
            ip_addresses_match(&a, &b, &counts),
            "Same MAC + same subnet, unique in batch, should match (IP-alias multi-homing)"
        );
    }

    #[test]
    fn multi_homed_host_separate_payloads_merge() {
        // The real discovery flow: each scanned IP arrives as its own single-IP payload,
        // so `incoming_mac_counts` for that payload is always {MAC: 1}. Therefore the
        // VLAN guard never trips across payloads, and the second IP merges into the host
        // created by the first — one Host, multiple IPAddress children. This is the
        // model-(a) outcome, contradicting "a separate host per IP".
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x02]);

        // Payload 1: first IP (different subnet to exercise the MAC branch, not ip+subnet)
        let first = make_interface("192.168.1.50".parse().unwrap(), Uuid::new_v4(), Some(mac));
        // Payload 2: second IP, scanned independently
        let second = make_interface("10.0.0.50".parse().unwrap(), Uuid::new_v4(), Some(mac));

        // Counts are per-payload; each single-IP payload yields {mac: 1}.
        let counts_for_second = mac_counts_for_payload(std::slice::from_ref(&second));
        assert_eq!(counts_for_second.get(&mac), Some(&1));

        assert!(
            ip_addresses_match(&second, &first, &counts_for_second),
            "Independently scanned same-MAC IPs must merge into one host (model a)"
        );
    }

    #[test]
    fn multi_ip_single_payload_does_not_mac_merge() {
        // Contrast: when one payload carries BOTH same-MAC IPs (count==2), the MAC branch
        // is intentionally disabled (treated as VLAN/bridge sub-interfaces). They are not
        // merged *via MAC*; they remain distinct IPAddress rows under whatever host owns
        // them. Guards the existing behavior so a future fix can't regress it silently.
        let mac = MacAddress::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0x03]);
        let a = make_interface("172.16.0.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let b = make_interface("172.16.5.1".parse().unwrap(), Uuid::new_v4(), Some(mac));
        let counts = mac_counts_for_payload(&[a.clone(), b.clone()]);
        assert_eq!(counts.get(&mac), Some(&2));
        assert!(
            !ip_addresses_match(&a, &b, &counts),
            "Two same-MAC IPs in one payload must not MAC-merge (VLAN sub-interface guard)"
        );
    }
}
