-- When a scan last carried evidence that something is adjacent to this port.
--
-- `last_seen_at` answers "was this port observed", which is not the same question as "is this
-- adjacency still supported". A port keeps appearing in the ifTable long after its neighbour
-- record stops arriving, so a link whose evidence has completely disappeared still reads
-- `Current` and is still drawn solid and port-precise. `neighbor` is a column on `interfaces`,
-- so the link has no subject of its own to be judged on; this is that subject.
--
-- Stamped only by a scan that actually read an LLDP chassis id, a CDP device id, or a bridge-FDB
-- port holding exactly one address — the three sources L2 resolution consumes. Left untouched
-- otherwise, so it always names the last scan that saw a neighbour rather than the last scan that
-- ran. Judged against the same `networks.stale_after_hours` window as every other freshness
-- verdict; there is deliberately no second window.
--
-- NULL means no scan has ever carried evidence for this row, which must read as *unknown* and
-- never as stale: every row predating this column starts NULL, and a link must not be flagged —
-- or lose its binding — the moment this ships. Hence nullable with no default and no backfill.
--
-- Additive and prod-safe: ADD COLUMN with no default is metadata-only (no rewrite), which matters
-- because `interfaces` carries SCD2 history rows. No index — nothing filters on this column in
-- SQL; the verdict is derived per request. No contract step: nothing is renamed or dropped, and
-- older servers ignore the column.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

ALTER TABLE interfaces
    ADD COLUMN IF NOT EXISTS neighbor_seen_at TIMESTAMPTZ;
