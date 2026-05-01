-- Add Pattern B flag columns on organizations driven by the
-- OrganizationBillingSubscriber. These power Phase 5 eligibility gates and
-- the downgrade banner without needing an event-sourced ledger.
--
-- ADD COLUMN ... DEFAULT false on a boolean is metadata-only in PG11+,
-- so this is safe on a populated table. NULL defaults are also metadata-only.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE organizations
    ADD COLUMN last_paused_at           timestamptz,
    ADD COLUMN trial_extended_used      boolean NOT NULL DEFAULT false,
    ADD COLUMN last_downgrade_at        timestamptz,
    ADD COLUMN last_downgrade_from_plan jsonb;
