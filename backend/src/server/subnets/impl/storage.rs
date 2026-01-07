use chrono::{DateTime, Utc};
use cidr::IpCidr;
use sqlx::Row;
use sqlx::postgres::PgRow;
use std::str::FromStr;
use uuid::Uuid;

use crate::server::{
    shared::{
        entities::EntityDiscriminants,
        storage::traits::{SqlValue, StorableEntity},
        types::{entities::EntitySource, metadata::HasId},
    },
    subnets::r#impl::{
        base::{Subnet, SubnetBase},
        types::SubnetType,
    },
};

impl StorableEntity for Subnet {
    type BaseData = SubnetBase;

    fn table_name() -> &'static str {
        "subnets"
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn network_id(&self) -> Option<Uuid> {
        Some(self.base.network_id)
    }

    fn organization_id(&self) -> Option<Uuid> {
        None
    }

    fn new(base: Self::BaseData) -> Self {
        let now = chrono::Utc::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }

    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn set_source(&mut self, source: EntitySource) {
        self.base.source = source;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // source is set at creation time (Manual or Discovery), cannot be changed
        self.base.source = existing.base.source.clone();
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Subnet
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    name,
                    network_id,
                    source,
                    cidr,
                    subnet_type,
                    description,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "name",
                "description",
                "cidr",
                "source",
                "subnet_type",
                "network_id",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                SqlValue::IpCidr(cidr),
                SqlValue::EntitySource(source),
                SqlValue::String(subnet_type.id().to_string()),
                SqlValue::Uuid(network_id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        // Parse fields safely
        let cidr: IpCidr = serde_json::from_str(&row.get::<String, _>("cidr"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize cidr: {}", e))?;
        let subnet_type = SubnetType::from_str(&row.get::<String, _>("subnet_type"))
            .map_err(|e| anyhow::anyhow!("Failed to parse subnet_type: {}", e))?;
        let source: EntitySource =
            serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?;

        Ok(Subnet {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: SubnetBase {
                name: row.get("name"),
                description: row.get("description"),
                network_id: row.get("network_id"),
                source,
                cidr,
                subnet_type,
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
        })
    }
}
