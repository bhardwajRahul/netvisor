-- Migration: Transform topology entity snapshots to match new schema
-- This migration updates the JSONB snapshot columns in topologies table to:
-- 1. Remove obsolete fields from hosts (target, interfaces, services, ports)
-- 2. Transform service bindings to new format with service_id, network_id, timestamps
-- 3. Populate interfaces, ports, bindings columns from embedded data
-- 4. Clean up subnet_type quotes
-- 5. Transform groups (group_type discriminant, binding_ids, color normalization)

-- ============================================================================
-- STEP 1: Populate interfaces column from old hosts.interfaces JSONB
-- ============================================================================
-- The old hosts had embedded interfaces, we need to extract them to the
-- separate interfaces JSONB column with proper structure

UPDATE topologies
SET interfaces = (
    SELECT COALESCE(jsonb_agg(
        jsonb_build_object(
            'id', iface->>'id',
            'created_at', COALESCE(host->>'created_at', NOW()::text),
            'updated_at', COALESCE(host->>'updated_at', NOW()::text),
            'network_id', host->>'network_id',
            'host_id', host->>'id',
            'subnet_id', iface->>'subnet_id',
            'ip_address', iface->>'ip_address',
            'mac_address', iface->>'mac_address',
            'name', iface->>'name'
        )
    ), '[]'::jsonb)
    FROM jsonb_array_elements(hosts) AS host,
         jsonb_array_elements(COALESCE(host->'interfaces', '[]'::jsonb)) AS iface
    WHERE iface->>'id' IS NOT NULL
)
WHERE interfaces = '[]'::jsonb
  AND EXISTS (
    SELECT 1 FROM jsonb_array_elements(hosts) h
    WHERE h->'interfaces' IS NOT NULL AND jsonb_array_length(h->'interfaces') > 0
  );

-- ============================================================================
-- STEP 2: Populate ports column from old hosts.ports JSONB
-- ============================================================================
-- The old hosts had embedded ports as [{id, number, protocol, type}]
-- New format needs {id, created_at, updated_at, host_id, network_id, number, protocol, type}

UPDATE topologies
SET ports = (
    SELECT COALESCE(jsonb_agg(
        jsonb_build_object(
            'id', port->>'id',
            'created_at', COALESCE(host->>'created_at', NOW()::text),
            'updated_at', COALESCE(host->>'updated_at', NOW()::text),
            'host_id', host->>'id',
            'network_id', host->>'network_id',
            'number', (port->>'number')::int,
            'protocol', port->>'protocol',
            'type', port->>'type'
        )
    ), '[]'::jsonb)
    FROM jsonb_array_elements(hosts) AS host,
         jsonb_array_elements(COALESCE(host->'ports', '[]'::jsonb)) AS port
    WHERE port->>'id' IS NOT NULL
)
WHERE ports = '[]'::jsonb
  AND EXISTS (
    SELECT 1 FROM jsonb_array_elements(hosts) h
    WHERE h->'ports' IS NOT NULL AND jsonb_array_length(h->'ports') > 0
  );

-- ============================================================================
-- STEP 3: Populate bindings column from old services.bindings JSONB
-- ============================================================================
-- Old format: {type, id, interface_id, port_id?}
-- New format: {id, created_at, updated_at, service_id, network_id, type, interface_id, port_id?}
-- NOTE: This extracts bindings from embedded service bindings to the separate column

UPDATE topologies
SET bindings = (
    SELECT COALESCE(jsonb_agg(
        CASE
            -- Already in new format (has service_id) - keep as-is
            WHEN binding->>'service_id' IS NOT NULL THEN
                binding
            -- Old Interface binding - transform
            WHEN binding->>'type' = 'Interface' THEN
                jsonb_build_object(
                    'id', binding->>'id',
                    'created_at', COALESCE(svc->>'created_at', NOW()::text),
                    'updated_at', COALESCE(svc->>'updated_at', NOW()::text),
                    'service_id', svc->>'id',
                    'network_id', svc->>'network_id',
                    'type', 'Interface',
                    'interface_id', binding->>'interface_id'
                )
            -- Old Port binding - transform
            ELSE
                jsonb_build_object(
                    'id', binding->>'id',
                    'created_at', COALESCE(svc->>'created_at', NOW()::text),
                    'updated_at', COALESCE(svc->>'updated_at', NOW()::text),
                    'service_id', svc->>'id',
                    'network_id', svc->>'network_id',
                    'type', 'Port',
                    'port_id', binding->>'port_id',
                    'interface_id', binding->>'interface_id'
                )
        END
    ), '[]'::jsonb)
    FROM jsonb_array_elements(services) AS svc,
         jsonb_array_elements(COALESCE(svc->'bindings', '[]'::jsonb)) AS binding
    WHERE binding->>'id' IS NOT NULL
)
WHERE bindings = '[]'::jsonb
  AND EXISTS (
    SELECT 1 FROM jsonb_array_elements(services) s
    WHERE s->'bindings' IS NOT NULL AND jsonb_array_length(s->'bindings') > 0
  );

-- ============================================================================
-- STEP 4: Transform hosts - remove obsolete fields
-- ============================================================================
-- Remove: target, interfaces, services, ports

UPDATE topologies
SET hosts = (
    SELECT COALESCE(jsonb_agg(
        host - 'target' - 'interfaces' - 'services' - 'ports'
    ), '[]'::jsonb)
    FROM jsonb_array_elements(hosts) AS host
)
WHERE EXISTS (
    SELECT 1 FROM jsonb_array_elements(hosts) h
    WHERE h ? 'target' OR h ? 'interfaces' OR h ? 'services' OR h ? 'ports'
);

-- ============================================================================
-- STEP 5: Transform services - update bindings to new format
-- ============================================================================
-- The bindings inside services also need to be updated to include service_id and network_id
-- Old: {type, id, interface_id, port_id?}
-- New: {id, created_at, updated_at, service_id, network_id, type, interface_id, port_id?}
-- IMPORTANT: Keep already-migrated bindings as-is, only transform old-format ones

UPDATE topologies
SET services = (
    SELECT COALESCE(jsonb_agg(
        CASE
            WHEN svc->'bindings' IS NULL OR jsonb_array_length(svc->'bindings') = 0 THEN
                svc
            ELSE
                jsonb_set(
                    svc,
                    '{bindings}',
                    (
                        SELECT COALESCE(jsonb_agg(
                            CASE
                                -- Already migrated binding (has service_id) - keep as-is
                                WHEN binding->>'service_id' IS NOT NULL THEN
                                    binding
                                -- Old Interface binding - transform
                                WHEN binding->>'type' = 'Interface' THEN
                                    jsonb_build_object(
                                        'id', binding->>'id',
                                        'created_at', COALESCE(svc->>'created_at', NOW()::text),
                                        'updated_at', COALESCE(svc->>'updated_at', NOW()::text),
                                        'service_id', svc->>'id',
                                        'network_id', svc->>'network_id',
                                        'type', 'Interface',
                                        'interface_id', binding->>'interface_id'
                                    )
                                -- Old Port binding - transform
                                ELSE
                                    jsonb_build_object(
                                        'id', binding->>'id',
                                        'created_at', COALESCE(svc->>'created_at', NOW()::text),
                                        'updated_at', COALESCE(svc->>'updated_at', NOW()::text),
                                        'service_id', svc->>'id',
                                        'network_id', svc->>'network_id',
                                        'type', 'Port',
                                        'port_id', binding->>'port_id',
                                        'interface_id', binding->>'interface_id'
                                    )
                            END
                        ), '[]'::jsonb)
                        FROM jsonb_array_elements(svc->'bindings') AS binding
                        WHERE binding->>'id' IS NOT NULL
                    )
                )
        END
    ), '[]'::jsonb)
    FROM jsonb_array_elements(services) AS svc
)
WHERE EXISTS (
    SELECT 1 FROM jsonb_array_elements(services) s,
                  jsonb_array_elements(COALESCE(s->'bindings', '[]'::jsonb)) b
    WHERE b->>'service_id' IS NULL AND b->>'id' IS NOT NULL
);

-- ============================================================================
-- STEP 6: Transform subnets - clean subnet_type quotes
-- ============================================================================
-- Old: "\"Lan\"" (JSON-encoded string)
-- New: "Lan" (plain string)

UPDATE topologies
SET subnets = (
    SELECT COALESCE(jsonb_agg(
        CASE
            WHEN subnet->>'subnet_type' LIKE '"%"' THEN
                jsonb_set(subnet, '{subnet_type}', to_jsonb(TRIM(BOTH '"' FROM subnet->>'subnet_type')))
            ELSE
                subnet
        END
    ), '[]'::jsonb)
    FROM jsonb_array_elements(subnets) AS subnet
)
WHERE EXISTS (
    SELECT 1 FROM jsonb_array_elements(subnets) s
    WHERE s->>'subnet_type' LIKE '"%"'
);

-- ============================================================================
-- STEP 7: Transform groups - group_type, binding_ids, color
-- ============================================================================
-- Old group_type: {"group_type": "RequestPath", "service_bindings": ["uuid1", ...]}
-- New group_type: "RequestPath" (just the discriminant)
-- New binding_ids: ["uuid1", ...] (from old service_bindings)
-- Color: normalize to INITCAP

UPDATE topologies
SET groups = (
    SELECT COALESCE(jsonb_agg(
        -- Start with group minus old fields we'll replace
        (grp - 'group_type' - 'service_bindings')
        -- Add the new group_type (discriminant only)
        || jsonb_build_object(
            'group_type',
            CASE
                WHEN grp->'group_type'->>'group_type' IS NOT NULL THEN
                    grp->'group_type'->>'group_type'
                WHEN jsonb_typeof(grp->'group_type') = 'string' THEN
                    grp->>'group_type'
                ELSE
                    'RequestPath'
            END
        )
        -- Add binding_ids from old service_bindings
        || jsonb_build_object(
            'binding_ids',
            COALESCE(grp->'group_type'->'service_bindings', '[]'::jsonb)
        )
        -- Normalize color to INITCAP
        || CASE
            WHEN grp->>'color' IS NOT NULL AND grp->>'color' != INITCAP(grp->>'color') THEN
                jsonb_build_object('color', INITCAP(grp->>'color'))
            ELSE
                '{}'::jsonb
        END
    ), '[]'::jsonb)
    FROM jsonb_array_elements(groups) AS grp
)
WHERE EXISTS (
    SELECT 1 FROM jsonb_array_elements(groups) g
    WHERE jsonb_typeof(g->'group_type') = 'object'
       OR (g->>'color' IS NOT NULL AND g->>'color' != INITCAP(g->>'color'))
);
