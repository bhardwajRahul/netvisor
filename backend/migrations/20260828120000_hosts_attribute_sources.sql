-- no-transaction
--
-- DOWNTIME MIGRATION
--
-- Per-value provenance for every attribute discovery writes on a host, and the conversion of
-- `name_source` from the naming ladder's private vocabulary to the shared one.
--
-- Why: every one of these fields merged first-write-wins through an `is_none()` gate, so on a
-- switch answering both SNMP and EtherNet/IP whichever probe landed first owned the value
-- permanently and the source that should usually win could not displace it. Four sources already
-- write `firmware_revision` and ENTITY-MIB will be the fifth. Precedence was the order SNMP, the
-- controllers and the app probes happen to run in — which nothing stated and nothing enforced.
-- `<field>_source` records what produced each value, and the merge compares rungs instead of
-- asking whether the field was still empty.
--
-- The values here are `AttributeSource` as serde writes it: externally tagged, so a fieldless source
-- is the bare name `"Manual"` and one carrying a probe is `{"Probe":"Snmp"}`. Thirty-three of the
-- thirty-five carry nothing, and a tag on a variant with no content distinguishes nothing — hence
-- the bare string rather than the adjacently tagged object `hosts.virtualization_metadata` and
-- `interfaces.lldp_chassis_id` use, which have payloads on every variant.
--
-- Deploy sequence: stop the server, run migrations, start the new server. `name_source` changes
-- type and its whole vocabulary, and an old container would read neither — so this release ships
-- in a coordinated downtime window, exactly as 20260803120001_virtualization_service_id_column.sql
-- did. Each `squawk-ignore` below marks a statement whose unsafety that window is what makes
-- acceptable.
--
-- `-- no-transaction` means a failure part-way leaves a partial state, so every statement is
-- written to survive being re-run: the adds are `IF NOT EXISTS`, the backfill restarts its keyset
-- from the beginning and skips itself once the old column is gone, the drop is `IF EXISTS`, and
-- the rename checks the catalog first. Re-running the whole file from any partial state finishes
-- it. Verified against a database left in the one awkward state — after the drop, before the
-- rename — which is where a naive version fails, because the backfill still names `name_source`.

SET lock_timeout = '5s';
SET statement_timeout = '0';

-- 1. The eleven new pairs. `ADD COLUMN` with a non-volatile default is metadata-only on PG11+, so
--    no rewrite: an immutable cast of a literal qualifies.
--
--    `Unspecified` rather than `Manual`, which is the opposite of what 20260819120000 chose for
--    `name_source`, and for a reason that does not apply here: no UI or API path has ever let a
--    person type these values (`UpdateHostRequest` accepts none of them, and the host edit form
--    renders them read-only), so there is nothing to protect and everything to let discovery
--    improve. A row left at `Unspecified` is displaced by the first real reading, which is the
--    outcome we want.
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS sys_descr_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS sys_object_id_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS sys_location_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS sys_contact_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS management_url_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS chassis_id_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS sys_name_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS manufacturer_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS model_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS serial_number_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS firmware_revision_source JSONB NOT NULL DEFAULT '"Unspecified"'::jsonb;

-- 2. The name's replacement column.
--
--    `Manual` is the default here, keeping the asymmetry 20260819120000 reasoned about for any row
--    the backfill below does not reach: choosing wrong in that direction leaves a name stale, and
--    choosing wrong the other way silently renames something a user named.
ALTER TABLE hosts ADD COLUMN IF NOT EXISTS name_source_jsonb JSONB NOT NULL DEFAULT '"Manual"'::jsonb;

-- 3. One batched pass, doing both jobs.
--
--    Batched at 1000 rows with a COMMIT per batch (hence `-- no-transaction`), keyset-paginated by
--    `id` exactly as 20260819120000_hosts_name_source.sql and 20260827120000_hosts_firmware_revision.sql
--    do: `hosts` carries SCD2 history and snapshot rows, so it is the largest table this touches.
--
--    The name remap sends each retired rung to the shared source that means the same thing:
--
--      Unnamed          -> Unspecified   (the row's `name` is already '')
--      Unspecified      -> Unspecified
--      Ip               -> OwnAddress
--      DetectedService  -> ServiceMatch
--      Hostname         -> ReverseDns
--      DnsSd            -> DnsSdInstanceName
--      Integration      -> Authored(UnifiController)
--      Manual           -> Manual
--
--    Two of those deserve a note. `Hostname` bundled reverse DNS and SNMP `sysName` at one rung and
--    the column cannot tell them apart, so it lands on the lower of the two — a name that is
--    weaker than it should be is corrected by the next scan, where the reverse would wrongly
--    outrank a real reading. `Integration` picks UniFi because an Instant On row is
--    indistinguishable from a UniFi one here and both land at the same rung, so the choice affects
--    the label and not the ordering.
--
--    Anything unrecognised keeps the `Manual` default, the same safe direction as before.
--
--    The same pass promotes the six fields `CreateHostRequest` accepts to `Manual` on rows a person
--    created, so a value someone typed on creation is not displaced by the next scan. It mirrors
--    the manual promotion in 20260826120000_subnets_cidr_source.sql.
--
--    The whole pass is guarded on the old column still existing, which is what makes a re-run
--    after a failure between the drop and the rename below a no-op rather than an error.
--    PL/pgSQL prepares a statement the first time its branch is reached, so the `UPDATE` naming
--    `h.name_source` is never parsed when the column is gone.
DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
         WHERE table_name = 'hosts' AND column_name = 'name_source'
    ) THEN
        RETURN;
    END IF;

    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM hosts WHERE id > last_id ORDER BY id LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        UPDATE hosts h
           SET name_source_jsonb = CASE h.name_source
                   WHEN 'Unnamed'         THEN '"Unspecified"'::jsonb
                   WHEN 'Unspecified'     THEN '"Unspecified"'::jsonb
                   WHEN 'Ip'              THEN '"OwnAddress"'::jsonb
                   WHEN 'DetectedService' THEN '"ServiceMatch"'::jsonb
                   WHEN 'Hostname'        THEN '"ReverseDns"'::jsonb
                   WHEN 'DnsSd'           THEN '"DnsSdInstanceName"'::jsonb
                   WHEN 'Integration'     THEN '{"Authored":"UnifiController"}'::jsonb
                   ELSE '"Manual"'::jsonb
               END,
               sys_descr_source = CASE
                   WHEN h.source->>'type' = 'Manual' AND h.sys_descr IS NOT NULL
                       THEN '"Manual"'::jsonb
                   ELSE h.sys_descr_source
               END,
               sys_object_id_source = CASE
                   WHEN h.source->>'type' = 'Manual' AND h.sys_object_id IS NOT NULL
                       THEN '"Manual"'::jsonb
                   ELSE h.sys_object_id_source
               END,
               sys_location_source = CASE
                   WHEN h.source->>'type' = 'Manual' AND h.sys_location IS NOT NULL
                       THEN '"Manual"'::jsonb
                   ELSE h.sys_location_source
               END,
               sys_contact_source = CASE
                   WHEN h.source->>'type' = 'Manual' AND h.sys_contact IS NOT NULL
                       THEN '"Manual"'::jsonb
                   ELSE h.sys_contact_source
               END,
               management_url_source = CASE
                   WHEN h.source->>'type' = 'Manual' AND h.management_url IS NOT NULL
                       THEN '"Manual"'::jsonb
                   ELSE h.management_url_source
               END,
               chassis_id_source = CASE
                   WHEN h.source->>'type' = 'Manual' AND h.chassis_id IS NOT NULL
                       THEN '"Manual"'::jsonb
                   ELSE h.chassis_id_source
               END
         WHERE h.id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;

-- 4. Retire the old column and take its name. Both are catalog-only — `DROP COLUMN` does not
--    rewrite the table, and neither does a rename — so the cost of the whole migration is the one
--    pass above, which the vocabulary remap needed whatever type the column had.
--
--    These must not be separated from steps 1-3 by a deploy, which is what the downtime window is
--    for: between the drop and the rename there is no `name_source` at all.
-- squawk-ignore ban-drop-column
ALTER TABLE hosts DROP COLUMN IF EXISTS name_source;

-- Guarded rather than bare: a re-run after this point finds no `name_source_jsonb` to rename, and
-- a plain `ALTER ... RENAME` would fail on a migration that had already got this far.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
         WHERE table_name = 'hosts' AND column_name = 'name_source_jsonb'
    ) THEN
        ALTER TABLE hosts RENAME COLUMN name_source_jsonb TO name_source;
    END IF;
END $$;
