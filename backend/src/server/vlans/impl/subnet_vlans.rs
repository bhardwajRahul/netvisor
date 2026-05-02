//! Subnet-VLAN junction table and storage.
//!
//! Manages the many-to-many relationship between subnets and VLANs.

use std::collections::HashMap;
use std::fmt::Display;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::server::shared::storage::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    snapshot::{DiscoveryTracked, FkMaps, Snapshotable},
    traits::{SqlValue, Storable, Storage},
};

/// The base data for a SubnetVlanRecord junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct SubnetVlanRecordBase {
    pub subnet_id: Uuid,
    pub vlan_id: Uuid,
}

impl SubnetVlanRecordBase {
    pub fn new(subnet_id: Uuid, vlan_id: Uuid) -> Self {
        Self { subnet_id, vlan_id }
    }
}

/// A junction record linking a subnet to a VLAN
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct SubnetVlanRecord {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub valid_from: DateTime<Utc>,
    #[serde(default)]
    pub valid_to: Option<DateTime<Utc>>,
    #[serde(default)]
    pub lineage_id: Option<Uuid>,
    #[serde(default)]
    pub last_seen_at: DateTime<Utc>,
    #[serde(default)]
    pub last_discovery_id: Option<Uuid>,
    #[serde(default)]
    pub first_discovery_id: Option<Uuid>,
    pub base: SubnetVlanRecordBase,
}

impl SubnetVlanRecord {
    pub fn new(base: SubnetVlanRecordBase) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            base,
        }
    }

    pub fn subnet_id(&self) -> Uuid {
        self.base.subnet_id
    }

    pub fn vlan_id(&self) -> Uuid {
        self.base.vlan_id
    }
}

impl Display for SubnetVlanRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SubnetVlan(subnet={}, vlan={})",
            self.base.subnet_id, self.base.vlan_id
        )
    }
}

impl Storable for SubnetVlanRecord {
    type BaseData = SubnetVlanRecordBase;

    fn table_name() -> &'static str {
        "subnet_vlans"
    }

    fn new(base: Self::BaseData) -> Self {
        SubnetVlanRecord::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec![
                "id",
                "subnet_id",
                "vlan_id",
                "created_at",
                "valid_from",
                "valid_to",
                "lineage_id",
                "last_seen_at",
                "last_discovery_id",
                "first_discovery_id",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.subnet_id),
                SqlValue::Uuid(self.base.vlan_id),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.valid_from),
                SqlValue::OptionTimestamp(self.valid_to),
                SqlValue::OptionalUuid(self.lineage_id),
                SqlValue::Timestamp(self.last_seen_at),
                SqlValue::OptionalUuid(self.last_discovery_id),
                SqlValue::OptionalUuid(self.first_discovery_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(SubnetVlanRecord {
            id: row.get("id"),
            created_at: row.get("created_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: SubnetVlanRecordBase {
                subnet_id: row.get("subnet_id"),
                vlan_id: row.get("vlan_id"),
            },
        })
    }
}

impl Snapshotable for SubnetVlanRecord {
    fn id_value(&self) -> Uuid { self.id }
    fn set_id_value(&mut self, id: Uuid) { self.id = id; }
    fn valid_from(&self) -> DateTime<Utc> { self.valid_from }
    fn valid_to(&self) -> Option<DateTime<Utc>> { self.valid_to }
    fn lineage_id(&self) -> Option<Uuid> { self.lineage_id }
    fn set_valid_from(&mut self, t: DateTime<Utc>) { self.valid_from = t; }
    fn set_valid_to(&mut self, t: Option<DateTime<Utc>>) { self.valid_to = t; }
    fn set_lineage_id(&mut self, id: Option<Uuid>) { self.lineage_id = id; }

    fn remap_fks_for_clone(&mut self, maps: &FkMaps) {
        if let Some(closed) = maps.subnets.get(&self.base.subnet_id) {
            self.base.subnet_id = *closed;
        }
        if let Some(closed) = maps.vlans.get(&self.base.vlan_id) {
            self.base.vlan_id = *closed;
        }
    }
}

impl DiscoveryTracked for SubnetVlanRecord {
    fn last_seen_at(&self) -> DateTime<Utc> { self.last_seen_at }
    fn last_discovery_id(&self) -> Option<Uuid> { self.last_discovery_id }
    fn first_discovery_id(&self) -> Option<Uuid> { self.first_discovery_id }
    fn set_last_seen_at(&mut self, t: DateTime<Utc>) { self.last_seen_at = t; }
    fn set_last_discovery_id(&mut self, id: Option<Uuid>) { self.last_discovery_id = id; }
    fn set_first_discovery_id(&mut self, id: Option<Uuid>) { self.first_discovery_id = id; }
}

/// Storage operations for subnet_vlans junction table.
pub struct SubnetVlanStorage {
    storage: GenericPostgresStorage<SubnetVlanRecord>,
}

impl SubnetVlanStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get all VLAN IDs linked to a subnet
    pub async fn get_vlan_ids_for_subnet(&self, subnet_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("subnet_id", subnet_id);
        let records = self.storage.get_all(filter).await?;
        Ok(records.iter().map(|r| r.vlan_id()).collect())
    }

    /// Get all subnet IDs linked to a VLAN
    pub async fn get_subnet_ids_for_vlan(&self, vlan_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter = StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("vlan_id", vlan_id);
        let records = self.storage.get_all(filter).await?;
        Ok(records.iter().map(|r| r.subnet_id()).collect())
    }

    /// Batch get: VLAN ID → subnet IDs for multiple VLANs
    pub async fn get_for_vlans(&self, vlan_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if vlan_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<SubnetVlanRecord>::new_from_uuids_column("vlan_id", vlan_ids);
        let records = self.storage.get_all(filter).await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for record in records {
            result
                .entry(record.vlan_id())
                .or_default()
                .push(record.subnet_id());
        }

        Ok(result)
    }

    /// Link a subnet to a VLAN (idempotent)
    pub async fn link(&self, subnet_id: &Uuid, vlan_id: &Uuid) -> Result<()> {
        let record = SubnetVlanRecord::new(SubnetVlanRecordBase::new(*subnet_id, *vlan_id));
        // Ignore unique constraint violations (idempotent)
        let _ = self.storage.create(&record).await;
        Ok(())
    }

    /// Unlink a subnet from a VLAN
    pub async fn unlink(&self, subnet_id: &Uuid, vlan_id: &Uuid) -> Result<()> {
        let filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("subnet_id", subnet_id)
                .uuid_column("vlan_id", vlan_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }

    /// Replace all VLAN links for a subnet
    pub async fn save_for_subnet(&self, subnet_id: &Uuid, vlan_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;

        // Delete existing links
        let filter =
            StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("subnet_id", subnet_id);
        tx.delete_by_filter(filter).await?;

        // Insert new links
        for vlan_id in vlan_ids {
            let record = SubnetVlanRecord::new(SubnetVlanRecordBase::new(*subnet_id, *vlan_id));
            tx.create(&record).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Delete all subnet links for a VLAN (cleanup on VLAN delete)
    pub async fn delete_for_vlan(&self, vlan_id: &Uuid) -> Result<()> {
        let filter = StorableFilter::<SubnetVlanRecord>::new_from_uuid_column("vlan_id", vlan_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }
}
