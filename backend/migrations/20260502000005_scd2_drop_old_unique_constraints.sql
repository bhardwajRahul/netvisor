-- Drop old inline UNIQUE constraints. These are auto-named PG constraints
-- (table_columns_key) on tables; ALTER TABLE DROP CONSTRAINT must run in a
-- transaction. The old UNIQUE indexes that were created as CREATE [UNIQUE]
-- INDEX (rather than as inline constraints) are dropped CONCURRENTLY in the
-- previous no-transaction migration.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

-- ports inline UNIQUE → auto-named ports_host_id_port_number_protocol_key.
ALTER TABLE ports DROP CONSTRAINT IF EXISTS ports_host_id_port_number_protocol_key;

-- ip_addresses (table renamed from interfaces in 20260410000000) — the
-- constraint name was generated against the old table name and survived
-- the rename.
ALTER TABLE ip_addresses DROP CONSTRAINT IF EXISTS interfaces_host_id_subnet_id_ip_address_key;

-- entity_tags inline UNIQUE → auto-named.
ALTER TABLE entity_tags DROP CONSTRAINT IF EXISTS entity_tags_entity_id_entity_type_tag_id_key;

-- dependency_members named CONSTRAINT.
ALTER TABLE dependency_members DROP CONSTRAINT IF EXISTS dependency_members_dep_service_unique;

-- subnet_vlans inline UNIQUE → auto-named.
ALTER TABLE subnet_vlans DROP CONSTRAINT IF EXISTS subnet_vlans_subnet_id_vlan_id_key;
