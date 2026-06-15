use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use email_address::EmailAddress;
use mac_address::MacAddress;
use uuid::Uuid;

use crate::server::{
    daemons::r#impl::base::DaemonMode,
    shared::{entities::EntityDiscriminants, storage::traits::SqlValue},
    users::r#impl::permissions::UserOrgPermissions,
};

use super::traits::Storable;

/// Builder pattern for common WHERE clauses with optional pagination and JOINs.
/// Generic over entity type T to automatically qualify column names with the table name.
#[derive(Clone)]
pub struct StorableFilter<T: Storable> {
    _marker: PhantomData<T>,
    conditions: Vec<String>,
    values: Vec<SqlValue>,
    limit_value: Option<u32>,
    offset_value: Option<u32>,
    joins: Vec<String>,
}

impl<T: Storable> Default for StorableFilter<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Storable> StorableFilter<T> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
            conditions: Vec::new(),
            values: Vec::new(),
            limit_value: None,
            offset_value: None,
            joins: Vec::new(),
        }
    }

    pub fn new_from_org_id(org_id: &Uuid) -> Self {
        Self::new().organization_id(org_id)
    }

    /// Empty filter (no WHERE conditions). Useful for chaining ad-hoc helper
    /// methods like `id_or_lineage_in` + `as_of` without an initial scope.
    pub fn new_unfiltered() -> Self {
        Self::new()
    }

    pub fn new_from_network_ids(network_ids: &[Uuid]) -> Self {
        Self::new().network_ids(network_ids)
    }

    pub fn new_from_entity_id(entity_id: &Uuid) -> Self {
        Self::new().entity_id(entity_id)
    }

    pub fn new_from_entity_ids(entity_ids: &[Uuid]) -> Self {
        Self::new().entity_ids(entity_ids)
    }

    pub fn new_from_api_key(api_key: String) -> Self {
        Self::new().api_key(api_key)
    }

    pub fn new_from_email(email: &EmailAddress) -> Self {
        Self::new().email(email)
    }

    pub fn new_from_oidc_subject(oidc_subject: String) -> Self {
        Self::new().oidc_subject(oidc_subject)
    }

    pub fn new_from_password_reset_token(token: &str) -> Self {
        Self::new().password_reset_token(token)
    }

    pub fn new_from_email_verification_token(token: &str) -> Self {
        Self::new().email_verification_token(token)
    }

    pub fn new_from_host_ids(host_ids: &[Uuid]) -> Self {
        Self::new().host_ids(host_ids)
    }

    pub fn new_from_service_id(service_id: &Uuid) -> Self {
        Self::new().service_id(service_id)
    }

    pub fn new_from_subnet_id(subnet_id: &Uuid) -> Self {
        Self::new().subnet_id(subnet_id)
    }

    pub fn new_from_binding_id(binding_id: &Uuid) -> Self {
        Self::new().binding_id(binding_id)
    }

    pub fn new_from_user_id(user_id: &Uuid) -> Self {
        Self::new().user_id(user_id)
    }

    pub fn new_from_user_ids(user_ids: &[Uuid]) -> Self {
        Self::new().user_ids(user_ids)
    }

    pub fn new_from_interface_id(ip_address_id: &Uuid) -> Self {
        Self::new().ip_address_id(ip_address_id)
    }

    pub fn new_from_dependency_ids(dependency_ids: &[Uuid]) -> Self {
        Self::new().dependency_ids(dependency_ids)
    }

    pub fn new_from_uuid_column(column: &str, id: &Uuid) -> Self {
        Self::new().uuid_column(column, id)
    }

    pub fn new_from_uuids_column(column: &str, ids: &[Uuid]) -> Self {
        Self::new().uuids_column(column, ids)
    }

    pub fn new_for_scheduled_discoveries() -> Self {
        Self::new().scheduled_discovery()
    }

    pub fn new_for_unresolved_lldp_in_network(network_id: Uuid) -> Self {
        Self::new().unresolved_lldp_in_network(network_id)
    }

    pub fn new_for_unresolved_fdb_in_network(network_id: Uuid) -> Self {
        Self::new().unresolved_fdb_in_network(network_id)
    }

    pub fn new_without_brevo_company_id() -> Self {
        Self::new().without_brevo_company_id()
    }

    pub fn new_with_brevo_company_id() -> Self {
        Self::new().with_brevo_company_id()
    }

    pub fn new_with_stripe_customer_id(id: &str) -> Self {
        Self::new().stripe_customer_id(id)
    }

    pub fn new_with_expiry_before(timestamp: DateTime<Utc>) -> Self {
        Self::new().expires_before(timestamp)
    }

    pub fn new_for_daemon_poller_system_job() -> Self {
        Self::new()
            .daemon_mode(DaemonMode::ServerPoll)
            .is_unreachable(false)
            .standby(false)
    }

    pub fn new_for_active_daemons() -> Self {
        Self::new().standby(false).is_unreachable(false)
    }

    /// Qualify a column name with the table name.
    fn qualify_column(&self, column: &str) -> String {
        format!("{}.{}", T::table_name(), column)
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

    /// Add a JOIN clause to the filter.
    /// Example: `filter.join("LEFT JOIN services AS s ON hosts.service_id = s.id")`
    pub fn join(mut self, join_clause: &str) -> Self {
        self.joins.push(join_clause.to_string());
        self
    }

    /// Generate the combined JOIN clause string.
    pub fn to_join_clause(&self) -> String {
        self.joins.join(" ")
    }

    /// Returns true if this filter has any JOIN clauses.
    pub fn has_joins(&self) -> bool {
        !self.joins.is_empty()
    }

    pub fn entity_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn entity_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

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

        let col = self.qualify_column("network_id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn user_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("user_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn user_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("user_id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn hidden_is(mut self, hidden: bool) -> Self {
        let col = self.qualify_column("hidden");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Bool(hidden));
        self
    }

    /// SCD2 current-state filter: only live rows (`valid_to IS NULL`).
    /// Used by the topology read path and reconciliation natural-key
    /// matching to ignore closed historical copies.
    pub fn live(mut self) -> Self {
        let col = self.qualify_column("valid_to");
        self.conditions.push(format!("{} IS NULL", col));
        self
    }

    /// SCD2 as-of filter: rows that were live at timestamp `t`.
    /// Used by snapshot-view consumers to read historical state.
    pub fn as_of(mut self, t: chrono::DateTime<chrono::Utc>) -> Self {
        let valid_from = self.qualify_column("valid_from");
        let valid_to = self.qualify_column("valid_to");
        let from_idx = self.values.len() + 1;
        let to_idx = self.values.len() + 2;
        self.conditions.push(format!(
            "{vf} <= ${fi} AND ({vt} IS NULL OR {vt} > ${ti})",
            vf = valid_from,
            vt = valid_to,
            fi = from_idx,
            ti = to_idx,
        ));
        self.values.push(SqlValue::Timestamp(t));
        self.values.push(SqlValue::Timestamp(t));
        self
    }

    /// SCD2 read-path filter: `as_of(t)` when a snapshot timestamp is supplied,
    /// otherwise current-state `live()`. Frontend-facing GETs use this so they
    /// hide closed historical copies by default and read snapshot-pinned state
    /// when `at` is set.
    pub fn live_or_as_of(self, at: Option<chrono::DateTime<chrono::Utc>>) -> Self {
        match at {
            Some(t) => self.as_of(t),
            None => self.live(),
        }
    }

    /// Lineage filter for "all closed copies tracking back to this live id."
    /// Used to walk version history of a single logical entity.
    pub fn lineage_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("lineage_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Filter to closed copies stamped by a specific snapshot. Snapshot views
    /// read these directly: the closed copies have distinct ids from their
    /// live counterparts and survive live-row deletion.
    pub fn snapshot_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("snapshot_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Match rows whose `id` or `lineage_id` is in the supplied set. Used by
    /// the as-of tag-name resolver: when a tag was close-and-cloned, the live
    /// id and the closed id both refer to the same logical tag — a single
    /// `id IN (...)` check would miss the closed copies. The OR-join keeps the
    /// caller's id list compact (no need to expand into the lineage set first).
    pub fn id_or_lineage_in(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }
        let id_col = self.qualify_column("id");
        let lineage_col = self.qualify_column("lineage_id");
        let id_idx = self.values.len() + 1;
        let lineage_idx = self.values.len() + 2;
        self.conditions.push(format!(
            "({} = ANY(${}) OR {} = ANY(${}))",
            id_col, id_idx, lineage_col, lineage_idx
        ));
        self.values.push(SqlValue::UuidArray(ids.to_vec()));
        self.values.push(SqlValue::UuidArray(ids.to_vec()));
        self
    }

    /// Filter snapshots / similar timestamped rows by `taken_at < t`. Used by
    /// the daily retention task to identify rows past the retention window.
    pub fn taken_at_lt(mut self, t: DateTime<Utc>) -> Self {
        let col = self.qualify_column("taken_at");
        self.conditions
            .push(format!("{} < ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Timestamp(t));
        self
    }

    pub fn host_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("host_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn subnet_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("subnet_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn mac_address(mut self, mac: &MacAddress) -> Self {
        let col = self.qualify_column("mac_address");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::MacAddress(*mac));
        self
    }

    pub fn password_reset_token(mut self, token: &str) -> Self {
        let col = self.qualify_column("password_reset_token");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(token.to_string()));
        self
    }

    pub fn email_verification_token(mut self, token: &str) -> Self {
        let col = self.qualify_column("email_verification_token");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(token.to_string()));
        self
    }

    pub fn name(mut self, name: String) -> Self {
        let col = self.qualify_column("name");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(name));
        self
    }

    pub fn service_definition_not_in(mut self, definitions: &[String]) -> Self {
        if definitions.is_empty() {
            return self;
        }
        let col = self.qualify_column("service_definition");
        let placeholders: Vec<String> = definitions
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();
        self.conditions
            .push(format!("{} NOT IN ({})", col, placeholders.join(", ")));
        for def in definitions {
            self.values.push(SqlValue::String(def.clone()));
        }
        self
    }

    pub fn dependency_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("dependency_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn dependency_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("dependency_id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn binding_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("binding_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    pub fn host_ids(mut self, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            // Empty IN clause should match nothing
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column("host_id");
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    pub fn api_key(mut self, api_key: String) -> Self {
        let col = self.qualify_column("key");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(api_key));
        self
    }

    /// Filter by a value within a JSONB column. E.g. `json_field_eq("credential_type", "type", "Snmp")`
    /// generates `credential_type->>'type' = $N`.
    pub fn json_field_eq(mut self, column: &str, key: &str, value: &str) -> Self {
        let col = self.qualify_column(column);
        self.conditions
            .push(format!("{}->>'{}' = ${}", col, key, self.values.len() + 1));
        self.values.push(SqlValue::String(value.to_string()));
        self
    }

    pub fn scheduled_discovery(mut self) -> Self {
        self.conditions
            .push("run_type->>'type' = 'Scheduled'".to_string());
        self.conditions
            .push("(run_type->>'enabled')::boolean = true".to_string());
        self
    }

    pub fn historical_discovery(mut self) -> Self {
        self.conditions
            .push("run_type->>'type' = 'Historical'".to_string());
        self
    }

    pub fn exclude_historical(mut self) -> Self {
        self.conditions
            .push("run_type->>'type' != 'Historical'".to_string());
        self
    }

    pub fn oidc_subject(mut self, subject: String) -> Self {
        let col = self.qualify_column("oidc_subject");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(subject));
        let provider_col = self.qualify_column("oidc_provider");
        self.conditions
            .push(format!("{} IS NOT NULL", provider_col));
        self
    }

    pub fn email(mut self, email: &EmailAddress) -> Self {
        let col = self.qualify_column("email");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Email(email.clone()));
        self
    }

    pub fn organization_id(mut self, organization_id: &Uuid) -> Self {
        let col = self.qualify_column("organization_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*organization_id));
        self
    }

    pub fn topology_id(mut self, topology_id: &Uuid) -> Self {
        let col = self.qualify_column("topology_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*topology_id));
        self
    }

    pub fn user_permissions(mut self, permissions: &UserOrgPermissions) -> Self {
        let col = self.qualify_column("permissions");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::UserOrgPermissions(*permissions));
        self
    }

    pub fn user_permissions_in(mut self, permissions: &[UserOrgPermissions]) -> Self {
        if permissions.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }
        let col = self.qualify_column("permissions");
        let placeholders: Vec<String> = permissions
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();
        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));
        for p in permissions {
            self.values.push(SqlValue::UserOrgPermissions(*p));
        }
        self
    }

    pub fn expires_before(mut self, timestamp: DateTime<Utc>) -> Self {
        let col = self.qualify_column("expires_at");
        self.conditions
            .push(format!("{} < ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Timestamp(timestamp));
        self
    }

    pub fn created_before(mut self, timestamp: DateTime<Utc>) -> Self {
        let col = self.qualify_column("created_at");
        self.conditions
            .push(format!("{} < ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Timestamp(timestamp));
        self
    }

    pub fn last_seen_before(mut self, timestamp: DateTime<Utc>) -> Self {
        let col = self.qualify_column("last_seen_at");
        self.conditions
            .push(format!("{} < ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Timestamp(timestamp));
        self
    }

    pub fn updated_before(mut self, timestamp: DateTime<Utc>) -> Self {
        let col = self.qualify_column("updated_at");
        self.conditions
            .push(format!("{} < ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Timestamp(timestamp));
        self
    }

    pub fn created_between(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let col = self.qualify_column("created_at");
        let start_idx = self.values.len() + 1;
        let end_idx = self.values.len() + 2;
        self.conditions
            .push(format!("{col} >= ${start_idx} AND {col} <= ${end_idx}"));
        self.values.push(SqlValue::Timestamp(start));
        self.values.push(SqlValue::Timestamp(end));
        self
    }

    pub fn updated_between(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let col = self.qualify_column("updated_at");
        let start_idx = self.values.len() + 1;
        let end_idx = self.values.len() + 2;
        self.conditions
            .push(format!("{col} >= ${start_idx} AND {col} <= ${end_idx}"));
        self.values.push(SqlValue::Timestamp(start));
        self.values.push(SqlValue::Timestamp(end));
        self
    }

    pub fn valid_to_between(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let col = self.qualify_column("valid_to");
        let start_idx = self.values.len() + 1;
        let end_idx = self.values.len() + 2;
        self.conditions
            .push(format!("{col} >= ${start_idx} AND {col} <= ${end_idx}"));
        self.values.push(SqlValue::Timestamp(start));
        self.values.push(SqlValue::Timestamp(end));
        self
    }

    pub fn last_seen_between(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let col = self.qualify_column("last_seen_at");
        let start_idx = self.values.len() + 1;
        let end_idx = self.values.len() + 2;
        self.conditions
            .push(format!("{col} >= ${start_idx} AND {col} <= ${end_idx}"));
        self.values.push(SqlValue::Timestamp(start));
        self.values.push(SqlValue::Timestamp(end));
        self
    }

    /// Generic u16 filter for any SMALLINT column.
    pub fn u16_column(mut self, column: &str, value: u16) -> Self {
        let col = self.qualify_column(column);
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::U16(value));
        self
    }

    /// Generic UUID filter for any column name.
    /// Used by generic child entity handlers to filter by parent_column dynamically.
    pub fn uuid_column(mut self, column: &str, id: &Uuid) -> Self {
        let col = self.qualify_column(column);
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Generic UUID IN filter for any column name.
    /// Used by generic child entity services to filter by parent_column dynamically.
    pub fn uuids_column(mut self, column: &str, ids: &[Uuid]) -> Self {
        if ids.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let col = self.qualify_column(column);
        let placeholders: Vec<String> = ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 1))
            .collect();

        self.conditions
            .push(format!("{} IN ({})", col, placeholders.join(", ")));

        for id in ids {
            self.values.push(SqlValue::Uuid(*id));
        }

        self
    }

    /// Filter by service_id (for bindings)
    pub fn service_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("service_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Filter by mode (for daemons)
    pub fn daemon_mode(mut self, mode: DaemonMode) -> Self {
        let col = self.qualify_column("mode");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::DaemonMode(mode));
        self
    }

    /// Filter by mode (for daemons)
    pub fn is_unreachable(mut self, is_unreachable: bool) -> Self {
        let col = self.qualify_column("is_unreachable");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Bool(is_unreachable));
        self
    }

    pub fn standby(mut self, standby: bool) -> Self {
        let col = self.qualify_column("standby");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Bool(standby));
        self
    }

    /// Filter by entity_type (for entity_tags junction table)
    pub fn entity_type(mut self, entity_type: &EntityDiscriminants) -> Self {
        let col = self.qualify_column("entity_type");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        // Use EntityDiscriminant to match JSON serialization used when inserting
        self.values.push(SqlValue::EntityDiscriminant(*entity_type));
        self
    }

    /// Filter by tag_id (for entity_tags junction table)
    pub fn tag_id(mut self, id: &Uuid) -> Self {
        let col = self.qualify_column("tag_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*id));
        self
    }

    /// Filter entities that have ANY of the specified tags.
    /// Uses a subquery against the entity_tags junction table.
    ///
    /// Example SQL: `entities.id IN (SELECT entity_id FROM entity_tags WHERE entity_type = 'Service' AND tag_id IN ($1, $2))`
    pub fn has_any_tags(mut self, tag_ids: &[Uuid], entity_type: EntityDiscriminants) -> Self {
        if tag_ids.is_empty() {
            return self;
        }

        let col = self.qualify_column("id");
        let entity_type_idx = self.values.len() + 1;
        let placeholders: Vec<String> = tag_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", self.values.len() + i + 2))
            .collect();

        self.conditions.push(format!(
            "{} IN (SELECT entity_id FROM entity_tags WHERE entity_type = ${} AND tag_id IN ({}))",
            col,
            entity_type_idx,
            placeholders.join(", ")
        ));

        self.values.push(SqlValue::EntityDiscriminant(entity_type));
        for id in tag_ids {
            self.values.push(SqlValue::Uuid(*id));
        }

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

    // =========================================================================
    // LLDP resolution filters
    // =========================================================================

    /// Filter by IP address (for ip_addresses table)
    pub fn ip_address(mut self, ip: std::net::IpAddr) -> Self {
        let col = self.qualify_column("ip_address");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::IpAddr(ip));
        self
    }

    /// Filter by if_descr (for interfaces table)
    pub fn if_descr(mut self, descr: &str) -> Self {
        let col = self.qualify_column("if_descr");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(descr.to_string()));
        self
    }

    /// Filter by if_name (for interfaces table)
    pub fn if_name(mut self, name: &str) -> Self {
        let col = self.qualify_column("if_name");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(name.to_string()));
        self
    }

    /// Filter by chassis_id (for hosts table)
    pub fn chassis_id(mut self, chassis_id: &str) -> Self {
        let col = self.qualify_column("chassis_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(chassis_id.to_string()));
        self
    }

    /// Filter by sys_name (for hosts table)
    pub fn sys_name(mut self, sys_name: &str) -> Self {
        let col = self.qualify_column("sys_name");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(sys_name.to_string()));
        self
    }

    /// Filter by ip_address_id FK (for interfaces table)
    pub fn ip_address_id(mut self, ip_address_id: &Uuid) -> Self {
        let col = self.qualify_column("ip_address_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(*ip_address_id));
        self
    }

    /// Filter interfaces with unresolved LLDP/CDP neighbors in a network.
    /// Matches entries that have LLDP or CDP data but no neighbor (neither interface nor host).
    pub fn unresolved_lldp_in_network(mut self, network_id: Uuid) -> Self {
        let network_col = self.qualify_column("network_id");
        let lldp_chassis_col = self.qualify_column("lldp_chassis_id");
        let cdp_device_col = self.qualify_column("cdp_device_id");
        let cdp_addr_col = self.qualify_column("cdp_address");
        let neighbor_if_entry_col = self.qualify_column("neighbor_interface_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");

        self.conditions
            .push(format!("{} = ${}", network_col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(network_id));

        // Has LLDP or CDP data but not yet resolved (no neighbor of either type)
        self.conditions.push(format!(
            "({} IS NOT NULL OR {} IS NOT NULL OR {} IS NOT NULL)",
            lldp_chassis_col, cdp_device_col, cdp_addr_col
        ));
        self.conditions
            .push(format!("{} IS NULL", neighbor_if_entry_col));
        self.conditions
            .push(format!("{} IS NULL", neighbor_host_col));

        self
    }

    /// Filter interfaces with unresolved single-MAC FDB data in a network.
    /// Matches entries that have exactly 1 learned MAC, no existing neighbor,
    /// and no LLDP/CDP data (FDB is lower-priority than protocol-based discovery).
    pub fn unresolved_fdb_in_network(mut self, network_id: Uuid) -> Self {
        let network_col = self.qualify_column("network_id");
        let fdb_col = self.qualify_column("fdb_macs");
        let neighbor_if_entry_col = self.qualify_column("neighbor_interface_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");
        let lldp_chassis_col = self.qualify_column("lldp_chassis_id");
        let cdp_device_col = self.qualify_column("cdp_device_id");

        self.conditions
            .push(format!("{} = ${}", network_col, self.values.len() + 1));
        self.values.push(SqlValue::Uuid(network_id));

        // Has single-MAC FDB data, no neighbor, no LLDP/CDP
        self.conditions.push(format!(
            "{} IS NOT NULL AND jsonb_array_length({}) = 1",
            fdb_col, fdb_col
        ));
        self.conditions
            .push(format!("{} IS NULL", neighbor_if_entry_col));
        self.conditions
            .push(format!("{} IS NULL", neighbor_host_col));
        self.conditions
            .push(format!("{} IS NULL", lldp_chassis_col));
        self.conditions.push(format!("{} IS NULL", cdp_device_col));

        self
    }

    /// Filter interfaces that have any resolved neighbor (full or partial resolution)
    pub fn has_neighbor(mut self) -> Self {
        let neighbor_if_entry_col = self.qualify_column("neighbor_interface_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");

        self.conditions.push(format!(
            "({} IS NOT NULL OR {} IS NOT NULL)",
            neighbor_if_entry_col, neighbor_host_col
        ));

        self
    }

    /// Filter interfaces with full neighbor resolution (specific remote port known)
    pub fn has_neighbor_if_entry(mut self) -> Self {
        let col = self.qualify_column("neighbor_interface_id");
        self.conditions.push(format!("{} IS NOT NULL", col));
        self
    }

    /// Filter interfaces connected to a specific host (either resolution type)
    pub fn neighbor_host(mut self, host_id: Uuid) -> Self {
        let neighbor_if_entry_col = self.qualify_column("neighbor_interface_id");
        let neighbor_host_col = self.qualify_column("neighbor_host_id");

        // Either directly connected to host (partial resolution)
        // Or connected to an interface on that host (full resolution)
        // For full resolution, we need a subquery
        self.conditions.push(format!(
            "({} = ${} OR {} IN (SELECT id FROM interfaces WHERE host_id = ${}))",
            neighbor_host_col,
            self.values.len() + 1,
            neighbor_if_entry_col,
            self.values.len() + 1
        ));
        self.values.push(SqlValue::Uuid(host_id));

        self
    }

    // =========================================================================
    // Organization filters
    // =========================================================================

    /// Filter for organizations that haven't been synced to Brevo yet
    pub fn without_brevo_company_id(mut self) -> Self {
        let col = self.qualify_column("brevo_company_id");
        self.conditions.push(format!("{} IS NULL", col));
        self
    }

    /// Filter for organizations that have already been synced to Brevo
    pub fn with_brevo_company_id(mut self) -> Self {
        let col = self.qualify_column("brevo_company_id");
        self.conditions.push(format!("{} IS NOT NULL", col));
        self
    }

    /// Filter for organizations by Stripe customer ID
    pub fn stripe_customer_id(mut self, id: &str) -> Self {
        let col = self.qualify_column("stripe_customer_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(id.to_string()));
        self
    }

    /// Filter for credentials that have non-null target_ips
    pub fn with_target_ips(mut self) -> Self {
        let col = self.qualify_column("target_ips");
        self.conditions.push(format!("{} IS NOT NULL", col));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hosts::r#impl::base::Host;
    use crate::server::snapshots::types::base::Snapshot;
    use crate::server::tags::r#impl::base::Tag;
    use chrono::TimeZone;

    fn ts(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).unwrap()
    }

    #[test]
    fn id_or_lineage_in_emits_or_clause() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let filter = StorableFilter::<Tag>::new_unfiltered().id_or_lineage_in(&ids);

        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.id = ANY($1)"),
            "expected id ANY clause in: {}",
            where_clause
        );
        assert!(
            where_clause.contains("tags.lineage_id = ANY($2)"),
            "expected lineage_id ANY clause in: {}",
            where_clause
        );
        assert!(
            where_clause.contains(" OR "),
            "expected OR-join in: {}",
            where_clause
        );
        assert_eq!(
            filter.values().len(),
            2,
            "expected two array params (id_set, lineage_set)"
        );
    }

    #[test]
    fn id_or_lineage_in_empty_input_matches_nothing() {
        let filter = StorableFilter::<Tag>::new_unfiltered().id_or_lineage_in(&[]);
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("FALSE"),
            "empty id list should produce FALSE: {}",
            where_clause
        );
        assert_eq!(
            filter.values().len(),
            0,
            "no params should be bound for empty input"
        );
    }

    #[test]
    fn taken_at_lt_emits_less_than_clause() {
        let cutoff = chrono::Utc::now();
        let filter = StorableFilter::<Snapshot>::new_unfiltered().taken_at_lt(cutoff);

        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("snapshots.taken_at < $1"),
            "expected `taken_at < $N` in: {}",
            where_clause
        );
        assert_eq!(filter.values().len(), 1);
    }

    #[test]
    fn live_filter_emits_valid_to_is_null() {
        let filter = StorableFilter::<Tag>::new_unfiltered().live();
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_to IS NULL"),
            "expected `valid_to IS NULL` in: {}",
            where_clause
        );
    }

    #[test]
    fn as_of_filter_emits_window_predicate() {
        let t = chrono::Utc::now();
        let filter = StorableFilter::<Tag>::new_unfiltered().as_of(t);
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_from <= $1"),
            "expected lower bound in: {}",
            where_clause
        );
        assert!(
            where_clause.contains("tags.valid_to IS NULL OR tags.valid_to > $2"),
            "expected upper bound in: {}",
            where_clause
        );
    }

    #[test]
    fn live_or_as_of_none_is_live() {
        // `at = None` (live view) must hide closed historical copies, i.e. behave
        // exactly like `.live()`. This is the read-path guard that stops snapshot
        // close-and-clone copies leaking into entity lists as empty-shell dupes.
        let filter = StorableFilter::<Tag>::new_unfiltered().live_or_as_of(None);
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_to IS NULL"),
            "expected live predicate in: {}",
            where_clause
        );
        assert!(
            !where_clause.contains("valid_from <="),
            "live view must not emit an as-of window: {}",
            where_clause
        );
    }

    #[test]
    fn live_or_as_of_some_is_as_of() {
        // `at = Some(t)` (snapshot view) must read SCD2 state as of `t`.
        let t = chrono::Utc::now();
        let filter = StorableFilter::<Tag>::new_unfiltered().live_or_as_of(Some(t));
        let where_clause = filter.to_where_clause();
        assert!(
            where_clause.contains("tags.valid_from <= $1"),
            "expected as-of lower bound in: {}",
            where_clause
        );
        assert!(
            where_clause.contains("tags.valid_to IS NULL OR tags.valid_to > $2"),
            "expected as-of upper bound in: {}",
            where_clause
        );
    }

    #[test]
    fn created_between_emits_inclusive_range() {
        let f = StorableFilter::<Host>::new().created_between(ts(100), ts(200));
        assert_eq!(f.conditions.len(), 1);
        let c = &f.conditions[0];
        assert!(c.contains("created_at"), "condition was: {c}");
        assert!(
            c.contains(">= $1") && c.contains("<= $2"),
            "condition was: {c}"
        );
        assert_eq!(f.values.len(), 2);
        match (&f.values[0], &f.values[1]) {
            (SqlValue::Timestamp(a), SqlValue::Timestamp(b)) => {
                assert_eq!(*a, ts(100));
                assert_eq!(*b, ts(200));
            }
            _ => panic!("expected two Timestamp values"),
        }
    }

    #[test]
    fn updated_between_uses_updated_at_column() {
        let f = StorableFilter::<Host>::new().updated_between(ts(0), ts(1));
        assert!(f.conditions[0].contains("updated_at"));
    }

    #[test]
    fn valid_to_between_uses_valid_to_column() {
        let f = StorableFilter::<Host>::new().valid_to_between(ts(0), ts(1));
        assert!(f.conditions[0].contains("valid_to"));
    }

    #[test]
    fn last_seen_between_uses_last_seen_at_column() {
        let f = StorableFilter::<Host>::new().last_seen_between(ts(0), ts(1));
        assert!(f.conditions[0].contains("last_seen_at"));
    }

    #[test]
    fn between_helpers_advance_param_index() {
        let f = StorableFilter::<Host>::new()
            .created_between(ts(10), ts(20))
            .updated_between(ts(30), ts(40));
        assert_eq!(f.values.len(), 4);
        assert!(f.conditions[0].contains("$1") && f.conditions[0].contains("$2"));
        assert!(f.conditions[1].contains("$3") && f.conditions[1].contains("$4"));
    }

    #[test]
    fn user_permissions_in_emits_in_clause() {
        let f = StorableFilter::<crate::server::users::r#impl::base::User>::new()
            .user_permissions_in(&[UserOrgPermissions::Owner, UserOrgPermissions::Admin]);
        assert_eq!(f.conditions.len(), 1);
        let c = &f.conditions[0];
        assert!(c.contains("permissions"));
        assert!(c.contains("IN ($1, $2)"), "condition was: {c}");
        assert_eq!(f.values.len(), 2);
    }

    #[test]
    fn user_permissions_in_empty_emits_false() {
        let f = StorableFilter::<crate::server::users::r#impl::base::User>::new()
            .user_permissions_in(&[]);
        assert_eq!(f.conditions, vec!["FALSE".to_string()]);
        assert_eq!(f.values.len(), 0);
    }
}
