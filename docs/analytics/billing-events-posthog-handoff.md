# Billing Events → PostHog: Analysis Handoff

**Audience:** PostHog's AI agent (and analysts) building conversion, churn, and
lifecycle analyses for Scanopy.
**Purpose:** Catalog every billing-related PostHog event, its trigger, and its
typed properties; specify the analyses to build and exactly which
events/properties to key on; and define the discriminator that separates
**conversion** from **downgrade-to-Free** so the two are never conflated.

This is an audit of events that **exist** — property names are taken verbatim
from the Rust payload definitions. Do not invent properties. Where a field is
not present, the analysis section says so.

---

## 1. How billing events reach PostHog

Every `BillingOperation` (Rust enum, `backend/src/server/shared/events/types.rs:124-265`)
is forwarded to PostHog by `PosthogService`'s billing subscriber
(`backend/src/server/posthog/subscriber.rs`). The subscriber matches **all**
billing events (`EventFilter::all()`), so the catalog below is exhaustive and
stays exhaustive as new variants are added.

For each event the subscriber emits one PostHog `capture` with:

| PostHog field | Source | Notes |
|---|---|---|
| **event name** | `operation.to_string()` | strum `snake_case` of the variant — e.g. `CheckoutCompleted` → **`checkout_completed`**. These snake_case names are what you query on. |
| `organization_id` | event scope | top-level string property; also the PostHog **group** key (group type `organization`). |
| `auth_type` | actor | `user` / `api_key` / `daemon` / `system`. |
| `user_id`, `email` | actor (if a user) | present when the event was user-initiated; **absent** for `system`/webhook-initiated events (see §5). |
| `daemon_id` | actor (if a daemon) | rare for billing. |
| **`metadata`** | `serde_json::to_value(operation)` | the **entire event payload** as a nested object. Every typed property in the catalog is queryable as **`metadata.<field>`**. The variant is tagged at **`metadata.type`** (serde PascalCase — e.g. `metadata.type = "CheckoutCompleted"`). |

Person + group properties are also updated on every event:
- `plan_type` (person and group `organization`) ← `operation.plan().name()` where the variant carries a plan.
  ⚠️ See §5 for a known caveat on `plan_type` for downgrade-to-Free events.

**`metadata.plan` shape.** `plan` is a `BillingPlan` enum serialized with a
`type` tag: `metadata.plan.type` is the plan key — one of `Free`, `Starter`,
`Pro`, `Team`, `Business`, `Enterprise`, `Community`, `Demo`,
`CommercialSelfHosted` (`backend/src/server/billing/types/base/plans.rs:20-30`).
`metadata.plan.type == "Free"` is the canonical "is this the Free plan" test.

---

## 2. Event catalog

22 billing events. "Trigger" cites the emission `file:line`. Properties are the
fields under `metadata.*` (plus the common properties from §1 on every event).

### Acquisition / conversion

| Event (`name`) | Trigger | Key `metadata` properties |
|---|---|---|
| `checkout_started` | `checkout.rs:116` (hosted Checkout), `:241` (in-app trial sub), `:299` (in-app paid sub) | `plan`, `has_trial` |
| `checkout_completed` | `checkout.rs:154` (Free direct-activation, no Stripe), `webhooks.rs:288` (first paid sub / upgrade-from-Free, via `customer.subscription.created/updated`) | `plan`, `included_networks`, `included_seats`, `mrr_amount_cents`, `is_trialing`, `next_renewal_at` |
| `trial_started` | `webhooks.rs:312` (sub enters `trialing`) | `plan`, `trial_end`, `trial_days` |
| `trial_will_end` | `webhooks.rs:571` (`customer.subscription.trial_will_end`, ~3d before expiry) | `plan`, `has_payment_method` |
| `trial_ended` | `webhooks.rs:330` (trialing→active conversion) | `plan`, `converted` (**always `true`** — see note), `next_renewal_at` |
| `plan_changed` | `webhooks.rs:388` (tier switch on an active sub) | `from`, `to`, `is_downgrade`, `next_renewal_at` |
| `stripe_customer_created` | `checkout.rs:511` (first Stripe customer for the org) | `customer_id` |

> **`trial_ended.converted` is only ever `true`.** A trial that is *not*
> converted (no payment method when the trial expires) is not represented by
> `trial_ended { converted: false }` — that payload is never emitted. Instead
> Stripe deletes the unpaid subscription and we emit **`subscription_cancelled`
> with `was_trialing = true`** (see §3). Bucket unconverted trials there, not on
> `trial_ended`.

### Churn / cancellation

| Event (`name`) | Trigger | Key `metadata` properties |
|---|---|---|
| `cancellation_initiated` | `webhooks.rs:199` (first cancel-scheduled webhook; `cancel_at` set) | `reason_code`, `stripe_feedback`, `stripe_reason`, `comment`, `save_offer_shown` (array), `save_offer_redeemed`, `planned_period_end` |
| `cancellation_feedback_provided` | `webhooks.rs:223` (feedback on the same webhook) / `:243` (follow-up Portal webhook) | `stripe_feedback`, `stripe_reason`, `comment` |
| `subscription_cancelled` | `webhooks.rs:846` (`customer.subscription.deleted`, at period end) | `plan` (the plan being cancelled), `reason_code`, `stripe_feedback`, `stripe_reason`, `internal_reason`, `comment`, `period_end`, `was_trialing`, `mrr_amount_cents`, `tenure_days` |
| `reactivated` | `webhooks.rs:506` (pending cancellation cleared; `cancel_at` → null) | `trialing`, `next_renewal_at` |

> **Cancellation is two-phase.** `cancellation_initiated` is *intent* (scheduled,
> user keeps the plan until `planned_period_end`); `subscription_cancelled` is
> *realized* churn at period end. A `reactivated` between the two is a save.

### Retention levers (save-offer / pause / trial-extend)

| Event (`name`) | Trigger | Key `metadata` properties |
|---|---|---|
| `discount_applied` | `lifecycle.rs:585` (save-offer discount accepted) | `percent_off`, `expires_at` |
| `paused` | `webhooks.rs:419` (sub `pause_collection` set) | `plan`, `duration_days`, `resumes_at` |
| `resumed` | `webhooks.rs:462` (pause cleared) | `was_early` (always `false` from the webhook — SDK doesn't surface manual-vs-auto resume) |
| `trial_extended` | `webhooks.rs:484` (one-time +7d trial extension) | `days_added`, `new_trial_end` |

### Payments

| Event (`name`) | Trigger | Key `metadata` properties |
|---|---|---|
| `payment_succeeded` | `lifecycle.rs:643` (every `invoice.paid`, incl. $0 trial-setup invoice) | `invoice` (object) |
| `payment_failed` | `plan_changes.rs:430` (`invoice.payment_failed`) | `invoice_id`, `amount_cents`, `plan`, `attempt_count` |
| `payment_action_required` | `plan_changes.rs:473` (`invoice.payment_action_required`, 3DS/SCA) | `invoice_id`, `hosted_invoice_url` |
| `payment_recovered` | `lifecycle.rs:613` (`invoice.paid` while previously `past_due`) | `invoice_id`, `amount_cents`, `plan`, `attempt_count`, `next_renewal_at` |
| `payment_method_added` | `webhooks.rs:613` (`payment_method.attached`) — sole emitter | *(none)* |
| `payment_method_removed` | `webhooks.rs:650` (last `payment_method.detached`) | *(none)* |

> `payment_method_added` fires once per add-card action (the
> `payment_method.attached` webhook is the single emitter), so it can be counted
> as a distinct action without de-duping.

### Feature limits (gating telemetry, not subscription lifecycle)

| Event (`name`) | Trigger | Key `metadata` properties |
|---|---|---|
| `feature_limit_hit` | `auth/middleware/features.rs:196` & `:246`, `hosts/handlers.rs:402`, `invites/handlers.rs:115`, `daemons/service/processing.rs:571` | `limit_type`, `current_count`, `limit`, `plan`, `source` |

---

## 3. Conversion vs downgrade-to-Free — the discriminator (READ THIS FIRST)

Do **not** conflate *trial→paid conversion* with *trial→Free* (or any landing on
Free). Bucket each outcome by the rules below. All keys are `metadata.*`.

| Outcome | Event `name` | Discriminator |
|---|---|---|
| **New paid plan (conversion)** | `checkout_completed` | `metadata.mrr_amount_cents > 0` (equivalently `metadata.plan.type != "Free"`). `metadata.is_trialing` tells you whether it converts immediately or starts a trial-with-card. |
| **Trial→paid conversion** | `trial_ended` | `metadata.converted == true` (the only value emitted). Trialing sub transitioned to active. |
| **Free signup (NOT a conversion)** | `checkout_completed` | `metadata.plan.type == "Free"` AND `metadata.mrr_amount_cents == 0` AND `metadata.next_renewal_at == null`. Free is activated directly with no Stripe subscription. |
| **Trial expired unpaid → Free** | `subscription_cancelled` | `metadata.was_trialing == true`. (There is **no** `trial_ended{converted:false}` event.) |
| **Paid → Free (active cancellation)** | `subscription_cancelled` | `metadata.was_trialing == false`. Optionally preceded by `cancellation_initiated`. |
| **Paid → paid tier change** | `plan_changed` | `metadata.is_downgrade == false` (upgrade) or `true` (downgrade to a lower paid tier or to Free). `from`/`to` carry the plans. |

**One-line rule for the agent:**
- *Conversion* = `checkout_completed` with `metadata.mrr_amount_cents > 0`, **or** `trial_ended` with `metadata.converted == true`.
- *Downgrade-to-Free* = `subscription_cancelled` (split trial vs paid on `metadata.was_trialing`). Never count `checkout_completed{plan.type:"Free"}` as conversion.

---

## 4. Recommended analyses

For each: the events/properties to use and the funnel/cohort shape.

### 4.1 Trial → paid conversion
- **Funnel:** `trial_started` → `trial_ended` (`metadata.converted == true`).
- **Cohort** by `metadata.plan.type` (entry plan) and by trial source
  (`checkout_started.has_trial`).
- **Conversion rate** = users with `trial_ended{converted:true}` ÷ users with
  `trial_started`, windowed to `trial_started.trial_end + grace`.
- **Non-conversion** = `trial_started` followed by `subscription_cancelled`
  with `was_trialing == true` (NOT `trial_ended`). Exclude these from the
  numerator.

### 4.2 Churn / cancellation
- **Intent→realized funnel:** `cancellation_initiated` → `subscription_cancelled`,
  with `reactivated` (before `planned_period_end`) as the save/exit branch.
- **Save rate** = `reactivated` ÷ `cancellation_initiated`.
- **Churn cohorts/breakdowns:** `subscription_cancelled.reason_code`,
  `.stripe_feedback`, `.was_trialing`, `.tenure_days` (banded), and
  `.mrr_amount_cents` (revenue churn). Cohort by `metadata.plan.type`.
- **Voluntary vs involuntary:** voluntary churn has a preceding
  `cancellation_initiated`; involuntary (dunning) churn correlates with
  `payment_failed` / `payment_action_required` without `payment_recovered`.

### 4.3 Discount (save-offer) usage & impact
- **Usage:** `discount_applied` (`percent_off`, `expires_at`). Tie to the
  cancel flow via `cancellation_initiated.save_offer_shown` (was the discount
  offered) and `.save_offer_redeemed`.
- **Impact on retention:** compare subsequent `subscription_cancelled` rate of
  users with `discount_applied` vs users shown a discount
  (`save_offer_shown` contains the discount offer) who did not take it. Watch for
  delayed churn after `expires_at`.

### 4.4 Pause usage & impact
- **Usage funnel:** `paused` (`duration_days`, `resumes_at`) → `resumed`.
- **Resume rate** = `resumed` ÷ `paused`; **post-resume churn** =
  `subscription_cancelled` within N days after `resumed`.
- Pause as a save lever: `paused` reachable from `cancellation_initiated`
  (`save_offer_shown` containing the pause offer) — compare retention of
  pause-takers vs full-cancellers. `resumed.was_early` is not reliable (always
  `false` from the webhook); do not analyze it.

### 4.5 Trial-extend usage & impact
- **Usage:** `trial_extended` (`days_added`, `new_trial_end`) — one-time per org.
- **Conversion lift:** conversion rate (per §4.1) of trials with a
  `trial_extended` event vs trials without, cohorted by entry
  `metadata.plan.type`.

### 4.6 Revenue / dunning (supporting)
- MRR movement from `checkout_completed.mrr_amount_cents`,
  `subscription_cancelled.mrr_amount_cents`, `plan_changed` (`from`/`to`).
- Dunning recovery funnel: `payment_failed` → `payment_recovered` (recovered) vs
  `payment_failed` → `subscription_cancelled` (lost). `payment_succeeded` fires
  on every paid invoice (including the $0 trial-setup invoice) — use it for
  invoice volume, not as a conversion signal.

---

## 5. Coverage notes & data-quality caveats

- **Exhaustive forwarding.** As of this audit, all 22 `BillingOperation`
  variants reach PostHog. The previously-missing `paused`, `resumed`,
  `trial_extended`, `discount_applied`, `reactivated`, `payment_succeeded`,
  `payment_method_added`, `payment_method_removed` are now forwarded — the
  §4.3/4.4/4.5 analyses are unblocked from this change forward (no backfill of
  historical events).
- **`plan_type` person/group property on downgrade-to-Free.** On
  `subscription_cancelled` (and an unconverted `trial_ended`), the person/group
  `plan_type` is set to the *resulting* plan — `Free` — not the outgoing paid
  plan, so plan cohorts reflect the post-churn state correctly. (The event's
  `metadata.plan` still carries the cancelled plan for revenue-churn analysis.)
- **Attribution.** Webhook-driven events (most lifecycle events) are emitted by
  `system`/owner attribution; `user_id`/`email` may be absent. Use
  `organization_id` (and the `organization` group) as the stable join key, not
  `user_id`.
- **Idempotency / duplicates.** Billing events are single-emission per action.
  The cancellation pair (`cancellation_initiated` + `cancellation_feedback_provided`)
  models Stripe's two-webhook reality and is expected, not a duplicate.
