-- no-transaction

-- Replace existing UNIQUE constraints on natural keys with partial UNIQUE
-- indexes filtered on `valid_to IS NULL`. Closed historical rows must NOT
-- participate in uniqueness — at any moment, multiple closed copies plus
-- one live row can coexist for the same natural key.
--
-- This migration only CREATES the new partial indexes. The next migration
-- (scd2_drop_old_unique_constraints) drops the now-redundant non-partial
-- UNIQUEs. Doing the create-first / drop-second ordering ensures no window
-- where the natural key is unenforced.

-- statement_timeout is permissive (0 = no limit) since CREATE INDEX
-- CONCURRENTLY can run long on large tables; lock_timeout still bounds the
-- brief lock acquisition inside the build.
SET lock_timeout = '5s';
SET statement_timeout = '0';

CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_ports_unique_live
    ON ports (host_id, port_number, protocol)
    WHERE valid_to IS NULL;

CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_ip_addresses_unique_live
    ON ip_addresses (host_id, subnet_id, ip_address)
    WHERE valid_to IS NULL;

-- Replaces idx_interfaces_host_name from 20260417000000_reindex_interfaces_identity.sql.
-- The existing index lives on under the same name; we create the new one with
-- _live suffix and drop the old in the drop-old migration.
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_interfaces_host_name_live
    ON interfaces (host_id, if_name)
    WHERE if_name IS NOT NULL AND valid_to IS NULL;

-- Replaces idx_tags_org_name from 20251210045929_tags.sql.
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_tags_org_name_live
    ON tags (organization_id, name)
    WHERE valid_to IS NULL;

-- Replaces the inline UNIQUE constraint on entity_tags
-- (entity_tags_entity_id_entity_type_tag_id_key from 20260106204402_entity_tags_junction.sql).
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_entity_tags_unique_live
    ON entity_tags (entity_id, entity_type, tag_id)
    WHERE valid_to IS NULL;

-- Replaces the inline UNIQUE constraint on vlans
-- (idx_vlans_network_number from 20260406130000_add_vlans.sql).
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_vlans_network_number_live
    ON vlans (network_id, vlan_number)
    WHERE valid_to IS NULL;

-- Replaces dependency_members_dep_service_unique
-- (from 20260405120000_rename_groups_to_dependencies.sql).
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_dependency_members_unique_live
    ON dependency_members (dependency_id, service_id)
    WHERE valid_to IS NULL;

-- Replaces the inline UNIQUE constraint on subnet_vlans (subnet_id, vlan_id).
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_subnet_vlans_unique_live
    ON subnet_vlans (subnet_id, vlan_id)
    WHERE valid_to IS NULL;

-- Drop old non-partial UNIQUE indexes (created via CREATE [UNIQUE] INDEX in
-- prior migrations, not as inline constraints). These need DROP INDEX
-- CONCURRENTLY which can't be in a transaction; co-located here in the
-- no-transaction migration. The ALTER TABLE DROP CONSTRAINT steps for inline
-- UNIQUE constraints live in the next migration (must run in a transaction).
DROP INDEX CONCURRENTLY IF EXISTS idx_interfaces_host_name;
DROP INDEX CONCURRENTLY IF EXISTS idx_tags_org_name;
DROP INDEX CONCURRENTLY IF EXISTS idx_vlans_network_number;
