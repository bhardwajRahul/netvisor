use anyhow::Result;
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use validator::ValidationError;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    interfaces::r#impl::base::{Interface, Neighbor},
    ip_addresses::service::IPAddressService,
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, Storage},
        },
    },
    tags::entity_tags::EntityTagService,
};

pub struct InterfaceService {
    storage: Arc<GenericPostgresStorage<Interface>>,
    event_bus: Arc<EventBus>,
    ip_address_service: Arc<IPAddressService>,
}

impl EventBusService<Interface> for InterfaceService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Interface) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &Interface) -> Option<Uuid> {
        None
    }
}

impl CrudService<Interface> for InterfaceService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Interface>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl ChildCrudService<Interface> for InterfaceService {}

impl InterfaceService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Interface>>,
        event_bus: Arc<EventBus>,
        ip_address_service: Arc<IPAddressService>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            ip_address_service,
        }
    }

    /// Get all if entries for a specific host, ordered by ifIndex.
    /// SCD2: only live rows.
    pub async fn get_for_host(&self, host_id: &Uuid) -> Result<Vec<Interface>> {
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[*host_id]).live();
        self.storage.get_all_ordered(filter, "if_index ASC").await
    }

    /// Get if entries for multiple hosts, ordered by ifIndex within each host.
    /// `at = None` reads live rows; `Some(t)` reads SCD2 state as of `t`
    /// (snapshot-view hydration).
    pub async fn get_for_hosts(
        &self,
        host_ids: &[Uuid],
        at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<HashMap<Uuid, Vec<Interface>>> {
        if host_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<Interface>::new_from_host_ids(host_ids).live_or_as_of(at);
        let entries = self
            .storage
            .get_all_ordered(filter, "host_id ASC, if_index ASC")
            .await?;

        let mut result: HashMap<Uuid, Vec<Interface>> = HashMap::new();
        for entry in entries {
            result.entry(entry.base.host_id).or_default().push(entry);
        }
        Ok(result)
    }

    /// Validate FK relationships for an Interface.
    ///
    /// Validates:
    /// - ip_address_id must reference an Interface on the same host
    /// - If both Interface and Interface have MAC addresses, they should match
    /// - neighbor (when Interface) must reference an Interface on a different host, same network
    ///
    /// Note: Neighbor::Host validation is done in handlers (requires access to HostService)
    pub async fn validate_relationships(&self, entry: &Interface) -> Result<()> {
        // 1. ip_address_id: must be on SAME host, and MAC addresses should match if both present
        if let Some(ip_address_id) = entry.base.ip_address_id {
            let ip_address = self
                .ip_address_service
                .get_by_id(&ip_address_id)
                .await?
                .ok_or_else(|| {
                    ValidationError::new("ip_address_id references a non-existent Interface")
                })?;

            if ip_address.base.host_id != entry.base.host_id {
                return Err(ValidationError::new(
                    "ip_address_id must reference an Interface on the same host",
                )
                .into());
            }

            // Validate MAC address consistency if both have MAC addresses
            if let (Some(if_entry_mac), Some(ip_address_mac)) =
                (&entry.base.mac_address, &ip_address.base.mac_address)
                && if_entry_mac != ip_address_mac
            {
                return Err(ValidationError::new(
                    "ip_address_id references an Interface with a different MAC address",
                )
                .into());
            }
        }

        // 2. neighbor (Interface variant): must be on DIFFERENT host, same network
        if let Some(Neighbor::Interface(neighbor_id)) = &entry.base.neighbor {
            // Cannot connect to self
            if *neighbor_id == entry.id {
                return Err(ValidationError::new("Interface cannot connect to itself").into());
            }

            // Get the neighbor Interface
            let neighbor_interface = self.get_by_id(neighbor_id).await?.ok_or_else(|| {
                ValidationError::new("neighbor Interface references a non-existent Interface")
            })?;

            // Must be different host
            if neighbor_interface.base.host_id == entry.base.host_id {
                return Err(
                    ValidationError::new("neighbor Interface must be on a different host").into(),
                );
            }

            // Must be same network
            if neighbor_interface.base.network_id != entry.base.network_id {
                return Err(
                    ValidationError::new("neighbor Interface must be in the same network").into(),
                );
            }
        }

        // Note: Neighbor::Host validation is handled in handlers which have access to HostService

        Ok(())
    }

    /// Lookup an interface by (host_id, if_name). Returns None if no match.
    /// Used as tier-1 dedup during discovery — if_name (ifXTable.ifName) is the
    /// most stable SNMP port identifier, surviving reboots and if_index shifts.
    pub async fn get_by_host_and_name(
        &self,
        host_id: &Uuid,
        if_name: &str,
    ) -> Result<Option<Interface>> {
        // SCD2: only live rows participate in natural-key match.
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[*host_id])
            .if_name(if_name)
            .live();
        let mut rows = self.storage.get_all(filter).await?;
        Ok(rows.pop())
    }

    /// Lookup interfaces whose ip_address_id is in the given set.
    /// Used by subnet_vlans reconciliation to aggregate native_vlan_id observations.
    pub async fn get_by_ip_address_ids(&self, ids: &[Uuid]) -> Result<Vec<Interface>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        // SCD2: only live rows.
        let filter =
            StorableFilter::<Interface>::new_from_uuids_column("ip_address_id", ids).live();
        self.storage.get_all(filter).await
    }

    /// Create or update an interface during discovery, matching on a tiered identity:
    ///
    /// 1. `(host_id, if_name)` when incoming `if_name.is_some()` — strong identifier
    ///    from ifXTable, survives reboots/config reloads
    /// 2. `(host_id, if_index)` — fallback for legacy devices without ifXTable, and
    ///    for pre-existing rows written before the `if_name` column was added (first
    ///    post-upgrade rescan finds those rows by `if_index` and writes `if_name`,
    ///    after which tier 1 owns the match)
    /// 3. `(host_id, mac_address)` with single-MAC guard — last resort for ports
    ///    that got both renamed and renumbered but kept their NIC
    ///
    /// On match, preserves id + created_at + mac_address + if_name (via
    /// `preserve_immutable_fields`) and overwrites the rest with the incoming payload.
    /// Skips relationship validation (data from trusted SNMP source).
    pub async fn create_or_update_from_discovery(
        &self,
        entry: Interface,
        authentication: AuthenticatedEntity,
    ) -> Result<Interface> {
        let existing = self.find_matching_existing(&entry).await?;

        if let Some(existing_entry) = existing {
            let mut updated = entry;
            updated.id = existing_entry.id;
            updated.preserve_immutable_fields(&existing_entry);
            self.update(&mut updated, authentication).await
        } else {
            // SCD2 origin: no match found, this is a new insert. Stamp
            // created_at + valid_from to the entity's already-refreshed
            // `last_seen_at` (set by the discovery handler) so all four
            // temporal columns line up at one canonical scan_time.
            use crate::server::shared::storage::snapshot::DiscoveryTracked;
            let mut entry = entry;
            entry.originate_scan_timestamps(entry.last_seen_at);
            self.create(entry, authentication).await
        }
    }

    /// Tiered lookup: if_name → if_index → mac_address with single-MAC guard.
    async fn find_matching_existing(&self, entry: &Interface) -> Result<Option<Interface>> {
        let host_id = entry.base.host_id;

        tracing::debug!(
            host_id = %host_id,
            incoming_if_index = entry.base.if_index,
            incoming_if_name = ?entry.base.if_name,
            incoming_mac = ?entry.base.mac_address,
            "InterfaceService::find_matching_existing: start"
        );

        // Tier 1: (host_id, if_name) — strong identifier when present
        if let Some(ref if_name) = entry.base.if_name {
            let found = self.get_by_host_and_name(&host_id, if_name).await?;
            tracing::debug!(
                host_id = %host_id,
                incoming_if_name = %if_name,
                matched = found.is_some(),
                matched_id = ?found.as_ref().map(|f| f.id),
                matched_if_index = ?found.as_ref().map(|f| f.base.if_index),
                "InterfaceService tier-1 lookup (host, if_name)"
            );
            if let Some(found) = found {
                return Ok(Some(found));
            }
        } else {
            tracing::debug!(
                host_id = %host_id,
                "InterfaceService tier-1 skipped: incoming if_name is None"
            );
        }

        // Load host's interfaces once for tiers 2 + 3
        let existing = self.get_for_host(&host_id).await?;

        // Tier 2: (host_id, if_index)
        if let Some(found) = existing
            .iter()
            .find(|e| e.base.if_index == entry.base.if_index)
        {
            tracing::debug!(
                host_id = %host_id,
                incoming_if_index = entry.base.if_index,
                matched_id = %found.id,
                "InterfaceService tier-2 matched on if_index"
            );
            return Ok(Some(found.clone()));
        }

        // Tier 3: (host_id, mac_address) with single-MAC guard. A MAC shared by
        // multiple existing rows indicates VLAN sub-interfaces or bond members and
        // must not collapse into a single match — only accept a 1:1 MAC pairing.
        if let Some(mac) = entry.base.mac_address {
            let mut candidates = existing.iter().filter(|e| e.base.mac_address == Some(mac));
            if let Some(first) = candidates.next()
                && candidates.next().is_none()
            {
                tracing::debug!(
                    host_id = %host_id,
                    matched_id = %first.id,
                    "InterfaceService tier-3 matched on mac_address"
                );
                return Ok(Some(first.clone()));
            }
        }

        tracing::debug!(
            host_id = %host_id,
            incoming_if_index = entry.base.if_index,
            incoming_if_name = ?entry.base.if_name,
            "InterfaceService: no tier match, will create new row"
        );
        Ok(None)
    }
}
