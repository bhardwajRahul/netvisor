-- Add snapshot_id FK columns to topologies + every Snapshotable entity table.
-- NULL on live rows; non-NULL on closed copies and on snapshot topology rows.
-- ON DELETE CASCADE means deleting a snapshots row reaps closed entity rows
-- and snapshot topology rows automatically — retention becomes one DELETE.
--
-- App-layer invariant (not enforced as DB CHECK because of the SCD2 transition
-- window): (valid_to IS NULL) ⇔ (snapshot_id IS NULL). Live rows: both NULL.
-- Closed rows: both set.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE topologies
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE hosts
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE ip_addresses
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE ports
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE services
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE interfaces
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE bindings
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE subnets
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE vlans
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE subnet_vlans
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE dependencies
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE dependency_members
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE tags
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;

ALTER TABLE entity_tags
    ADD COLUMN snapshot_id UUID NULL REFERENCES snapshots(id) ON DELETE CASCADE;
