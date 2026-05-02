-- no-transaction

-- Supporting indexes for SCD2 read paths:
--
-- _live: backs the current-state filter `valid_to IS NULL` in topology read
--        paths (eight `get_all` calls) and reconciliation pre-match queries.
-- _as_of: backs as-of queries `valid_from <= T AND (valid_to IS NULL OR
--         valid_to > T)`. Composite covers both bounds. Not partial — closed
--         rows participate in as-of reads.
-- _lineage: backs the parent-lookup subqueries during snapshot close-and-clone
--           (closed_id by lineage_id at a given valid_to) and the
--           "all versions of entity X" lineage exploration query. Filtered
--           on closed rows since live rows have lineage_id = NULL.

-- Note: SET lock_timeout / statement_timeout are intentionally NOT issued
-- here. PostgreSQL's simple_query protocol treats every multi-statement
-- send as a single implicit transaction, which conflicts with CREATE
-- INDEX CONCURRENTLY's "cannot run inside a transaction block" rule
-- regardless of the `-- no-transaction` header. Operators running this
-- migration manually can SET these in their psql session before applying.
--
-- DiscoveryTracked tables — most are network-scoped.
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_hosts_live ON hosts (network_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_hosts_as_of ON hosts (network_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_hosts_lineage ON hosts (lineage_id) WHERE valid_to IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_ip_addresses_live ON ip_addresses (network_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_ip_addresses_as_of ON ip_addresses (network_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_ip_addresses_lineage ON ip_addresses (lineage_id) WHERE valid_to IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_ports_live ON ports (network_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_ports_as_of ON ports (network_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_ports_lineage ON ports (lineage_id) WHERE valid_to IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_services_live ON services (network_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_services_as_of ON services (network_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_services_lineage ON services (lineage_id) WHERE valid_to IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_interfaces_live ON interfaces (network_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_interfaces_as_of ON interfaces (network_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_interfaces_lineage ON interfaces (lineage_id) WHERE valid_to IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_bindings_live ON bindings (network_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_bindings_as_of ON bindings (network_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_bindings_lineage ON bindings (lineage_id) WHERE valid_to IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_subnets_live ON subnets (network_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_subnets_as_of ON subnets (network_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_subnets_lineage ON subnets (lineage_id) WHERE valid_to IS NOT NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_vlans_live ON vlans (network_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_vlans_as_of ON vlans (network_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_vlans_lineage ON vlans (lineage_id) WHERE valid_to IS NOT NULL;

-- subnet_vlans junction has no network_id; key by subnet_id (parent on the
-- network side). The snapshot close-and-clone filters via subnet membership
-- in the snapshot's network.
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_subnet_vlans_live ON subnet_vlans (subnet_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_subnet_vlans_as_of ON subnet_vlans (subnet_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_subnet_vlans_lineage ON subnet_vlans (lineage_id) WHERE valid_to IS NOT NULL;

-- dependencies network-scoped.
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dependencies_live ON dependencies (network_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dependencies_as_of ON dependencies (network_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dependencies_lineage ON dependencies (lineage_id) WHERE valid_to IS NOT NULL;

-- dependency_members junction: key by dependency_id.
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dependency_members_live ON dependency_members (dependency_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dependency_members_as_of ON dependency_members (dependency_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dependency_members_lineage ON dependency_members (lineage_id) WHERE valid_to IS NOT NULL;

-- tags org-scoped.
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_tags_live ON tags (organization_id) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_tags_as_of ON tags (organization_id, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_tags_lineage ON tags (lineage_id) WHERE valid_to IS NOT NULL;

-- entity_tags junction: key by (entity_id, entity_type) for current-state
-- "tags on entity X" lookups.
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entity_tags_live ON entity_tags (entity_id, entity_type) WHERE valid_to IS NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entity_tags_as_of ON entity_tags (entity_id, entity_type, valid_from, valid_to);
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entity_tags_lineage ON entity_tags (lineage_id) WHERE valid_to IS NOT NULL;
