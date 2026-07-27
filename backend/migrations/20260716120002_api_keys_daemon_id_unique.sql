-- no-transaction
--
-- Enforce the 1:1 daemon<->key binding with a partial UNIQUE index on api_keys.daemon_id.
-- CONCURRENTLY keeps the build from blocking writes on the populated api_keys table, and
-- requires running outside a transaction (hence the header above and the separate file from
-- the 20260716120000 column/backfill migration).
--
-- WHERE daemon_id IS NOT NULL exempts legacy network-shared keys (daemon_id = NULL), so any
-- number of them coexist; only provisioned 1:1 keys are constrained to one-per-daemon.
--
-- Release pre-check: the build fails if two live keys already map to the same daemon. The
-- backfill maps from daemons.api_key_id (one key per daemon) and provisioning mints a fresh
-- key each time, so no collision is expected; the release-runner test audits messy/orphan
-- data before this applies.
SET lock_timeout = '5s';
-- statement_timeout = '0' disables the per-statement timeout so a large concurrent
-- build is not aborted midway; lock_timeout still bounds the brief lock it takes.
SET statement_timeout = '0';
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_api_keys_daemon_id
    ON api_keys (daemon_id)
    WHERE daemon_id IS NOT NULL;
