-- no-transaction
--
-- Per-field provenance for a host's display name (GH #680).
--
-- Until now the only answer to "did a person type this name, or did we derive it?" was
-- `name.parse::<IpAddr>().is_ok()` in the upsert merge. That recognises exactly one derived
-- shape, so a name derived from a detected service was indistinguishable from a hand-typed one
-- and froze forever, and a name a controller holds ("Core Switch") had no rung to occupy at all.
-- `name_source` records which rung of the ladder produced `name`, and the merge compares ranks
-- instead of guessing from the string.
--
-- Additive and prod-safe: ADD COLUMN with a constant default is metadata-only (no rewrite).
-- `Manual` is the default on purpose — it is the top of the ladder, so any row the backfill below
-- does not reach keeps its name for good. Choosing wrong in that direction leaves a name stale;
-- choosing wrong the other way silently renames something a user named.
--
-- No contract step: nothing is renamed or dropped, and older servers ignore the column.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE hosts
    ADD COLUMN IF NOT EXISTS name_source TEXT NOT NULL DEFAULT 'Manual';

-- Backfill: apply the heuristics the code used to apply on every merge, exactly once. Anything
-- still recognisable as machine-derived is demoted so discovery may improve on it later; anything
-- else stays `Manual` and is never touched again.
--
-- Batched at 1000 rows with a COMMIT per batch (hence `-- no-transaction`): `hosts` carries SCD2
-- history and snapshot rows, so it is the largest table this could touch.
DO $$
DECLARE
    last_id UUID := '00000000-0000-0000-0000-000000000000';
    batch UUID[];
BEGIN
    LOOP
        SELECT array_agg(id ORDER BY id)
          INTO batch
          FROM (SELECT id FROM hosts WHERE id > last_id ORDER BY id LIMIT 1000) t;

        EXIT WHEN batch IS NULL;

        UPDATE hosts h
           SET name_source = CASE
                -- No name at all: the state the UniFi-imported devices in #680 are in.
                WHEN h.name = '' THEN 'Unspecified'
                -- An IPv4 or IPv6 literal, i.e. the bottom rung of the ladder.
                WHEN h.name ~ '^[0-9]{1,3}(\.[0-9]{1,3}){3}$' THEN 'Ip'
                WHEN h.name ~ '^[0-9A-Fa-f:]+$' AND h.name LIKE '%:%' THEN 'Ip'
                -- Reverse DNS / reported hostname, copied into name by the old ladder.
                WHEN h.hostname IS NOT NULL AND h.name = h.hostname THEN 'Hostname'
                -- Named after one of its own detected services ("SSH", "Docker"). Only for hosts
                -- discovery created: on a manually created host the same string is a coincidence.
                WHEN h.source->>'type' IN ('Discovery', 'DiscoveryWithMatch')
                     AND EXISTS (
                         SELECT 1 FROM services s
                          WHERE s.host_id = h.id
                            AND s.valid_to IS NULL
                            AND s.name = h.name
                     ) THEN 'DetectedService'
                ELSE 'Manual'
           END
         WHERE h.id = ANY(batch);

        last_id := batch[array_length(batch, 1)];
        COMMIT;
    END LOOP;
END $$;
