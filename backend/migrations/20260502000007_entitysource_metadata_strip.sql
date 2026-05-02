-- Strip the now-redundant `metadata: Vec<DiscoveryMetadata>` field from
-- EntitySource JSONB. The metadata's only consumer was a list of "last 5
-- discoveries" which has been superseded by the typed
-- last_discovery_id / first_discovery_id FKs (and last_seen_at) on the
-- entity rows themselves.
--
-- Runs AFTER scd2_backfill_with_metadata (which salvages dates from
-- metadata into last_seen_at).
--
-- The Rust side drops the field from the EntitySource enum variants in
-- the same release; this migration reshapes existing JSONB to match.

SET lock_timeout = '5s';

-- Strip from variants that carry metadata: Discovery and DiscoveryWithMatch.
-- The discriminator is `type` (cf. #[serde(tag = "type")]).
UPDATE hosts
SET source = source - 'metadata'
WHERE source ? 'metadata'
  AND source->>'type' IN ('Discovery', 'DiscoveryWithMatch');

UPDATE services
SET source = source - 'metadata'
WHERE source ? 'metadata'
  AND source->>'type' IN ('Discovery', 'DiscoveryWithMatch');

UPDATE subnets
SET source = source - 'metadata'
WHERE source ? 'metadata'
  AND source->>'type' IN ('Discovery', 'DiscoveryWithMatch');

UPDATE dependencies
SET source = source - 'metadata'
WHERE source ? 'metadata'
  AND source->>'type' IN ('Discovery', 'DiscoveryWithMatch');

UPDATE vlans
SET source = source - 'metadata'
WHERE source ? 'metadata'
  AND source->>'type' IN ('Discovery', 'DiscoveryWithMatch');
