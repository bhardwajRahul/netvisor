-- Per-daemon integration targeting on the discovery entity.
--
-- Replaces the global, race-prone credential.target_ips (see #637) and the discovery
-- modal's one-shot pending_credential_ids with a single persistent per-daemon list of
-- integration targets (credentialed cred->IP, or credential-less local socket).
--
-- Additive, prod-safe: ADD COLUMN with a constant default is metadata-only (no rewrite).
-- target_ips and pending_credential_ids are retired in code this release and dropped in a
-- follow-on contract migration (expand-and-contract).
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE discovery
    ADD COLUMN integration_targets JSONB NOT NULL DEFAULT '[]'::jsonb;
