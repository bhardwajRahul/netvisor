-- Per-user email preferences. The discovery_digest toggle gates the per-
-- session digest email shipped in Phase 2 round 3.
--
-- ADD COLUMN ... DEFAULT on a JSONB column is metadata-only in PG11+, so
-- this is safe on a populated table.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE users
    ADD COLUMN email_settings jsonb NOT NULL DEFAULT '{"discovery_digest": true}'::jsonb;
