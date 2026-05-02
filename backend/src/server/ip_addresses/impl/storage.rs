use std::net::IpAddr;

use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use mac_address::MacAddress;
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            child::ChildStorableEntity,
            snapshot::{DiscoveryTracked, FkMaps, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
    },
};

/// CSV row representation for IPAddress export
#[derive(Serialize)]
pub struct IPAddressCsvRow {
    pub id: Uuid,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub name: Option<String>,
    pub host_id: Uuid,
    pub subnet_id: Uuid,
    pub network_id: Uuid,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for IPAddress {
    type BaseData = IPAddressBase;

    fn table_name() -> &'static str {
        "ip_addresses"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            valid_from,
            valid_to,
            lineage_id,
            last_seen_at,
            last_discovery_id,
            first_discovery_id,
            base:
                Self::BaseData {
                    network_id,
                    host_id,
                    subnet_id,
                    ip_address,
                    mac_address,
                    name,
                    position,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "network_id",
                "host_id",
                "subnet_id",
                "ip_address",
                "mac_address",
                "name",
                "position",
                "created_at",
                "updated_at",
                "valid_from",
                "valid_to",
                "lineage_id",
                "last_seen_at",
                "last_discovery_id",
                "first_discovery_id",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Uuid(network_id),
                SqlValue::Uuid(host_id),
                SqlValue::Uuid(subnet_id),
                SqlValue::IpAddr(ip_address),
                SqlValue::OptionalMacAddress(mac_address),
                SqlValue::OptionalString(name),
                SqlValue::I32(position),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::Timestamp(valid_from),
                SqlValue::OptionTimestamp(valid_to),
                SqlValue::OptionalUuid(lineage_id),
                SqlValue::Timestamp(last_seen_at),
                SqlValue::OptionalUuid(last_discovery_id),
                SqlValue::OptionalUuid(first_discovery_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        // Read ip_address from INET column using IpNetwork
        let ip_network: IpNetwork = row
            .try_get("ip_address")
            .map_err(|e| anyhow::anyhow!("Failed to read ip_address: {}", e))?;
        let ip_address: IpAddr = ip_network.ip();

        // Read mac_address from MACADDR column
        let mac_address: Option<MacAddress> = row
            .try_get("mac_address")
            .map_err(|e| anyhow::anyhow!("Failed to read mac_address: {}", e))?;

        Ok(IPAddress {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: IPAddressBase {
                network_id: row.get("network_id"),
                host_id: row.get("host_id"),
                subnet_id: row.get("subnet_id"),
                ip_address,
                mac_address,
                name: row.get("name"),
                position: row.get("position"),
            },
        })
    }
}

impl Entity for IPAddress {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    type CsvRow = IPAddressCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        IPAddressCsvRow {
            id: self.id,
            ip_address: self.base.ip_address.to_string(),
            mac_address: self.base.mac_address.map(|m| m.to_string()),
            name: self.base.name.clone(),
            host_id: self.base.host_id,
            subnet_id: self.base.subnet_id,
            network_id: self.base.network_id,
            position: self.base.position,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::IPAddress
    }

    const ENTITY_NAME_SINGULAR: &'static str = "IP Address";
    const ENTITY_NAME_PLURAL: &'static str = "IP Addresses";
    const ENTITY_DESCRIPTION: &'static str = "IP addresses assigned to hosts. Each address belongs to a host and a subnet, optionally has a MAC address, and represents an observed or configured address on the network.";

    fn entity_category() -> EntityCategory {
        EntityCategory::NetworkInfrastructure
    }

    fn network_id(&self) -> Option<Uuid> {
        Some(self.base.network_id)
    }

    fn organization_id(&self) -> Option<Uuid> {
        None
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        self.created_at = existing.created_at;
        // MAC address is immutable once set
        if existing.base.mac_address.is_some() {
            self.base.mac_address = existing.base.mac_address;
        }
    }
}

impl ChildStorableEntity for IPAddress {
    fn parent_column() -> &'static str {
        "host_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.host_id
    }
}

impl Snapshotable for IPAddress {
    fn id_value(&self) -> Uuid {
        self.id
    }
    fn set_id_value(&mut self, id: Uuid) {
        self.id = id;
    }
    fn valid_from(&self) -> DateTime<Utc> {
        self.valid_from
    }
    fn valid_to(&self) -> Option<DateTime<Utc>> {
        self.valid_to
    }
    fn lineage_id(&self) -> Option<Uuid> {
        self.lineage_id
    }
    fn set_valid_from(&mut self, t: DateTime<Utc>) {
        self.valid_from = t;
    }
    fn set_valid_to(&mut self, t: Option<DateTime<Utc>>) {
        self.valid_to = t;
    }
    fn set_lineage_id(&mut self, id: Option<Uuid>) {
        self.lineage_id = id;
    }

    fn remap_fks_for_clone(&mut self, maps: &FkMaps) {
        if let Some(closed) = maps.hosts.get(&self.base.host_id) {
            self.base.host_id = *closed;
        }
        if let Some(closed) = maps.subnets.get(&self.base.subnet_id) {
            self.base.subnet_id = *closed;
        }
    }
}

impl DiscoveryTracked for IPAddress {
    fn last_seen_at(&self) -> DateTime<Utc> {
        self.last_seen_at
    }
    fn last_discovery_id(&self) -> Option<Uuid> {
        self.last_discovery_id
    }
    fn first_discovery_id(&self) -> Option<Uuid> {
        self.first_discovery_id
    }
    fn set_last_seen_at(&mut self, t: DateTime<Utc>) {
        self.last_seen_at = t;
    }
    fn set_last_discovery_id(&mut self, id: Option<Uuid>) {
        self.last_discovery_id = id;
    }
    fn set_first_discovery_id(&mut self, id: Option<Uuid>) {
        self.first_discovery_id = id;
    }

    fn scanned_in_session_filter(
        scanned: &crate::server::daemons::r#impl::api::ScannedEntityIds,
    ) -> crate::server::shared::storage::filter::StorableFilter<Self> {
        crate::server::shared::storage::filter::StorableFilter::<Self>::new_from_uuids_column(
            "id",
            &scanned.ip_address_ids,
        )
    }
}
