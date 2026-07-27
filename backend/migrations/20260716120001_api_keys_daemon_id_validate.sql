-- Validate the api_keys.daemon_id foreign key added NOT VALID in 20260716120000.
--
-- Split into its own migration (transaction) on purpose: running VALIDATE in the same
-- transaction as the ADD CONSTRAINT would hold a lock that blocks reads for the whole
-- validation scan. On its own, VALIDATE takes only a SHARE UPDATE EXCLUSIVE lock and does
-- not block reads or writes. The backfill in 20260716120000 has already populated daemon_id,
-- so every existing row satisfies the constraint.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE api_keys VALIDATE CONSTRAINT api_keys_daemon_id_fkey;
