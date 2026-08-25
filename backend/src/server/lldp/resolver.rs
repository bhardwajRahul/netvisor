//! LLDP resolution trait and implementation.
//!
//! This module provides:
//! - `LldpResolver` trait for LLDP neighbor resolution database lookups
//! - `LldpResolverImpl` production implementation using database services

use std::collections::HashSet;
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
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Storage, Unique},
        },
    },
};

use super::IdentityResolution;

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
///
/// # A MAC identifies a port only when it is unique within the device
///
/// `ifPhysAddress` is not required to differ between a device's ports, and on a large family of
/// switches it does not: the chassis base MAC is reported on every port (GH #668). Every lookup
/// below whose column is non-unique therefore resolves only on a single match — enforced by the
/// storage layer itself through [`Unique`], and reported as [`IdentityResolution::Ambiguous`]
/// where the caller can act on the distinction. Picking an arbitrary row instead does not
/// produce a missing link, it produces a
/// *wrong* one: a port-precise edge drawn to whichever port the database happened to return first,
/// which reads as authoritative.
#[async_trait]
pub trait LldpResolver: Send + Sync {
    /// Find host by MAC address (via ip_addresses.mac_address, then interfaces.mac_address).
    ///
    /// Many interfaces on one host may legitimately carry the MAC — that is one host, and it
    /// resolves. Two *different* hosts carrying it is a duplicate this cannot choose between, and
    /// resolves to nothing so the caller's later tiers get their turn.
    async fn find_host_by_mac(&self, mac: &str, network_id: Uuid) -> IdentityResolution;

    /// Find host by IP address (via ip_addresses table).
    async fn find_host_by_ip(&self, ip: &IpAddr, network_id: Uuid) -> IdentityResolution;

    /// Find host by interface name (via interfaces.if_descr).
    async fn find_host_by_if_name(&self, name: &str, network_id: Uuid) -> IdentityResolution;

    /// Find host by chassis_id field on hosts table.
    ///
    /// Resolves only when exactly one host carries the identifier — see
    /// [`LldpResolver::find_host_by_sys_name`] for why the count matters.
    async fn find_host_by_chassis_id(
        &self,
        chassis_id: &str,
        network_id: Uuid,
    ) -> IdentityResolution;

    /// Find host by sys_name field on hosts table.
    ///
    /// Resolves only when exactly one host in the network carries the name. SNMP `sysName` is
    /// operator-assigned and frequently left at a vendor default ("switch", "MikroTik"), so a
    /// first-match lookup would attach links to an arbitrary one of several identically named
    /// devices. Ambiguity is reported as "unresolved", not as a guess.
    async fn find_host_by_sys_name(&self, sys_name: &str, network_id: Uuid) -> IdentityResolution;

    /// Find the one interface on `host_id` carrying this MAC.
    ///
    /// Reports [`IdentityResolution::Ambiguous`] rather than choosing when the device repeats the
    /// MAC across several *physical* ports, so the neighbour degrades to a device-level edge and
    /// the reason reaches the resolution summary. Virtual interfaces are not candidate far ends
    /// and do not contest the lookup — see `if_type::EXCLUDED_IF_TYPES`.
    async fn find_if_entry_by_mac(&self, mac: &str, host_id: Uuid) -> IdentityResolution;

    /// Find the one interface on `host_id` whose `ifDescr`, `ifName` or `ifAlias` is this name.
    ///
    /// All three columns are non-unique, so each is resolved on a single match only.
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
    async fn find_host_by_mac(&self, mac: &str, network_id: Uuid) -> IdentityResolution {
        let Ok(mac_addr) = mac.parse::<mac_address::MacAddress>() else {
            return IdentityResolution::NotFound;
        };

        // Primary: Interface MAC (populated from ARP or SNMP ipAddrTable enrichment)
        let filter = StorableFilter::<IPAddress>::new_from_network_ids(&[network_id])
            .mac_address(&mac_addr)
            .live();
        if let Ok(Unique::One(ip_address)) = self.ip_address_service.get_unique(filter).await {
            return IdentityResolution::Resolved(ip_address.base.host_id);
        }

        // Fallback: Interface MAC (from SNMP ifPhysAddress, always present for SNMP hosts).
        //
        // Collapsed to distinct hosts before the single-match rule is applied: a switch reporting
        // its chassis MAC on all 48 ports returns 48 rows and one host, and that host is the
        // answer. Only rows spanning more than one host are genuinely ambiguous.
        let filter = StorableFilter::<Interface>::new_from_network_ids(&[network_id])
            .mac_address(&mac_addr)
            .live();
        let Ok(entries) = self.interface_service.get_all(filter).await else {
            return IdentityResolution::NotFound;
        };
        let host_ids: HashSet<Uuid> = entries.iter().map(|e| e.base.host_id).collect();

        match host_ids.len() {
            0 => IdentityResolution::NotFound,
            1 => IdentityResolution::Resolved(host_ids.into_iter().next().unwrap_or_default()),
            _ => IdentityResolution::Ambiguous,
        }
    }

    async fn find_host_by_ip(&self, ip: &IpAddr, network_id: Uuid) -> IdentityResolution {
        let filter = StorableFilter::<IPAddress>::new_from_network_ids(&[network_id])
            .ip_address(*ip)
            .live();
        let Ok(found) = self.ip_address_service.get_unique(filter).await else {
            return IdentityResolution::NotFound;
        };

        IdentityResolution::from_unique(found.map(|ip| ip.base.host_id))
    }

    async fn find_host_by_if_name(&self, name: &str, network_id: Uuid) -> IdentityResolution {
        let filter = StorableFilter::<Interface>::new_from_network_ids(&[network_id])
            .if_descr(name)
            .live();
        let Ok(found) = self.interface_service.get_unique(filter).await else {
            return IdentityResolution::NotFound;
        };

        IdentityResolution::from_unique(found.map(|entry| entry.base.host_id))
    }

    async fn find_host_by_chassis_id(
        &self,
        chassis_id: &str,
        network_id: Uuid,
    ) -> IdentityResolution {
        let filter = StorableFilter::<Host>::new_from_network_ids(&[network_id])
            .chassis_id(chassis_id)
            .live();

        let Ok(found) = self.host_storage.get_unique(filter).await else {
            return IdentityResolution::NotFound;
        };

        IdentityResolution::from_unique(found.map(|host| host.id))
    }

    async fn find_host_by_sys_name(&self, sys_name: &str, network_id: Uuid) -> IdentityResolution {
        let filter = StorableFilter::<Host>::new_from_network_ids(&[network_id])
            .sys_name(sys_name)
            .live();

        let Ok(found) = self.host_storage.get_unique(filter).await else {
            return IdentityResolution::NotFound;
        };

        IdentityResolution::from_unique(found.map(|host| host.id))
    }

    async fn find_if_entry_by_mac(&self, mac: &str, host_id: Uuid) -> IdentityResolution {
        // Parse MAC string to MacAddress type
        let Ok(mac_addr) = mac.parse::<mac_address::MacAddress>() else {
            return IdentityResolution::NotFound;
        };

        // A MAC names a port only when exactly one physical interface on the host carries it.
        // The lookup this replaced had no ORDER BY and no LIMIT, so on a device that repeats one
        // MAC across its ports it returned an arbitrary port and the link looked port-precise.
        //
        // Virtual rows are excluded because they contest a lookup they can never win: a VLAN or
        // loopback interface is not the far end of a cable, and on the customer's Westermo six
        // `propVirtual` VLAN rows share the chassis base MAC while all ten physical ports have
        // unique addresses. Counting them turned every such lookup `Ambiguous` and cost the port.
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
            .mac_address(&mac_addr)
            .physical_if_types()
            .live();
        match self.interface_service.get_unique(filter).await {
            Ok(Unique::One(entry)) => IdentityResolution::Resolved(entry.id),
            Ok(Unique::None) | Err(_) => IdentityResolution::NotFound,
            Ok(Unique::Multiple) => IdentityResolution::Ambiguous,
        }
    }

    async fn find_if_entry_by_name(&self, name: &str, host_id: Uuid) -> Option<Uuid> {
        // Try if_descr first (long name: "GigabitEthernet1/0/1")
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
            .if_descr(name)
            .live();
        if let Ok(Unique::One(entry)) = self.interface_service.get_unique(filter).await {
            return Some(entry.id);
        }
        // Try if_name (short name: "Gi1/0/1")
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
            .if_name(name)
            .live();
        if let Ok(Unique::One(entry)) = self.interface_service.get_unique(filter).await {
            return Some(entry.id);
        }
        // Try if_alias (the operator-assigned description). On Westermo WeOS the ifDescr carries
        // the media type in front of the name ("100-T eth9") while ifName and ifAlias both hold
        // the bare "eth9", so a neighbour advertising the bare name matches neither column above.
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
            .if_alias(name)
            .live();
        if let Ok(Unique::One(entry)) = self.interface_service.get_unique(filter).await {
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
            if let Ok(Unique::One(entry)) = self.interface_service.get_unique(filter).await {
                return Some(entry.id);
            }
            let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
                .if_name(suffix)
                .live();
            if let Ok(Unique::One(entry)) = self.interface_service.get_unique(filter).await {
                return Some(entry.id);
            }
            let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
                .if_alias(suffix)
                .live();
            if let Ok(Unique::One(entry)) = self.interface_service.get_unique(filter).await {
                return Some(entry.id);
            }
        }
        None
    }

    async fn find_if_entry_by_if_index(&self, if_index: i32, host_id: Uuid) -> Option<Uuid> {
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[host_id])
            .if_index(if_index)
            .live();
        let entry = self
            .interface_service
            .get_unique(filter)
            .await
            .ok()?
            .found()?;

        Some(entry.id)
    }

    async fn find_if_entry_by_ip(&self, ip: &IpAddr, host_id: Uuid) -> Option<Uuid> {
        // Find interface with this IP on the target host
        let filter = StorableFilter::<IPAddress>::new_from_host_ids(&[host_id])
            .ip_address(*ip)
            .live();
        let ip_address = self
            .ip_address_service
            .get_unique(filter)
            .await
            .ok()?
            .found()?;

        // Find Interface linked to this interface via ip_address_id FK
        let filter = StorableFilter::<Interface>::new_from_interface_id(&ip_address.id).live();
        let entry = self
            .interface_service
            .get_unique(filter)
            .await
            .ok()?
            .found()?;

        Some(entry.id)
    }
}
