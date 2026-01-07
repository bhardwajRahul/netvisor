-- Entity Tags Junction Table
-- Replaces per-entity tags columns with a centralized many-to-many relationship

-- Create the junction table
CREATE TABLE entity_tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_id UUID NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (entity_id, entity_type, tag_id)
);

-- Index for finding all entities with a specific tag
CREATE INDEX idx_entity_tags_tag_id ON entity_tags(tag_id);

-- Index for finding all tags for a specific entity
CREATE INDEX idx_entity_tags_entity ON entity_tags(entity_id, entity_type);

-- Migrate existing tag data from all taggable entities
-- Using unnest to expand the UUID arrays into individual rows

-- Hosts
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'Host', unnest(tags) FROM hosts WHERE tags IS NOT NULL AND array_length(tags, 1) > 0;

-- Services
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'Service', unnest(tags) FROM services WHERE tags IS NOT NULL AND array_length(tags, 1) > 0;

-- Subnets
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'Subnet', unnest(tags) FROM subnets WHERE tags IS NOT NULL AND array_length(tags, 1) > 0;

-- Groups
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'Group', unnest(tags) FROM groups WHERE tags IS NOT NULL AND array_length(tags, 1) > 0;

-- Networks
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'Network', unnest(tags) FROM networks WHERE tags IS NOT NULL AND array_length(tags, 1) > 0;

-- Discovery
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'Discovery', unnest(tags) FROM discovery WHERE tags IS NOT NULL AND array_length(tags, 1) > 0;

-- Daemons
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'Daemon', unnest(tags) FROM daemons WHERE tags IS NOT NULL AND array_length(tags, 1) > 0;

-- Daemon API Keys (stored in api_keys table)
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'DaemonApiKey', unnest(tags) FROM api_keys WHERE tags IS NOT NULL AND array_length(tags, 1) > 0;

-- User API Keys
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'UserApiKey', unnest(tags) FROM user_api_keys WHERE tags IS NOT NULL AND array_length(tags, 1) > 0;

-- Drop the legacy trigger that tries to update the old tags columns
-- The junction table's ON DELETE CASCADE now handles tag cleanup automatically
DROP TRIGGER IF EXISTS trigger_remove_deleted_tag_from_entities ON tags;
DROP FUNCTION IF EXISTS remove_deleted_tag_from_entities();

-- Drop the legacy tags columns from all entities
ALTER TABLE hosts DROP COLUMN IF EXISTS tags;
ALTER TABLE services DROP COLUMN IF EXISTS tags;
ALTER TABLE subnets DROP COLUMN IF EXISTS tags;
ALTER TABLE groups DROP COLUMN IF EXISTS tags;
ALTER TABLE networks DROP COLUMN IF EXISTS tags;
ALTER TABLE discovery DROP COLUMN IF EXISTS tags;
ALTER TABLE daemons DROP COLUMN IF EXISTS tags;
ALTER TABLE api_keys DROP COLUMN IF EXISTS tags;
ALTER TABLE user_api_keys DROP COLUMN IF EXISTS tags;
