-- Contract migration for two 0.17.4 column drops (see docs/db-expand-contract-ledger.md).
--
-- 1. credentials.target_ips (INET[])
--    Expand: superseded by integration_targets / host_credentials.
--    Code-removal (0.17.3): storage r/w, the base.rs field, and the dedup +
--    loopback readers were all removed. Only stale doc comments remain.
-- 2. daemons.capabilities (JSONB)
--    Expand: interfaced subnets moved to the daemon_interfaced_subnets junction
--    (0.17.3).
--    Code-removal (0.17.3): the DaemonCapabilities type was deleted; the column
--    is neither read nor written. The inbound LegacyCapabilities API blob is a
--    wire type, not this column.
--
-- Precondition: 0.17.3 is fully deployed (cloud) — no running container reads or
-- writes either column — so the expand-and-contract "contract" is paid and these
-- drops are safe under a rolling deploy. Do NOT merge/release this migration
-- before the founder confirms 0.17.3 is fully deployed.
--
-- squawk flags every DROP COLUMN as unsafe (zero-downtime correctness normally
-- requires expand-and-contract). That requirement is already satisfied here, so
-- ban-drop-column is excluded for this file in scripts/lint-migrations.sh.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE credentials
    DROP COLUMN target_ips;

ALTER TABLE daemons
    DROP COLUMN capabilities;
