# Phase 5 features — scope memo

Companion to `docs/phase5-spec.md` (settled feature/UX decisions). The foundation refactor that this doc builds on shipped on `feat/event-model-typed-payloads` (merged to dev, 2026-05-01): typed `BillingOperation` sum-type variants, generic `Subscriber<O>` trait + `EventFilter<O>` (filter on the strum-derived discriminant), and four flag columns added to `organizations` (`last_paused_at`, `trial_extended_used`, `last_downgrade_at`, `last_downgrade_from_plan`) populated by the OrganizationService BillingOperation subscriber. `phase5-data-model.md` describes an earlier Pattern A design (`subscription_events` ledger + `SubscriptionService` derivation) that was NOT implemented — see the SUPERSEDED note at the top of that doc. This features doc reflects the actual Pattern B foundation. Authoritative references for implementation workers: `backend/migrations/20260501000000_add_organization_billing_flags.sql`, `backend/src/server/organizations/impl/base.rs` (column types), `backend/src/server/organizations/subscriber.rs` (subscriber).

Grain: scope memo, not implementation plan. Each feature lists what it does, the backend + frontend surface, and what it depends on. Implementation specifics (file:line, function signatures, exact endpoint shapes) live in the per-worker TASK.md when the worktree spawns; this doc is for orientation, dependency tracking, and worker spawn planning.

## Feature inventory at a glance

| # | Feature | Bundle | Depends on data model? | Backend | Frontend |
|---|---|---|---|---|---|
| 1 | `PAYMENT_METHOD_ADDED_BODY` wiring | quick-win 5 | No | ✓ | small |
| 2 | `period_end` in post-cancel email | quick-win 8 | No | ✓ | — |
| 3 | `payment_recovered` email | quick-win 9 | No | ✓ | — |
| 4 | Trial urgency ramp (T-7d / T-3d / T-1d surfaces) | A | No | — | ✓ |
| 5 | First-invoice amount before Stripe redirect | A | No | ✓ | ✓ |
| 6 | Trial value recap (in-app card + T-3d email) | A | small | ✓ | ✓ |
| 7 | Post-Stripe confirmation moment | A | No | — | ✓ |
| 8 | Pause flow (save offer + Stripe pause + Scanopy state) | A + B | **Yes** | ✓ | ✓ |
| 9 | Trial extend (self-serve +7d, once per lifetime) | A | **Yes** | ✓ | ✓ |
| 10 | In-app cancel modal (3-step: reason → save offer → confirm) | B | **Yes** | ✓ | ✓ |
| 11 | Downgrade banner (top-of-page, 14-day window) | C | **Yes** | small | ✓ |
| 12 | Downgrade email rewrite (per-feature delta) | C | No | ✓ | — |
| 13 | "What changed" page | C | No | ✓ | ✓ |
| 14 | Recovery affordances (Networks / Members / API keys tabs) | C | No | — | ✓ |

Features 1–7, 12–14 can spawn workers in parallel right now (no data-model dependency, or dependency is minor). Features 8–11 wait until `docs/phase5-data-model.md` ships.

---

## Quick wins (ship now; no data-model dependency)

### 1. `PAYMENT_METHOD_ADDED_BODY` wiring (item 5)

Trial users who add a payment method mid-trial currently get zero acknowledgement — no email, no toast, no in-app surface. The template exists; the send function doesn't.

- **Backend.** Add `send_payment_method_added_email()` to `email/service.rs` mirroring `send_payment_failed_email`. Call it from `handle_payment_method_attached` in `billing/service.rs` after the existing `tracing::info!` log. Owner lookup pattern is already in scope (`handle_invoice_paid` shows it).
- **Frontend.** Wire `pushSuccess(...)` for the `payment_method_setup_completed` flow in `AppShell.svelte` (currently silent on Stripe-return for setup mode) so the in-app moment also gets a toast.
- **Spec ref.** `docs/phase5-spec.md` Part 1 §A5; Part 4 quick wins.
- **Size.** ~15 LoC backend + a few lines frontend.

### 2. `period_end` in post-cancel email (item 8)

Today's `SUBSCRIPTION_CANCELLED_BODY` is generic — doesn't tell the user when access ends. Stripe gives us `current_period_end`; we just don't thread it through.

- **Backend only.** Capture `sub.current_period_end` in `handle_subscription_deleted`, thread through `process_subscription_deleted_side_effects` (it already has `#[allow(clippy::too_many_arguments)]`), into `send_subscription_cancelled_email`, into `build_subscription_cancelled_email`. Add `{period_end_date}` placeholder to `SUBSCRIPTION_CANCELLED_BODY` template.
- **Spec ref.** Part 1 §B6; Part 4 quick wins.
- **Size.** Touches 4 files; no new schema.

### 3. `payment_recovered` email (item 9)

The `payment_recovered` PostHog event fires today when a past-due payment recovers. No email is sent — the customer who fixed their card never knows it worked until the next charge cycle.

- **Backend only.** Add `PAYMENT_RECOVERED_TITLE` + `PAYMENT_RECOVERED_BODY` in `templates.rs`, `build_payment_recovered_email` in `traits.rs`, `send_payment_recovered_email` in `email/service.rs` (all mirror the `payment_failed` shape). Hook from `handle_invoice_paid` right where the `payment_recovered` event is emitted today; needs a small owner-lookup addition.
- **Spec ref.** Part 1 §B5; Part 4 quick wins.
- **Size.** ~30 LoC including new template.

---

## Bundle A — trial-side

### 4. Trial urgency ramp (T-7d sidebar pill / T-3d banner / T-1d modal)

Today the only in-app trial signal is a static amber InfoCard in Settings → Billing. Add escalating surfaces outside Settings as the trial nears expiry, so users who never visit Settings still see urgency.

- **Backend.** None. `org.trial_end_date` is already available client-side via the org payload; days-left computation is pure frontend.
- **Frontend.**
  - **T-7d:** flip `Sidebar.svelte`'s `showUpgradeButton` derivation to also include trialing state when `daysLeft <= 7`; change icon/copy accordingly.
  - **T-3d:** add `TrialEndingBanner` component slotted into the `+page.svelte` global banner chain (between `EmailVerificationBanner` and `LicenseGraceBanner` precedents). Uses existing `AppBanner` primitive, warning variant, with "Add Payment Method" CTA.
  - **T-1d:** add `TrialExpiryModal` component mounted in `+page.svelte`. One-time-per-day dismissal via localStorage (small show-once primitive borrowed from `InlineInfo`'s `dismissableKey`).
- **Spec ref.** Part 2 Bundle A; Part 3 first UX take.
- **Dependencies.** None blocking. Quick win 1 (post-Stripe toast) pairs nicely.

### 5. First-invoice amount before Stripe redirect

Today users see range-based plan cards in `BillingPlanForm`, then jump to Stripe Checkout to find the actual total (base + seats + networks). All the data is already on the backend — we just don't serve a confirmed total.

- **Backend.** New endpoint `POST /api/billing/checkout-preview` (auth: `Authorized<Owner>`), body `{ plan: BillingPlan }`, response `{ base_cents, included_seats, current_seats, extra_seat_cents, included_networks, current_networks, extra_network_cents, billing_period_total_cents, currency, trial_end_date }`. Implementation mirrors `preview_plan_change`'s service-side counting; multiplies by per-extra cents from the target plan config.
- **Frontend.** `BillingPlanForm.svelte` calls the preview endpoint when user picks a plan; displays the confirmed total + line items inline before the "Continue to Stripe" button. Optionally surface in a small confirmation sheet right before the Stripe redirect.
- **Spec ref.** Part 1 §A2.
- **Dependencies.** None.

### 6. Trial value recap (in-app card + T-3d email)

"Scanopy discovered X hosts, Y services across Z networks during your trial." Two surfaces showing the same five metrics: hosts discovered, networks mapped, daemons connected, services identified, days into trial.

- **Backend.** Service queries for the five metrics (most are derivable from existing endpoints — see Part 1 §A3). The T-3d email path needs the metrics pre-computed at email-send time as part of the existing trial-ending email job. Empty-state handling: surface aha-moment getting-started CTA instead of "0 hosts" when metrics are all zero.
- **Frontend.** New card in `BillingTab.svelte` slotted near the trial countdown InfoCard. Rendered only during `plan_status === 'trialing'`. Reads from existing `useHostsQuery({limit:1})`, `useNetworksQuery()`, `useDaemonsQuery()`; one new query for service count.
- **Spec ref.** Part 2 Q3.
- **Dependencies.** None on the Phase 5 foundation refactor — the email-send job's pre-compute reads from existing tables (`hosts`, `networks`, `services`, `daemons`) the same way the in-app card does. No dependency on the BillingOperation typed payloads or the OrganizationService BillingOperation subscriber.

### 7. Post-Stripe confirmation moment

Today `plan_status` silently flips `trialing → active` after Stripe redirect; `billing_completed` event fires but no UI surface confirms the conversion to the user beyond a transient toast.

- **Frontend only.** Two surfaces: (a) keep the existing toast for immediate feedback; (b) add a one-time `AppBanner` at top-of-page ("Welcome to {plan}") with `dismissableKey` until dismissed. Mounted in `+page.svelte` global banner chain. Gates on a "recently activated" condition (e.g., `plan_status` flipped to `active` within last 24h, no prior dismissal).
- **Spec ref.** Part 1 §A6.
- **Dependencies.** None. Pairs with quick win 1's `pushSuccess` for the payment_method_setup case.

### 8. Pause flow

Save-offer-during-cancel for v1 (no standalone Settings entry). User picks 30 / 60 / 90 day duration; resume date displayed; resume-early always available; once per rolling 6-month window per org.

- **Backend.** New endpoints `POST /api/billing/pause` (body: `{ duration_days: 30 | 60 | 90 }`), `POST /api/billing/resume`. Service calls `UpdateSubscription::pause_collection(...)` on Stripe AND emits `BillingOperation::Paused { duration_days, resumes_at, plan }` event. Resume sets pause_collection back, emits `Resumed { was_early: bool }`. Eligibility check is a direct read of `organizations.last_paused_at` (column populated by the OrganizationService BillingOperation subscriber on `Paused` events; 6-month rolling). On resume of an auto-paused sub via Stripe webhook, reflect via `Resumed { was_early: false }`.
- **Frontend.** Pause panel inside the cancel modal (feature 10) with duration picker, "Pause until {date}" preview, eligibility-aware messaging ("You can pause again on {next-eligible-date}" when ineligible) — `next-eligible-date` computed client-side from `org.last_paused_at + 6 months`. Resume-early button on `BillingTab.svelte` while `plan_status === 'paused'`.
- **Spec ref.** Part 2 Q2; Part 4 (eligibility decisions).
- **Dependencies.** **Data model foundation** (typed `BillingOperation::Paused`/`Resumed` variants, `organizations.last_paused_at` flag column, OrganizationService BillingOperation subscriber). Stripe SDK `pause_collection`.

### 9. Trial extend (self-serve, +7d, once per lifetime)

Self-serve "Extend your trial by 7 days (one-time extension)" link on the BillingTab card during T-3d / T-1d window. Eligibility-gated; no link shown if already used.

- **Backend.** New endpoint `POST /api/billing/extend-trial`. Service calls `UpdateSubscription::trial_end(...)` on Stripe pushing the timestamp forward by 7 days, emits `BillingOperation::TrialExtended { days_added: 7, new_trial_end }`. Eligibility is a direct read of the `organizations.trial_extended_used` boolean (column flipped to true by the OrganizationService BillingOperation subscriber on `TrialExtended` events; never reset).
- **Frontend.** Link/button on `BillingTab.svelte` trial-countdown InfoCard, gated on `!org.trial_extended_used && trialDaysLeft <= 3`. Confirm modal explains "one-time extension" framing.
- **Spec ref.** Part 2 Q2; Part 4.
- **Dependencies.** **Data model foundation** (typed `BillingOperation::TrialExtended` variant, `organizations.trial_extended_used` flag column, OrganizationService BillingOperation subscriber). Stripe SDK `trial_end`.

---

## Bundle B — cancel-side

### 10. In-app cancel modal (3-step)

Replaces the current "Manage Subscription" → Stripe Portal handoff with a Scanopy-side modal. Step 1: reason picker (7-value Scanopy-canonical enum) + free-text comment. Step 2: reason-dependent save offer (pause for `pausing` / `not_using_enough` / `too_expensive`; discount for `too_expensive`; skip for others). Step 3: confirmation with `period_end` disclosure + retention summary.

- **Backend.** New endpoint `POST /api/billing/cancel` (body: `{ reason_code, comment?, save_offer_redeemed? }`). Calls `UpdateSubscription::cancel_at_period_end(true).cancellation_details(...)` on Stripe (maps Scanopy reason to Stripe enum where possible; stashes canonical reason in `Subscription.metadata["scanopy_cancel_reason"]`). Emits `BillingOperation::CancellationInitiated { reason_code, stripe_feedback, comment, save_offer_shown, save_offer_redeemed, planned_period_end }`. The webhook-triggered `SubscriptionCancelled` event still fires at period end (existing path). Discount save offer calls `UpdateSubscription::discounts(vec![...])` with a Stripe coupon ID. Pause save offer routes to feature 8's pause endpoint.
- **Frontend.** New `CancelSubscriptionModal.svelte` (3-step) replacing the `handleManageSubscription` button on `BillingTab.svelte`. Uses `@tanstack/svelte-form` per project convention. Reason picker as `RichSelect`; save offer panel renders conditionally per reason; confirmation step renders period_end + per-feature retention disclosure.
- **Spec ref.** Part 2 Q1.
- **Dependencies.** **Data model** (`CancellationInitiated` event). Stripe SDK `cancellation_details`, `discounts`. Feature 8 (pause) for the pause save offer.

---

## Bundle C — downgrade-side

### 11. Downgrade banner (top-of-page, 14-day window)

Persistent in-app banner via `AppBanner` while an org is on Free within 14 days of its most recent downgrade. Action: "Restore full access" → opens `BillingPlanModal`.

- **Backend.** None new — `organizations.last_downgrade_at` (`Option<DateTime<Utc>>`) and `last_downgrade_from_plan` (`Option<BillingPlan>`, full plan JSONB) are already populated by the OrganizationService BillingOperation subscriber on `PlanChanged { is_downgrade: true }` and `SubscriptionCancelled` events, and already serialized in the existing org payload. Banner gating reads them client-side.
- **Frontend.** New `DowngradeRecoveryBanner.svelte` slotted into `+page.svelte` global banner chain. Variant: warning. Gate: `org.plan?.type === 'Free' && org.last_downgrade_at != null && now() - org.last_downgrade_at < 14 days`. Action snippet: `triggerUpgrade({ source: 'downgrade_banner' })`. Banner copy can use `org.last_downgrade_from_plan.name` for richer phrasing ("Restore Pro access") since the full prior plan is in the payload. Dismissible via `dismissableKey` (extending the pattern used by `InlineInfo`); after 14 days, banner stops rendering even if not dismissed.
- **Spec ref.** Part 2 Q4.
- **Dependencies.** **Data model foundation** (the OrganizationService BillingOperation subscriber populating `last_downgrade_at` + `last_downgrade_from_plan` on org). `AppBanner.dismissableKey` extension (cross-cutting).

### 12. Downgrade email rewrite (per-feature delta)

Replace today's one-sentence `PLAN_CHANGED_BODY` with a per-feature enumeration of what stops and starts working, named `period_end` for paid cancels, link to the "what changed" page (feature 13).

- **Backend only.** Rewrite `PLAN_CHANGED_BODY` template to interpolate previous plan name + downgrade-feature-list + period_end + "what changed" link. Build function in `email/traits.rs` updated to thread the new fields. Invoked from existing `handle_subscription_deleted` path (and from the `PlanChanged` paid→paid downgrade webhook handler).
- **Spec ref.** Part 2 Q4.
- **Dependencies.** None blocking. Pairs with quick win 2 (period_end threading) — recommend ship 2 first so this rewrite uses the existing threaded date.

### 13. "What changed" page

Dedicated route or modal generated from a `BillingPlanFeatures` struct comparison. Linked from the downgrade banner CTA + downgrade email body. Renders a delta table: "Was on {previous_plan}, now on Free. These features are no longer available: ... These continue: ..."

- **Backend.** Tiny — new endpoint `GET /api/billing/feature-delta?from={plan_name}&to={plan_name}` returns the structured delta computed from the in-code `BillingPlanFeatures` struct comparison. Auth: `Authorized<Viewer>`.
- **Frontend.** New route `/settings/billing/changes` (or a modal mounted from the banner action). Renders the delta table with feature names, descriptions, and an "Upgrade to restore" CTA per gated feature. Reuses existing feature-name/description copy from the plan picker.
- **Spec ref.** Part 2 Q4.
- **Dependencies.** None blocking.

### 14. Recovery affordances (unwired surfaces)

Most contextual upgrade affordances on disabled controls already exist (per Part 1 §C3). Three surfaces are still unwired and need adding to match the existing patterns:

- **Networks tab** — inline counter on the Networks list header showing `{used}/{limit}` in amber when at limit; replace Create button with `<UpgradeButton feature="networks" />` at limit. Mirrors the `HostTab.svelte` pattern.
- **Settings → Members** — at-limit counter + `<UpgradeButton feature="invite_users" />` in place of invite button.
- **API keys tab** — disabled-state on the create-key form with "Upgrade to enable API access" copy + `<UpgradeButton feature="api_access" />`.
- **Frontend only.** All three follow the `triggerUpgrade(...)` integration pattern that the existing wired surfaces use.
- **Spec ref.** Part 2 Q4; Part 1 §C3 for unwired inventory.
- **Dependencies.** None.

---

## Cross-cutting infrastructure

These are utilities/patterns multiple features need; whichever worktree builds them first gets the credit, downstream features reuse:

- **`AppBanner.dismissableKey` extension.** Today `InlineInfo` has the dismissal pattern; `AppBanner` does not. Feature 7 (post-Stripe confirmation banner) and feature 11 (downgrade banner) both need persistent dismissibility. First worktree adds it; both features consume.
- **Pattern B billing flag columns + OrganizationService BillingOperation subscriber.** Already shipped on `feat/event-model-typed-payloads` (merged to dev). Migration `20260501000000_add_organization_billing_flags.sql` added four columns to `organizations`: `last_paused_at: timestamptz NULL`, `trial_extended_used: bool NOT NULL DEFAULT false`, `last_downgrade_at: timestamptz NULL`, `last_downgrade_from_plan: jsonb NULL` (full `BillingPlan`). The OrganizationService implements `Subscriber<BillingOperation>` (`backend/src/server/organizations/subscriber.rs`) filtering on `Paused`, `TrialExtended`, `PlanChanged`, and `SubscriptionCancelled` discriminants and writing the flags from typed event variants. Features 8 (pause), 9 (trial extend), 11 (downgrade banner) read these columns directly off the org payload — no `SubscriptionService` derivation methods, no `subscription_events` ledger.
- **One-time-per-day modal show primitive.** Feature 4 (T-1d modal) and possibly feature 7 (welcome banner) want a `dismissedToday` localStorage helper. Small utility; first worktree to need it adds it to `$lib/shared/utils/`.
- **PostHog filter list update.** Per `docs/phase5-data-model.md`, new BillingOperation variants (`Paused`, `Resumed`, `TrialExtended`, `CancellationInitiated`, `PaymentMethodAdded`/`Removed`) need to be added to the PostHog subscriber's filter list if PostHog should capture them. Default-yes. Coordinator decides whether this rides with the data-model worker or a follow-up.
- **Stripe coupon setup.** Feature 10's discount save offer requires a coupon created in the Stripe dashboard, referenced by ID at runtime. Out of code scope but needs to happen before feature 10 ships; flag on the worker spawn.

---

## Suggested worker spawn ordering

Not a hard schedule; just dependencies:

1. **Now (parallel-able, no data-model dep):** quick wins 1, 2, 3; features 4, 5, 7, 12, 13, 14.
2. **After data model lands:** features 6, 8, 9, 10, 11.
3. **Within group 2:** feature 8 (pause backend) and feature 9 (trial extend backend) can spawn together; feature 10 (in-app cancel) depends on feature 8 for its pause save offer integration; feature 11 (downgrade banner) is independent of features 8/9/10 but shares the data model dependency.

Worker spawn cardinality: each feature could be its own worker, or related features could bundle (e.g. all three quick wins to one worker; features 4 + 7 as one trial-UI worker; features 11 + 12 + 13 as one downgrade-comms worker). Coordinator decides per cycle based on review bandwidth.

## Out of scope (explicit)

- Refactoring `EntityEvent` / `AuthEvent` / `OnboardingEvent` to typed payloads — same architectural pattern as the data model refactor; deliberate follow-up scope, not Phase 5.
- Standalone-Settings pause entry point — cancel-flow-only for v1 per Part 3 UX decision.
- Save offer types beyond pause + discount (support handoff, plan-downgrade-as-alternative) — deferred per Part 2 Q1.
- Admin UI for reviewing cancellations — out of Phase 5; deferred until product use case materializes.
- A/B testing infrastructure — not in this brief's scope.
- Items already in flight via Phase 1 telemetry (cancellation_details capture, etc.) — already merged; this phase builds on them.
