-- DOWNTIME MIGRATION
--
-- Contract phase of renaming organizations.plan_limit_notifications -> notifications.
-- Pairs with the expand in 20260727120000_rename_plan_limit_notifications_to_notifications.sql
-- (shipped v0.17.6), which added `notifications` and backfilled it from this column.
--
-- Sequencing: the expand shipped in v0.17.6 and the contract was due in v0.17.7,
-- but slipped through v0.17.7 and v0.17.8. It lands here in v0.17.9. See the
-- expand/contract ledger (docs/db-expand-contract-ledger.md, maintained by the
-- coordinator outside this repo) for the full row.
--
-- WHY THIS NEEDS A DOWNTIME DEPLOY: v0.17.6 through v0.17.8 server containers
-- dual-write plan_limit_notifications on every organization write. Under a
-- rolling deploy an old container would still be serving traffic while this
-- migration runs, and its INSERT/UPDATE would fail against a dropped column.
-- v0.17.9 MUST therefore be cut with `Deploy-Mode: downtime` in the release
-- body -- select "downtime" in the prepare_release workflow dispatch. Without
-- that marker release.yml runs the backward-compat container harness against a
-- schema that assumes otherwise, and the deploy itself may be performed as a
-- rolling one.
--
-- Rollback is roll-forward only: reverting to v0.17.8 after this applies breaks
-- every organization write.
--
-- Squawk flags every DROP COLUMN as unsafe, because correctness under a
-- zero-downtime deploy requires expand-and-contract. That contract is being
-- paid here, in a coordinated downtime window, so we accept the unsafety.
-- Registered in DOWNTIME_FILES in backend/scripts/lint-migrations.sh.
--
-- The column has no dependent objects -- no view, index, constraint or trigger
-- references it -- so no CASCADE is needed. `organizations` is one row per org,
-- and DROP COLUMN is metadata-only, so the ACCESS EXCLUSIVE lock is brief.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE organizations
    DROP COLUMN plan_limit_notifications;
