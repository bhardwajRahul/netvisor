use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
    snmp_credentials::r#impl::base::{SnmpCredential, SnmpCredentialBase},
};

/// CSV row representation for SnmpCredential export
#[derive(Serialize)]
pub struct SnmpCredentialCsvRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for SnmpCredential {
    type BaseData = SnmpCredentialBase;

    fn table_name() -> &'static str {
        "snmp_credentials"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

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

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    organization_id,
                    name,
                    version,
                    community,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "organization_id",
                "name",
                "version",
                "community",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Uuid(organization_id),
                SqlValue::String(name),
                SqlValue::String(version.to_string()),
                SqlValue::String(community.expose_secret().to_string()),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let version_str: String = row.get("version");
        let version = version_str.parse().unwrap_or_default();

        let community_str: String = row.get("community");
        Ok(SnmpCredential {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: SnmpCredentialBase {
                organization_id: row.get("organization_id"),
                name: row.get("name"),
                version,
                community: SecretString::from(community_str),
            },
        })
    }
}

impl Entity for SnmpCredential {
    type CsvRow = SnmpCredentialCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        SnmpCredentialCsvRow {
            id: self.id,
            organization_id: self.base.organization_id,
            name: self.base.name.clone(),
            version: self.base.version.to_string(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::SnmpCredential
    }

    const ENTITY_NAME_SINGULAR: &'static str = "SNMP Credential";
    const ENTITY_NAME_PLURAL: &'static str = "SNMP Credentials";
    const ENTITY_DESCRIPTION: &'static str = "SNMP credentials for network device discovery. Manage credentials used to query SNMP-enabled devices.";

    fn entity_category() -> EntityCategory {
        EntityCategory::DiscoveryAndDaemons
    }

    fn network_id(&self) -> Option<Uuid> {
        None
    }

    fn organization_id(&self) -> Option<Uuid> {
        Some(self.base.organization_id)
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }
}
