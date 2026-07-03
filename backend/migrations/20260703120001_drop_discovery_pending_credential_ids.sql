-- Drop `discovery.pending_credential_ids` (added 20260321120000). It is superseded
-- by `discovery.integration_targets` and has NO live code readers or writers at
-- v0.17.2 (only stale doc comments; it is not a field on the `Discovery` struct).
-- Because no deployed container references the column, dropping it now is safe even
-- under a rolling deploy — the expand-and-contract "contract" is already paid.
--
-- squawk flags every DROP COLUMN as unsafe (zero-downtime correctness normally
-- requires expand-and-contract). Here that requirement is already satisfied, so the
-- ban-drop-column rule is excluded for this file in scripts/lint-migrations.sh.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE discovery
    DROP COLUMN pending_credential_ids;
