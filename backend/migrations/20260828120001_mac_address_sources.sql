-- Provenance for the MAC addresses on `ip_addresses` and `interfaces`.
--
-- Four sites produce a MAC that is currently indistinguishable downstream: our own ARP sweep, a
-- device reporting its own `ifPhysAddress`, a *router's* `ipNetToMediaTable` cache — a third party
-- talking about somebody else — and a controller reporting a device it manages. Today the first
-- one to write wins for good, enforced by `preserve_immutable_fields` pinning whatever is already
-- set, so an ARP reply cannot correct a MAC a forwarding table reported.
--
-- That matters beyond tidiness: §6's host-minting rule turns on this distinction. Minting a host
-- from a MAC we have never contacted, on a third party's say-so, is how ghost hosts get created —
-- so minting needs `Queried`-or-better provenance, and there is no way to ask for that until the
-- column exists.
--
-- Values are `AttributeSource` as serde writes it, adjacently tagged, matching the columns added by
-- 20260828120000_hosts_attribute_sources.sql.
--
-- `Unspecified` is the right default rather than `Manual`, and unlike the host attributes there is
-- nothing to backfill: every MAC on file came from discovery, and none of them recorded which
-- vantage point it came from. `Unspecified` says exactly that, and §6's minting rule then refuses
-- to conjure hosts out of history it cannot vouch for — which is the conservative direction.
--
-- Additive and prod-safe: `ADD COLUMN` with a non-volatile default is metadata-only on PG11+, so
-- no rewrite and no batched loop. No contract step: nothing is renamed or dropped.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE ip_addresses
    ADD COLUMN IF NOT EXISTS mac_address_source JSONB NOT NULL DEFAULT '{"type":"Unspecified"}'::jsonb;

ALTER TABLE interfaces
    ADD COLUMN IF NOT EXISTS mac_address_source JSONB NOT NULL DEFAULT '{"type":"Unspecified"}'::jsonb;
