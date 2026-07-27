-- Expand phase of renaming organizations.plan_limit_notifications -> notifications.
--
-- The column now carries daemon-sunset ratchets in addition to plan-limit
-- ratchets, hence the generalized name. This is the EXPAND half of an
-- expand/contract: add the new column and backfill it from the old one. The
-- application dual-writes both columns this release so a not-yet-upgraded
-- container mid-rolling-deploy still reads fresh plan-limit ratchets. The old
-- column is dropped in the next release's CONTRACT migration -- see
-- docs/db-expand-contract-ledger.md.
--
-- Adding a JSONB column with a constant default is a metadata-only change in
-- Postgres 11+ (no table rewrite). `organizations` is small (one row per org),
-- so the backfill is a single statement.
SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE organizations ADD COLUMN notifications JSONB NOT NULL DEFAULT '{}';

UPDATE organizations SET notifications = plan_limit_notifications;
