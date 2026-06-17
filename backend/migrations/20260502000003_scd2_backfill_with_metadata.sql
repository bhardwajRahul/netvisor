-- Backfill valid_from / last_seen_at on existing rows.
--
-- For tables with EntitySource (hosts, services, subnets, dependencies, vlans),
-- last_seen_at is salvaged from the most recent discovery metadata entry's
-- date. Metadata is FIFO with newest at index 0 (cf. cap_metadata in
-- backend/src/server/shared/types/entities.rs). After this migration runs,
-- the metadata field is dropped from EntitySource via the
-- entitysource_metadata_strip migration.
--
-- For tables without EntitySource (ip_addresses, ports, interfaces, bindings),
-- fall back to updated_at. Snapshotable-only tables (subnet_vlans,
-- dependencies, dependency_members, tags, entity_tags) just align
-- valid_from with created_at — they have no last_seen_at column.

SET lock_timeout = '5s';

-- DiscoveryTracked tables with EntitySource: hosts, services, subnets, vlans.
-- (dependencies has EntitySource but is NOT DiscoveryTracked, no last_seen_at.)
UPDATE hosts SET
    valid_from = COALESCE(created_at, NOW()),
    last_seen_at = COALESCE(
        (source->'metadata'->0->>'date')::timestamptz,
        updated_at,
        NOW()
    )
WHERE valid_to IS NULL;

UPDATE services SET
    valid_from = COALESCE(created_at, NOW()),
    last_seen_at = COALESCE(
        (source->'metadata'->0->>'date')::timestamptz,
        updated_at,
        NOW()
    )
WHERE valid_to IS NULL;

UPDATE subnets SET
    valid_from = COALESCE(created_at, NOW()),
    last_seen_at = COALESCE(
        (source->'metadata'->0->>'date')::timestamptz,
        updated_at,
        NOW()
    )
WHERE valid_to IS NULL;

UPDATE vlans SET
    valid_from = COALESCE(created_at, NOW()),
    last_seen_at = COALESCE(
        (source->'metadata'->0->>'date')::timestamptz,
        updated_at,
        NOW()
    )
WHERE valid_to IS NULL;

-- DiscoveryTracked tables without EntitySource: ip_addresses, ports,
-- interfaces, bindings, subnet_vlans.
UPDATE ip_addresses SET
    valid_from = COALESCE(created_at, NOW()),
    last_seen_at = COALESCE(updated_at, NOW())
WHERE valid_to IS NULL;

UPDATE ports SET
    valid_from = COALESCE(created_at, NOW()),
    last_seen_at = COALESCE(updated_at, NOW())
WHERE valid_to IS NULL;

UPDATE interfaces SET
    valid_from = COALESCE(created_at, NOW()),
    last_seen_at = COALESCE(updated_at, NOW())
WHERE valid_to IS NULL;

UPDATE bindings SET
    valid_from = COALESCE(created_at, NOW()),
    last_seen_at = COALESCE(updated_at, NOW())
WHERE valid_to IS NULL;

-- Snapshotable-only (no last_seen_at): subnet_vlans, dependencies,
-- dependency_members, tags, entity_tags. Just align valid_from with created_at.
UPDATE subnet_vlans SET valid_from = COALESCE(created_at, NOW()) WHERE valid_to IS NULL;
UPDATE dependencies SET valid_from = COALESCE(created_at, NOW()) WHERE valid_to IS NULL;
UPDATE dependency_members SET valid_from = COALESCE(created_at, NOW()) WHERE valid_to IS NULL;
UPDATE tags SET valid_from = COALESCE(created_at, NOW()) WHERE valid_to IS NULL;
UPDATE entity_tags SET valid_from = COALESCE(created_at, NOW()) WHERE valid_to IS NULL;
