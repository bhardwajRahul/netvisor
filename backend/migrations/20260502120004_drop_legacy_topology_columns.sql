-- DOWNTIME MIGRATION
--
-- Strips the legacy lock/stale/parent state machine + entity-blob JSONB cache
-- + name/tags from the topologies table. After this migration, a topology row
-- holds only graph-layout state (nodes/edges/options) plus its identity
-- (network_id, snapshot_id). The point-in-time and entity-state semantics live
-- in the snapshots table + SCD2 substrate respectively.
--
-- Squawk will flag every DROP COLUMN as unsafe (correctness in zero-downtime
-- deploys requires expand-and-contract). This migration is run during a
-- coordinated downtime window — see CHANGELOG / release notes for the
-- deploy sequence — so we accept the unsafety here.

SET lock_timeout = '60s';
SET statement_timeout = '0';

ALTER TABLE topologies
    DROP COLUMN parent_id,
    DROP COLUMN is_locked,
    DROP COLUMN locked_at,
    DROP COLUMN locked_by,
    DROP COLUMN is_stale,
    DROP COLUMN last_refreshed,
    DROP COLUMN removed_hosts,
    DROP COLUMN removed_ip_addresses,
    DROP COLUMN removed_subnets,
    DROP COLUMN removed_services,
    DROP COLUMN removed_dependencies,
    DROP COLUMN removed_ports,
    DROP COLUMN removed_bindings,
    DROP COLUMN removed_interfaces,
    DROP COLUMN hosts,
    DROP COLUMN ip_addresses,
    DROP COLUMN ports,
    DROP COLUMN bindings,
    DROP COLUMN subnets,
    DROP COLUMN services,
    DROP COLUMN dependencies,
    DROP COLUMN interfaces,
    DROP COLUMN entity_tags,
    DROP COLUMN vlans,
    DROP COLUMN tags,
    DROP COLUMN name;
