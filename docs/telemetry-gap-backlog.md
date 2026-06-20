# Telemetry Gap Backlog

Consolidated from `POSTHOG_STRATEGY.md` (Priority 1/2), `avenue-2a-data-memo.md` Q1–Q7 gap registry, `avenue-2a-followup-memo.md` Q-A/Q-B/Q-C, and `docs/transition-moment-audit.md` (3a/3b/3c). Sorted by priority; within priority, by effort ascending.

Effort: **S** <1d · **M** 1–3d · **L** >3d or schema change.

---

## P0 — Unblocks load-bearing questions

### P0-1 · Capture Stripe `cancellation_details` on webhook
- **Change:** Deserialize `subscription.cancellation_details.{reason, feedback, comment}` in `handle_subscription_deleted` and forward to event payload (and to P0-3 table if adopted).
- **Emission:** `backend/src/server/billing/service.rs:1425` (deserialize) → `:1590` (attach to `subscription_cancelled` payload as `cancel_reason_code`, `cancel_feedback`, `cancel_comment`).
- **Unblocks:** Avenue 3 3b HIGH ("Stripe `cancellation_details` silently discarded"); top cancel reasons by plan/tenure; Q3b plan-at-cancel.
- **Effort:** S

### P0-2 · Enrich `subscription_cancelled` payload
- **Change:** Add `was_trialing` (already computed at `service.rs:1464`), `cancel_type` (`voluntary_paid` | `mid_trial` | `trial_lapse` | `dunning` | `admin` | `upgrade`), `plan_name` (deterministic, not from Stripe metadata), `mrr_amount_cents`, `tenure_days`, `period_end`.
- **Emission:** `backend/src/server/billing/service.rs:1590` (extend the `json!` payload in `process_subscription_deleted_side_effects`).
- **Unblocks:** Avenue 2a Q3 (verify 80%+ gross MRR churn from event data), Q3b plan tier, Q5 trial-lapse vs cancel split; Avenue 3 3b primary slicing blocker; eliminates need for `trial_ended converted=false` joins.
- **Effort:** S

### P0-3 · `mrr_amount_cents` on `checkout_completed`
- **Change:** Compute confirmed total (base + seats + networks) at checkout completion and emit on event; also emit `plan_name` deterministically (do not rely on Stripe `metadata.plan_name` — follow-up Q-A flagged it unreliable).
- **Emission:** `backend/src/server/billing/service.rs:535` and `:1024` (existing `checkout_completed` emit sites).
- **Unblocks:** Avenue 3 3a HIGH "first-invoice amount hidden" + "checkout_completed lacks charge amount"; plan-tier conversion breakdowns without `postgres.organizations` join; restores trust in `metadata.plan_name`.
- **Effort:** S

### P0-4 · `payment_method_added` event (the missing paid-conversion signal)
- **Change:** New `BillingOperation::PaymentMethodAdded` variant. Emit when Stripe `customer.subscription.updated` first sets `default_payment_method` on a trialing subscription, OR on `setup_intent.succeeded` for the customer. Properties: `org_id`, `plan_name`, `trial_days_remaining`, `mrr_amount_cents`, `is_during_trial: bool`. Also wire `send_payment_method_added_email` (orphaned template `PAYMENT_METHOD_ADDED_BODY` at `templates.rs:265`).
- **Emission:** New webhook branch in `service.rs:handle_*` switch (~`:869` neighborhood); add variant in `shared/events/types.rs:448`; subscriber filter at `posthog/subscriber.rs:115`.
- **Unblocks:** Every trial→paid metric (Avenue 2a Q1/Q2/Q7, follow-up Q-C). Currently `checkout_completed` fires on free selection — no reliable paid-conversion signal exists. Also fixes Avenue 3 3a MED (orphaned email).
- **Effort:** M

### P0-5 · `first_invoice_paid` event (defensive complement to P0-4)
- **Change:** New `BillingOperation::FirstInvoicePaid`. Emit on `invoice.paid` webhook where `billing_reason in ('subscription_create','subscription_cycle')` AND it's the first paid invoice for that customer. Properties: `org_id`, `plan_name`, `amount_paid_cents`, `days_since_trial_start`.
- **Emission:** New `invoice.paid` branch in webhook router (`service.rs:~869`); reuse same enum addition as P0-4.
- **Unblocks:** Distinguishes "card added but no charge yet" (P0-4) from "first dollar paid" — needed when trial conversion ≠ first charge (e.g. payment fails on first invoice). Decide: emit both or pick one. Recommendation: both — P0-4 is the leading indicator, P0-5 is the lagging confirmation.
- **Effort:** M

---

## P1 — Unblocks secondary questions

### P1-1 · `paywall_gate_hit` (passive-bounce signal)
- **Change:** Frontend event on disabled paywalled control click, distinct from `upgrade_button_clicked` (which fires only after intent). Properties: `feature`, `surface` (export_modal/discovery_form/share_panel/sidebar/billing_tab), `gate_type` (`limit_hit` | `plan_required`).
- **Emission:** `ui/src/lib/shared/utils/trigger-upgrade.ts` (wrap before modal open) + `ExportModal.svelte:172`, `DiscoveryDetailsForm.svelte:71`, `ShareConfigPanel.svelte:81`.
- **Unblocks:** Avenue 3 3c MED "can't measure passive-bounce on gates"; identifies which gates users hit but don't act on.
- **Effort:** S

### P1-2 · Enrich `payment_failed` / `payment_recovered` payloads
- **Change:** Add `plan_name`, `invoice_id`, `amount_cents`, `attempt_count`. Currently single-field (`org_id`).
- **Emission:** `backend/src/server/billing/service.rs:1941` (failed) and `:2089` (recovered).
- **Unblocks:** Avenue 3 3b/3c LOW; sized dunning funnel by plan/amount; Avenue 2a payment recovery target tracking.
- **Effort:** S

### P1-3 · `BillingTab` trial InfoCard instrumentation
- **Change:** Emit `trial_card_impression`, `trial_card_dismissed`, `trial_card_cta_clicked` (one-per-session impression dedup). Properties: `trial_days_left`, `has_payment_method`.
- **Emission:** `ui/src/lib/features/settings/BillingTab.svelte:171` (the amber InfoCard mount + click handler).
- **Unblocks:** Avenue 3 3a LOW "single most prominent in-app trial surface is blind in analytics"; quantifies in-app trial signal effectiveness.
- **Effort:** S

### P1-4 · UTM params on email CTAs
- **Change:** Append `?utm_source=email&utm_campaign=<template_id>&utm_medium=lifecycle` to every CTA in `templates.rs` (especially the two T-3d trial templates and `SUBSCRIPTION_CANCELLED_BODY`). Capture into `upgrade_button_clicked.source` on landing.
- **Emission:** `backend/src/server/email/templates.rs` link constructions.
- **Unblocks:** Avenue 3 3a MED "no conversion-source attribution from email clicks"; T-3d email contribution to conversion.
- **Effort:** S

### P1-5 · `downgraded_at` + `cancelled_at` + `period_end` columns on `organizations`
- **Change:** Migration adds three nullable timestamptz columns; populate from `handle_subscription_deleted` and `schedule_downgrade`. Backfill from PostHog `subscription_cancelled` / `plan_changed` events where possible.
- **Emission:** `backend/migrations/<new>` + writes at `service.rs:1472` (downgrade) and `:1751` (schedule_downgrade).
- **Unblocks:** Avenue 3 3b/3c HIGH "no `cancelled_at` / `period_end` columns" + "no `downgraded_at` column"; dwell-time / reactivation cohorts via SQL instead of PostHog event joins; Avenue 2a Q5 re-upgrade analysis; allows post-cancel email to name `period_end`.
- **Effort:** M

### P1-6 · Typed `cancellations` table — **DEFERRED unless a product driver emerges**
- **Reversed decision (founder, 2026-04-28):** The Avenue 2b recommendation to build this proactively was based on a shallow "SQL-shaped joins" argument. PostHog handles the analytical questions cleanly once `subscription_cancelled` carries the rich properties from P0-2 (`cancel_type`, `plan_name`, `mrr_amount_cents`, `tenure_days`) and Stripe revenue is wired through PostHog's Stripe connector. The 5× "Cannot answer simply" items from Avenue 3 3b are all PostHog-shaped: re-subscribe at 30/60/90d (event cohort), voluntary vs dunning split (`cancel_type` property), time-to-cancel by tenure (`tenure_days` property), cancel reasons by plan (`cancel_reason_code` × `plan_name`), save-offer acceptance (instrument when feature ships).
- **Build criterion:** product code needs to read cancel state. Examples: admin UI rendering per-org cancel history; automated save-offer email triggers keyed on cancel reason; comeback-campaign rules driven by tenure-at-cancel; reporting operations that can't tolerate PostHog ingest lag. None of these exist or are scheduled today. Revisit when one does.
- **Schema sketch (preserved for future reference):** `id uuid pk`, `organization_id uuid fk`, `cancelled_at timestamptz`, `period_end timestamptz`, `plan_name text`, `mrr_amount_cents int`, `cancel_type text` (enum: voluntary_paid/mid_trial/trial_lapse/dunning/admin/upgrade), `cancel_reason_code text` (Stripe enum), `cancel_feedback text`, `cancel_comment text`, `was_trialing bool`, `tenure_days int`, `created_at timestamptz`. Index on `(organization_id, cancelled_at)`.
- **Effort if/when adopted:** M
- **What to ship now instead:** P1-5 (the column additions to `organizations`) — those drive product UX (the post-cancel email currently can't name `period_end` because we don't store it; the `pending_cancellation` banner needs `cancelled_at`). That's the genuine product-functionality slice.

### P1-7a · Server-side UTM capture on signup landing — **NEW, gap missed by initial backlog**
- **Change:** Capture `utm_source`, `utm_medium`, `utm_campaign`, `utm_content`, `utm_term`, `gclid`, `fbclid`, and HTTP `referer` from the request URL when an unauthenticated user lands on a signup-flow page. Persist as initial-touch attributes on the eventual `org_created` event and as durable properties on the org group (paired with P1-7 below).
- **Why missed:** Avenue 2b consolidated documented gaps but the original Avenue 2a memo (Q4) flagged "UTM parameters are empty across the board" without identifying the upstream cause. Founder review (2026-04-28) confirmed UTM capture is not implemented server-side; the field is a void, not stale data.
- **Emission:** New middleware or signup-handler hook on the landing-page route(s); attach to session/onboarding state; emit on `org_created`.
- **Unblocks:** The `org_created` volume drop investigation (148→38 in matched 7-day windows around the late-April policy change). Without UTM capture, can't compare pre- and post-policy signups by source to disambiguate policy friction from organic variation. Also unblocks campaign-attribution analysis generally.
- **Effort:** S–M (depends on framework's middleware story; capturing is small, persisting through onboarding state is the bulk).

### P1-7 · `referral_source` as durable org-group property
- **Change:** Capture `referral_source` at `ReferralSourceCompleted` and persist to `postgres.organizations` (new column) AND PostHog group property; auto-attach to all subsequent events via `inject_org_group`.
- **Emission:** `posthog/subscriber.rs:329` (group_identify on OrgCreated) — extend to also fire on `ReferralSourceCompleted`; new column write in onboarding service.
- **Unblocks:** Avenue 2a Q4 ("fully unanswerable today"); D30/D90 retention by source; cohort comparison for Q-D org_created volume drop diagnosis.
- **Effort:** M

### P1-8 · Structured `discovery_failed` error codes
- **Change:** Replace free-text `error_reason` with typed enum: `connection_timeout`, `firewall_blocked`, `auth_failed`, `daemon_offline`, `subnet_unreachable`, `dns_failed`, `unknown`. Add `error_subnet`, `hosts_attempted`, `hosts_succeeded`. Backfill mapping from current `error_reason` strings.
- **Emission:** `daemon/discovery/service/` failure paths; `posthog/subscriber.rs:401`.
- **Unblocks:** Avenue 2a Q-D and "44%/34% never completed daemon setup" engineering tickets; sliceable Dashboard 7 error breakdown; `daemon_install_failed` distinction.
- **Effort:** M

### P1-9 · Email send/open/click ingestion from Brevo
- **Change:** Emit `email_sent` from Rust on enqueue (template_id, org_id, recipient_role); ingest Brevo webhook for `delivered`, `opened`, `clicked` → emit `email_opened` / `email_clicked` PostHog events.
- **Emission:** `email/service.rs` send paths + new Brevo webhook handler.
- **Unblocks:** Avenue 3 3a MED "did the T-3d email open?"; cross-channel email→checkout funnels.
- **Effort:** M

---

## P2 — Nice-to-have

### P2-1, P2-2, P2-3 — **VERIFY-AND-CLOSE** (founder note, 2026-04-28)
Founder confirms `topology_viewed`, `share_link_viewed`, `share_embed_viewed`, nudge events, and `checklist_dismissed` are emitted today. `POSTHOG_STRATEGY.md` is out of date and lists them as gaps. **Action:** instead of reshipping, verify each event fires with expected properties, then update `POSTHOG_STRATEGY.md` to reflect emission status. Close these backlog items once verified.

### P2-4 · `host_inspected`, `topology_customized`, `api_request_made`
- Already on `POSTHOG_STRATEGY.md` Priority 2. Engagement-depth signals.
- **Effort:** S–M

### P2-5 · `error_displayed`, `cookie_consent_given/denied`
- Already on `POSTHOG_STRATEGY.md` Priority 3. Keep at P2.
- **Effort:** S

### P2-6 · `402_gate_returned` middleware event
- **Change:** Emit from `billing.rs:120–131` 402 middleware with `gate_kind` (`subscription_required` / `host_limit` / `feature_unavailable`). Distinct from `feature_limit_hit` (which is per-handler).
- **Emission:** `backend/src/server/billing/billing.rs:120`.
- **Unblocks:** Avenue 3 3a LOW "402 middleware returns one generic message; analytics can't slice failure reasons."
- **Effort:** S

### P2-7 · `login` coverage audit for API/CLI sessions
- **Change:** Confirm `AuthOperation::Login` fires for API-key auth and daemon sessions, not just web. If not, add or document the gap.
- **Emission:** Audit `auth/middleware/auth.rs`.
- **Unblocks:** Avenue 2a Q6 caveat (login events may be incomplete for backend-only sessions); accuracy of WAO North Star.
- **Effort:** S

---

## Co-shipping events (introduced by Avenue 4 stickiness candidates)

These events have no independent priority — they ship with their parent feature. Listed here so they aren't forgotten during feature implementation. Reference: `docs/stickiness-candidates.md`.

### CS-1 · Change-detection digest events (candidate #1)
- **Events:** `change_digest_sent`, `change_digest_opened`, `change_digest_clicked`. Properties: `org_id`, `material_change_count`, `digest_period_days`, `change_types: string[]`.
- **Co-ships with:** "Your network changed" digest feature.
- **Soft prerequisite:** P1-9 (Brevo email open/click ingestion) — without it, only click-through landings are measurable.

### CS-2 · Snapshot pinning / audit-trail events (candidate #2)
- **Events:** `snapshot_pinned`, `snapshot_renamed`, `snapshot_diff_exported`. Properties: `snapshot_id`, `parent_snapshot_id`, `export_format`, `change_count`.
- **Co-ships with:** Change-window audit trail feature.
- **Disambiguates from:** P2-1 `topology_exported` (generic export) — diff exports are a distinct intent.

### CS-3 · Blast-radius query event — **deferred** (founder 2026-04-28)
- Candidate #3 moved to future work pending editable topology. Re-add this event spec when that workstream picks up.

### CS-4 · Share-view notification events (candidate #4)
- **Events:** `share_view_notified`, `share_view_notification_clicked`.
- **Co-ships with:** Share-engagement notification feature.
- **HARD prerequisite:** P2-2 (`share_link_viewed` / `share_embed_viewed`). If candidate #4 advances, **promote P2-2 from P2 to P0 or P1** — the measurement story collapses without it.

### CS-5 · Coverage-gap surface events (candidate #5)
- **Events:** `coverage_gap_shown`, `coverage_gap_resolved`, `coverage_gap_dismissed`. Properties: `gap_type` (uncovered_host/credentialless_subnet/unidentified_service), `gap_count`.
- **Co-ships with:** Coverage-completeness nudge feature.
- **Note:** Passive surface — clean retention attribution requires a no-surface control cohort. Plan A/B exposure logic accordingly.

---

## Cross-cutting decisions surfaced

1. **`cancellations` table deferred** (reversed from initial recommendation, founder 2026-04-28). PostHog handles the analytical questions cleanly once events are enriched (P0-1 + P0-2). Build the table only when product code needs to read cancel state (admin UI, save-offer triggers, comeback campaigns). Ship the column additions to `organizations` (P1-5) regardless — those drive product UX.
2. **`metadata.plan_name` is unreliable** (follow-up Q-A) — fix at emission (P0-2, P0-3), don't paper over with joins. Audit all billing events for any field read from Stripe `metadata` and replace with deterministic local resolution.
3. **`payment_method_added` AND `first_invoice_paid`** — emit both. P0-4 is the leading indicator (intent), P0-5 confirms revenue. Without both, "card added but first charge failed" is invisible.
4. **`cancel_type` lives on the event payload** (P0-2). If the cancellations table ever ships, mirror the value into the column at write time so the two sources can't disagree.
5. **POSTHOG_STRATEGY.md is partially out of date** — P2-1/P2-2/P2-3 events are emitted today despite being listed as gaps in that doc. Verify and update the strategy doc rather than reshipping.

---

## Sequencing recommendation

Round 1 (S-only, ~1 sprint): P0-1, P0-2, P0-3, P1-1, P1-2, P1-3, P1-4 — closes the cancel-slicing and checkout-amount blockers entirely; first concrete numbers on the 80% churn figure.

Round 2 (M, ~1 sprint): P0-4, P0-5, P1-5 — unlocks every trial→paid metric and adds the org-table columns that drive product UX. (P1-6 cancellations table dropped from this round per the deferred decision above.)

Round 3: P1-7a (server-side UTM capture — direct prerequisite for the `org_created` volume-drop investigation), P1-7 (referral_source as durable group property), P1-8 (structured discovery errors), P1-9 (email open/click funnel).

Round 4: verify-and-close P2-1 / P2-2 / P2-3 (already emitted, just need POSTHOG_STRATEGY.md updated) and pick up the remaining P2 items as bandwidth allows.
