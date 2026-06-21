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
--      JSONB EXTRACTION: each locked topology carries denormalized entity
--      blobs (hosts/ip_addresses/subnets/services/dependencies/ports/bindings/
--      interfaces/vlans + each entity's embedded tags + dependency members).
--      We extract every blob element into a closed-copy SCD2 row stamped with
--      snapshot_id = the new snapshot, valid_from = valid_to = last_seen_at =
--      locked_at, first/last_discovery_id = NULL. This gives the snapshot real
--      as-of-T data to render (replaces the former "option (a)" sparse path).
--      Closed copies CANNOT reuse the original entity ids — the same entity is
--      usually still live (PK collision). Each closed id is derived
--      deterministically as md5(snapshot_id || original_id)::uuid; every FK
--      (host_id, subnet_id, service_id, ip_address_id, port_id, vlan refs,
--      neighbor refs, dependency members) is remapped with the SAME derivation
--      so references stay internally consistent within the snapshot. lineage_id
--      = the original id. tag_id is left as-is (tag definitions are org-scoped
--      and never cloned). Best-effort: fields absent from the v0.16.2 blob are
--      defaulted (NULL where allowed, '' for required text). Idempotent via
--      ON CONFLICT (id) DO NOTHING on the derived id. Migration E drops the
--      legacy JSONB after this, so extraction is one-shot at upgrade time.
--      Limitation: if the SAME network was locked more than once, only the
--      first-applied lock's entities are extracted per derived id (best-effort).
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

        -- Extract the locked topology's denormalized JSONB entity blobs into
        -- closed-copy SCD2 rows (see header). Closed ids are deterministic:
        -- md5(new_snap_id || original_id)::uuid; FKs remapped the same way.

        -- hosts
        INSERT INTO hosts (
            id, created_at, updated_at, name, description, network_id, source,
            hostname, hidden, virtualization, sys_descr, sys_object_id,
            sys_location, sys_contact, management_url, chassis_id, sys_name,
            manufacturer, model, serial_number,
            snapshot_id, valid_from, valid_to, last_seen_at, lineage_id,
            first_discovery_id, last_discovery_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id'))::uuid,
            COALESCE((e->>'created_at')::timestamptz, locked_row.locked_at),
            COALESCE((e->>'updated_at')::timestamptz, locked_row.locked_at),
            COALESCE(e->>'name', ''),
            e->>'description',
            locked_row.network_id,
            COALESCE(e->'source', '{"type": "Manual"}'::jsonb),
            e->>'hostname',
            COALESCE((e->>'hidden')::boolean, FALSE),
            COALESCE(e->'virtualization', 'null'::jsonb),
            e->>'sys_descr', e->>'sys_object_id', e->>'sys_location',
            e->>'sys_contact', e->>'management_url', e->>'chassis_id',
            e->>'sys_name', e->>'manufacturer', e->>'model', e->>'serial_number',
            new_snap_id, locked_row.locked_at, locked_row.locked_at,
            locked_row.locked_at, (e->>'id')::uuid, NULL, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.hosts, '[]'::jsonb)) e
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        -- subnets
        INSERT INTO subnets (
            id, name, description, cidr, source, subnet_type, virtualization,
            network_id, created_at, updated_at,
            snapshot_id, valid_from, valid_to, last_seen_at, lineage_id,
            first_discovery_id, last_discovery_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id'))::uuid,
            COALESCE(e->>'name', ''),
            e->>'description',
            -- cidr is read via serde_json::from_str → store JSON-quoted text
            COALESCE((e->'cidr')::text, '""'),
            COALESCE(e->'source', '{"type": "Manual"}'::jsonb),
            COALESCE(e->>'subnet_type', 'Unknown'),
            COALESCE(e->'virtualization', 'null'::jsonb),
            locked_row.network_id,
            COALESCE((e->>'created_at')::timestamptz, locked_row.locked_at),
            COALESCE((e->>'updated_at')::timestamptz, locked_row.locked_at),
            new_snap_id, locked_row.locked_at, locked_row.locked_at,
            locked_row.locked_at, (e->>'id')::uuid, NULL, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.subnets, '[]'::jsonb)) e
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        -- vlans (organization_id kept as-is; orgs are not snapshot-cloned)
        INSERT INTO vlans (
            id, vlan_number, name, description, network_id, organization_id,
            source, created_at, updated_at,
            snapshot_id, valid_from, valid_to, last_seen_at, lineage_id,
            first_discovery_id, last_discovery_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id'))::uuid,
            COALESCE((e->>'vlan_number')::smallint, 0),
            COALESCE(e->>'name', ''),
            e->>'description',
            locked_row.network_id,
            (e->>'organization_id')::uuid,
            COALESCE(e->'source', '{"type": "Manual"}'::jsonb),
            COALESCE((e->>'created_at')::timestamptz, locked_row.locked_at),
            COALESCE((e->>'updated_at')::timestamptz, locked_row.locked_at),
            new_snap_id, locked_row.locked_at, locked_row.locked_at,
            locked_row.locked_at, (e->>'id')::uuid, NULL, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.vlans, '[]'::jsonb)) e
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        -- ip_addresses (host_id/subnet_id remapped to closed copies)
        INSERT INTO ip_addresses (
            id, network_id, host_id, subnet_id, ip_address, mac_address, name,
            position, created_at, updated_at,
            snapshot_id, valid_from, valid_to, last_seen_at, lineage_id,
            first_discovery_id, last_discovery_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id'))::uuid,
            locked_row.network_id,
            md5(new_snap_id::text || (e->>'host_id'))::uuid,
            md5(new_snap_id::text || (e->>'subnet_id'))::uuid,
            (e->>'ip_address')::inet,
            (e->>'mac_address')::macaddr,
            e->>'name',
            COALESCE((e->>'position')::int, 0),
            COALESCE((e->>'created_at')::timestamptz, locked_row.locked_at),
            COALESCE((e->>'updated_at')::timestamptz, locked_row.locked_at),
            new_snap_id, locked_row.locked_at, locked_row.locked_at,
            locked_row.locked_at, (e->>'id')::uuid, NULL, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.ip_addresses, '[]'::jsonb)) e
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        -- ports (host_id remapped; JSONB keys: number/type)
        INSERT INTO ports (
            id, host_id, network_id, port_number, protocol, port_type,
            created_at, updated_at,
            snapshot_id, valid_from, valid_to, last_seen_at, lineage_id,
            first_discovery_id, last_discovery_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id'))::uuid,
            md5(new_snap_id::text || (e->>'host_id'))::uuid,
            locked_row.network_id,
            COALESCE((e->>'number')::int, 0),
            COALESCE(e->>'protocol', 'Tcp'),
            COALESCE(e->>'type', 'Unknown'),
            COALESCE((e->>'created_at')::timestamptz, locked_row.locked_at),
            COALESCE((e->>'updated_at')::timestamptz, locked_row.locked_at),
            new_snap_id, locked_row.locked_at, locked_row.locked_at,
            locked_row.locked_at, (e->>'id')::uuid, NULL, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.ports, '[]'::jsonb)) e
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        -- services (host_id remapped; service_definition kept as JSON text)
        INSERT INTO services (
            id, created_at, updated_at, name, network_id, host_id,
            service_definition, virtualization, source, position,
            snapshot_id, valid_from, valid_to, last_seen_at, lineage_id,
            first_discovery_id, last_discovery_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id'))::uuid,
            COALESCE((e->>'created_at')::timestamptz, locked_row.locked_at),
            COALESCE((e->>'updated_at')::timestamptz, locked_row.locked_at),
            COALESCE(e->>'name', ''),
            locked_row.network_id,
            md5(new_snap_id::text || (e->>'host_id'))::uuid,
            COALESCE((e->'service_definition')::text, '""'),
            COALESCE(e->'virtualization', 'null'::jsonb),
            COALESCE(e->'source', '{"type": "Manual"}'::jsonb),
            COALESCE((e->>'position')::int, 0),
            new_snap_id, locked_row.locked_at, locked_row.locked_at,
            locked_row.locked_at, (e->>'id')::uuid, NULL, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.services, '[]'::jsonb)) e
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        -- bindings (service_id/ip_address_id/port_id remapped; JSONB key: type)
        INSERT INTO bindings (
            id, service_id, network_id, binding_type, ip_address_id, port_id,
            created_at, updated_at,
            snapshot_id, valid_from, valid_to, last_seen_at, lineage_id,
            first_discovery_id, last_discovery_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id'))::uuid,
            md5(new_snap_id::text || (e->>'service_id'))::uuid,
            locked_row.network_id,
            COALESCE(e->>'type', 'Port'),
            CASE WHEN e->>'ip_address_id' IS NOT NULL
                 THEN md5(new_snap_id::text || (e->>'ip_address_id'))::uuid END,
            CASE WHEN e->>'port_id' IS NOT NULL
                 THEN md5(new_snap_id::text || (e->>'port_id'))::uuid END,
            COALESCE((e->>'created_at')::timestamptz, locked_row.locked_at),
            COALESCE((e->>'updated_at')::timestamptz, locked_row.locked_at),
            new_snap_id, locked_row.locked_at, locked_row.locked_at,
            locked_row.locked_at, (e->>'id')::uuid, NULL, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.bindings, '[]'::jsonb)) e
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        -- interfaces (physical; host_id/ip_address_id/neighbor/vlan refs remapped;
        -- admin_status/oper_status mapped from SNMP names to ints)
        INSERT INTO interfaces (
            id, host_id, network_id, if_index, if_descr, if_name, if_alias,
            if_type, speed_bps, admin_status, oper_status, mac_address,
            ip_address_id, neighbor_interface_id, neighbor_host_id,
            lldp_chassis_id, lldp_port_id, lldp_sys_name, lldp_port_desc,
            lldp_mgmt_addr, lldp_sys_desc, cdp_device_id, cdp_port_id,
            cdp_platform, cdp_address, fdb_macs, native_vlan_id, vlan_ids,
            created_at, updated_at,
            snapshot_id, valid_from, valid_to, last_seen_at, lineage_id,
            first_discovery_id, last_discovery_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id'))::uuid,
            md5(new_snap_id::text || (e->>'host_id'))::uuid,
            locked_row.network_id,
            COALESCE((e->>'if_index')::int, 0),
            COALESCE(e->>'if_descr', ''),
            e->>'if_name',
            e->>'if_alias',
            COALESCE((e->>'if_type')::int, 0),
            (e->>'speed_bps')::bigint,
            CASE e->>'admin_status'
                WHEN 'Up' THEN 1 WHEN 'Down' THEN 2 WHEN 'Testing' THEN 3
                ELSE 1 END,
            CASE e->>'oper_status'
                WHEN 'Up' THEN 1 WHEN 'Down' THEN 2 WHEN 'Testing' THEN 3
                WHEN 'Unknown' THEN 4 WHEN 'Dormant' THEN 5
                WHEN 'NotPresent' THEN 6 WHEN 'LowerLayerDown' THEN 7
                ELSE 4 END,
            (e->>'mac_address')::macaddr,
            CASE WHEN e->>'ip_address_id' IS NOT NULL
                 THEN md5(new_snap_id::text || (e->>'ip_address_id'))::uuid END,
            CASE WHEN e->'neighbor'->>'type' = 'Interface'
                 THEN md5(new_snap_id::text || (e->'neighbor'->>'id'))::uuid END,
            CASE WHEN e->'neighbor'->>'type' = 'Host'
                 THEN md5(new_snap_id::text || (e->'neighbor'->>'id'))::uuid END,
            NULLIF(e->'lldp_chassis_id', 'null'::jsonb),
            NULLIF(e->'lldp_port_id', 'null'::jsonb),
            e->>'lldp_sys_name',
            e->>'lldp_port_desc',
            (e->>'lldp_mgmt_addr')::inet,
            e->>'lldp_sys_desc',
            e->>'cdp_device_id',
            e->>'cdp_port_id',
            e->>'cdp_platform',
            (e->>'cdp_address')::inet,
            NULLIF(e->'fdb_macs', 'null'::jsonb),
            CASE WHEN e->>'native_vlan_id' IS NOT NULL
                 THEN md5(new_snap_id::text || (e->>'native_vlan_id'))::uuid END,
            (
                SELECT jsonb_agg(to_jsonb(md5(new_snap_id::text || vid)::uuid))
                FROM jsonb_array_elements_text(e->'vlan_ids') vid
            ),
            COALESCE((e->>'created_at')::timestamptz, locked_row.locked_at),
            COALESCE((e->>'updated_at')::timestamptz, locked_row.locked_at),
            new_snap_id, locked_row.locked_at, locked_row.locked_at,
            locked_row.locked_at, (e->>'id')::uuid, NULL, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.interfaces, '[]'::jsonb)) e
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        -- dependencies (Snapshotable only: no discovery/last_seen columns)
        INSERT INTO dependencies (
            id, created_at, updated_at, name, description, network_id, source,
            dependency_type, color, edge_style, member_type,
            snapshot_id, valid_from, valid_to, lineage_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id'))::uuid,
            COALESCE((e->>'created_at')::timestamptz, locked_row.locked_at),
            COALESCE((e->>'updated_at')::timestamptz, locked_row.locked_at),
            COALESCE(e->>'name', ''),
            e->>'description',
            locked_row.network_id,
            COALESCE(e->'source', '{"type": "Manual"}'::jsonb),
            COALESCE(e->>'dependency_type', 'HubAndSpoke'),
            COALESCE(e->>'color', 'Gray'),
            COALESCE((e->'edge_style')::text, '"Straight"'),
            COALESCE(e->'members'->>'type', 'Services'),
            new_snap_id, locked_row.locked_at, locked_row.locked_at,
            (e->>'id')::uuid
        FROM topologies t, jsonb_array_elements(COALESCE(t.dependencies, '[]'::jsonb)) e
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        -- dependency_members: Services variant (service_ids[]) — service_id
        -- remapped, binding_id NULL. Synthesized deterministic id.
        INSERT INTO dependency_members (
            id, dependency_id, binding_id, service_id, position, created_at,
            snapshot_id, valid_from, valid_to, lineage_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id') || ':svc:' || (m.svc_id))::uuid,
            md5(new_snap_id::text || (e->>'id'))::uuid,
            NULL,
            md5(new_snap_id::text || m.svc_id)::uuid,
            (m.ord - 1)::int,
            locked_row.locked_at,
            new_snap_id, locked_row.locked_at, locked_row.locked_at, NULL
        FROM topologies t,
             jsonb_array_elements(COALESCE(t.dependencies, '[]'::jsonb)) e,
             jsonb_array_elements_text(e->'members'->'service_ids')
                 WITH ORDINALITY AS m(svc_id, ord)
        WHERE t.id = locked_row.id
          AND e->'members'->>'type' = 'Services'
        ON CONFLICT (id) DO NOTHING;

        -- dependency_members: Bindings variant (binding_ids[]) — binding_id
        -- remapped; service_id resolved from the binding's owning service.
        INSERT INTO dependency_members (
            id, dependency_id, binding_id, service_id, position, created_at,
            snapshot_id, valid_from, valid_to, lineage_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id') || ':bnd:' || (m.bnd_id))::uuid,
            md5(new_snap_id::text || (e->>'id'))::uuid,
            md5(new_snap_id::text || m.bnd_id)::uuid,
            md5(new_snap_id::text || (
                SELECT b->>'service_id'
                FROM jsonb_array_elements(COALESCE(t.bindings, '[]'::jsonb)) b
                WHERE b->>'id' = m.bnd_id
                LIMIT 1
            ))::uuid,
            (m.ord - 1)::int,
            locked_row.locked_at,
            new_snap_id, locked_row.locked_at, locked_row.locked_at, NULL
        FROM topologies t,
             jsonb_array_elements(COALESCE(t.dependencies, '[]'::jsonb)) e,
             jsonb_array_elements_text(e->'members'->'binding_ids')
                 WITH ORDINALITY AS m(bnd_id, ord)
        WHERE t.id = locked_row.id
          AND e->'members'->>'type' = 'Bindings'
        ON CONFLICT (id) DO NOTHING;

        -- entity_tags junction: associations come from each entity's embedded
        -- `tags` array (topologies.entity_tags holds tag DEFINITIONS, which stay
        -- live and are not cloned). entity_id remapped to the closed copy;
        -- tag_id kept as-is. entity_type is the JSON-quoted discriminant.
        INSERT INTO entity_tags (
            id, entity_id, entity_type, tag_id, created_at,
            snapshot_id, valid_from, valid_to, lineage_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id') || ':Host:' || tag)::uuid,
            md5(new_snap_id::text || (e->>'id'))::uuid,
            '"Host"',
            tag::uuid,
            locked_row.locked_at,
            new_snap_id, locked_row.locked_at, locked_row.locked_at, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.hosts, '[]'::jsonb)) e,
             jsonb_array_elements_text(COALESCE(e->'tags', '[]'::jsonb)) tag
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        INSERT INTO entity_tags (
            id, entity_id, entity_type, tag_id, created_at,
            snapshot_id, valid_from, valid_to, lineage_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id') || ':Subnet:' || tag)::uuid,
            md5(new_snap_id::text || (e->>'id'))::uuid,
            '"Subnet"',
            tag::uuid,
            locked_row.locked_at,
            new_snap_id, locked_row.locked_at, locked_row.locked_at, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.subnets, '[]'::jsonb)) e,
             jsonb_array_elements_text(COALESCE(e->'tags', '[]'::jsonb)) tag
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

        INSERT INTO entity_tags (
            id, entity_id, entity_type, tag_id, created_at,
            snapshot_id, valid_from, valid_to, lineage_id
        )
        SELECT
            md5(new_snap_id::text || (e->>'id') || ':Service:' || tag)::uuid,
            md5(new_snap_id::text || (e->>'id'))::uuid,
            '"Service"',
            tag::uuid,
            locked_row.locked_at,
            new_snap_id, locked_row.locked_at, locked_row.locked_at, NULL
        FROM topologies t, jsonb_array_elements(COALESCE(t.services, '[]'::jsonb)) e,
             jsonb_array_elements_text(COALESCE(e->'tags', '[]'::jsonb)) tag
        WHERE t.id = locked_row.id
        ON CONFLICT (id) DO NOTHING;

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

-- Legacy rows store nodes/edges as the v0.16.2 single-view ARRAY shape, which
-- the current `HashMap<TopologyView, Vec<Node>>` type cannot deserialize (the
-- typed storage layer reads these rows). The one-shot post-migration rebuild
-- (`TopologyService::rebuild_all_topologies`) repopulates every row's per-view
-- slices from entity data, so reset any non-object nodes/edges to an empty map
-- here: it makes the rows readable by `get_all` (which the rebuild calls) and
-- the rebuild overwrites them immediately afterward. Layout is not preserved
-- across this one-way shape change, which is expected for the upgrade.
UPDATE topologies
SET nodes = '{}'::jsonb, edges = '{}'::jsonb
WHERE jsonb_typeof(nodes) <> 'object' OR jsonb_typeof(edges) <> 'object';

-- `options` must hold a full `TopologyOptions` (both `local` and `request`
-- sub-objects, which are required by the type). Rows created with `options =
-- '{}'` (step 3's empty live rows) — or any legacy row missing a sub-object —
-- can't be deserialized by the typed storage layer that the rebuild reads.
-- Reset those to a serialized `TopologyOptions::default()`. Rows that already
-- carry a complete options object are left untouched (preserving user grouping
-- rules). The rebuild does not modify `options`, so this normalization must
-- happen here in SQL.
UPDATE topologies
SET options = $json${"local":{"no_fade_edges":false,"hide_edge_types":["Hypervisor"],"tag_filter":{"hidden_host_tag_ids":[],"hidden_service_tag_ids":[],"hidden_subnet_tag_ids":[]},"show_minimap":true,"bundle_edges":true},"request":{"hide_entities":{},"hide_metadata_values":{"L2Physical":{"Service":{"Category":["OpenPorts"]}},"L3Logical":{"Service":{"Category":["OpenPorts"]}},"Workloads":{"Service":{"Category":["OpenPorts"]}},"Application":{"Service":{"Category":["OpenPorts"]}}},"container_rules":{"L2Physical":[{"id":"e1b587cc-febd-4aa6-a8df-4d62580fefba","rule":"ByHost"}],"Workloads":[{"id":"e1b587cc-febd-4aa6-a8df-4d62580fefba","rule":"ByHost"}],"Application":[{"id":"a1a63f4c-d76f-43dd-9934-4cd376165ebb","rule":{"ByApplication":{"tag_ids":[]}}}],"L3Logical":[{"id":"41aeaed8-794b-4946-8600-48baa322e47d","rule":"BySubnet"},{"id":"30103a18-9b6d-4885-bd78-9faf7114656c","rule":"MergeDockerBridges"}]},"element_rules":[{"id":"a2032908-3db8-471d-8f7d-54a7692136e0","rule":"ByTrunkPort"},{"id":"22291a18-0272-4644-a88f-b6de17795631","rule":"ByVLAN"},{"id":"a97493bb-c08e-4f38-9d76-d67f2012f61b","rule":"ByPortOpStatus"},{"id":"b415c811-f286-40e1-8cab-62a9a9a35fda","rule":{"ByServiceCategory":{"categories":["NetworkCore","NetworkAccess","RemoteAccess","Workstation","Mobile","Printer","OpenPorts"],"title":"Infrastructure","is_infra_rule":true}}},{"id":"ad88164e-1ecb-42e0-b558-cae22eaf2bef","rule":{"ByTag":{"tag_ids":[],"title":null}}},{"id":"80e4ca0a-13b0-4aa3-a964-a4fcb8108fee","rule":"ByHypervisor"},{"id":"26236282-bd6e-4bba-b286-9b9e7d00d5df","rule":"ByContainerRuntime"},{"id":"7b9a19b9-13f2-49b4-a5e6-da2bdb22da5a","rule":"ByStack"}]}}$json$::jsonb
WHERE NOT (options ? 'local') OR NOT (options ? 'request');
