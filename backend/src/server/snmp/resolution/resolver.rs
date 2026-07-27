//! LLDP resolution trait and implementation.
//!
//! This module provides:
//! - `LldpResolver` trait for LLDP neighbor resolution database lookups
//! - `LldpResolverImpl` production implementation using database services

use std::net::IpAddr;
use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::server::{
    hosts::r#impl::base::Host,
    interfaces::{r#impl::base::Interface, service::InterfaceService},
    ip_addresses::{r#impl::base::IPAddress, service::IPAddressService},
    shared::{
        services::traits::CrudService,
        storage::{filter::StorableFilter, generic::GenericPostgresStorage, traits::Storage},
    },
};

/// Trait for LLDP resolution database lookups.
///
/// This trait abstracts database access for LLDP resolution, enabling:
/// - Dependency injection in the resolution methods on enums
/// - Easier testing with mock implementations
/// - Clean separation between LLDP types and database layer
///
/// Every lookup is scoped to live SCD2 rows (`valid_to IS NULL`). A neighbor resolved onto a
/// closed snapshot copy writes a `Neighbor::Interface` id the topology read path cannot see, so
/// the edge silently disappears — that is the `dangling` bucket in the L2 summary.
#[async_trait]
pub trait LldpResolver: Send + Sync {
    /// Find host by MAC address (via ip_addresses.mac_address).
    async fn find_host_by_mac(&self, mac: &str, network_id: Uuid) -> Option<Uuid>;

    /// Find host by IP address (via ip_addresses table).
    async fn find_host_by_ip(&self, ip: &IpAddr, network_id: Uuid) -> Option<Uuid>;

    /// Find host by interface name (via interfaces.if_descr).
    async fn find_host_by_if_name(&self, name: &str, network_id: Uuid) -> Option<Uuid>;

    /// Find host by chassis_id field on hosts table.
    ///
    /// Resolves only when exactly one host carries the identifier — see
    /// [`LldpResolver::find_host_by_sys_name`] for why the count matters.
    async fn find_host_by_chassis_id(&self, chassis_id: &str, network_id: Uuid) -> Option<Uuid>;

    /// Find host by sys_name field on hosts table.
    ///
    /// Resolves only when exactly one host in the network carries the name. SNMP `sysName` is
    /// operator-assigned and frequently left at a vendor default ("switch", "MikroTik"), so a
    /// first-match lookup would attach links to an arbitrary one of several identically named
    /// devices. Ambiguity is reported as "unresolved", not as a guess.
    async fn find_host_by_sys_name(&self, sys_name: &str, network_id: Uuid) -> Option<Uuid>;

    /// Find interface by MAC address.
    async fn find_if_entry_by_mac(&self, mac: &str, host_id: Uuid) -> Option<Uuid>;

    /// Find interface by name (if_descr or if_alias).
    async fn find_if_entry_by_name(&self, name: &str, host_id: Uuid) -> Option<Uuid>;

    /// Find interface by ifIndex on a known host.
    async fn find_if_entry_by_if_index(&self, if_index: i32, host_id: Uuid) -> Option<Uuid>;

    /// Find interface by IP address (via ip_address_id FK).
    async fn find_if_entry_by_ip(&self, ip: &IpAddr, host_id: Uuid) -> Option<Uuid>;
}

/// Production implementation of `LldpResolver`.
///
/// Uses database services to look up entities for LLDP neighbor resolution.
pub struct LldpResolverImpl {
    interface_service: Arc<InterfaceService>,
    ip_address_service: Arc<IPAddressService>,
    host_storage: Arc<GenericPostgresStorage<Host>>,
}

impl LldpResolverImpl {
    pub fn new(
        interface_service: Arc<InterfaceService>,
        ip_address_service: Arc<IPAddressService>,
        host_storage: Arc<GenericPostgresStorage<Host>>,
    ) -> Self {
        Self {
            interface_service,
            ip_address_service,
            host_storage,
        }
    }
}

#[async_trait]
impl LldpResolver for LldpResolverImpl {
    async fn find_host_by_mac(&self, mac: &str, network_id: Uuid) -> Option<Uuid> {
        let mac_addr: mac_address::MacAddress = mac.parse().ok()?;

        // Primary: Interface MAC (populated from ARP or SNMP ipAddrTable enrichment)
        let filter = StorableFilter::<IPAddress>::new_from_network_ids(&[network_id])
            .mac_address(&mac_addr)
            .live();
        if let Ok(Some(ip_address)) = self.ip_address_service.get_one(filter).await {
            return Some(ip_address.base.host_id);
        }

        // Fallback: Interface MAC (from SNMP ifPhysAddress, always present for SNMP hosts)
        let filter = StorableFilter::<Interface>::new_from_network_ids(&[network_id])
            .mac_address(&mac_addr)
            .live();
        let entry = self.interface_service.get_one(filter).await.ok()??;
        Some(entry.base.host_id)
    }

    async fn find_host_by_ip(&self, ip: &IpAddr, network_id: Uuid) -> Option<Uuid> {
        let filter = StorableFilter::<IPAddress>::new_from_network_ids(&[network_id])
            .ip_address(*ip)
            .live();
        let ip_address = self.ip_address_service.get_one(filter).await.ok()??;

        Some(ip_address.base.host_id)
    }

    async fn find_host_by_if_name(&self, name: &str, network_id: Uuid) -> Option<Uuid> {
        let filter = StorableFilter::<Interface>::new_from_network_ids(&[network_id])
            .if_descr(name)
            .live();
        let entry = self.interface_service.get_one(filter).await.ok()??;

        Some(entry.base.host_id)
    }

    async fn find_host_by_chassis_id(&self, chassis_id: &str, network_id: Uuid) -> Option<Uuid> {
        let filter = StorableFilter::<Host>::new_from_network_ids(&[network_id])
            .chassis_id(chassis_id)
            .live();
        let hosts = self.host_storage.get_all(filter).await.ok()?;

        only_match(hosts).map(|host| host.id)
    }

    async fn find_host_by_sys_name(&self, sys_name: &str, network_id: Uuid) -> Option<Uuid> {
        let filter = StorableFilter::<Host>::new_from_network_ids(&[network_id])
            .sys_name(sys_name)
            .live();
        let hosts = self.host_storage.get_all(filter).await.ok()?;

        only_match(hosts).map(|host| host.id)
    }

    async fn find_if_entry_by_mac(&self, mac: &str, host_id: Uuid) -> Option<Uuid> {
        // Parse MAC string to MacAddress type
        let mac_addr: mac_address::MacAddress = mac.parse().ok()?;

        // Find interface with this MAC on the specified host
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
            .mac_address(&mac_addr)
            .live();
        let entry = self.interface_service.get_one(filter).await.ok()??;

        Some(entry.id)
    }

    async fn find_if_entry_by_name(&self, name: &str, host_id: Uuid) -> Option<Uuid> {
        // Try if_descr first (long name: "GigabitEthernet1/0/1")
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
            .if_descr(name)
            .live();
        if let Ok(Some(entry)) = self.interface_service.get_one(filter).await {
            return Some(entry.id);
        }
        // Try if_name (short name: "Gi1/0/1")
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
            .if_name(name)
            .live();
        if let Ok(Some(entry)) = self.interface_service.get_one(filter).await {
            return Some(entry.id);
        }
        // Vendor quirk (MikroTik RouterOS): bridged ports advertise the port-ID as
        // "<bridge>/<port>" (e.g. "bridge-LAN/ether4-Center"), which never matches the
        // stored if_name/if_descr ("ether4-Center"). Retry with the segment after the
        // last '/' so port-level resolution still succeeds.
        if let Some((_, suffix)) = name.rsplit_once('/')
            && !suffix.is_empty()
        {
            let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
                .if_descr(suffix)
                .live();
            if let Ok(Some(entry)) = self.interface_service.get_one(filter).await {
                return Some(entry.id);
            }
            let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
                .if_name(suffix)
                .live();
            if let Ok(Some(entry)) = self.interface_service.get_one(filter).await {
                return Some(entry.id);
            }
        }
        None
    }

    async fn find_if_entry_by_if_index(&self, if_index: i32, host_id: Uuid) -> Option<Uuid> {
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
            .if_index(if_index)
            .live();
        let entry = self.interface_service.get_one(filter).await.ok()??;

        Some(entry.id)
    }

    async fn find_if_entry_by_ip(&self, ip: &IpAddr, host_id: Uuid) -> Option<Uuid> {
        // Find interface with this IP on the target host
        let filter = StorableFilter::<IPAddress>::new_from_host_ids(&[host_id])
            .ip_address(*ip)
            .live();
        let ip_address = self.ip_address_service.get_one(filter).await.ok()??;

        // Find Interface linked to this interface via ip_address_id FK
        let filter = StorableFilter::<Interface>::new_from_interface_id(&ip_address.id).live();
        let entry = self.interface_service.get_one(filter).await.ok()??;

        Some(entry.id)
    }
}

/// The single element of `matches`, or `None` when there were zero or more than one.
///
/// Used by the identity lookups whose column is not unique within a network (`chassis_id`,
/// `sys_name`). Two hosts sharing an operator-assigned name is a real configuration, and picking
/// an arbitrary one attaches physical links to the wrong device — worse than not resolving.
fn only_match<T>(mut matches: Vec<T>) -> Option<T> {
    match matches.len() {
        1 => matches.pop(),
        _ => None,
    }
}
