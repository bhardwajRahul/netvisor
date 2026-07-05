-- Move daemon interfaced-subnet ids out of the referential-integrity-free
-- `daemons.capabilities` JSONB blob into a real FK junction table. A new table
-- with FK constraints is squawk-safe (no existing rows to lock/validate).
--
-- Cardinality is many-to-many: daemons sharing a CIDR resolve to the same subnet
-- row, so the composite (daemon_id, subnet_id) is the primary key. ON DELETE
-- CASCADE means deleting a subnet removes its junction rows instead of leaving a
-- dangling id (the bug this replaces).
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE daemon_interfaced_subnets (
    daemon_id UUID NOT NULL REFERENCES daemons(id) ON DELETE CASCADE,
    subnet_id UUID NOT NULL REFERENCES subnets(id) ON DELETE CASCADE,
    PRIMARY KEY (daemon_id, subnet_id)
);

-- Reverse-lookup index (the forward direction is served by the PK prefix).
CREATE INDEX idx_daemon_interfaced_subnets_subnet_id
    ON daemon_interfaced_subnets (subnet_id);

-- One-time backfill from the legacy `capabilities.interfaced_subnet_ids` JSONB.
-- Only ids that still exist in `subnets` are inserted: existing capabilities very
-- likely reference already-deleted subnets, which would violate the FK. The
-- backfill is cosmetic (the next daemon heartbeat repopulates the junction), but
-- it must not FK-violate, hence the join against `subnets`.
INSERT INTO daemon_interfaced_subnets (daemon_id, subnet_id)
SELECT d.id, s.id
FROM daemons d
    CROSS JOIN LATERAL jsonb_array_elements_text(
        d.capabilities -> 'interfaced_subnet_ids'
    ) AS elem(subnet_id)
    JOIN subnets s ON s.id = elem.subnet_id::uuid
WHERE jsonb_typeof(d.capabilities -> 'interfaced_subnet_ids') = 'array'
ON CONFLICT DO NOTHING;
