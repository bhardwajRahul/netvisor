# Phase 5 spec — conversion-side remediation

Settled spec for Phase 5 implementation. Companion to `docs/conversion-side-remediation-brief.md` (the original problem framing and 8 product items) and `docs/transition-moment-audit.md` (the file:line audit driving every item). Built on branch `chore/phase5-design-prep`; investigations verified at HEAD.

Status: **decisions settled** in the design pass (2026-04-28). This document is the input to Phase 5 implementation worktrees. Bundle ship-order is intentionally out of scope here — that's a coordinator-spawn decision, not a spec decision.

Structure:
- **Part 1** — code-grounded investigations with file:line citations. Implementation reference; no decisions to revisit here.
- **Part 2** — settled feature decisions (cancel flow, pause/extend, value recap, downgrade communication).
- **Part 3** — settled UX decisions (urgency ramp, modal step counts, copy/tone).
- **Part 4** — schema changes summary, quick wins, scope shifts vs the prep doc, drift cleanup.

A few things shifted since the audit:

- **Phase 1 P0-1 (cancellation_details capture) has merged.** `extract_cancellation_details(sub.cancellation_details.as_ref())` is called at `backend/src/server/billing/service.rs:1471`, and `cancel_reason_code` / `cancel_feedback` / `cancel_comment` now ride on the `subscription_cancelled` event payload (`service.rs:1602–1604`). The audit's "silently discarded" finding (3b severity HIGH) is resolved on the ingest side.
- **Daemon standby now has a code path that sets `daemon.base.standby = true`** — but it's *inactivity*-driven (`daemons/service.rs:1981`, "no completed discovery in 30 days"), not plan-driven. The cross-cutting summary item 3 ("DaemonPoll gating advertised but not enforced in code") still stands for the *plan* dimension; the error-message copy at `error_codes.rs:300–301` is now misleading for the inactivity case it actually fires from.
- **Stripe SDK is `async-stripe 1.0.0-alpha.2`** (`backend/Cargo.toml`) with the `subscription`, `billing_portal_session`, and `checkout_session` features enabled — not `stripe-rust`. All Stripe API references in this doc resolve through that crate.

---

## Part 1 — Code-grounded investigations

### Bundle A — trial-side

#### A1. Trial UI surfaces today and where new ones would hook in

Today's surfaces, all confirmed at HEAD:

- **Settings → Billing trial-countdown InfoCard** — `ui/src/lib/features/settings/BillingTab.svelte:171–197`. Renders only when `org.plan_status === 'trialing' && trialDaysLeft !== null && !hasPaymentMethod`. Copy is hard-coded English (not i18n'd): "Trial ends in {N} days ({date})" + "Add a payment method to continue after the trial". Always-on across the trial timeline; no T-3d / T-1d branching.
- **Sidebar bottom upgrade button** — `ui/src/lib/shared/components/layout/Sidebar.svelte:748–762`. Gated by `showUpgradeButton = isFreePlan && isOwner && isBillingEnabled` (`Sidebar.svelte:101`). **Free-only — does not render during trial.**
- **Sidebar settings notification dot** — `Sidebar.svelte:108–114, 777–780`. `showBillingNotification` is true when `isPastDue || (isTrialing && !hasPayment)`. The only "trial signal" anywhere outside Settings → Billing is this 2.5px amber dot on the gear icon.
- **BillingTab status banners** — `BillingTab.svelte:345–353`. `InlineInfo settings_billing_trialActive` (trialing), `InlineDanger settings_billing_pastDue` (past_due), `InlineWarning` for `canceled` / `pending_cancellation`.

**Candidate insertion points for the urgency-ramp surfaces:**

- **Top-of-page banner** — the `+page.svelte` authenticated layout already mounts a chain of global banners above `<main>`: `EmailVerificationBanner` (`ui/src/routes/+page.svelte:225–227`), `DemoBanner` (`+page.svelte:228–230`), `LicenseLockedBanner` / `LicenseGraceBanner` / `LicenseExpiringBanner` (`+page.svelte:231–240`). All built on the shared `AppBanner.svelte` primitive (`ui/src/lib/shared/components/feedback/AppBanner.svelte`), variants `info` / `warning` / `danger`. **A `TrialEndingBanner` slotted into this chain at L240 (with a `trialDaysLeft <= N` gate) is the cleanest hook for the T-3d / T-1d in-app surface.** This is also where the downgrade banner (Bundle C item 10) would live.
- **Sidebar pill** — flip the `Sidebar.svelte:101` `showUpgradeButton` derivation from `isFreePlan && ...` to also include trial state, then change the icon/copy when `trialDaysLeft <= N`. Lowest-friction surface change; reuses an existing slot.
- **Home dashboard card** — `ui/src/lib/features/home/components/HomeTab.svelte:179–187` already conditionally renders `PlanUsage` and `NetworkMetrics`. A "trial countdown" card or "value recap" card slots into the same conditional block, mirroring the `{#if has('FirstDaemonRegistered')}` gating pattern.
- **T-1d modal** — there is no existing pre-expiry modal pattern in the layout root. The closest precedents are Modal mounts that key off URL state (`SettingsModal` in `Sidebar.svelte:793–799`, `BillingPlanModal` mounted by `triggerUpgrade`). A new `TrialExpiryModal` would mount in `+page.svelte` and check `trialDaysLeft === 1 && !dismissedTodayInLocalStorage`. No reusable "show-once" state primitive exists; would need a small `InlineInfo`-style `dismissableKey` borrow.

#### A2. First-invoice amount display

- **Today (UI estimate).** `ui/src/lib/features/billing/BillingPlanForm.svelte:132–136` `getEstimatedTotal(plan)` = `plan.base_cents + extraSeats × plan.seat_cents + extraNetworks × plan.network_cents`. The "extras" are simulator-only knobs (lines 107–126) that the user adjusts manually — they do **not** reflect the org's actual seat / network counts.
- **Backend data already on the wire.** The `GET /api/billing/plans` response (`handlers.rs:69–82`) returns `BillingPlan` with `base_cents`, `seat_cents`, `network_cents`, `included_seats`, `included_networks`, `included_hosts` per plan (`billing/types/base.rs:131–143`, `876–884`). The current organization payload (`useOrganizationQuery`) carries `org.plan` (with the same fields), and `BillingTab.svelte:79–90` already computes `extraSeats * seat_cents + extraNetworks * network_cents` against actual `usersData.length` and `networksData.length` from `useUsersQuery` / `useNetworksQuery` for *post-purchase* display. So the data needed to compute a confirmed first-invoice total is already on the client.
- **Closest existing endpoint.** `preview_plan_change` (`handlers.rs:295–320` → `service.rs:1809–1849`) returns only `ChangePlanPreview { excess_hosts, excess_networks, excess_seats }` (`billing/types/api.rs:23–28`). It returns *quantities*, not dollars — no base/extras/total.
- **No confirmed-total endpoint exists.** The cleanest sketch: `POST /api/billing/checkout-preview` body `{ plan: BillingPlan }`, response `{ base_cents, included_seats, current_seats, extra_seat_cents, included_networks, current_networks, extra_network_cents, billing_period_total_cents, currency: "USD", trial_end_date: Option<DateTime> }`. Implementation would mirror `preview_plan_change`'s service-side counting (`service.rs:1815–1825`) plus multiply by per-extra cents pulled from the target plan's config. Auth: `Authorized<Owner>`. The UI then renders this on `BillingPlanForm` before the Checkout/setup-payment redirect rather than computing locally.

#### A3. Trial value recap inputs

What the recap could quantify and where each comes from:

- **`host_count`** — `GET /api/hosts` returns `PaginatedApiResponse<HostResponse>` with `total_count` (`hosts/handlers.rs:205–263`, especially L257–262). `BillingTab.svelte:69` already calls `useHostsQuery({ limit: 1 })` precisely to extract `pagination?.total_count` cheaply. **Reusable as-is.**
- **`network_count`** — `useNetworksQuery()` (`ui/src/lib/features/networks/queries.ts:14`) returns the full list; `.length` is the count. Already used by `BillingTab.svelte:62`.
- **`seat_count`** — `useUsersQuery()` (`ui/src/lib/features/users/queries.ts:14`); `BillingTab.svelte:57–58` uses `usersData.length`.
- **`service_count`** — `services/queries.ts:63–68` notes the cache is "primarily populated by useHostsQuery." `useServicesQuery()` exists at L68 and supports filtering. For an aggregate count, mirror the host pattern: a `limit=1` page would yield `total_count` if the service-list endpoint supports pagination (it does — `services/handlers.rs` exposes a paginated list endpoint per the route registration). Confirmable from one extra query, no new endpoint.
- **`scans_run` (discovery sessions completed)** — `discovery/handlers.rs:390–412` exposes only `GET /discovery/active-sessions` (in-flight only). There is **no endpoint that returns a count of completed discovery sessions** (live or historical). The Discovery entity itself is paginated (the broader CRUD route registered at `discovery/handlers.rs:69` `get_active_sessions`, plus the entity-level CRUD), but a "scans run during trial period" count would need either: (a) a new aggregate endpoint filtered by `started_at >= org.created_at`, or (b) reuse the dashboard endpoint if it carries a count.
- **`daemons_registered`** — `useDaemonsQuery` (a list-and-count, similar pattern). Trivial. Also derivable from `org.onboarding` (`Sidebar.svelte:121` reads `OnboardingOperation` array) which already tracks `FirstDaemonRegistered`.
- **Dashboard aggregate** — `HomeTab.svelte:181` consumes `dashboard.plan_usage` (typed `components['schemas']['PlanUsage']`) and `dashboard.daemons` / `dashboard.networks`. There is already a dashboard endpoint that returns `plan_usage` (host/network/seat counts vs limits) — that's the closest existing aggregate to a recap. Surface placement on the home tab can read from this without adding new queries.

**Bottom line:** for the four most likely recap metrics (hosts, services, networks, daemons) the data is already on the client via existing queries. Only **scans_run** would need an endpoint addition, and only if that metric makes the cut.

#### A4. Stripe-side mechanics for pause / extend

`async-stripe 1.0.0-alpha.2` supports all three:

- **`pause_collection`** — `UpdateSubscription::pause_collection(UpdateSubscriptionPauseCollection)` (registry path `async-stripe-billing/src/subscription/requests.rs:6713`). The struct (L4947–4958) takes `behavior: UpdateSubscriptionPauseCollectionBehavior` (`KeepAsDraft` / `MarkUncollectible` / `Void`, L4963–4966) and optional `resumes_at: Timestamp`. **Caveat from the SDK doc comment** (L4944–4945): "the subscription status will be unchanged and will not be updated to `paused`." So Stripe pause does *not* flip the Stripe subscription status — Scanopy still sees `Active` from the webhook and would need to read `pause_collection` separately or maintain a Scanopy-side `plan_status = 'paused'`.
- **Trial-end push** — `UpdateSubscription::trial_end(UpdateSubscriptionTrialEnd)` (L6789, enum at L6413). Accepts a Timestamp or the special `Now` value. Push trial end forward by N days = compute new `Timestamp`, call this setter. Existing code reads `sub.trial_end` after webhook update (`service.rs:1133–1134`), so a push will flow through the existing handler with no schema changes.
- **Coupon / discount application** — `UpdateSubscription::discounts(Vec<DiscountsDataParam>)` (L6666). `DiscountsDataParam` accepts `coupon: String` (Stripe coupon ID). For an in-app save offer, create coupons in Stripe dashboard once, reference by ID at runtime.
- **Cancellation reason capture (Stripe-native)** — `UpdateSubscription::cancellation_details(UpdateSubscriptionCancellationDetails)` (L6611). The struct (L4498–4509) takes `comment: Option<String>` and `feedback: Option<UpdateSubscriptionCancellationDetailsFeedback>` — enum with 8 values: `customer_service`, `low_quality`, `missing_features`, `other`, `switched_service`, `too_complex`, `too_expensive`, `unused` (L4518–4541). This matters for Bundle B: 5 of the brief's 7 candidate reasons map directly (`too_expensive`, `missing_features`, `unused`, `switched_service`, `other`); `tech_issues` is close to `low_quality`; `pausing` has no Stripe equivalent.

No additional Cargo features need to be enabled — `subscription`, `billing_portal_session`, `checkout_session`, and `customer` are already on (`backend/Cargo.toml`).

#### A5. `PAYMENT_METHOD_ADDED_BODY` orphan (item 5)

- **Template** — `backend/src/server/email/templates.rs:263–273`. Title at L263, body at L265. Confirmed at HEAD.
- **Build function** — `email/traits.rs:234–237` `build_payment_method_added_email()` returns the rendered (subject, body) pair. Already wired through the EmailService trait via `fn send_billing_email`.
- **Send function** — does **not exist** anywhere in `email/traits.rs` or `email/service.rs` (grep `send_payment_method_added_email` returns 0 hits). The orphan is real and unchanged from the audit.
- **Hook site for first-card-add detection.** Stripe `payment_method.attached` is already routed: `service.rs:877–887` matches `EventType::PaymentMethodAttached`, calls `handle_payment_method_attached(customer_id, payment_method_id)` (`service.rs:1353–1387`). At L1367 the code already flips `organization.base.has_payment_method = true` and at L1372–1379 sets the customer's default invoice payment method. **The cleanest insertion point for a `send_payment_method_added_email` call is right after `tracing::info!(...)` at L1381–1384, inside the same handler.** Owner email lookup pattern is in scope nearby — `handle_invoice_paid` at L2071–2076 shows `user_service.get_organization_owners(&organization.id).await?` then `owners.first()`.
- **Should this gate by trial state?** The brief frames this as "adding a card mid-trial." But `payment_method.attached` also fires when (a) a paid user updates their card via Portal, or (b) post-cancel card replacement. Probably *don't* gate by trialing — the acknowledgement is useful in all three cases, and the existing template body is generic enough to read sensibly for non-trial card-adds too.
- **Quick-win shippability:** Highly shippable as a standalone fix — ~15 LoC: add `pub async fn send_payment_method_added_email(&self, to: EmailAddress) -> Result<()>` to `email/service.rs` (mirror `send_payment_failed_email` at L660), call it from `handle_payment_method_attached` after the existing `tracing::info!` log. No template changes, no schema changes, no UX decisions needed.

#### A6. Post-Stripe confirmation moment (item 6)

- **Today.** `ui/src/lib/shared/components/layout/AppShell.svelte:196–239` handles the Stripe-return URL parameter. Two paths:
  - `billing_flow=checkout` (L199–226): if `isBillingPlanActive(organization)` is already true (webhook pre-arrived), fires `trackEvent('billing_completed', { plan, amount, plan_status })` and `pushSuccess(billing_subscriptionActivated())` — i.e. a transient toast (L206–211). Otherwise polls `waitForBillingActivation` (L110–134), which on success fires the same event + toast (L118–124).
  - `billing_flow=payment_setup` (L227–239): fires `trackEvent('payment_method_setup_completed', { plan_type, plan_status })` and invalidates the org query. **No toast or other surface today** — the user sees nothing.
- **Where a confirmation surface would live.** Three patterns are already in the codebase:
  - **Toast** — `pushSuccess` from `$lib/shared/stores/feedback`, ~5s display. Already used here. Lowest-friction.
  - **Inline status banner in BillingTab** — `BillingTab.svelte:345–353` already branches on `plan_status`; an `'active' && recently_activated` branch could render a celebratory `InlineInfo` with usage tips. Limited reach (Settings-only).
  - **Top-of-page `AppBanner`** — same chain as the trial-ending banner. Could render a one-time "Trial complete — welcome to {plan}" with `dismissableKey` until dismissed. Most prominent; matches the existing `LicenseGraceBanner` shape.
  - **Dedicated modal** — no precedent for a post-action confirmation modal. Would require a new component.
- **Cleanest fit for the patterns already established:** `AppBanner` for trial→paid (high-emotion moment, reachable from any page after redirect), plus **wire `pushSuccess` for `payment_method_setup_completed`** to close the silent-card-add gap from item 5's UI side.

---

### Bundle B — cancel-side

#### B1. Current cancel flow

- **UI entry** — `BillingTab.svelte:138–148` `handleManageSubscription()` → `customerPortalMutation.mutateAsync()` (`useCustomerPortalMutation`).
- **Mutation** — `ui/src/lib/features/billing/queries.ts:56–71` POSTs to `/api/billing/portal` with `window.location.origin` as the return URL.
- **Handler** — `backend/src/server/billing/handlers.rs:368–386` `create_portal_session`, auth `Authorized<RequireVerified<Owner>>`, calls `billing_service.create_portal_session(organization_id, return_url)`.
- **Service** — `backend/src/server/billing/service.rs:1704–1733` `create_portal_session`. The Stripe call is L1721–1724:
  ```rust
  let session = CreateBillingPortalSession::new(CustomerId::from(customer_id.clone()))
      .return_url(return_url)
      .send(&self.stripe)
      .await?;
  ```
  **That is the entire configuration.** No `flow_data`, no `after_completion`, no per-session feature toggles. Whatever Stripe Portal renders (cancel reasons, save offers, plan changes) is whatever the Stripe dashboard configuration enables — and the dashboard config is invisible from this repo.

#### B2. In-app cancel endpoint shape

- **`schedule_downgrade`** — `service.rs:1762–1806`. Pulls active subscriptions for the customer, calls `UpdateSubscription::new(&sub.id).cancel_at_period_end(true).send(&self.stripe).await?` (L1786–1789). Returns a copy string. **It does not capture a reason.** Today it's only called from `create_checkout_session` when target=Free (`handlers.rs:120–125`), i.e. when a paid user picks the Free tier from the plan picker — there is no in-app cancel surface that exercises it.
- **What the cancel webhook does today.** `handle_subscription_deleted` (`service.rs:1425–1528`) calls `extract_cancellation_details(sub.cancellation_details.as_ref())` at L1471, threads `cancel_reason_code` / `cancel_feedback` / `cancel_comment` through `process_subscription_deleted_side_effects` (L1502–1525), and emits the `subscription_cancelled` event with those fields at L1602–1604. So the *enrichment plumbing is already done* — we just need to populate `sub.cancellation_details` from in-app input.
- **Minimum new surface for an in-app cancel.** The cleanest shape:
  ```
  POST /api/billing/cancel
  Body: { reason_code: String, feedback?: String, save_offer_redeemed?: String }
  Auth: Authorized<RequireVerified<Owner>>
  ```
  Service call:
  ```rust
  UpdateSubscription::new(&sub.id)
      .cancel_at_period_end(true)
      .cancellation_details(UpdateSubscriptionCancellationDetails {
          feedback: map_to_stripe_enum(reason_code),
          comment: feedback,
      })
      .send(&self.stripe).await?;
  ```
  When the user's billing period ends, Stripe fires `customer.subscription.deleted` with `sub.cancellation_details` populated, and `extract_cancellation_details` (`service.rs:2114–2121`) and the existing event payload do the rest. **No new event emission needed** — the in-app cancel rides the existing webhook path and inherits Phase 1 P0-1's enrichment for free.
- **Caveat for non-Stripe-mappable reasons.** The brief's `pausing` reason has no Stripe enum; `tech_issues` is close to `low_quality` but not identical. Two options:
  - (a) Accept lossy mapping (e.g. `pausing → other`) and write the canonical Scanopy reason code into the **comment** field.
  - (b) Stash the Scanopy reason in `Subscription.metadata` (Stripe accepts arbitrary key-value) at cancel time, and have `extract_cancellation_details` prefer metadata over `cancellation_details.feedback` when present.
  Option (b) is cleaner and is the recommendation for Q1 below.

#### B3. Webhook routing for in-app cancel — emit-server-side or rely-on-webhook?

Today `subscription_cancelled` is emitted exclusively from `process_subscription_deleted_side_effects` at `service.rs:1592–1607`. This fires when Stripe sends `customer.subscription.deleted`, regardless of who initiated.

If the in-app cancel endpoint just calls `UpdateSubscription::cancel_at_period_end(true).cancellation_details(...)`, the user's subscription stays active until period end, and Stripe fires `deleted` only at that boundary. So there's:
- **No double-fire risk** (the webhook is the only emission path).
- **A delayed-event problem**: the `subscription_cancelled` event arrives at period end (days/weeks later), not at click time. Anyone wanting "user clicked cancel today" needs a second event.

Recommended approach: emit a new `subscription_cancel_initiated` event (or `cancel_intent_captured`) immediately from the in-app handler with reason + save-offer state, and let the existing `subscription_cancelled` continue to fire from the webhook at period end (it will carry the same reason via `cancellation_details`). Two events, both useful: the click event for funnel timing, the webhook event for terminal lifecycle.

#### B4. Stripe save-offer primitives

All available in `async-stripe 1.0.0-alpha.2`:

- **Discount (% or $ off, N months)** — create the coupon once in Stripe dashboard, then `UpdateSubscription::discounts(vec![DiscountsDataParam { coupon: Some(coupon_id), .. }])` (`async-stripe-billing/src/subscription/requests.rs:6666`).
- **Pause** — `UpdateSubscription::pause_collection(UpdateSubscriptionPauseCollection::new(behavior).resumes_at(...))` (L6713). See A4 caveat about `pause_collection` not flipping Stripe status.
- **Plan downgrade-as-alternative** — `change_plan` is already implemented (`service.rs:1855` onward), uses `UpdateSubscription::items(...)` to swap the price item. Reusable from a save-offer flow with no new SDK calls.
- **Trial extend (for "just pausing" alternative on a trialing sub)** — `UpdateSubscription::trial_end(...)` per A4.
- **Support handoff** — no Stripe primitive needed; opens an in-app contact form / mailto.

#### B5. `payment_recovered` email site (item 9)

- **Event emission** — `service.rs:2099–2108` inside `handle_invoice_paid` (L2061), gated by `was_past_due` at L2090. Confirmed at HEAD (audit's L2089 cite is for the if-block start).
- **Cleanest hook site for `send_payment_recovered_email`.** Right after the `tracing::info!` at L2094–2097 ("Payment recovered for past-due organization"), before the event publish. Pattern mirrors `handle_invoice_payment_failed` (`service.rs:1941–1994`), which fetches owners and calls `send_payment_failed_email(owner.base.email.clone())`. The owner-lookup code is not yet in the past-due branch — would need a `let owners = self.user_service.get_organization_owners(&organization.id).await?;` between L2097 and L2099, then `if let Some(owner) = owners.first() && let Some(ref email_service) = self.email_service { ... }`.
- **Template / build / send fns.** None exist. New work:
  - Add `PAYMENT_RECOVERED_TITLE` + `PAYMENT_RECOVERED_BODY` to `email/templates.rs` (mirror `PAYMENT_FAILED_BODY` at L451–467).
  - Add `build_payment_recovered_email(&self) -> (String, String)` to `email/traits.rs` (mirror L239–242 `build_payment_failed_email`).
  - Add `pub async fn send_payment_recovered_email(&self, to: EmailAddress) -> Result<()>` to `email/service.rs` (mirror L660 `send_payment_failed_email`).
  - Hook it from `handle_invoice_paid` per above.
- **Quick-win shippability:** Same shape and roughly the same LoC as item 5. Independent of Bundle B item 7 (in-app cancel). Highly shippable standalone.

#### B6. `period_end` for the post-cancel email (item 8)

- **Where the email fires.** `process_subscription_deleted_side_effects` (`service.rs:1533`) calls `email_service.send_subscription_cancelled_email(owner.base.email.clone()).await` at L1611. The function signature today (`email/traits.rs:153`, `email/service.rs:648`) takes only `to: EmailAddress` — no `period_end` arg. Template (`templates.rs:245–261`) is static, no placeholders.
- **Is `current_period_end` in scope?** Yes — at the call site upstream. `handle_subscription_deleted` (`service.rs:1425`) receives the full Stripe `Subscription` object, which has `current_period_end: Timestamp` (Stripe-side field, populated by the webhook). At L1469 the synchronous phase captures `customer_id` from the org and several other fields; at L1497–1525 it spawns the async task with positional args. **`sub.current_period_end` is in scope at L1469 and is not currently passed through.**
- **What threading it through looks like.** Capture `let period_end = sub.current_period_end;` at ~L1469, pass it into `process_subscription_deleted_side_effects` (add to the positional args at L1532–1551, which already has `#[allow(clippy::too_many_arguments)]`), pass it into `send_subscription_cancelled_email` (extend signature in `traits.rs:153`, `service.rs:648`, `traits.rs:225` `build_subscription_cancelled_email`), and add a `{period_end_date}` placeholder to `SUBSCRIPTION_CANCELLED_BODY` at `templates.rs:245`.
- **Quick-win shippability:** Slightly larger than items 5/9 because it touches 4 files (service.rs, traits.rs, service.rs in email module, templates.rs) plus the template body, but no new database columns and no new endpoints. The brief flagged this as preferable to waiting for the deferred P1-5 `organizations.period_end` column. **Standalone-shippable** but worth coordinating with item 9 since both touch `send_*_email` family additions.

---

### Bundle C — downgrade-side

#### C1. What breaks on downgrade — drift check vs audit's table

The audit's per-feature table (3c) is verified at HEAD. Drift notes:

| Surface | Audit cite | Status at HEAD |
|---|---|---|
| Scheduler early-return for Free orgs | `discovery/service.rs:619–636` | **Confirmed** (L619–636 unchanged: gates by `org.base.plan.as_ref().is_some_and(|p| p.is_free())`). |
| Embed render gate | `shares/handlers.rs:393–401` | **Confirmed** (L398–401: 402 "Embed access requires a plan with embeds feature"). |
| API key auth-time block | `auth/middleware/auth.rs:426–444` | **Confirmed** (L426–444: 402 "Your plan does not include api access" at user-API-key auth). |
| `RequireFeature` gates | `auth/middleware/features.rs:127–222` | **Confirmed.** `InviteUsersFeature` (L128–140), `ApiKeyFeature` (L142–154), `ShareViewsFeature` (L156–170), `CreateNetworkFeature` (L172–222). |
| Host creation cap | `hosts/handlers.rs:373–410` | **Confirmed** (L373–410: emits `feature_limit_hit`, returns `BillingHostLimitReached`). |
| `DaemonStandby` error code | `error_codes.rs:300–301` | **Drift.** The error code still exists, but at `daemons/service.rs:1981` `daemon.base.standby = true` is now set by `check_daemon_inactivity` (L1934, "no completed discovery in 30 days"), not by a plan check. The error message copy ("Your plan does not support DaemonPoll mode") now misleads in the case it actually fires from. **Plan-driven DaemonPoll gating remains unimplemented.** |
| Existing share-render does not check `share_views` | `shares/handlers.rs:374–586` | **Confirmed.** Share render (`get_share_topology`) only gates `embeds`; existing shares keep working post-downgrade. |
| Free middleware exemption | `auth/middleware/billing.rs:97–104` | **Confirmed** (Free orgs pass through billing middleware without status checks; per-feature gating happens at the auth/extractor layer). |

**One worth flagging in the design pass:** the daemon standby drift means item 3 of the cross-cutting summary ("DaemonPoll gating advertised in copy but not enforced in code") is half-resolved (standby IS now set programmatically) but the part about plan-feature-gating is still unimplemented. This is a Phase 5 scope question the founder should know about — the misleading-copy issue likely warrants a quick rewording fix in `error_codes.rs:300` independent of Phase 5.

#### C2. Banner / modal placement for downgrade communication

- **The mounting point.** `ui/src/routes/+page.svelte:225–240` is the chain of global banners: `EmailVerificationBanner` (L225–227), `DemoBanner` (L228–230), `LicenseLockedBanner` / `LicenseGraceBanner` / `LicenseExpiringBanner` (L231–240). All sit above `<main>` and below the sidebar. Built on `AppBanner.svelte` (`ui/src/lib/shared/components/feedback/AppBanner.svelte`), which takes `variant: 'info' | 'warning' | 'danger'`, an icon, an HTML-renderable body, and an optional actions Snippet.
- **Closest precedent.** `LicenseGraceBanner` (`ui/src/lib/shared/components/feedback/LicenseGraceBanner.svelte`) is the most direct analog: a `warning`-variant `AppBanner` with date-aware copy interpolated via paraglide (`license_graceBanner({ intendedExpiry, hardExpiry })`). A `DowngradeRecoveryBanner` with `warning` variant + `AppBanner` action slot for "Restore full access" → `triggerUpgrade({ source: 'downgrade_banner' })` would slot into the same chain at L240 and reuse the existing visual language.
- **Persistence.** No global "show-once-then-dismiss" primitive exists. `InlineInfo` has a `dismissableKey` prop (used at `ShareConfigPanel.svelte:160`); the same pattern could be added to `AppBanner` if dismissibility is wanted. Without that, the simplest gate is `org.plan_status === 'active' && org.plan?.type === 'Free' && Date.now() - org.downgraded_at < N_DAYS` — but `downgraded_at` is not on the organizations table today (audit 3c severity HIGH: "no `downgraded_at` column"). Time-bounded persistence requires that column or PostHog event-stream lookup.
- **Modal pattern.** No precedent for a "modal-on-next-login-after-downgrade." Closest existing modal-driven moments are `BillingPlanModal` (triggered by `triggerUpgrade`) and `SettingsModal` (URL-driven). A `DowngradeConsequencesModal` would mount in `+page.svelte` and check a "first login after plan_changed → Free" condition — same `downgraded_at` dependency.

#### C3. Recovery affordance hook sites (item 11)

Inventory of where contextual "restore full access" affordances would attach:

- **Scheduled discovery toggle** — `ui/src/lib/features/discovery/components/DiscoveryModal/DiscoveryDetailsForm.svelte:71–87, 174–198`. The `runTypeOptions` array (L71–87) marks the `Scheduled` option `disabled: !hasScheduledDiscovery` and adds a yellow "Upgrade" tag with `ArrowUpCircle`. The `RichSelect` at L182–191 wires `onDisabledClick` to `triggerUpgrade({ feature: 'scheduled_discovery', source: 'discovery_form' })`. **Already wired** — the contextual affordance exists.
- **Scheduled discovery card paused state** — `ui/src/lib/features/discovery/components/cards/DiscoveryScheduledCard.svelte:134–138`. Renders `discovery_schedulePausedFreePlan` ("Paused — upgrade to resume scheduled runs") when on Free. **Already wired** — but the card itself doesn't expose a click-to-upgrade button; the user would click into the Discovery and see the form gate.
- **Embed code panel** — `ui/src/lib/features/shares/components/ShareConfigPanel.svelte:81, 199–212`. `hasEmbedsFeature` derived at L81 from `billingPlans.getMetadata(plan.type).features.embeds`. When false (L201–205), renders `InlineInfo title={shares_embedsRequirePlan()} body={shares_upgradeForEmbeds()}` plus `<UpgradeButton feature="embeds" />`. **Already wired.**
- **Export modal gated formats** — `ui/src/lib/features/topology/components/ExportModal.svelte:172–203`. `formatOptions` derived at L172 marks gated formats `disabled: !featureGates[f.featureKey]?.()` with an "Upgrade" tag (L173, L187). `handleDisabledFormatClick` at L198–203 routes through `handleUpgrade(format.featureKey)` which triggers the upgrade modal. **Already wired.**
- **Disabled share-create button** — Stronger than the audit framing. `ui/src/lib/features/shares/components/SharesModal.svelte:258–263`: when `!hasShareViews`, the entire share list is replaced with `<EmptyState title={shares_noSharesYet()}>` containing `<UpgradeButton feature="share_views" />`. There's no "create share" button to disable — the create surface is fully replaced by the empty-state + upgrade prompt. **Already wired** (just differently than the audit implied).
- **Hosts at-limit state** — `ui/src/lib/features/hosts/components/HostTab.svelte:81–141, 361–382` per audit. `<UpgradeButton feature="hosts" />` is rendered inline when `count >= limit`. **Already wired.**
- **Plan Usage card** — `ui/src/lib/features/home/components/PlanUsage.svelte:66–89` per audit. Header contains `<UpgradeButton feature="plan_usage" />` when any resource ≥80%. **Already wired.**

**What's NOT yet wired with a contextual affordance:**
- **Network-create at-limit** on the Networks tab — server returns 402 via `CreateNetworkFeature`, but no proactive UI affordance was located on the Networks listing.
- **Seat-invite at-limit** on Settings → Members — no inline badge / counter / upgrade button on the members surface (the gate fires on submit).
- **API-key-create when `api_access: false`** — no proactive disabled state on the API key create form was located.
- **Generic "your existing API key just stopped working"** — when an API key gets the 402 from `auth.rs:440–443` at request time, the consumer (a third-party script) sees a 402, but there's no in-app surface in the Scanopy UI that says "your API key is now blocked because of your plan." This is a downgrade-only failure mode that has no UI feedback at all.

For each unwired surface, the **shape of the contextual affordance** to add:
- **Networks tab** — mirror `HostTab` pattern: an inline counter on the Networks list header showing `{used}/{limit}` in amber when at limit, with `<UpgradeButton feature="networks" />` replacing the Create button.
- **Members tab** — same shape on Settings → Members: at-limit counter + `<UpgradeButton feature="invite_users" />` in place of the invite button.
- **API keys tab** — disabled-state on the create-key form with "Upgrade to enable API access" copy + `<UpgradeButton feature="api_access" />`.
- **Top-level downgrade banner** (per C2) — the catch-all for the API-key-blocked-at-auth-time failure that has no in-app surface, because the banner itself surfaces the loss-of-access without depending on the user happening to land on the right tab.

---


## Part 2 — Settled feature decisions

### Cancel flow

**Surface.** Modal v1, layered over Settings → Billing. Replaces the current `BillingTab.svelte:138–148` "Manage Subscription" handoff to Stripe Portal. Existing `BillingPlanModal` / `SettingsModal` patterns are the precedent.

**Step count.** Three steps in a single modal:
1. **Reason capture** — required reason picker + optional free-text comment field.
2. **Save offer** — reason-dependent; some reasons skip this step entirely (see triggering below).
3. **Confirmation** — discloses `period_end` (when access ends), data retention policy, what stops working at period end. Final cancel button on this step.

Explicit Back affordance between steps so the user can correct a wrong reason.

**Reason taxonomy.** Hybrid model with three layers:

- **Scanopy-canonical 7-value enum** (source of truth):
  - `too_expensive`
  - `missing_feature`
  - `not_using_enough`
  - `better_alternative`
  - `tech_issues`
  - `pausing`
  - `other`
- **Stripe-side mapping** (carried via Stripe so cancellation_details survives in their dashboard):
  - `too_expensive → too_expensive`
  - `missing_feature → missing_features`
  - `not_using_enough → unused`
  - `better_alternative → switched_service`
  - `other → other`
  - `tech_issues → low_quality` (closest map; not exact)
  - `pausing` → no Stripe enum equivalent; map to `other`
- **Canonical reason in `Subscription.metadata["scanopy_cancel_reason"]`** at the moment we call `UpdateSubscription`. The webhook ingest path (`extract_cancellation_details` at `service.rs:2114–2121`) extends to prefer the metadata value over `cancellation_details.feedback` when both are present, so the canonical Scanopy reason survives even when the Stripe enum is lossy.
- **Free-text comment** stored in Stripe's `cancellation_details.comment` and on the cancellations table.

**Save offer types — v1 subset.** Two offers, no more:
- **Pause** — surfaces the pause flow (see Pause section below).
- **Discount** — coupon application via `UpdateSubscription::discounts(...)`. Stripe coupon created once in dashboard, referenced by ID at runtime.

Defer for now: support handoff (founder already has `billing@scanopy.net`), plan-downgrade-as-alternative (already partially exposed via plan picker; muddies the cancel flow).

**Save offer triggering — reason-dependent:**
- **Pause shown** for: `pausing`, `not_using_enough`, `too_expensive`.
- **Discount shown** for: `too_expensive` only.
- **No save offer** (skip step 2, go straight to confirmation) for: `missing_feature`, `better_alternative`, `tech_issues`, `other`.

A user with `too_expensive` sees both pause and discount as options on step 2.

**Persistence — typed `cancellations` table.** Backfill from PostHog event stream is ruled out, so PostHog-only would be a one-way door for any future product feature that reads cancel state at runtime (admin review UI, automated comeback flows, re-onboarding personalization). The table ships in Phase 5.

Schema:
```
cancellations
├── id: uuid (PK)
├── organization_id: uuid (FK → organizations)
├── plan: text (snapshot of plan name at cancel time)
├── mrr_cents: bigint (snapshot of MRR at cancel time, for cohort analysis)
├── reason_code: text (Scanopy-canonical enum value)
├── stripe_feedback: text NULL (Stripe enum mapping if applicable)
├── comment: text NULL (free-text from user)
├── save_offer_shown: text[] (which offers were displayed)
├── save_offer_redeemed: text NULL ('pause' | 'discount' | NULL)
├── period_end: timestamptz (when access actually ends)
├── cancelled_at: timestamptz (when user clicked cancel — not period_end)
└── tenure_days: integer (days from org.created_at to cancelled_at)
```

Continue emitting the enriched `subscription_cancelled` PostHog event at period-end (it already carries the canonical reason via `extract_cancellation_details`). Add a new `cancel_intent_captured` PostHog event emitted immediately at click time from the in-app cancel handler, carrying reason + save-offer state for funnel timing.

**Confirmation step (last step of same flow).** Discloses:
- `period_end` date when access ends ("You'll keep access through {date}.")
- Data retention ("Your data is preserved on the Free plan; existing shares keep working; scheduled discoveries pause; new API keys can't be issued.")
- A single "Confirm cancel" button.

---

### Pause / extend mechanics

**Pause** — Stripe-native `pause_collection` for billing suspension + Scanopy-side `plan_status='paused'` for feature gating. Per the SDK doc comment at `async-stripe-billing/src/subscription/requests.rs:4944–4945`, Stripe's `pause_collection` does *not* flip the Stripe subscription status to "paused" — Stripe keeps it `active`. Without the Scanopy-side status, the existing middleware would treat the user as fully active and not gate paused-state UX. Both layers are needed.

On pause: call `UpdateSubscription::pause_collection(UpdateSubscriptionPauseCollection::new(behavior).resumes_at(<chosen_date>))` AND set `organization.plan_status = "paused"`. On resume (auto via `resumes_at` or manual via "resume early" button): call `UpdateSubscription` with `pause_collection` cleared AND set `plan_status = "active"`. Sync via the `customer.subscription.updated` webhook handler at `service.rs:930` for cases where pause state changes via Stripe Portal or admin tooling.

**Pause duration.** User picks at confirm time from preset durations:
- 30 days
- 60 days
- 90 days

Resume date displayed clearly at confirm time ("Billing resumes 2026-06-27"). Cap at 90 days so the 6-month eligibility window leaves a meaningful paid period before the next eligibility unlock. **Resume-early button always available** in Settings → Billing while paused — user can shorten the pause at any time.

**Pause eligibility — once per rolling 6-month window per org.** New column: `organizations.last_paused_at: timestamptz NULL`. Eligibility check:
```
eligible := last_paused_at IS NULL OR (now() - last_paused_at >= interval '6 months')
```

When ineligible, the UI displays the next-eligible date ("You can pause again on 2026-09-12") instead of showing the pause flow as a dead-disabled option. This applies both to pause-as-save-offer in the cancel flow and any future pause-as-Settings-option (see Part 3 — pause flow placement).

**Trial extend** — Stripe-native via `UpdateSubscription::trial_end(UpdateSubscriptionTrialEnd::Timestamp(new_end))`. No Scanopy-side override. The existing webhook handler at `service.rs:1133–1134` already reads `sub.trial_end` and reflects it to `organization.trial_end_date`, so the push flows through the existing handler without schema changes.

**Trial extend duration.** **+7 days.** Frames as "an extra week to decide" rather than "a full second trial." Easier to loosen later if data shows +7 is too short than to tighten from +14 after users have anchored on the longer number.

**Trial extend eligibility — once per org lifetime.** New column: `organizations.has_used_trial_extend: boolean NOT NULL DEFAULT false`. UI on the BillingTab card includes the explicit one-time framing: "Extend your trial by 7 days (one-time extension)."

**Soft-downgrade with restore — collapsed; no new backend state.** The existing single Free state plus the in-product recovery work (downgrade banner + contextual affordances + downgrade communication, all under the downgrade-communication section below) is the implementation. No `plan_status='soft_downgraded'`, no flag distinguishing trial-lapse vs cancel vs voluntary-downgrade Free-arrival paths. Re-entry uses existing setup-payment-method + create-checkout flows; `trial_end_date` is already preserved on the org so restore-to-trialed-plan logic works.

---

### Trial value recap

**Surfaces — two:**
- **BillingTab card** (in-app, lazy on render). Slots into `BillingTab.svelte` in the same vertical stack as the existing trial-countdown InfoCard (L171–197) and Current Plan card (L200+). Reads from existing queries (`useHostsQuery({limit:1})`, `useNetworksQuery()`, `useDaemonsQuery`) plus one new query for service count. Renders only during trialing state.
- **T-3d email.** Pre-computed at the email-send job. Same metric set.

Skip dashboard widget for v1 — competes with `PlanUsage` / `NetworkMetrics` cards on the home tab without adding reach the BillingTab card doesn't already get during trial.

**Metrics — five, in this order:**
1. **Hosts discovered** ("Scanopy discovered 47 hosts on your network")
2. **Networks mapped** ("across 3 networks")
3. **Daemons connected** ("with 2 daemons collecting data")
4. **Services identified** ("identifying 124 services")
5. **Days into trial** ("during your 14-day trial")

Pulled from existing endpoints (per Part 1 §A3). **Skip** scans run, shares created, time invested, audit trail entries — the chosen 5 emphasize what Scanopy did *for* the user, not what the user did. The one exception (daemons connected) is included specifically because daemon install is the highest-friction onboarding step and recalling that effort raises the cost of walking away.

**Computation timing.**
- BillingTab card: lazy on settings open (cheap; ~4 parallel queries with `limit=1` pagination tricks where applicable).
- T-3d email: pre-computed at email-send time as part of the email job, using direct DB aggregation. Ensures the email's claim ("you discovered 47 hosts") reflects state at *send time*, not state at *open time* hours/days later.

**Empty-state handling — aha-moment intervention, not suppression.** When the would-be recap shows 0 networks / 0 services (user did minimal exploration during trial), render a different card: replace the recap with a getting-started prompt that lifts a task from the existing `GettingStartedChecklist.svelte` (used by `Sidebar.svelte:120–122`), framed as "Run your first discovery to see what's on your network — your trial ends in {N} days." Same vertical slot, different card content.

---

### Downgrade communication

**Channels — two:**
- **Email at downgrade** (mandatory). Replaces today's one-sentence `PLAN_CHANGED_BODY` (`templates.rs:206–221`) with a richer body that names `period_end` for paid cancels (or trial-end date for trial-lapse), enumerates the per-feature delta from prior plan to Free, and links to the in-product "what changed" page (below).
- **Top-of-page in-app `AppBanner`** persistent for the post-downgrade window. Slots into the `+page.svelte:225–240` chain alongside `LicenseGraceBanner`. Variant: `warning`. Action snippet: "Restore full access" → opens `BillingPlanModal` via `triggerUpgrade({ source: 'downgrade_banner' })`.

Skip modal-on-next-login. The banner is the right balance of reach and respect; a forced modal on a user who has just lost access is punitive.

**Banner persistence — 14 days time-bound, dismissible at any time within the window.** After 14 days, banner stops rendering even if not dismissed. Backed by:

```
ALTER TABLE organizations ADD COLUMN downgraded_at timestamptz NULL;
ALTER TABLE organizations ADD COLUMN previous_plan jsonb NULL;
```

Set on **any** plan downgrade — paid→paid (Pro→Starter etc), paid→Free (cancel period-end), trial-lapse→Free. Set in three code paths:
- `handle_subscription_update` at `service.rs:930` — when `plan_changed` is a downgrade (i.e. `is_downgrade=true`).
- `handle_subscription_deleted` at `service.rs:1425` — when `plan_status` flips to Free at cancel period-end.
- The trial-lapse path that fires `TRIAL_EXPIRED_BODY` (per audit, `service.rs:1186`).

`previous_plan` captures the plan that was active *before* the downgrade, stored as the full `BillingPlan` JSON (mirroring how `organization.plan` is stored). Enables future paid-downgrade banner copy ("Restore Pro access") without a second migration. **Banner UI in v1 only renders for the Free case** (`org.plan?.type === 'Free' && org.downgraded_at IS NOT NULL && now() - downgraded_at < 14 days`), but the data model supports paid-downgrade retention surfaces later.

Banner gets a `dismissableKey` prop (extending the pattern used by `InlineInfo` at `ShareConfigPanel.svelte:160`) so per-user dismissal sticks across sessions.

**Per-feature loss messaging — three layers:**
- **Banner** — single-line summary ("Your account is on Free. Restore full access by upgrading.")
- **Email** — per-feature enumeration in the body ("Scheduled discoveries paused. New shares disabled. API keys blocked. Existing exports limited to PNG/CSV.")
- **"What changed" page** — dedicated route or modal, linked from both banner and email, generated from the `BillingPlanFeatures` struct comparison between `previous_plan.features()` and `Free.features()`. Renders the full delta table on demand. Backend already has the data; this is presentation-layer work only.

**Recovery affordances — both top-level banner and contextual buttons.** Top-level banner per above. Contextual buttons on the surfaces that just stopped working — most are already wired (per Part 1 §C3): scheduled discovery toggle, scheduled discovery card, embed code panel, export modal, share-create flow, hosts at-limit, plan usage card. **New affordances to add:**
- **Networks tab** — inline counter on the Networks list header, `<UpgradeButton feature="networks" />` replaces Create button at limit.
- **Settings → Members** — at-limit counter + `<UpgradeButton feature="invite_users" />` in place of invite button.
- **API keys tab** — disabled-state on the create-key form with "Upgrade to enable API access" copy + `<UpgradeButton feature="api_access" />`.

The top-level banner covers the API-key-blocked-at-auth-time silent-fail (third-party scripts get a 402, but the Scanopy UI doesn't surface it anywhere) — banner ensures the user sees *something* in the UI even if they never visit the API keys tab.

---

## Part 3 — Settled UX decisions

**Trial urgency ramp.** New surfaces appear at T-3d / T-1d, rather than escalating the existing settings-only InfoCard:
- **Always-on (already exists):** sidebar settings notification dot when trialing-without-card (`Sidebar.svelte:108–114, 777–780`).
- **T-7d:** sidebar pill ("Trial ends in {N} days") flips on by extending the `Sidebar.svelte:101` `showUpgradeButton` derivation to include trialing state.
- **T-3d:** top-of-page `AppBanner` (warning variant) with "Trial ends in {N} days. Add a payment method to keep access." Action button on the banner.
- **T-1d:** one-time modal that mounts in `+page.svelte`, gated on `trialDaysLeft === 1 && !dismissedTodayInLocalStorage` (small show-once primitive borrowed from `InlineInfo`'s `dismissableKey`).

The existing settings-only InfoCard becomes the destination users land on when they click any of the above surfaces — not the surface that escalates.

**Cancel modal step count.** Three steps (reason → save offer or skip → confirmation), per Part 2. Single-step would pack the reason picker, save offer, and disclosure into one viewport — at minimum competing for attention; at worst making the save offer feel like a sneaky speed-bump between "confirm cancel" and "cancel button." Multi-step is legible and reversible (Back button preserves entered state).

**Pause flow placement.** Cancel-flow-only for v1. **No standalone Settings option.** Pause is highest-leverage as a save offer (the user is already on the way out; pause is the alternative-to-leaving). A standalone Settings entry point dilutes pause's "consider-staying" framing into routine billing-management. v2 promotion to Settings only after we observe pause's adoption pattern in the cancel funnel.

**Trial extend mechanics.** Self-serve, on the BillingTab card during the T-3d / T-1d window. Copy: "Need more time? Extend your trial by 7 days (one-time extension)." Eligibility check (`!has_used_trial_extend`) gates the link; ineligible users see no link at all (rather than a disabled state with a confusing "you've already used this" message — once they've extended, the affordance simply isn't shown again).

**Recovery banner copy/tone.** Neutral, factual. Banner body: "Your account is on Free. Restore full access by upgrading." The action verb is **Restore** (not Upgrade) — frames the action as resuming a state the user once had, gentler than implying a new commitment. Apologetic tone wrongly implies we did something the user is owed an apology for; opportunity-framing reads as upsell-spam at the worst possible moment.

---

## Part 4 — Schema, quick wins, scope shifts

### Schema changes summary

All in one phase-5 migration set (or split as the implementing worker prefers):

**New table:**
```sql
CREATE TABLE cancellations (
  id uuid PRIMARY KEY,
  organization_id uuid NOT NULL REFERENCES organizations(id),
  plan text NOT NULL,
  mrr_cents bigint NOT NULL,
  reason_code text NOT NULL,
  stripe_feedback text NULL,
  comment text NULL,
  save_offer_shown text[] NOT NULL DEFAULT '{}',
  save_offer_redeemed text NULL,
  period_end timestamptz NOT NULL,
  cancelled_at timestamptz NOT NULL,
  tenure_days integer NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX CONCURRENTLY idx_cancellations_org ON cancellations(organization_id);
CREATE INDEX CONCURRENTLY idx_cancellations_cancelled_at ON cancellations(cancelled_at);
```
(With required boilerplate per `CLAUDE.md` migration rules: `SET lock_timeout = '5s';` first; `CREATE INDEX CONCURRENTLY` with sqlx `-- no-transaction` header.)

**New columns on `organizations`:**
```sql
ALTER TABLE organizations ADD COLUMN downgraded_at timestamptz NULL;
ALTER TABLE organizations ADD COLUMN previous_plan jsonb NULL;
ALTER TABLE organizations ADD COLUMN last_paused_at timestamptz NULL;
ALTER TABLE organizations ADD COLUMN has_used_trial_extend boolean NOT NULL DEFAULT false;
```

All four columns are independently set; no foreign-key relationships between them.

**Storage layer additions** — per `CLAUDE.md` "no `SqlValue::JsonValue`" rule, `previous_plan` JSONB column requires a typed `SqlValue` enum variant for `BillingPlan`. Reuse the existing variant if `organizations.plan` already uses one; otherwise add one in the same migration set.

**Test fixtures** — new `cancellations` table requires an entry in `get_entity_deserializers()` in `backend/src/server/shared/storage/tests.rs` per `CLAUDE.md` rules.

### Quick wins (ship standalone, ahead of bundle work)

These are independent of every Phase 5 design decision and can ship as standalone PRs the moment a worker picks them up. Recommended order: 5 + 9 in parallel (same shape), then 8 (slightly larger; touches the cancel email which Bundle B in-app cancel will also touch — better to land it first so Bundle B builds on a `period_end`-aware email template).

- **Item 5 — `PAYMENT_METHOD_ADDED_BODY` wiring.** Hook site: `service.rs:1381–1384` inside `handle_payment_method_attached`. Add `pub async fn send_payment_method_added_email(&self, to: EmailAddress) -> Result<()>` to `email/service.rs` (mirror `send_payment_failed_email` at L660), call it from the handler. ~15 LoC. Pair with `pushSuccess(...)` for `payment_method_setup_completed` at `AppShell.svelte:227–239` so the in-app moment also gets a toast.
- **Item 8 — `period_end` in post-cancel email.** Capture `let period_end = sub.current_period_end;` at `service.rs:1469`, thread through `process_subscription_deleted_side_effects` (`service.rs:1532–1551`, already has `#[allow(clippy::too_many_arguments)]`), into `send_subscription_cancelled_email` (extend signature in `email/traits.rs:153`, `email/service.rs:648`, `email/traits.rs:225` `build_subscription_cancelled_email`), add `{period_end_date}` placeholder to `SUBSCRIPTION_CANCELLED_BODY` at `templates.rs:245`. Touches 4 files.
- **Item 9 — `payment_recovered` email.** Hook site: `service.rs:2099` in `handle_invoice_paid`. Owner-lookup pattern from `handle_invoice_paid` at L2071–2076. Add new `PAYMENT_RECOVERED_TITLE` + `PAYMENT_RECOVERED_BODY` in `templates.rs`, `build_payment_recovered_email` in `traits.rs`, `send_payment_recovered_email` in `service.rs`. ~30 LoC including new template.

### Scope shifts vs prep doc

The prep doc framed several things as recommendations; the design pass settled them differently or expanded scope. Worth flagging for whoever reads only this spec without the prep doc history:

- **Cancellations table moves into Phase 5 scope.** Prep doc recommended PostHog-only with the typed table deferred (as P1-6). Backfill from PostHog is ruled out — without the table now, any future product feature that reads cancel state at runtime is permanently blind to historical cancellations. Decision: ship the table in Phase 5.
- **Downgrade tracking is broader than the prep doc framed.** Original framing was a Free-specific column ("downgraded_at fires only on Free arrival"). Decision: column triggers on *any* downgrade, including paid→paid (Pro→Starter etc.), with a `previous_plan` companion column. Banner UI scoped to Free in v1 — but data model supports paid-downgrade retention surfaces later without a second migration.
- **Pause eligibility is more generous than prep-doc default.** Prep doc proposed once-per-org-lifetime as the simplest default. Decision: once per rolling 6-month window per org, with the next-eligible date displayed in the UI when ineligible. Requires `last_paused_at` column.
- **Trial extend duration is shorter than prep-doc default.** Prep doc proposed +14d (matching standard trial length). Decision: +7d ("an extra week to decide" framing rather than "a full second trial").
- **Soft-downgrade with restore — confirmed collapsed.** No new backend state. The work folds into the downgrade-communication section: banner + contextual recovery affordances + downgrade-moment email + "what changed" page. Bundle A item 4 reduces to two sub-features (pause + trial extend) rather than three.
- **Bundle ship-order — out of spec scope.** Prep doc Part 4 included a recommendation here; design pass dropped it as a coordinator-spawn concern, not a spec concern.

### Drift cleanup (independent of Phase 5)

The `error_codes.rs:300–301` `DaemonStandby` copy ("Your plan does not support DaemonPoll mode. The daemon is on standby. Upgrade your plan and restart the daemon to resume.") is now misleading. The only code path that sets `daemon.base.standby = true` is `daemons/service.rs:1981` (inactivity-driven, "no completed discovery in 30 days") — there is no plan-feature gate that puts daemons on standby. Worth a one-line copy fix to something like "The daemon is on standby due to inactivity. Restart the daemon to resume." This is independent of Phase 5 scope but worth the Phase 5 worker's awareness so it doesn't surprise during implementation.
