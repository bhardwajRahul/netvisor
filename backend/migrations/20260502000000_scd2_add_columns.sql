-- SCD2 foundation: add valid_from, valid_to, lineage_id to all 13 Snapshotable
-- tables. DiscoveryTracked tables additionally get last_seen_at,
-- last_discovery_id, first_discovery_id (nullable UUID columns; FK constraints
-- to discoveries(id) are added in the next migration via NOT VALID + VALIDATE).
--
-- valid_from / last_seen_at default to NOW() so existing rows behave as live
-- rows (valid_to NULL) starting at this migration. The next migration
-- (scd2_backfill_with_metadata) refines those values from created_at /
-- updated_at / source.metadata where available.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

-- DiscoveryTracked tables (9): get full SCD2 + audit columns.
ALTER TABLE hosts
    ADD COLUMN valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to           TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id         UUID        NULL,
    ADD COLUMN last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN last_discovery_id  UUID        NULL,
    ADD COLUMN first_discovery_id UUID        NULL;

ALTER TABLE ip_addresses
    ADD COLUMN valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to           TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id         UUID        NULL,
    ADD COLUMN last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN last_discovery_id  UUID        NULL,
    ADD COLUMN first_discovery_id UUID        NULL;

ALTER TABLE ports
    ADD COLUMN valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to           TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id         UUID        NULL,
    ADD COLUMN last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN last_discovery_id  UUID        NULL,
    ADD COLUMN first_discovery_id UUID        NULL;

ALTER TABLE services
    ADD COLUMN valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to           TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id         UUID        NULL,
    ADD COLUMN last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN last_discovery_id  UUID        NULL,
    ADD COLUMN first_discovery_id UUID        NULL;

ALTER TABLE interfaces
    ADD COLUMN valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to           TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id         UUID        NULL,
    ADD COLUMN last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN last_discovery_id  UUID        NULL,
    ADD COLUMN first_discovery_id UUID        NULL;

ALTER TABLE bindings
    ADD COLUMN valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to           TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id         UUID        NULL,
    ADD COLUMN last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN last_discovery_id  UUID        NULL,
    ADD COLUMN first_discovery_id UUID        NULL;

ALTER TABLE subnets
    ADD COLUMN valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to           TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id         UUID        NULL,
    ADD COLUMN last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN last_discovery_id  UUID        NULL,
    ADD COLUMN first_discovery_id UUID        NULL;

ALTER TABLE vlans
    ADD COLUMN valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to           TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id         UUID        NULL,
    ADD COLUMN last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN last_discovery_id  UUID        NULL,
    ADD COLUMN first_discovery_id UUID        NULL;

ALTER TABLE subnet_vlans
    ADD COLUMN valid_from         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to           TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id         UUID        NULL,
    ADD COLUMN last_seen_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN last_discovery_id  UUID        NULL,
    ADD COLUMN first_discovery_id UUID        NULL;

-- Snapshotable-only tables (4): just SCD2 columns. No discovery FK columns
-- because these aren't scanned by daemon discovery (user-managed).
ALTER TABLE dependencies
    ADD COLUMN valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to   TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id UUID        NULL;

ALTER TABLE dependency_members
    ADD COLUMN valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to   TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id UUID        NULL;

ALTER TABLE tags
    ADD COLUMN valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to   TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id UUID        NULL;

ALTER TABLE entity_tags
    ADD COLUMN valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN valid_to   TIMESTAMPTZ NULL,
    ADD COLUMN lineage_id UUID        NULL;
