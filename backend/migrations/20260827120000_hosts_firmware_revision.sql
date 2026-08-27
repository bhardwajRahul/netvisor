-- no-transaction
--
-- A device-level firmware/software revision column.
--
-- Nothing on `hosts` or `if_entries` held a version. Three sources already read one and had
-- nowhere to put it, so two of them flattened it into `sys_descr` as prose — the UniFi integration
-- writing "UniFi firmware 6.5.59" and Instant On writing "Instant On firmware 2.9.0". A structured
-- value turned into a sentence is not comparable, filterable or alertable, and `sys_descr` is the
-- SNMP system description, which is not what either of them read.
--
-- The industrial probes (Modbus `0x2B` MajorMinorRevision, EtherNet/IP revision) would have been
-- the third and fourth. ENTITY-MIB `entPhysicalFirmwareRev` will be the fifth and fills the same
-- column.
--
-- Device-level rather than per-module: everything downstream — the host row, the host tab,
-- snapshot close/clone, consolidate — is host-shaped, and the NCCoE asset-inventory minimum says
-- "product version", singular. Capturing `entPhysicalTable` as child rows later renders
-- device-level as "the chassis row" without invalidating anything stored here.
--
-- Additive and prod-safe: a nullable ADD COLUMN with no default is metadata-only (no rewrite).
-- No contract step — nothing is renamed or dropped, and older servers ignore the column.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE hosts
    ADD COLUMN IF NOT EXISTS firmware_revision TEXT;

-- Move the two faked values onto the real column and clear the sentence they were hidden in.
--
-- History rows are included deliberately: leaving them behind would make a host's own timeline
-- disagree with itself about where its firmware version lives. The predicate is anchored on the
-- exact prefixes those two integrations wrote, so it cannot reach a genuine SNMP sysDescr — no
-- device describes itself with a string starting "UniFi firmware ".
--
-- Batched at 1000 rows with a COMMIT per batch (hence `-- no-transaction`), keyset-paginated by
-- `id` exactly as 20260819120000_hosts_name_source.sql does: `hosts` carries SCD2 history and
-- snapshot rows, so it is the largest table this could touch.
DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (
              SELECT id
                FROM hosts
               WHERE id > last_id
                 AND sys_descr IS NOT NULL
                 AND (sys_descr LIKE 'UniFi firmware %' OR sys_descr LIKE 'Instant On firmware %')
               ORDER BY id
               LIMIT 1000
          ) t;

        EXIT WHEN batch IS NULL;

        UPDATE hosts h
           SET firmware_revision = NULLIF(
                   regexp_replace(h.sys_descr, '^(UniFi|Instant On) firmware ', ''),
                   ''
               ),
               sys_descr = NULL
         WHERE h.id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;
