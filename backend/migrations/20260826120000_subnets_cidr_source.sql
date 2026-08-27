-- How much a subnet's CIDR is trusted (GH #668).
--
-- Every CIDR Scanopy holds today came from somewhere it could read directly: a device's own
-- `ipAdEntNetMask`, a daemon's own interface, a container runtime's IPAM config, a person typing
-- one in. Inferring a range from the addresses LLDP/CDP neighbours publish for themselves adds a
-- fourth kind that is none of those — a guess, good enough to draw and not good enough to assert —
-- and a guess that cannot be told apart from an observation is worse than no guess at all.
--
-- `cidr_source` is the discriminant of the `SubnetCidrSource` enum, so the values here are that
-- enum's variant names: `Inferred` < `Observed` < `Confirmed`, and the ordering is the whole
-- mechanism (`Subnet::apply_cidr` compares rungs; nothing else writes the pair).
--
-- Additive and prod-safe: ADD COLUMN with a constant default is metadata-only on PG11+, no
-- rewrite. `Observed` is the default because it is what every pre-existing row actually is —
-- nothing before this migration could have inferred a range.
--
-- No contract step: nothing is renamed or dropped, and older servers ignore the column.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE subnets
    ADD COLUMN IF NOT EXISTS cidr_source TEXT NOT NULL DEFAULT 'Observed';

-- A subnet a person created is one they asserted, which is the top rung and is never displaced by
-- anything a scan reads. Single statement rather than a batched loop: `subnets` is small by nature
-- (one row per segment, not per host), and the predicate is indexed by the table's own size rather
-- than needing one.
UPDATE subnets
   SET cidr_source = 'Confirmed'
 WHERE source->>'type' = 'Manual'
   AND cidr_source = 'Observed';
