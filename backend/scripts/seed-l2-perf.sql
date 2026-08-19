-- Seed a large synthetic L2 Physical topology for render-performance profiling.
--
-- THROWAWAY. This exists to reproduce a customer's slow L2 Physical view locally
-- (perf/l2-topology-scale). It is not product tooling and is not wired into any
-- Makefile target — run it by hand, once, on a dev database.
--
-- Prerequisite: populate demo data first. Demo data supplies the org, network,
-- subnets and topology row; this script only scales the L2 layer on top of the
-- network that already has physical links.
--
-- Run:
--   docker exec -i scanopy-postgres psql -U postgres -d scanopy -v ON_ERROR_STOP=1 \
--     < backend/scripts/seed-l2-perf.sql
--
-- Idempotent: every id is derived from md5(<stable key>), so re-running updates
-- the same rows rather than duplicating them. To undo, see the DELETE at the
-- bottom of this file (commented out).
--
-- Shape produced (edit these three numbers to change the scale):
--   n_switches            core switches, each a Host container in the L2 view
--   hosts_per_switch      hosts hanging off each switch -> the tall columns
--   extra_ifaces_per_host additional unlinked ports per host, for realistic
--                         element counts inside each host container
--   device_level_pct      percentage of uplinks resolved only as far as the
--                         far-end device, drawing a dashed NeighborLink between
--                         host containers instead of a port-precise PhysicalLink
--
-- Defaults give 8 switches + 400 hosts = 408 host containers, ~1600 interface
-- elements and 400 PhysicalLink edges.
--
-- Override any of them on the command line rather than editing this file, so a
-- profile can be reproduced from the invocation:
--
--   # ~600 devices — the interactive-performance profile
--   psql ... -v hosts_per_switch=75 < backend/scripts/seed-l2-perf.sql
--
--   # ~17k nodes — the shape of the customer's out-of-memory report. Their
--   # estate averaged ~24.6 nodes per host (one container + ~22 interfaces +
--   # grouping subcontainers), which is what a Windows/hypervisor SNMP agent
--   # reports: teaming members and vswitch uplinks all present as if_type 6 and
--   # survive EXCLUDED_IF_TYPES.
--   psql ... -v hosts_per_switch=88 -v extra_ifaces_per_host=22 \
--     < backend/scripts/seed-l2-perf.sql
--
--   # the post-guard edge mix: switches that report one chassis MAC on every
--   # port name no port on each other, so almost every link degrades to a
--   # device-level NeighborLink. That is what the August 2026 customer's L2
--   # Physical actually contained, and the two edge types are laid out and drawn
--   # differently enough that a view proven on PhysicalLinks alone is not proven.
--   psql ... -v device_level_pct=99 < backend/scripts/seed-l2-perf.sql

\if :{?n_switches} \else \set n_switches 8 \endif
\if :{?hosts_per_switch} \else \set hosts_per_switch 50 \endif
\if :{?extra_ifaces_per_host} \else \set extra_ifaces_per_host 2 \endif
\if :{?device_level_pct} \else \set device_level_pct 0 \endif

BEGIN;

-- ---------------------------------------------------------------------------
-- Target network
-- ---------------------------------------------------------------------------
-- Prefer the network that already has port-precise neighbours, i.e. the one
-- whose L2 Physical view is already enabled (topology/service/main.rs gates the
-- view on at least one interface carrying Neighbor::Interface). Then the network
-- with the most live hosts, then any network at all — so the script still does
-- something useful on a database seeded differently.
CREATE TEMPORARY TABLE l2perf_target ON COMMIT DROP AS
WITH linked AS (
    SELECT i.network_id, count(*) AS weight, 1 AS priority
    FROM interfaces i
    WHERE i.valid_to IS NULL
      AND i.neighbor_interface_id IS NOT NULL
    GROUP BY i.network_id
),
populated AS (
    SELECT h.network_id, count(*) AS weight, 2 AS priority
    FROM hosts h
    WHERE h.valid_to IS NULL
    GROUP BY h.network_id
),
any_network AS (
    SELECT n.id AS network_id, 0 AS weight, 3 AS priority
    FROM networks n
)
SELECT network_id
FROM (
    SELECT * FROM linked
    UNION ALL SELECT * FROM populated
    UNION ALL SELECT * FROM any_network
) c
ORDER BY priority, weight DESC
LIMIT 1;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM l2perf_target) THEN
        RAISE EXCEPTION 'No network found. Create a network (or populate demo data) first.';
    END IF;
END $$;

-- ---------------------------------------------------------------------------
-- Switches
-- ---------------------------------------------------------------------------
INSERT INTO hosts (
    id, network_id, name, hostname, description, source,
    created_at, updated_at, sys_name, manufacturer, model
)
SELECT
    md5('l2perf:switch:' || s)::uuid,
    t.network_id,
    'l2perf-switch-' || lpad(s::text, 2, '0'),
    'l2perf-switch-' || lpad(s::text, 2, '0'),
    'Synthetic L2 perf switch',
    '{"type": "Discovery"}'::jsonb,
    now(), now(),
    'l2perf-switch-' || lpad(s::text, 2, '0'),
    'Scanopy', 'Synthetic-48P'
FROM l2perf_target t, generate_series(1, :n_switches) AS s
ON CONFLICT (id) DO UPDATE SET updated_at = now();

-- ---------------------------------------------------------------------------
-- Access hosts
-- ---------------------------------------------------------------------------
-- One host per switch port. `h` is the global host ordinal so names stay unique;
-- host names are the key the demo-data neighbour resolver uses, and duplicates
-- would silently drop links.
INSERT INTO hosts (
    id, network_id, name, hostname, description, source,
    created_at, updated_at, sys_name, manufacturer, model
)
SELECT
    md5('l2perf:host:' || s || ':' || p)::uuid,
    t.network_id,
    'l2perf-host-' || lpad(s::text, 2, '0') || '-' || lpad(p::text, 3, '0'),
    'l2perf-host-' || lpad(s::text, 2, '0') || '-' || lpad(p::text, 3, '0'),
    'Synthetic L2 perf host',
    '{"type": "Discovery"}'::jsonb,
    now(), now(),
    'l2perf-host-' || lpad(s::text, 2, '0') || '-' || lpad(p::text, 3, '0'),
    'Scanopy', 'Synthetic-Node'
FROM l2perf_target t,
     generate_series(1, :n_switches) AS s,
     generate_series(1, :hosts_per_switch) AS p
ON CONFLICT (id) DO UPDATE SET updated_at = now();

-- ---------------------------------------------------------------------------
-- Switch ports
-- ---------------------------------------------------------------------------
-- if_type 6 = ethernetCsmacd. Anything in l2_builder's EXCLUDED_IF_TYPES
-- (24, 53, 71, 131, 135, 136, 209) is filtered out of the view, so physical
-- ports must not use those.
INSERT INTO interfaces (
    id, host_id, network_id, if_index, if_descr, if_alias, if_type,
    speed_bps, admin_status, oper_status, created_at, updated_at
)
SELECT
    md5('l2perf:switchport:' || s || ':' || p)::uuid,
    md5('l2perf:switch:' || s)::uuid,
    t.network_id,
    p,
    'Port 1/0/' || p,
    'to l2perf-host-' || lpad(s::text, 2, '0') || '-' || lpad(p::text, 3, '0'),
    6,
    1000000000,
    1, 1,
    now(), now()
FROM l2perf_target t,
     generate_series(1, :n_switches) AS s,
     generate_series(1, :hosts_per_switch) AS p
ON CONFLICT (id) DO UPDATE SET updated_at = now();

-- ---------------------------------------------------------------------------
-- Host uplinks -> PhysicalLink edges
-- ---------------------------------------------------------------------------
-- neighbor_interface_id is what l2_builder turns into a PhysicalLink. Only one
-- direction is needed: the builder dedups the pair on sorted endpoint ids.
--
-- The first :device_level_pct percent of each switch's ports instead set
-- neighbor_host_id, which is the state a link degrades to when the far end
-- reports one MAC across every port: the device is known and the port is not, so
-- l2_builder draws a device-level NeighborLink between the two Host containers.
-- Exactly one of the two columns may be set — chk_neighbor_exclusive — and they
-- produce different node endpoints (interface elements vs host containers),
-- different strokes and different layout behaviour, so a view exercised only on
-- port-precise links has not been exercised on what the customer actually had.
INSERT INTO interfaces (
    id, host_id, network_id, if_index, if_descr, if_alias, if_type,
    speed_bps, admin_status, oper_status, neighbor_interface_id, neighbor_host_id,
    lldp_sys_name, lldp_port_desc, created_at, updated_at
)
SELECT
    md5('l2perf:uplink:' || s || ':' || p)::uuid,
    md5('l2perf:host:' || s || ':' || p)::uuid,
    t.network_id,
    1,
    'eth0',
    'uplink',
    6,
    1000000000,
    1, 1,
    CASE WHEN p > (:hosts_per_switch * :device_level_pct) / 100
         THEN md5('l2perf:switchport:' || s || ':' || p)::uuid END,
    CASE WHEN p <= (:hosts_per_switch * :device_level_pct) / 100
         THEN md5('l2perf:switch:' || s)::uuid END,
    'l2perf-switch-' || lpad(s::text, 2, '0'),
    'Port 1/0/' || p,
    now(), now()
FROM l2perf_target t,
     generate_series(1, :n_switches) AS s,
     generate_series(1, :hosts_per_switch) AS p
ON CONFLICT (id) DO UPDATE SET
    neighbor_interface_id = EXCLUDED.neighbor_interface_id,
    neighbor_host_id = EXCLUDED.neighbor_host_id,
    updated_at = now();

-- ---------------------------------------------------------------------------
-- Extra unlinked host ports
-- ---------------------------------------------------------------------------
-- These carry no neighbour, so they add no edges — they exist to give each host
-- container a realistic number of child elements to lay out and render.
INSERT INTO interfaces (
    id, host_id, network_id, if_index, if_descr, if_alias, if_type,
    speed_bps, admin_status, oper_status, created_at, updated_at
)
SELECT
    md5('l2perf:extra:' || s || ':' || p || ':' || e)::uuid,
    md5('l2perf:host:' || s || ':' || p)::uuid,
    t.network_id,
    e + 1,
    'eth' || e,
    NULL,
    6,
    1000000000,
    1,
    CASE WHEN e % 2 = 0 THEN 2 ELSE 1 END,  -- alternate down/up for visual variety
    now(), now()
FROM l2perf_target t,
     generate_series(1, :n_switches) AS s,
     generate_series(1, :hosts_per_switch) AS p,
     generate_series(1, :extra_ifaces_per_host) AS e
ON CONFLICT (id) DO UPDATE SET updated_at = now();

-- ---------------------------------------------------------------------------
-- Report
-- ---------------------------------------------------------------------------
SELECT
    (SELECT name FROM networks WHERE id = (SELECT network_id FROM l2perf_target)) AS network,
    (SELECT count(*) FROM hosts h
       WHERE h.network_id = (SELECT network_id FROM l2perf_target)
         AND h.valid_to IS NULL) AS live_hosts,
    (SELECT count(*) FROM interfaces i
       WHERE i.network_id = (SELECT network_id FROM l2perf_target)
         AND i.valid_to IS NULL) AS live_interfaces,
    (SELECT count(*) FROM interfaces i
       WHERE i.network_id = (SELECT network_id FROM l2perf_target)
         AND i.valid_to IS NULL
         AND i.neighbor_interface_id IS NOT NULL) AS physical_links,
    (SELECT count(*) FROM interfaces i
       WHERE i.network_id = (SELECT network_id FROM l2perf_target)
         AND i.valid_to IS NULL
         AND i.neighbor_host_id IS NOT NULL) AS neighbor_links;

COMMIT;

-- ---------------------------------------------------------------------------
-- Undo
-- ---------------------------------------------------------------------------
-- Interfaces cascade from hosts, so removing the synthetic hosts is enough.
--
--   DELETE FROM hosts WHERE name LIKE 'l2perf-%';
