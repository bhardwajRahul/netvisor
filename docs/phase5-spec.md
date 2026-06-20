# Phase 5 — billing/subscription retention: shipped vs. deferred

> **Status as of 2026-06.** This replaces the original pre-implementation Phase 5 spec and consolidates the former `phase5-features.md` (feature inventory) and `phase5-data-model.md` (data-model plan). Full audit in `DOCS_AUDIT_2026-06.md`.

Phase 5 covered conversion- and retention-side work across the trial → paid → cancel → downgrade lifecycle. This ledger records what landed, where it diverged from the spec, and the few items that were never built. All paths are relative to the backend/UI source tree.

## What shipped

**Cancel flow + reason capture + save offers.**
- UI: `ui/src/lib/features/billing/CancelSubscriptionModal.svelte`.
- Endpoint: `POST /api/billing/cancel` — `backend/src/server/billing/handlers.rs:651`, `Authorized<Owner>`.
- Types: `CancelReason` (`billing/types/base.rs:40`), `SaveOffer` (`base.rs:125`).
- Commits `5e72e0f8c`, `188185eea`.

**Pause / discount / extend / resume endpoints.**
- `pause_subscription` — `handlers.rs:520` (`POST /pause`), service `service.rs:2159`.
- `resume_subscription` — `handlers.rs:555` (`POST /resume`), service `service.rs:2280`.
- `extend_trial` — `handlers.rs:616` (`POST /extend-trial`), service `service.rs:2367`.
- `apply_discount_save_offer` — `handlers.rs:684` (`POST /cancel/apply-discount`), service `service.rs:2641`.

**Trial urgency ramp** (commit `df2763393`).
- Sidebar pill at T-7d (`Sidebar.svelte:803`), `TrialEndingBanner.svelte` at T-3d, `TrialExpiryModal.svelte` at T-1d (both in `ui/src/lib/shared/components/feedback/`).
- Mounted in `ui/src/routes/+page.svelte`.

**Quick-win emails** (`backend/src/server/email/service.rs`).
- `send_subscription_cancelled_email` (now takes `period_end_date`) — `:295`.
- `send_payment_method_added_email` — `:308`.
- `send_payment_recovered_email` — `:316`.
- `send_subscription_paused_email` — `:333`.

**Post-Stripe confirmation banner.** `ui/src/lib/shared/components/feedback/PostStripeWelcomeBanner.svelte`.

**Recovery affordances.** `UpgradeButton` rendered on Networks/Members/API-keys tabs: `NetworksTab.svelte:215`, `UserTab.svelte:190`, `UserApiKeyTab.svelte:196`.

**Schema.** Migration `backend/migrations/20260501000000_add_organization_billing_flags.sql` adds denormalized flag columns on `organizations` (`last_paused_at`, `trial_extended_used`, `last_downgrade_at`, `last_downgrade_from_plan`, `last_discount_at`, `discount_save_offer_percent_off`, `discount_save_offer_active_until`, `next_renewal_at`). Written via `OrganizationBillingSubscriber` (Pattern B mirroring subscriber).

**Telemetry.** `BillingOperation::CancellationInitiated` (`shared/events/types.rs:218`) + `CancellationFeedbackProvided` (`:232`), commit `74ac498cd`. Emitted to the event bus and Brevo.

## Built differently than specced

**Data model — event-sourcing rejected.** The spec planned an event-sourced `subscription_events` ledger plus a `SubscriptionService` deriving state on read. This was rejected during implementation. The shipped design is denormalized flag columns on `organizations`, written by a mirroring subscriber (`backend/src/server/organizations/subscriber.rs`, `OrganizationBillingSubscriber` at `:64`). The migration's leading comment documents the choice.

**`cancellations` table not built.** The spec called for a typed `cancellations` table. It was not built. Cancellation persistence is Stripe metadata + the org flag columns + bus/Brevo events. (No `cancellations` or `subscription_events` migration exists.)

**Schema/enum names diverged.** `downgraded_at` → `last_downgrade_at`; `previous_plan` → `last_downgrade_from_plan`; `has_used_trial_extend` → `trial_extended_used`. The canonical 7-value cancel-reason taxonomy was replaced with Stripe-identity names — `CancelReason` serializes (snake_case) to `too_expensive`, `missing_features`, `switched_service`, `unused`, `low_quality`, `customer_service`, `too_complex`, `other`.

**Trial value recap — card removed, email kept.** The in-app recap card was built then deliberately removed (commit `76c748e8a`); `TrialValueRecapCard.svelte` no longer exists. The recap EMAIL and its metrics live: `TrialRecapMetrics` (`email/service.rs:35`), `compute_trial_recap_metrics` (`:711`).

## NOT built (no planned-work successor)

These are the only forward-looking Phase 5 items. None have a planned-work entry.

- **Downgrade-recovery banner.** The data columns exist (`last_downgrade_at`, `last_downgrade_from_plan`), but no banner UI was built.
- **"What changed" page.** No dedicated route or modal enumerating the per-feature delta on downgrade.
- **Authoritative upcoming-invoice preview.** Only a client-side estimate shipped (`BillingPlanForm.getEstimatedTotal`, `BillingPlanForm.svelte:146`). The server `GET /change-plan/preview` (`handlers.rs:310`) returns overage counts only, not a Stripe upcoming-invoice total. No `checkout-preview` endpoint.
- **Downgrade email per-feature delta.** `send_plan_changed_email` (`email/service.rs:291`) still takes only `plan_name` — no per-feature loss enumeration.
