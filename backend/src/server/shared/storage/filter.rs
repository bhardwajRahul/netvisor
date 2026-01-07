use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use uuid::Uuid;

use mac_address::MacAddress;

use crate::server::{
    shared::{entities::EntityDiscriminants, storage::traits::SqlValue},
    users::r#impl::permissions::UserOrgPermissions,
};

/// Builder pattern for common WHERE clauses with optional pagination.
#[derive(Clone)]
pub struct EntityFilter {
    conditions: Vec<String>,
    values: Vec<SqlValue>,
    limit_value: Option<u32>,
    offset_value: Option<u32>,
}

impl EntityFilter {
    pub fn unfiltered() -> Self {
        Self {
            conditions: Vec::new(),
            values: Vec::new(),
            limit_value: None,
            offset_value: None,
        }
    }

    /// Set the maximum number of results to return.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Set the number of results to skip.
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset_value = Some(offset);
        self
    }

    /// Get the limit value, if set.
    pub fn get_limit(&self) -> Option<u32> {
        self.limit_value
    }

    /// Get the offset value, if set.
    pub fn get_offset(&self) -> Option<u32> {
        self.offset_value
    }

    /// Generate LIMIT clause if limit is set.
    pub fn to_limit_clause(&self) -> String {
        match self.limit_value {
            Some(limit) => format!("LIMIT {}", limit),
            None => String::new(),
        }
    }

    /// Generate OFFSET clause if offset is set.
    pub fn to_offset_clause(&self) -> String {
        match self.offset_value {
            Some(offset) if offset > 0 => format!("OFFSET {}", offset),
            _ => String::new(),
        }
    }

    /// Generate combined LIMIT and OFFSET clause.
    pub fn to_pagination_clause(&self) -> String {
        let mut parts = Vec::new();
        if let Some(limit) = self.limit_value {
            parts.push(format!("LIMIT {}", limit));
        }
        if let Some(offset) = self.offset_value
            && offset > 0
        {
            parts.push(format!("OFFSET {}", offset));
        }
        parts.join(" ")
    }

    pub fn entity_id(mut self, id: &Uuid) -> Self {
        self.conditions
            .push(format!("id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn entity_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("id IN ({})", placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn network_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("network_id IN ({})", placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn user_id(mut self, id: &Uuid) -> Self {
        self.conditions
            .push(format!("user_id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn hidden_is(mut self, hidden: bool) -> Self {
        self.conditions
            .push(format!("hidden = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Bool(hidden));
        self
    }

    pub fn host_id(mut self, id: &Uuid) -> Self {
        self.conditions
            .push(format!("host_id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn subnet_id(mut self, id: &Uuid) -> Self {
        self.conditions
            .push(format!("subnet_id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn mac_address(mut self, mac: &MacAddress) -> Self {
        self.conditions
            .push(format!("mac_address = ${}", self.values.len() + 1));
        self.values.push(SqlValue::MacAddress(*mac));
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.conditions
            .push(format!("name = ${}", self.values.len() + 1));
        self.values.push(SqlValue::String(name));
        self
    }

    pub fn group_id(mut self, id: &Uuid) -> Self {
        self.conditions
            .push(format!("group_id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn group_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("group_id IN ({})", placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn binding_id(mut self, id: &Uuid) -> Self {
        self.conditions
            .push(format!("binding_id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn host_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("host_id IN ({})", placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn api_key(mut self, api_key: String) -> Self {
        self.conditions
            .push(format!("key = ${}", self.values.len() + 1));
        self.values.push(SqlValue::String(api_key));
        self
    }

    pub fn scheduled_discovery(mut self) -> Self {
        self.conditions
            .push("run_type->>'type' = 'Scheduled'".to_string());
        self.conditions
            .push("(run_type->>'enabled')::boolean = true".to_string());
        self
    }

    pub fn oidc_subject(mut self, subject: String) -> Self {
        self.conditions
            .push(format!("oidc_subject = ${}", self.values.len() + 1));
        self.values.push(SqlValue::String(subject));
        self.conditions
            .push("oidc_provider IS NOT NULL".to_string());
        self
    }

    pub fn email(mut self, email: &EmailAddress) -> Self {
        self.conditions
            .push(format!("email = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Email(email.clone()));
        self
    }

    pub fn organization_id(mut self, organization_id: &Uuid) -> Self {
        self.conditions
            .push(format!("organization_id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*organization_id));
        self
    }

    pub fn topology_id(mut self, topology_id: &Uuid) -> Self {
        self.conditions
            .push(format!("topology_id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*topology_id));
        self
    }

    pub fn user_permissions(mut self, permissions: &UserOrgPermissions) -> Self {
        self.conditions
            .push(format!("permissions = ${}", self.values.len() + 1));
        self.values.push(SqlValue::UserOrgPermissions(*permissions));
        self
    }

    pub fn expires_before(mut self, timestamp: DateTime<Utc>) -> Self {
        self.conditions
            .push(format!("expires_at < ${}", self.values.len() + 1));
        self.values.push(SqlValue::Timestamp(timestamp));
        self
    }

    /// Generic UUID filter for any column name.
    /// Used by generic child entity handlers to filter by parent_column dynamically.
    pub fn uuid_column(mut self, column: &str, id: &Uuid) -> Self {
        self.conditions
            .push(format!("{} = ${}", column, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Generic UUID IN filter for any column name.
    /// Used by generic child entity services to filter by parent_column dynamically.
    pub fn uuid_columns(mut self, column: &str, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", column, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    /// Filter by service_id (for bindings)
    pub fn service_id(mut self, id: &Uuid) -> Self {
        self.conditions
            .push(format!("service_id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Filter by entity_type (for entity_tags junction table)
    pub fn entity_type(mut self, entity_type: &EntityDiscriminants) -> Self {
        self.conditions
            .push(format!("entity_type = ${}", self.values.len() + 1));
        self.values.push(SqlValue::String(entity_type.to_string()));
        self
    }

    /// Filter by tag_id (for entity_tags junction table)
    pub fn tag_id(mut self, id: &Uuid) -> Self {
        self.conditions
            .push(format!("tag_id = ${}", self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn to_where_clause(&self) -> String {
        if self.conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", self.conditions.join(" AND "))
        }
    }

    pub fn values(&self) -> &[SqlValue] {
        &self.values
    }
}
