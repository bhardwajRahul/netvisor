-- Add `users.session_epoch` to support session invalidation on password
-- change/reset ("log out everywhere"). Each session records the epoch it was
-- created under; the auth extractor rejects a session whose epoch is below the
-- user's current value, and a password change bumps the user's epoch.
--
-- Additive and backward-compatible: a constant NOT NULL DEFAULT does not rewrite
-- the table in modern Postgres, and old binaries simply ignore the column.
-- Sessions created before this migration carry no epoch and are treated as
-- epoch 0, so they stay valid until the user's epoch is first bumped.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE users
    ADD COLUMN session_epoch BIGINT NOT NULL DEFAULT 0;
