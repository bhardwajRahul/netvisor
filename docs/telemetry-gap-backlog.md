# Telemetry Gap Backlog

> **Status as of 2026-06.** See `DOCS_AUDIT_2026-06.md` for the full audit. Roughly 40% of this backlog has been closed via `feat/billing-telemetry-enrichments` and `feat/phase5-subscription-mechanics`; the remaining ~60% is still open. Each item below is annotated inline with its current status — `**[DONE 2026-06]**`, `**[PARTIAL 2026-06]**`, or left as-is when genuinely open. Telemetry flows event-bus → PostHog subscriber + Brevo subscriber (no direct PostHog calls from billing code); "emitted" means published to the bus, "consumed" means a subscriber's `filter()` admits it.

Consolidated from `POSTHOG_STRATEGY.md` (Priority 1/2), `avenue-2a-data-memo.md` Q1–Q7 gap registry, `avenue-2a-followup-memo.md` Q-A/Q-B/Q-C, and `docs/transition-moment-audit.md` (3a/3b/3c). Sorted by priority; within priority, by effort ascending.

Effort: **S** <1d · **M** 1–3d · **L** >3d or schema change.

---

## P0 — Unblocks load-bearing questions

### P0-1 · Capture Stripe `cancellation_details` on webhook — **[DONE 2026-06]**
- Shipped: `extract_cancellation_details(sub.cancellation_details.as_ref())` (`backend/src/server/billing/service.rs:2785`, also called at `:1010` and `:1561`) pulls `{reason, feedback, comment}` off the Stripe subscription. Commit `74ac498cd`.
- **Original change:** Deserialize `subscription.cancellation_details.{reason, feedback, comment}` and forward to the cancel event payload.
- **Unblocked:** Avenue 3 3b HIGH ("Stripe `cancellation_details` silently discarded"); top cancel reasons by plan/tenure; Q3b plan-at-cancel.
- **Effort:** S

### P0-2 · Enrich `subscription_cancelled` payload — **[DONE 2026-06, with one deviation]**
- Shipped: the `SubscriptionCancelled` operation now carries `was_trialing`, `period_end`, `mrr_amount_cents`, and `tenure_days`. Computed at `backend/src/server/billing/service.rs:1554` (`was_trialing`), `:1563` (`mrr_amount_cents` via `mrr_from_subscription`), `:1564` (`tenure_days`), and attached to the published `BillingOperation::SubscriptionCancelled { period_end, was_trialing, mrr_amount_cents, tenure_days, .. }` at `:1704`.
- **Deviation — `cancel_type` was NOT built.** The proposed `cancel_type` taxonomy (`voluntary_paid`/`mid_trial`/`trial_lapse`/`dunning`/`admin`/`upgrade`) does not exist as a payload field. Instead, the cancel *reason* rides on a separate `BillingOperation::CancellationInitiated` operation (the cancel-scheduled signal, emitted at `service.rs:1025` with `reason_code`), distinct from the terminal `SubscriptionCancelled`. Analysis that assumed a single `cancel_type` enum on the cancel event must join `CancellationInitiated.reason_code` instead. `plan_name` is resolved deterministically (not from Stripe metadata) — see P0-3 / cross-cutting decision 2.
- **Unblocked:** Avenue 2a Q3, Q3b plan tier, Q5 trial-lapse vs cancel split (modulo the `cancel_type` deviation above).
- **Effort:** S

### P0-3 · `mrr_amount_cents` on `checkout_completed` — **[DONE 2026-06]**
- Shipped: `BillingOperation::CheckoutCompleted` carries `mrr_amount_cents` and a deterministically-resolved plan. Emit sites at `backend/src/server/billing/service.rs:512` and `:1122`.
- **Original change:** Compute confirmed total (base + seats + networks) at checkout completion; emit `plan_name` deterministically (not from Stripe `metadata.plan_name`, flagged unreliable in Q-A).
- **Unblocked:** Avenue 3 3a HIGH "first-invoice amount hidden" + "checkout_completed lacks charge amount"; plan-tier conversion breakdowns without `postgres.organizations` join.
- **Effort:** S

### P0-4 · `payment_method_added` event — **[PARTIAL 2026-06]**
- **Done:** the `BillingOperation::PaymentMethodAdded` variant exists (`backend/src/server/shared/events/types.rs:256`) and is emitted from the webhook path when the default invoice payment method is attached (`service.rs:1451`). The email side is wired — `send_payment_method_added_email` dispatches the `PaymentMethodAdded` template (`email/service.rs:308`, message at `email/messages/payment_method_added.rs`), consumed by the email subscriber at `email/subscriber.rs:142`. So the orphaned-email gap (Avenue 3 3a MED) is closed.
- **Still missing — no analytics value yet, for two reasons:**
  1. **It's a property-less unit struct.** `PaymentMethodAdded` carries no fields (no `org_id`/`plan_name`/`trial_days_remaining`/`mrr_amount_cents`/`is_during_trial`). Even if consumed, it would be an undifferentiated count.
  2. **No analytics subscriber consumes it.** It is NOT in the PostHog subscriber `filter()` (`posthog/subscriber.rs:230–245` lists CheckoutCompleted, SubscriptionCancelled, CancellationInitiated, PaymentFailed, PaymentRecovered, etc. — not PaymentMethodAdded), and the Brevo subscriber explicitly no-ops it (`brevo/service.rs:130`).
- **Remaining work:** add properties to the variant AND add a consumer (PostHog filter entry + handler) before any trial→paid metric (Avenue 2a Q1/Q2/Q7, follow-up Q-C) can use it.
- **Effort:** M

### P0-5 · `first_invoice_paid` event (defensive complement to P0-4)
- **Change:** New `BillingOperation::FirstInvoicePaid`. Emit on `invoice.paid` webhook where `billing_reason in ('subscription_create','subscription_cycle')` AND it's the first paid invoice for that customer. Properties: `org_id`, `plan_name`, `amount_paid_cents`, `days_since_trial_start`.
- **Emission:** New `invoice.paid` branch in webhook router (`service.rs:~869`); reuse same enum addition as P0-4.
- **Unblocks:** Distinguishes "card added but no charge yet" (P0-4) from "first dollar paid" — needed when trial conversion ≠ first charge (e.g. payment fails on first invoice). Decide: emit both or pick one. Recommendation: both — P0-4 is the leading indicator, P0-5 is the lagging confirmation.
- **Effort:** M

---

## P1 — Unblocks secondary questions

### P1-1 · `paywall_gate_hit` (passive-bounce signal) — **[PARTIAL 2026-06]**
- **Done:** a `paywall_gate_hit` event exists and fires from `triggerUpgrade()` (`ui/src/lib/features/billing/trigger-upgrade.ts:50`) with `feature`, `surface`, and `gate_type`.
- **Still missing the stated purpose.** `paywall_gate_hit` is emitted in the same `triggerUpgrade()` call, immediately before `upgrade_button_clicked` (`trigger-upgrade.ts:56`) — i.e. in lockstep with the upgrade *click/intent*. It therefore does NOT measure passive bounce (users who *see* a disabled gate but never act). The two events are effectively redundant. **Remaining ask:** a disabled-vs-enabled distinguisher — fire a gate-impression/exposure event when a locked control is *rendered* or hovered, separate from the click path, so passive-bounce is actually observable.
- **Unblocks (only once the above lands):** Avenue 3 3c MED "can't measure passive-bounce on gates".
- **Effort:** S

### P1-2 · Enrich `payment_failed` / `payment_recovered` payloads — **[DONE 2026-06]**
- Shipped: both variants now carry `invoice_id`, `amount_cents`, `plan`, and `attempt_count` (`backend/src/server/shared/events/types.rs:177` PaymentFailed, `:191` PaymentRecovered). Both are consumed by the PostHog subscriber (`posthog/subscriber.rs:240,242`).
- **Unblocked:** Avenue 3 3b/3c LOW; sized dunning funnel by plan/amount; Avenue 2a payment recovery target tracking.
- **Effort:** S

### P1-3 · `BillingTab` trial InfoCard instrumentation — **[PARTIAL 2026-06]**
- **Done:** `trial_card_impression` fires (one-per-session via `trackOncePerSession`) at `ui/src/lib/features/settings/BillingTab.svelte:188`.
- **Still missing:** there is no `trial_card_dismissed` event because the trial InfoCard is not dismissible (the generic `InfoCard` supports `dismissible`, but the trial card isn't rendered dismissible). **Remaining work:** if dwell/dismiss signal is wanted, make the card dismissible and emit `trial_card_dismissed`; the CTA-click case is already covered by `paywall_gate_hit`/`upgrade_button_clicked` from the card's button.
- **Effort:** S

### P1-4 · UTM params on email CTAs — **[DONE 2026-06]**
- Shipped: every email CTA gets `utm_source=email&utm_campaign=<campaign>&utm_medium=<medium>` via the `Email` trait's `utm_qs()` / `with_utm()` helpers (`backend/src/server/email/messages/mod.rs:198–211`); `campaign()`/`utm_medium()` are per-message. (The old `templates.rs` was reorganized into `email/messages/`.)
- **Unblocked:** Avenue 3 3a MED "no conversion-source attribution from email clicks"; T-3d email contribution to conversion.
- **Effort:** S

### P1-5 · timestamp columns on `organizations` — **[PARTIAL 2026-06]**
- **Done (but different columns than named):** migration `20260501000000_add_organization_billing_flags.sql` added `last_downgrade_at` (`:14`) and `next_renewal_at` (`:33`), populated by the organizations subscriber (`organizations/subscriber.rs:160` for `last_downgrade_at`; `next_renewal_at` written across the trial/renewal arms). So the downgrade-timestamp and renewal/period-tracking needs are covered.
- **Still missing the explicitly-named `cancelled_at` and `period_end` columns.** There is no `cancelled_at` column and no dedicated `period_end` column on `organizations` (`next_renewal_at` is the closest, but is renewal-oriented, not cancel-scheduled period end). **Remaining work:** add `cancelled_at` (and decide whether `next_renewal_at` doubles as period-end or a distinct `period_end` column is needed) if SQL-side cancel cohorts / a `pending_cancellation` banner naming the period end are required.
- **Unblocks (partially):** Avenue 3 3b/3c HIGH dwell-time/reactivation cohorts; Avenue 2a Q5 re-upgrade analysis.
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

### P1-7 · `referral_source` as durable org-group property — **[PARTIAL 2026-06]**
- **Done:** `referral_source` is captured and set as a Brevo *company* attribute (`scanopy_referral_source` — `brevo/types.rs:16,47`, written from the `ReferralSourceCompleted` handler at `brevo/service.rs:168`). `ReferralSourceCompleted` is also in the PostHog onboarding `filter()` (`posthog/subscriber.rs:337`), so it fires as an event.
- **Still missing:** no *durable PostHog group property* for referral source (the `group_identify` calls at `posthog/subscriber.rs:292,388` don't set it), so it doesn't auto-attach to subsequent events; and there is NO `referral_source` column on `organizations` (no migration adds it). **Remaining work:** add the PostHog group property and the postgres column if SQL-side / cohort-attribution access is needed.
- **Unblocks (only partially today):** Avenue 2a Q4; D30/D90 retention by source; cohort comparison for Q-D org_created volume-drop diagnosis.
- **Effort:** M

### P1-8 · Structured `discovery_failed` error codes
- **Change:** Replace free-text `error_reason` with typed enum: `connection_timeout`, `firewall_blocked`, `auth_failed`, `daemon_offline`, `subnet_unreachable`, `dns_failed`, `unknown`. Add `error_subnet`, `hosts_attempted`, `hosts_succeeded`. Backfill mapping from current `error_reason` strings.
- **Emission:** discovery failure paths; the free-text `error_reason` is forwarded as-is at `backend/src/server/posthog/subscriber.rs:513–514` (and lives on `shared/events/traits.rs:192`).
- **Unblocks:** Avenue 2a Q-D and "44%/34% never completed daemon setup" engineering tickets; sliceable Dashboard 7 error breakdown; `daemon_install_failed` distinction.
- **Effort:** M

### P1-9 · Email send/open/click ingestion from Brevo
- **Change:** Emit `email_sent` from Rust on enqueue (template_id, org_id, recipient_role); ingest Brevo webhook for `delivered`, `opened`, `clicked` → emit `email_opened` / `email_clicked` PostHog events.
- **Emission:** `email/service.rs` send paths + new Brevo webhook handler.
- **Unblocks:** Avenue 3 3a MED "did the T-3d email open?"; cross-channel email→checkout funnels.
- **Effort:** M

---

## P2 — Nice-to-have

### P2-1, P2-2, P2-3 — **[DONE 2026-06] (verified emitted)**
Confirmed emitted in current code: `topology_viewed` (`ui/src/lib/features/topology/components/TopologyTab.svelte:384`, once-per-topology), `checklist_dismissed` (`ui/src/lib/features/home/components/GettingStartedChecklist.svelte:179`), and the nudge events. (Founder note, 2026-04-28: `POSTHOG_STRATEGY.md` was out of date listing these as gaps.) `POSTHOG_STRATEGY.md` should be updated to reflect emission status; no reship needed.

### P2-4 · `host_inspected`, `topology_customized`, `api_request_made`
- Already on `POSTHOG_STRATEGY.md` Priority 2. Engagement-depth signals.
- **Effort:** S–M

### P2-5 · `error_displayed`, `cookie_consent_given/denied`
- Already on `POSTHOG_STRATEGY.md` Priority 3. Keep at P2.
- **Effort:** S

### P2-6 · `402_gate_returned` middleware event — **OPEN**
- **Change:** Emit from the 402 billing middleware with `gate_kind` (`subscription_required` / `host_limit` / `feature_unavailable`). Distinct from `feature_limit_hit` (which is per-handler).
- **Emission:** `backend/src/server/auth/middleware/billing.rs:126` (the `StatusCode::PAYMENT_REQUIRED` response site; middleware relocated here from the former `billing/billing.rs`). No telemetry event is emitted here today.
- **Unblocks:** Avenue 3 3a LOW "402 middleware returns one generic message; analytics can't slice failure reasons."
- **Effort:** S

### P2-7 · `login` coverage audit for API/CLI sessions — **OPEN (gap confirmed)**
- **Audit finding (2026-06):** `AuthOperation::LoginSuccess` is emitted only on web/interactive login paths — password login (`backend/src/server/auth/service.rs:370`) and OIDC (`auth/oidc.rs:152,285`). API-key-authenticated requests and daemon sessions emit NO login event. So the gap the item anticipated is real: WAO/login counts are web-only.
- **Change:** Decide whether to emit a session/auth event for API-key and daemon auth, or formally document the exclusion. (Note: API-key/daemon traffic is high-volume; per telemetry guidance, weigh signal value before emitting per-request.)
- **Unblocks:** Avenue 2a Q6 caveat; accuracy of WAO North Star.
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
