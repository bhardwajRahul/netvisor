-- Topology rows in dev today fall into three buckets:
--   * is_locked = TRUE (a saved point-in-time capture)
--   * unlocked, no parent_id (the "live view" for its network)
--   * parent_id IS NOT NULL (a branch — view configuration, not a snapshot)
--
-- New shape: per network, exactly one row with snapshot_id IS NULL (live view)
-- and zero or more rows with snapshot_id pointing at a snapshots record.
--
-- Backfill steps (idempotent on re-run):
--   1. Convert each locked topology into a snapshots row:
--        snapshots.taken_at        = topologies.locked_at
--        snapshots.created_by_user_id = topologies.locked_by
--        topologies.snapshot_id    = snapshots.id
--      Skip locks with locked_at IS NULL (data corruption — emit NOTICE).
--      NOTE on JSONB content: the legacy entity-blob JSONB columns
--      (hosts/ip_addresses/...) are NOT extracted into closed SCD2 rows here.
--      The foundation backfill set live rows' valid_from to NOW() / metadata
--      time, so for the common case where locked_at >= live.valid_from the
--      live row already covers as-of(locked_at). For old locks where
--      locked_at < live.valid_from, loading the migrated snapshot returns
--      sparse SCD2 state (entities that didn't exist as-of T are absent) —
--      this is option (a) from TASK.md ("the only honest answer"). Migration E
--      drops the legacy JSONB so this is a one-way conversion.
--
--   2. Per network: keep the most-recently-updated unlocked, non-parent-branched
--      row as the live view (snapshot_id = NULL) and DELETE the rest.
--      If a network has no qualifying row, INSERT a fresh empty live-view row.
--
--   3. DELETE any remaining rows whose snapshot_id is still NULL beyond the
--      one canonical live view per network.

SET lock_timeout = '5s';
SET statement_timeout = '0';

DO $$
DECLARE
    locked_row     RECORD;
    network_row    RECORD;
    new_snap_id    UUID;
    skipped_count  INTEGER := 0;
    converted_count INTEGER := 0;
BEGIN
    -- 1. Convert locked topologies to snapshots.
    FOR locked_row IN
        SELECT id, network_id, locked_at, locked_by
        FROM topologies
        WHERE is_locked = TRUE
          AND snapshot_id IS NULL
        ORDER BY locked_at NULLS LAST, id
    LOOP
        IF locked_row.locked_at IS NULL THEN
            skipped_count := skipped_count + 1;
            RAISE NOTICE 'Skipping locked topology % (network %): locked_at IS NULL',
                locked_row.id, locked_row.network_id;
            CONTINUE;
        END IF;

        new_snap_id := gen_random_uuid();
        INSERT INTO snapshots (id, network_id, taken_at, created_by_user_id)
        VALUES (new_snap_id, locked_row.network_id, locked_row.locked_at, locked_row.locked_by);

        UPDATE topologies
        SET snapshot_id = new_snap_id,
            updated_at = NOW()
        WHERE id = locked_row.id;

        converted_count := converted_count + 1;
    END LOOP;

    RAISE NOTICE 'Converted % locked topologies to snapshots; skipped % (locked_at NULL)',
        converted_count, skipped_count;

    -- 2. Per network, choose one live-view row; delete the rest.
    FOR network_row IN
        SELECT DISTINCT n.id AS network_id
        FROM networks n
    LOOP
        -- Delete everything else with snapshot_id NULL on this network beyond
        -- the most-recently-updated unlocked, non-parent-branched row.
        -- (Locked rows already have snapshot_id set and are excluded.)
        DELETE FROM topologies
        WHERE network_id = network_row.network_id
          AND snapshot_id IS NULL
          AND id <> COALESCE(
              (SELECT id FROM topologies
               WHERE network_id = network_row.network_id
                 AND snapshot_id IS NULL
                 AND parent_id IS NULL
                 AND is_locked = FALSE
               ORDER BY updated_at DESC, id
               LIMIT 1),
              '00000000-0000-0000-0000-000000000000'::uuid
          );

        -- 3. If no live-view row exists, create one with empty graph state.
        IF NOT EXISTS (
            SELECT 1 FROM topologies
            WHERE network_id = network_row.network_id AND snapshot_id IS NULL
        ) THEN
            INSERT INTO topologies (
                id, network_id, name, snapshot_id,
                nodes, edges, options,
                hosts, ip_addresses, ports, bindings, subnets, services,
                dependencies, interfaces, entity_tags, vlans,
                is_stale, last_refreshed, is_locked,
                removed_hosts, removed_ip_addresses, removed_subnets,
                removed_services, removed_dependencies, removed_ports,
                removed_bindings, removed_interfaces,
                tags,
                created_at, updated_at
            ) VALUES (
                gen_random_uuid(), network_row.network_id, '', NULL,
                '{}'::jsonb, '{}'::jsonb, '{}'::jsonb,
                '[]'::jsonb, '[]'::jsonb, '[]'::jsonb, '[]'::jsonb, '[]'::jsonb, '[]'::jsonb,
                '[]'::jsonb, '[]'::jsonb, '[]'::jsonb, '[]'::jsonb,
                FALSE, NOW(), FALSE,
                '{}'::uuid[], '{}'::uuid[], '{}'::uuid[],
                '{}'::uuid[], '{}'::uuid[], '{}'::uuid[],
                '{}'::uuid[], '{}'::uuid[],
                '{}'::uuid[],
                NOW(), NOW()
            );
        END IF;
    END LOOP;
END $$;
