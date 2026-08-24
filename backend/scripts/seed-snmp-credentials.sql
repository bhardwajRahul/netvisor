-- Seed the SNMP credentials needed to scan the SNMP simulation environment.
--
-- The sim env (tools/snmp/SNMP-TEST-ENV.md) deliberately spreads its 21 devices across five
-- different credentials — three v2c communities, one v1-only, one v3 USM user — so that a scan
-- exercises credential selection, the v1/v2c/v3 negotiation paths, and the "try the next
-- credential" fallback rather than one community answering everything.
--
-- Each credential is assigned to every network in the database (Broadcast scope, via
-- network_credentials). Broadcast is the only option that works before a scan has run: PerHost
-- assignment needs hosts to exist, and the sim devices are exactly what the first scan discovers.
-- The daemon tries each assigned credential in turn against each IP.
--
-- Idempotent. Credential ids are derived from (organization_id, name), so re-running updates the
-- existing rows in place instead of accumulating duplicates, and any hand-edit to a seeded
-- credential is reset to the values here.

BEGIN;

-- The credential set, kept in one place so both inserts below agree on names.
CREATE TEMPORARY TABLE seed_snmp_credentials (name TEXT PRIMARY KEY, credential_type JSONB)
    ON COMMIT DROP;

INSERT INTO seed_snmp_credentials (name, credential_type) VALUES
    -- .230 switch-core-01, .231 switch-access-01, .235 ap-wireless-01, .238 switch-exos-01,
    -- .239 switch-voss-01, .240 switch-netgear-01, .241 switch-aruba-01, .243 switch-flaky-01,
    -- .244 switch-dlink-01, .245 switch-tplink-01, .246 switch-unsorted-01,
    -- .247 switch-macport-01, .248 switch-mute-01, .249 switch-stuck-01,
    -- .250 switch-dell-01
    ('SNMP sim — netdefault (v2c)',
     '{"type":"SnmpV2c","community":{"mode":"Inline","value":"netdefault"}}'),
    -- .232 router-gw-01, .233 firewall-01
    ('SNMP sim — secret42 (v2c)',
     '{"type":"SnmpV2c","community":{"mode":"Inline","value":"secret42"}}'),
    -- .234 printer-lobby, .242 switch-omada-01
    ('SNMP sim — public (v2c)',
     '{"type":"SnmpV2c","community":{"mode":"Inline","value":"public"}}'),
    -- .236 legacy-switch-01 — VACM refuses v2c/v3, so this must be a v1 credential
    ('SNMP sim — legacyv1 (v1)',
     '{"type":"SnmpV1","community":{"mode":"Inline","value":"legacyv1"}}'),
    -- .237 secure-switch-01 — USM AuthPriv, SHA-256 / AES-128; no rocommunity, so v1/v2c are denied
    ('SNMP sim — scanopyv3 (v3 AuthPriv)',
     '{"type":"SnmpV3",
       "security_name":"scanopyv3",
       "auth_protocol":"Sha256",
       "auth_password":{"mode":"Inline","value":"authpass12345"},
       "priv_protocol":"Aes128",
       "priv_password":{"mode":"Inline","value":"privpass12345"}}');

-- One credential per (organization owning a network, credential). md5(...)::uuid gives a stable
-- id per organization, so two organizations each get their own copy rather than fighting over one
-- row, and a re-run finds the same ids.
INSERT INTO credentials (id, organization_id, name, credential_type, created_at, updated_at)
SELECT
    md5(org.id::text || c.name)::uuid,
    org.id,
    c.name,
    c.credential_type,
    NOW(),
    NOW()
FROM (SELECT DISTINCT organization_id AS id FROM networks) org
CROSS JOIN seed_snmp_credentials c
ON CONFLICT (id) DO UPDATE
    SET name = EXCLUDED.name,
        credential_type = EXCLUDED.credential_type,
        updated_at = NOW();

INSERT INTO network_credentials (network_id, credential_id)
SELECT n.id, md5(n.organization_id::text || c.name)::uuid
FROM networks n
CROSS JOIN seed_snmp_credentials c
ON CONFLICT (network_id, credential_id) DO NOTHING;

-- Report what the run actually touched. A network count of 0 is the interesting case: it means
-- the database has no networks yet, so nothing was seeded and nothing will scan.
SELECT
    (SELECT COUNT(*) FROM networks) AS networks,
    (SELECT COUNT(*) FROM seed_snmp_credentials) AS credentials_per_network,
    (SELECT COUNT(*) FROM network_credentials nc
      JOIN seed_snmp_credentials c
        ON nc.credential_id = md5((SELECT organization_id::text FROM networks WHERE id = nc.network_id) || c.name)::uuid
    ) AS assignments;

COMMIT;
