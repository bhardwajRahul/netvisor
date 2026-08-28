-- no-transaction
--
-- Index the network-scoped MAC lookup that MAC-based host identity does.
--
-- Host dedup gained a MAC tier, consulted only once the address and chassis-id tiers have failed:
-- a device with no IP at all has nothing else to be recognised by, and a device whose address
-- moved is otherwise minted a second time. It answers "does this network already hold a host
-- carrying this address?" with a lookup keyed on `(network_id, mac_address)` against both
-- `ip_addresses` and `interfaces`.
--
-- `interfaces` already has what that needs: `idx_interfaces_mac_address`, a plain index on
-- `mac_address` alone (20260410000000_rename_interfaces_and_if_entries.sql:26, renamed from the
-- if_entries original). A MAC is selective enough that the network predicate is a cheap recheck on
-- the handful of rows it returns, so no second index there.
--
-- `ip_addresses` does not. Its only MAC index is `(host_id, mac_address)` partial
-- (20260106000000_interface_mac_index.sql, written when the table was still named `interfaces`),
-- and a network-wide lookup supplies no host_id — so the leading column is unusable and the query
-- would sequentially scan every address row in the deployment. Hence this one, with the columns
-- the other way round.
--
-- Partial on `mac_address IS NOT NULL` for the same reason the host-scoped one is: a large share
-- of address rows carry no MAC (anything discovered off-L2), and they can never satisfy an
-- equality predicate on it. Matches what the planner needs, and keeps the index off the rows that
-- would only pad it.
--
-- CONCURRENTLY because `ip_addresses` is populated and hot on the discovery path; it cannot run
-- inside a transaction, hence the header above and the file of its own. Additive and index-only:
-- no column changes, no rewrite, nothing an older container reads differently, so this is safe
-- under a rolling deploy and needs no coexistence window.

SET lock_timeout = '5s';
-- statement_timeout = '0' disables the per-statement timeout so a large concurrent build is not
-- aborted midway; lock_timeout still bounds the brief lock it takes.
SET statement_timeout = '0';

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_ip_addresses_network_mac
    ON ip_addresses (network_id, mac_address)
    WHERE mac_address IS NOT NULL;
