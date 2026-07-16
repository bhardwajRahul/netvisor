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
CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS idx_api_keys_daemon_id
    ON api_keys (daemon_id)
    WHERE daemon_id IS NOT NULL;
