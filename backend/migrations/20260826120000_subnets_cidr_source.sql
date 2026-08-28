-- How much a subnet's CIDR is trusted (GH #668).
--
-- Every CIDR Scanopy holds today came from somewhere it could read directly: a device's own
-- `ipAdEntNetMask`, a daemon's own interface, a container runtime's IPAM config, a person typing
-- one in. Inferring a range from the addresses LLDP/CDP neighbours publish for themselves adds a
-- fourth kind that is none of those — a guess, good enough to draw and not good enough to assert —
-- and a guess that cannot be told apart from an observation is worse than no guess at all.
--
-- `cidr_source` is an `AttributeSource`, the same shared ladder every provenanced value uses —
-- values are serde's adjacently tagged form, so a fieldless source is `{"type":"Manual"}` and one
-- carrying a probe is `{"type":"Probe","probe":"Snmp"}`. It started life as a private three-rung
-- `SubnetCidrSource`; this migration is edited in place rather than followed by a second one
-- because it has shipped in no release, so the column has never existed anywhere as the old shape.
--
-- Additive and prod-safe: ADD COLUMN with a non-volatile default is metadata-only on PG11+, no
-- rewrite.
--
-- `Unspecified` is the default because it is the honest description of every pre-existing row:
-- each came from something that could read a range directly — a device's own `ipAdEntNetMask`, a
-- daemon's own interface, a container runtime's IPAM config — and nothing recorded which. Naming
-- one of them would assert something no row can support. Nothing is lost by the choice: discovery
-- never moves an existing subnet's CIDR (`SubnetService::create` dedups *by* CIDR, and range
-- inference skips every range a live subnet already holds), so a different value arriving for an
-- existing row only ever comes from a person.
--
-- No contract step: nothing is renamed or dropped, and older servers ignore the column.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE subnets
    ADD COLUMN IF NOT EXISTS cidr_source JSONB NOT NULL DEFAULT '{"type":"Unspecified"}'::jsonb;

-- A subnet a person created is one they asserted, which is the top rung and is never displaced by
-- anything a scan reads. Single statement rather than a batched loop: `subnets` is small by nature
-- (one row per segment, not per host), and the predicate is indexed by the table's own size rather
-- than needing one.
UPDATE subnets
   SET cidr_source = '{"type":"Manual"}'::jsonb
 WHERE source->>'type' = 'Manual'
   AND cidr_source = '{"type":"Unspecified"}'::jsonb;
