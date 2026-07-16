-- Server-provisioned daemon identity: bind an api_key 1:1 to its daemon.
--
-- Adds api_keys.daemon_id as the forward link (the reverse of daemons.api_key_id).
-- A 1:1 key resolves to exactly one daemon, which lets the two-flag install work and
-- retires the untrusted X-Daemon-ID header for provisioned daemons. Legacy network-shared
-- keys keep daemon_id = NULL and are exempt, so coexistence holds.
--
-- ON DELETE CASCADE: deleting a daemon also deletes its 1:1 key, so a rotated-out daemon
-- never leaves an orphaned live credential that can still authenticate to the network.
-- (The reverse, daemons.api_key_id, stays ON DELETE SET NULL: deleting a key disables the
-- daemon but keeps its record + history so it can be re-provisioned.)
--
-- The FK is VALIDATEd in the paired 20260716120001 migration (a separate transaction —
-- validating in the same transaction as the ADD would hold a lock blocking reads), and the
-- partial UNIQUE index that enforces 1:1 is created CONCURRENTLY in 20260716120002
-- (CONCURRENTLY cannot run inside a transaction).

SET lock_timeout = '5s';
SET statement_timeout = '30s';

-- Additive, nullable: safe on a populated table, no rewrite, no default backfill.
ALTER TABLE api_keys ADD COLUMN daemon_id UUID;

-- Add the FK NOT VALID first (fast, takes only a brief lock and does not scan the table),
-- then VALIDATE in a separate statement (scans without blocking concurrent writes).
ALTER TABLE api_keys
    ADD CONSTRAINT api_keys_daemon_id_fkey
    FOREIGN KEY (daemon_id) REFERENCES daemons(id) ON DELETE CASCADE
    NOT VALID;

-- Backfill the forward link from the existing reverse link. Bounded by the number of
-- provisioned (ServerPoll) daemons that already carry an api_key_id, which is small
-- (a handful per org) and far below the ~1000-row batching threshold, so a single
-- statement is safe here. Legacy DaemonPoll rows have api_key_id = NULL and are skipped.
UPDATE api_keys ak
SET daemon_id = d.id
FROM daemons d
WHERE d.api_key_id = ak.id
  AND ak.daemon_id IS NULL;
