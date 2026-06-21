-- no-transaction

-- Indexes on snapshot_id for cascade delete performance + the
-- "find rows for snapshot X" path. Each statement runs as its own PG
-- simple_query via the custom migration runner (see
-- server/shared/storage/migration_runner.rs).

SET lock_timeout = '5s';

SET statement_timeout = '0';

-- topologies has no snapshot_id column (snapshot graphs build on request).

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_hosts_snapshot_id ON hosts (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_ip_addresses_snapshot_id ON ip_addresses (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_ports_snapshot_id ON ports (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_services_snapshot_id ON services (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_interfaces_snapshot_id ON interfaces (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_bindings_snapshot_id ON bindings (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_subnets_snapshot_id ON subnets (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_vlans_snapshot_id ON vlans (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_subnet_vlans_snapshot_id ON subnet_vlans (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dependencies_snapshot_id ON dependencies (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_dependency_members_snapshot_id ON dependency_members (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_tags_snapshot_id ON tags (snapshot_id) WHERE snapshot_id IS NOT NULL;
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_entity_tags_snapshot_id ON entity_tags (snapshot_id) WHERE snapshot_id IS NOT NULL;
