-- Per-network staleness threshold: how long an entity may go unobserved by
-- discovery before the UI and the discovery digest render it as "Stale".
--
-- Network-scoped rather than org-scoped because staleness is only meaningful
-- relative to scan cadence, and cadence is a property of a network's
-- discoveries. A network swept every 15 minutes and one swept monthly need
-- very different thresholds.
--
-- Strictly additive against a production table: nullable, no NOT NULL, no DDL
-- default and no backfill, so there is no table rewrite and no lock beyond the
-- brief catalog update. NULL means "unset" — the effective default lives in
-- application code (DEFAULT_STALE_AFTER_HOURS), which keeps the fallback in one
-- place and lets it change without a second migration.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE networks
    ADD COLUMN stale_after_hours BIGINT NULL;
