-- Add Pattern B flag columns on organizations driven by the
-- OrganizationBillingSubscriber. These power Phase 5 eligibility gates and
-- the downgrade banner without needing an event-sourced ledger.
--
-- ADD COLUMN ... DEFAULT false on a boolean is metadata-only in PG11+,
-- so this is safe on a populated table. NULL defaults are also metadata-only.

SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE organizations
    ADD COLUMN last_paused_at                   timestamptz,
    ADD COLUMN trial_extended_used              boolean NOT NULL DEFAULT false,
    ADD COLUMN last_downgrade_at                timestamptz,
    ADD COLUMN last_downgrade_from_plan         jsonb,
    -- Save-offer discount tracking. `last_discount_at` drives the once-ever
    -- eligibility gate (loosen to a timestamp comparison if the policy ever
    -- moves to a rolling window). `discount_save_offer_percent_off` carries
    -- the live percentage so the BillingTab chip reads the actual value
    -- rather than a hard-coded one — different coupons render correctly
    -- without a code change. `discount_save_offer_active_until` gates the
    -- chip's render condition (`> now()`); the row is otherwise inert.
    ADD COLUMN last_discount_at                 timestamptz,
    ADD COLUMN discount_save_offer_percent_off  bigint,
    ADD COLUMN discount_save_offer_active_until timestamptz,
    -- Mirror of Stripe sub.items.data[0].current_period_end, written by the
    -- OrganizationBillingSubscriber on every event that re-anchors the
    -- billing period (checkout, trial start/end, plan change, renewal,
    -- pause/resume, reactivate). Cleared by SubscriptionCancelled. Powers
    -- the "Next renewal on …" / "First invoice on …" / "Subscription ends
    -- on …" line in BillingPlanModal; the UI interprets the value based
    -- on plan_status.
    ADD COLUMN next_renewal_at                  timestamptz;
