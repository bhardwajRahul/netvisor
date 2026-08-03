-- DOWNTIME MIGRATION
--
-- Moves the virtualizing service's id out of the `virtualization` JSONB blobs and into a real
-- `virtualization_service_id` column with a foreign key, on hosts, services and subnets.
--
-- Why: container-bridge subnets only deduplicate against each other when CIDR, network AND the
-- owning service id all match, and that id is a pending UUID the daemon mints fresh every scan.
-- The host create path remaps it, but creates subnets before services exist and never recorded
-- the id hop a service upsert can introduce, so bridge rows kept an id matching no live service.
-- Dedup then failed on every subsequent scan and the network gained a duplicate row each time —
-- GH #650 reported 353 rows for 70 distinct CIDRs. The remap bug is fixed in application code;
-- this column is what stops the same class of drift being silent, because a dangling id now
-- fails the insert instead of quietly poisoning dedup forever.
--
-- It also turns three linear scans into indexed lookups (the bridge dedup and the bridge repair
-- both loaded every live subnet in the network to compare a JSONB field), removes the unindexed
-- `hosts.virtualization->'details'->>'service_id'` expression join behind the "virtualized by"
-- sort, and lets `ON DELETE SET NULL` clean up after a deleted runtime service instead of
-- leaving subnets pointing at a dead id.
--
-- Deploy sequence: stop the server, run migrations, start the new server. Renames and a dropped
-- column are unsafe under a rolling deploy — an old container would still be reading the old
-- names — so this release ships in a coordinated downtime window. Each `squawk-ignore` below
-- marks a statement whose unsafety that window is what makes acceptable.
--
-- `service_id` is non-optional in all seven virtualization payload structs, so the column models
-- the real cardinality; the `Option<Uuid>` the old accessors returned could never be `None`.

SET lock_timeout = '60s';
SET statement_timeout = '0';

-- 1. Rename the surviving JSONB to say what it now holds: everything about the virtualization
--    EXCEPT the service reference (vm_name/vm_id for hosts, container_name/container_id/
--    compose_project for services).
-- squawk-ignore renaming-column
ALTER TABLE hosts RENAME COLUMN virtualization TO virtualization_metadata;

-- squawk-ignore renaming-column
ALTER TABLE services RENAME COLUMN virtualization TO virtualization_metadata;

-- 2. The new column. Validated inline rather than NOT VALID + VALIDATE because nothing is
--    serving traffic; ON DELETE SET NULL because deleting a Docker service must orphan its
--    bridge subnets, not delete them.
--    (Each stays on one line: the directive applies to the line that follows it, and the rule
--    reports against the REFERENCES clause.)
-- squawk-ignore adding-foreign-key-constraint
ALTER TABLE hosts ADD COLUMN virtualization_service_id UUID REFERENCES services(id) ON DELETE SET NULL;

-- squawk-ignore adding-foreign-key-constraint
ALTER TABLE services ADD COLUMN virtualization_service_id UUID REFERENCES services(id) ON DELETE SET NULL;

-- squawk-ignore adding-foreign-key-constraint
ALTER TABLE subnets ADD COLUMN virtualization_service_id UUID REFERENCES services(id) ON DELETE SET NULL;

-- 3. Backfill. Hosts and services are adjacently tagged ({"type":..,"details":{..}}); subnets are
--    internally tagged and flat. Only ids that actually resolve are carried over: the rows this
--    bug produced point at services that were never created, and copying those would fail the
--    foreign key on a table we cannot then repair. Leaving them NULL quarantines exactly the
--    corrupt rows, and a NULL-owner bridge subnet deduplicates on CIDR + network alone, so the
--    duplicates collapse onto one row as they are next seen rather than accumulating further.
DO $$
DECLARE
    quarantined BIGINT;
BEGIN
    UPDATE hosts h
    SET virtualization_service_id = (h.virtualization_metadata->'details'->>'service_id')::uuid
    WHERE h.virtualization_metadata->'details'->>'service_id' IS NOT NULL
      AND EXISTS (
          SELECT 1 FROM services s
          WHERE s.id = (h.virtualization_metadata->'details'->>'service_id')::uuid
      );

    SELECT count(*) INTO quarantined
    FROM hosts h
    WHERE h.virtualization_metadata->'details'->>'service_id' IS NOT NULL
      AND h.virtualization_service_id IS NULL;
    RAISE NOTICE 'hosts: % row(s) referenced a service that no longer exists', quarantined;

    UPDATE services sv
    SET virtualization_service_id = (sv.virtualization_metadata->'details'->>'service_id')::uuid
    WHERE sv.virtualization_metadata->'details'->>'service_id' IS NOT NULL
      AND EXISTS (
          SELECT 1 FROM services s
          WHERE s.id = (sv.virtualization_metadata->'details'->>'service_id')::uuid
      );

    SELECT count(*) INTO quarantined
    FROM services sv
    WHERE sv.virtualization_metadata->'details'->>'service_id' IS NOT NULL
      AND sv.virtualization_service_id IS NULL;
    RAISE NOTICE 'services: % row(s) referenced a service that no longer exists', quarantined;

    UPDATE subnets sn
    SET virtualization_service_id = (sn.virtualization->>'service_id')::uuid
    WHERE sn.virtualization->>'service_id' IS NOT NULL
      AND EXISTS (
          SELECT 1 FROM services s
          WHERE s.id = (sn.virtualization->>'service_id')::uuid
      );

    SELECT count(*) INTO quarantined
    FROM subnets sn
    WHERE sn.virtualization->>'service_id' IS NOT NULL
      AND sn.virtualization_service_id IS NULL;
    RAISE NOTICE 'subnets: % row(s) referenced a service that no longer exists', quarantined;
END $$;

-- 4. Drop the now-duplicated key from the JSONB that survives, so there is one source of truth.
UPDATE hosts
SET virtualization_metadata = virtualization_metadata #- '{details,service_id}'
WHERE virtualization_metadata->'details'->>'service_id' IS NOT NULL;

UPDATE services
SET virtualization_metadata = virtualization_metadata #- '{details,service_id}'
WHERE virtualization_metadata->'details'->>'service_id' IS NOT NULL;

-- 5. Subnet virtualization held the service id and nothing else, and the runtime it named is
--    already carried by subnet_type (DockerBridge / PodmanBridge). With the id in its own column
--    the blob has no remaining content, so it goes rather than being renamed to an empty shell.
-- squawk-ignore ban-drop-column
ALTER TABLE subnets DROP COLUMN virtualization;

-- 6. Indexes for the lookups this column exists to make cheap. Plain rather than CONCURRENTLY:
--    nothing is writing during the downtime window, and CONCURRENTLY cannot run in the
--    transaction sqlx wraps this migration in. Partial, because the overwhelming majority of
--    rows are not virtualized at all.
-- squawk-ignore require-concurrent-index-creation
CREATE INDEX IF NOT EXISTS idx_hosts_virtualization_service_id ON hosts (virtualization_service_id) WHERE virtualization_service_id IS NOT NULL;

-- squawk-ignore require-concurrent-index-creation
CREATE INDEX IF NOT EXISTS idx_services_virtualization_service_id ON services (virtualization_service_id) WHERE virtualization_service_id IS NOT NULL;

-- squawk-ignore require-concurrent-index-creation
CREATE INDEX IF NOT EXISTS idx_subnets_virtualization_service_id ON subnets (virtualization_service_id) WHERE virtualization_service_id IS NOT NULL;
