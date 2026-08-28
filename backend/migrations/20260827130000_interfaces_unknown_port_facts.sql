-- Let an interface record that we do not know its SNMP facts, rather than inventing them.
--
-- LLDP and CDP tell us a far end has a port and what that port is called, and nothing else about
-- it: no ifIndex, no ifType, no admin or operational status. Those four columns were NOT NULL, so
-- the only way to store such a port was to make values up -- ifIndex 0, ifType 1 "other", both
-- statuses Up -- which is indistinguishable from a real walk that read exactly those. A port that
-- is down would have been recorded as up, and every inferred port would have claimed ifIndex 0.
--
-- Identity does not depend on ifIndex, so nothing here weakens it. The
-- `(host_id, if_index)` unique constraint was dropped in
-- 20260417000000_reindex_interfaces_identity.sql; the live uniqueness is
-- `(host_id, if_name) WHERE if_name IS NOT NULL AND valid_to IS NULL`, and
-- `match_existing_interface` tries if_name first. An interface minted from a neighbour
-- advertisement is keyed on the port name the far end published, which is the same string that
-- device's own ifTable returns as ifName -- so a later SNMP walk upgrades the row in place
-- instead of duplicating it. if_descr stays NOT NULL and is populated with that same name.
--
-- squawk flags this as ban-drop-not-null and it is right to: the previously released container
-- reads if_index with `row.get::<i32>`, which panics rather than errors on NULL, so an old and a
-- new container cannot both be live while NULL rows exist. This ships in a release cut as
-- `downtime` (org.scanopy.deploy_mode), so the two never coexist and the DDL and the first NULL
-- write go out together. The exclusion is registered per-file in
-- backend/scripts/lint-migrations.sh, not repo-wide. See the expand/contract ledger.
--
-- Catalog-only: DROP NOT NULL updates pg_attribute and neither rewrites nor scans the table,
-- which matters because `interfaces` carries SCD2 history rows. No backfill -- every existing row
-- keeps the value it has, and NULL means "never read" only for rows written from here on.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE interfaces
    ALTER COLUMN if_index DROP NOT NULL,
    ALTER COLUMN if_type DROP NOT NULL,
    ALTER COLUMN admin_status DROP NOT NULL,
    ALTER COLUMN oper_status DROP NOT NULL;
