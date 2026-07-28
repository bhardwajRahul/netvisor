-- Rename the constraints left stale by the April 2026 entity rename
-- (20260410000000_rename_interfaces_and_if_entries.sql). That migration renamed
-- the tables, columns and idx_* indexes, but not the constraints: Postgres
-- preserves constraint names across ALTER TABLE ... RENAME. So a foreign-key
-- violation on `interfaces` still reports `if_entries_interface_id_fkey`, naming
-- a table that has not existed since April and a column (`interface_id`) that
-- was renamed to `ip_address_id` -- actively misleading during an incident. The
-- staleness runs both ways: `ip_addresses` (formerly `interfaces`) carries
-- `interfaces_*` names, which are worse still, because they name a table that
-- does exist but is now a different table.
--
-- All twelve names are Postgres auto-generated (<table>_<column>_fkey,
-- <table>_pkey) -- 20260116030000_if_entries.sql and
-- 20251221040000_interfaces_table.sql declare only inline PRIMARY KEY /
-- REFERENCES with no CONSTRAINT clause -- so every database that replayed this
-- migration chain has exactly these names. Bare renames are therefore safe, and
-- unlike a conditional rename they fail loudly if that ever stops being true
-- rather than silently leaving the misleading name in place.
--
-- Metadata-only: one catalog row per constraint, no table rewrite, no index
-- rebuild, no FK revalidation. sqlx wraps the file in a transaction, so the
-- ACCESS EXCLUSIVE locks on all three tables are held until commit -- still
-- sub-millisecond, and lock_timeout bounds the wait to acquire them.
--
-- Safe in one deploy, so this is not an expand/contract change and has no
-- ledger entry. The only code that reads constraint names is
-- GenericPostgresStorage::friendly_unique_violation_message
-- (server/shared/storage/generic.rs), which substring-matches on unique
-- violations only; none of the twelve is a unique constraint except the two
-- primary keys, whose collision would be a UUIDv4 collision (not a reachable
-- state). An old container mid-rolling-deploy is unaffected.
--
-- STATEMENT ORDER MATTERS. RENAME CONSTRAINT on an index-backed constraint (a
-- primary key) also renames the backing index, and index names are unique per
-- schema, not per table. Today `if_entries_pkey` indexes `interfaces` while
-- `interfaces_pkey` indexes `ip_addresses`, so renaming interfaces.if_entries_pkey
-- to `interfaces_pkey` first fails with `relation "interfaces_pkey" already
-- exists`. Both primary keys are therefore renamed first, ip_addresses before
-- interfaces, so ip_addresses releases the name before interfaces claims it.
-- Foreign keys have no backing index and are namespaced per table, so their
-- relative order is unconstrained.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

-- Primary keys first, ip_addresses before interfaces (see header).
ALTER TABLE ip_addresses RENAME CONSTRAINT interfaces_pkey TO ip_addresses_pkey;
ALTER TABLE interfaces RENAME CONSTRAINT if_entries_pkey TO interfaces_pkey;

-- ip_addresses (formerly `interfaces`).
ALTER TABLE ip_addresses RENAME CONSTRAINT interfaces_host_id_fkey TO ip_addresses_host_id_fkey;
ALTER TABLE ip_addresses RENAME CONSTRAINT interfaces_network_id_fkey TO ip_addresses_network_id_fkey;
ALTER TABLE ip_addresses RENAME CONSTRAINT interfaces_subnet_id_fkey TO ip_addresses_subnet_id_fkey;

-- interfaces (formerly `if_entries`). `if_entries_interface_id_fkey` is the one
-- that surfaced this: it guards interfaces.ip_address_id -> ip_addresses(id).
ALTER TABLE interfaces RENAME CONSTRAINT if_entries_host_id_fkey TO interfaces_host_id_fkey;
ALTER TABLE interfaces RENAME CONSTRAINT if_entries_network_id_fkey TO interfaces_network_id_fkey;
ALTER TABLE interfaces RENAME CONSTRAINT if_entries_interface_id_fkey TO interfaces_ip_address_id_fkey;
ALTER TABLE interfaces RENAME CONSTRAINT if_entries_native_vlan_id_fkey TO interfaces_native_vlan_id_fkey;
ALTER TABLE interfaces RENAME CONSTRAINT if_entries_neighbor_host_id_fkey TO interfaces_neighbor_host_id_fkey;
ALTER TABLE interfaces RENAME CONSTRAINT if_entries_neighbor_if_entry_id_fkey TO interfaces_neighbor_interface_id_fkey;

-- bindings.interface_id was renamed to ip_address_id in April; its FK was not.
ALTER TABLE bindings RENAME CONSTRAINT bindings_interface_id_fkey TO bindings_ip_address_id_fkey;
