-- Create the snapshots table. Each row is a point-in-time capture of a
-- network's topology. Closed entity rows + snapshot topology rows carry
-- snapshot_id FKs (added in the next migration) so retention is a single
-- DELETE FROM snapshots WHERE taken_at < cutoff that cascades.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE snapshots (
    id                  UUID PRIMARY KEY,
    network_id          UUID NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
    taken_at            TIMESTAMPTZ NOT NULL,
    created_by_user_id  UUID NULL REFERENCES users(id) ON DELETE SET NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_snapshots_network_taken_at
    ON snapshots (network_id, taken_at DESC);
