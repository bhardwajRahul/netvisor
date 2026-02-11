-- Add entity_tags column to topologies table for storing tag definitions snapshot
ALTER TABLE topologies ADD COLUMN entity_tags JSONB NOT NULL DEFAULT '[]';
