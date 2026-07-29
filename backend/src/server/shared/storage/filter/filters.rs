//! Column qualification, pagination, joins, and the core entity/column/timestamp/tag filter builders.
use super::*;

impl<T: Storable> StorableFilter<T> {
    /// Qualify a column name with the table name.
    pub(crate) fn qualify_column(&self, column: &str) -> String {
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

    /// Case-insensitive free-text search across `T::search_predicates()`.
    ///
    /// The entity's fragments are OR'd together and each `{}` is replaced with
    /// the one bound `%pattern%` parameter, so a multi-column search costs a
    /// single bind. LIKE wildcards typed by the user are escaped: searching
    /// `100%` looks for that literal string rather than matching every row.
    ///
    /// Matches nothing when the query is blank or the entity declares no
    /// predicates. Falling through to "no condition" would answer an
    /// unsupported search with the entire table, which reads as success.
    pub fn text_search(mut self, query: &str) -> Self {
        let predicates = T::search_predicates();
        let trimmed = query.trim();

        if predicates.is_empty() || trimmed.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }

        let param = format!("${}", self.values.len() + 1);
        let clauses: Vec<String> = predicates.iter().map(|p| p.replace("{}", &param)).collect();

        self.conditions.push(format!("({})", clauses.join(" OR ")));
        self.values
            .push(SqlValue::String(format!("%{}%", escape_like(trimmed))));
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

    /// Transient one-shot rescan configurations only.
    pub fn targeted_discovery(mut self) -> Self {
        self.conditions
            .push("run_type->>'type' = 'Targeted'".to_string());
        self
    }

    /// Exclude historical rows produced by a one-shot rescan.
    ///
    /// The flag lives inside the persisted session payload
    /// (`run_type.results.targeted`) because the transient `Targeted` parent is
    /// deleted at terminal, leaving the historical row's own `run_type` as a
    /// plain `Historical`.
    pub fn exclude_targeted_results(mut self) -> Self {
        self.conditions.push(
            "COALESCE((run_type->'results'->>'targeted')::boolean, false) = false".to_string(),
        );
        self
    }

    /// Discovery *configurations* a user owns — excludes `Historical` records of
    /// past runs and transient `Targeted` rescan rows, neither of which is
    /// something anyone configured. Mirrors `RunType::is_live_config`.
    ///
    /// Prefer this over [`Self::exclude_historical`] anywhere the question is
    /// "does this daemon have discoveries the user configured": a `Targeted` row
    /// left behind by a crashed session would otherwise read as one, and at
    /// `create_default_discovery_jobs` that permanently blocks the daemon from
    /// ever getting its default discovery.
    pub fn live_configs(mut self) -> Self {
        self.conditions
            .push("run_type->>'type' NOT IN ('Historical', 'Targeted')".to_string());
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

    /// Restrict to subnets of a given type. `subnet_type` is a plain text column
    /// holding the variant name (see `SubnetType::id`).
    pub fn subnet_type(mut self, subnet_type: &str) -> Self {
        let col = self.qualify_column("subnet_type");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(subnet_type.to_string()));
        self
    }

    pub fn last_seen_before(mut self, timestamp: DateTime<Utc>) -> Self {
        let col = self.qualify_column("last_seen_at");
        self.conditions
            .push(format!("{} < ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::Timestamp(timestamp));
        self
    }

    /// SQL form of the staleness verdict, evaluated per row against **its own**
    /// network's cutoff.
    ///
    /// The entity lists span every network the caller can reach and are
    /// server-paginated, so a single cutoff would be wrong (each network
    /// configures its own window) and a client-side filter would only filter
    /// the current page. `cutoffs` is `(network_id, cutoff_instant)`, resolved
    /// by the handler from each network's `stale_after_hours`.
    ///
    /// Emits one parenthesised OR-of-ANDs, in the same shape as
    /// [`Self::id_or_lineage_in`], plus the discovery-managed guard: an entity
    /// discovery never refreshes (manual, system) has a frozen `last_seen_at`
    /// and must never be reported stale. This mirrors
    /// [`DiscoveryTracked::freshness`](crate::server::shared::storage::snapshot::DiscoveryTracked::freshness)
    /// — the two must be changed together.
    ///
    /// `stale = false` inverts to "fresh or not discovery-managed". An empty
    /// `cutoffs` pushes `FALSE`, matching `id_or_lineage_in`'s precedent for an
    /// empty input rather than silently matching everything.
    pub fn stale_by_network(mut self, cutoffs: &[(Uuid, DateTime<Utc>)], stale: bool) -> Self {
        if cutoffs.is_empty() {
            self.conditions.push("FALSE".to_string());
            return self;
        }
        let network_col = self.qualify_column("network_id");
        let seen_col = self.qualify_column("last_seen_at");
        let source_col = self.qualify_column("source");
        // Only entities discovery actually refreshes can go stale.
        let managed = format!("{source_col}->>'type' IN ('Discovery', 'DiscoveryWithMatch')");
        let comparison = if stale { "<" } else { ">=" };

        let mut clauses = Vec::with_capacity(cutoffs.len());
        for (network_id, cutoff) in cutoffs {
            let net_idx = self.values.len() + 1;
            let cutoff_idx = self.values.len() + 2;
            clauses.push(format!(
                "({network_col} = ${net_idx} AND {seen_col} {comparison} ${cutoff_idx})"
            ));
            self.values.push(SqlValue::Uuid(*network_id));
            self.values.push(SqlValue::Timestamp(*cutoff));
        }
        let per_network = clauses.join(" OR ");

        self.conditions.push(if stale {
            format!("({managed} AND ({per_network}))")
        } else {
            // Not stale = inside its window, or not discovery-managed at all.
            format!("(NOT ({managed}) OR ({per_network}))")
        });
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

    /// Entities exposed on one of `ports` and/or over `protocol`, matched
    /// through the `bindings` junction.
    ///
    /// `IN (subquery)` rather than a JOIN so a service bound to several ports is
    /// still one row — a JOIN would repeat it and inflate the paginated
    /// `COUNT(*)`. Both predicates are evaluated against the same port row, so
    /// asking for 53 over TCP cannot be satisfied by a service that has a 53/UDP
    /// binding and some unrelated TCP one.
    pub fn bound_to_port(mut self, ports: &[u16], protocol: Option<TransportProtocol>) -> Self {
        let mut predicates = Vec::new();

        if !ports.is_empty() {
            let placeholders: Vec<String> = ports
                .iter()
                .enumerate()
                .map(|(i, _)| format!("${}", self.values.len() + i + 1))
                .collect();
            predicates.push(format!("p.port_number IN ({})", placeholders.join(", ")));
            for port in ports {
                self.values.push(SqlValue::I32(i32::from(*port)));
            }
        }

        if let Some(protocol) = protocol {
            self.values.push(SqlValue::String(protocol.to_string()));
            predicates.push(format!("p.protocol = ${}", self.values.len()));
        }

        if predicates.is_empty() {
            return self;
        }

        let col = self.qualify_column("id");
        self.conditions.push(format!(
            "{} IN (SELECT b.service_id FROM bindings b \
             JOIN ports p ON b.port_id = p.id \
             WHERE b.valid_to IS NULL AND p.valid_to IS NULL AND {})",
            col,
            predicates.join(" AND ")
        ));

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

/// Neutralize LIKE wildcards in user input so they match literally.
/// Postgres' default LIKE escape character is a backslash, so no `ESCAPE`
/// clause is needed alongside this.
fn escape_like(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}
