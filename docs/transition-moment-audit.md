# Transition-Moment Audit

> **Status as of 2026-06.** This is the pre-remediation diagnostic baseline (as of v0.16.2). ~70% of its HIGH/MED findings shipped via `feat/phase5-subscription-mechanics`, `feat/phase5-trial-ui`, `audit/banner-conditions-and-payment-prompt`, and `fix/billing-tab-ux-polish`. The as-built record is `docs/phase5-spec.md`; the telemetry findings are tracked in `docs/telemetry-gap-backlog.md`. Full audit in `DOCS_AUDIT_2026-06.md`.
>
> **RESOLVED examples:** in-app trial urgency ramp; wired `PAYMENT_METHOD_ADDED` email; post-Stripe confirmation banner; Stripe `cancellation_details` capture (`billing/service.rs:2785`, `74ac498cd`); enriched cancel events; in-app cancel flow with reason capture; `payment_recovered` email; stored `next_renewal_at` column.
>
> **STILL OPEN (no successor spec):** (a) DaemonPoll plan-gating — `daemon.base.standby` is set only by the 30-day inactivity sweep (`daemons/service.rs:2001`), not a plan gate; the `DaemonStandby` error copy (`error_codes.rs:300-301`) is still misleading; (b) `PLAN_CHANGED_BODY` feature-delta enumeration + in-product "what changed" surface; (c) `downgraded_at`/`cancelled_at` columns (only `next_renewal_at` was added).

Diagnostic of three billing transitions: (a) trial → paid (add-a-card), (b) cancel, (c) downgrade-to-free. Scanopy has low trial→paid conversion and 80%+ paid churn; this audit reports what happens today, what's silent, and what the current telemetry can/can't answer. No fixes proposed — that's a downstream pass.

All findings cite file:line. Audited against `dev` branch.

---

## 3a. Add-a-card moment (trial → paid)

### Current state

**Checkout mechanism.** Stripe Checkout (full-page redirect) for paid plans (`backend/src/server/billing/handlers.rs:96–200`). For brand-new trial orgs, server-side direct subscription creation skips Checkout. Paid path = 2 context switches: Scanopy → Stripe Checkout → success URL.

**Trial emails** — all webhook-driven, no cron:
- `TRIAL_STARTED_BODY` (`templates.rs:126–143`, fired from `service.rs:1043–1076`) — "No credit card required during the trial — add a payment method anytime"
- `TRIAL_ENDING_BODY_NO_PAYMENT` at T-3d (`templates.rs:147–164`) — urgency-leading: "If no payment method is added, your account will be downgraded to the Free plan"
- `TRIAL_ENDING_BODY_HAS_PAYMENT` at T-3d, payment-on-file (`templates.rs:166–182`) — reassurance: "You'll be billed {base_price}* for your {plan_name}…"
- `TRIAL_EXPIRED_BODY` at T+0 lapse (`templates.rs:186–202`)
- `TRIAL_CONVERTED_BODY` at T+0 success (`templates.rs:277–293`)
- `PAYMENT_METHOD_ADDED_BODY` (`templates.rs:265–273`) — template defined; **send function never called** (grep for `send_payment_method_added_email` returns 0 hits). User who adds a card mid-trial gets no acknowledgement. **[RESOLVED 2026-06]** — email is now wired.

**In-app trial surfaces:**
- `BillingTab.svelte:171–197` — single amber `InfoCard` shown when `plan_status === 'trialing' && !hasPaymentMethod`. Copy: "Trial ends in X days ({date})" + "Add Payment Method" button. Always-on while trialing-without-card; not time-gated to ramp urgency. **[RESOLVED 2026-06]** — in-app urgency ramp shipped.
- `Sidebar.svelte:748–762` — persistent amber "Upgrade" item at sidebar bottom when `isFreePlan && isOwner && isBillingEnabled`. (Free only, not shown during trial.)
- `BillingTab.svelte:345–353` — `InlineInfo` with `settings_billing_trialActive`: "Your trial is active. You won't be charged until your trial ends." (settings-only)

**First-invoice amount.** `BillingPlanForm.svelte:132–136` computes a local estimate for the simulator. The backend does not return a confirmed pre-checkout charge; the real total (base + seats + networks) first appears on Stripe's hosted page.

**Value recap.** None. No surface quantifies what the trial delivered (hosts discovered, services mapped, credentials stored).

**Middle paths.** None. Trial duration is fixed in `plans.rs`. No pause, no extend, no soft-downgrade-with-restore. Binary pay-or-lose.

**Telemetry coverage:**
- `trial_started` PostHog event with `plan_name`, `is_commercial`, `trial_end_date`, `trial_days`, `org_id` (`posthog/subscriber.rs:223–231`, fired from `service.rs:1046`)
- `trial_will_end` at T-3d with `has_payment_method` (`service.rs:1584`)
- `trial_ended` with `converted: bool` (`service.rs:1085, 1186`)
- `checkout_started` (`service.rs:481, 617`) and `checkout_completed` (`service.rs:535, 1024`); the latter carries `included_networks`, `included_seats`
- Frontend `upgrade_button_clicked` (`trigger-upgrade.ts:30`) with `feature` + `source`; `billing_completed` (`AppShell.svelte`) on Stripe-return with `plan`, `amount`, `plan_status`

### Friction points

- The only time-varying in-app trial signal is the day count in `BillingTab.svelte` InfoCard. No dashboard/sidebar countdown, no pre-expiry modal, no tone shift. All escalation lives in the two T-3d email variants.
- The two T-3d emails diverge sharply (urgency vs reassurance) but no in-app mirror exists for either tone.
- Pricing pages show base rate; the first place the user sees the real total with line items is Stripe Checkout.
- No post-Stripe confirmation moment in product. `plan_status` silently flips `trialing → active`. Frontend fires `billing_completed` but doesn't render a confirmation surface.
- `PAYMENT_METHOD_ADDED_BODY` is orphaned — adding a card mid-trial produces zero ack.

### Silent-fail surfaces

- Trial-start email delivery failure → no in-app fallback confirms the trial started; the `BillingTab` card is settings-only.
- Payment-method-added action: no email, no toast, no confirmation surface in product.
- Trial → paid silent transition: `TRIAL_CONVERTED_BODY` fires but the in-product moment is just a status flip.

### Measurement gaps

- The deep-link `/?modal=settings&tab=billing` (used by both trial emails and the in-app InfoCard CTA) does not carry `utm_source` or equivalent. `upgrade_button_clicked` carries a `source` field; email clicks have no equivalent. Cross-channel attribution is partial.
- No PostHog event on email send/queue; Brevo provider-side open/click data isn't synced. Can't answer "did the T-3d email open?"
- `checkout_completed` carries `plan_name`, `is_commercial`, `included_networks`, `included_seats` but **not the actual charge amount in cents**. Can't measure "saw-amount → abandoned" vs "saw-amount → converted" without a separate Stripe-side join.
- `BillingTab` InfoCard has no impression / dismiss / CTA-click event. The single most prominent in-app trial surface is blind in analytics.
- 402 middleware (`billing.rs:120–131`) returns one generic message. Plan-status branches in code (`canceled` vs other) but only canceled gets a distinct copy — analytics can't easily slice failure reasons.

### Severity

- **HIGH** — No in-app urgency ramp between T-14d and T+0 (static InfoCard + email only). **[RESOLVED 2026-06]**
- **HIGH** — First-invoice amount not shown before Stripe redirect; no Scanopy-confirmed pre-checkout total. (Still open — only a client-side estimate shipped; see absorbed open items below.)
- **HIGH** — No pause / extend / soft-downgrade option.
- **MED** — `PAYMENT_METHOD_ADDED_BODY` is orphaned. **[RESOLVED 2026-06]**
- **MED** — `checkout_completed` event lacks the actual charge amount.
- **MED** — No conversion-source attribution from email clicks.
- **MED** — No value recap surface. (Built then reverted, `76c748e8a`; see absorbed open items below.)
- **MED** — No post-Stripe confirmation moment in product. **[RESOLVED 2026-06]**
- **LOW** — `BillingTab` InfoCard is silent in analytics.

---

## 3b. Cancel moment (paid → churned)

### Current state

**Cancel UI.** 100% off-platform. `BillingTab.svelte:138–144` "Manage Subscription" → `useCustomerPortalMutation` (`queries.ts:56–71`) → `POST /api/billing/portal` → `create_portal_session` (`service.rs:1693–1722`). The session is built with only `CustomerId` and `return_url`; no flow types, save offers, pause flows, or feedback prompts configured server-side. Stripe Portal behavior is whatever the Stripe dashboard is configured to show (cannot be inspected from this repo).

**`subscription_cancelled` PostHog event** (`subscriber.rs:223–231`, fired from `service.rs:1608`) payload:
```json
{ "subscription_status": "cancelled", "plan_name": "...", "org_id": "..." }
```

**`plan_changed` PostHog event also fires on cancel→Free** (`service.rs:1638`) with `old_plan`, `new_plan: "Free"`, `is_downgrade: true`, `plan_status: "active"`. The `is_downgrade` flag distinguishes upgrades from downgrades within the same event.

**`trial_ended` fires with `converted: false`** (`service.rs:1186`) when a trial lapses without payment, allowing trial-lapse to be distinguished from paid-cancel **by event-stream join** (matching `trial_ended converted=false` near a `subscription_cancelled` for the same `org_id`).

**Webhook handler.** `handle_subscription_deleted` (`service.rs:1425`) receives Stripe's `customer.subscription.deleted`. Stripe populates `subscription.cancellation_details.{reason, feedback, comment}` when a user selects reasons in the Portal. **Our handler does not deserialize this field.** The only metadata read is `cancel_reason == "upgrade"` (L1435) to skip auto-Free for plan changes. **[RESOLVED 2026-06]** — `cancellation_details` now captured (`billing/service.rs:2785`, `74ac498cd`).

**Org schema** (`migrations/20251110181948_orgs-billing.sql`): `plan`, `plan_status`, `stripe_customer_id`, `trial_end_date`, `has_payment_method`, `created_at`. No `cancelled_at`, `period_end`, `cancel_reason`, `cancellation_feedback`. No dedicated cancellations table.

**Post-cancel email.** `SUBSCRIPTION_CANCELLED_BODY` (`templates.rs:245–261`) fires from `process_subscription_deleted_side_effects` (`service.rs:1598–1604`). Subject: "Your Scanopy Subscription Has Been Cancelled". Body: "Your Scanopy subscription has been cancelled. Your account has been moved to the Free plan. You can continue using Scanopy with up to 25 hosts and manual discovery. Resubscribe anytime from your Settings page." No `period_end` date, no retention policy, no export/backup CTA.

**Dunning coverage** — present and complete on the fail side, partial on recovery:
- `PAYMENT_FAILED_BODY` (`templates.rs:451–467`) fires from `handle_invoice_payment_failed` (`service.rs:1941–1994`)
- `PAYMENT_ACTION_REQUIRED_BODY` (`templates.rs:471–486`) fires from `handle_invoice_payment_action_required` (`service.rs:1996–2048`) — 3DS/SCA
- `payment_failed` and `payment_action_required` PostHog events fire (only `org_id` payload — no plan, no invoice metadata)
- `payment_recovered` PostHog event fires (`service.rs:2089`) with only `org_id`
- **No recovery email** — the `PaymentRecovered` event has no template; user whose card recovers gets silence. **[RESOLVED 2026-06]** — `payment_recovered` email shipped.

**In-app post-cancel signal.** `InlineWarning` in `BillingTab.svelte:351` when `plan_status === 'pending_cancellation'`, copy `settings_billing_downgrade_pending`: "Your plan will change to Free at the end of your current billing cycle. To cancel this change, upgrade to a paid plan or manage your subscription in the billing portal." Settings-only.

**Save offers in-product.** None. `schedule_downgrade()` (`service.rs:~1751`, sets `cancel_at_period_end: true`) exists but is only invoked from the "choose Free" path, not as a mid-cancel retention lever.

### Friction points

- No Scanopy-side "Are you sure?" gate. A misclick on "Manage Subscription" lands the user in a third-party portal one click later.
- Post-cancel email says "Resubscribe anytime" but doesn't name `period_end` (we don't store it), data retention, or what will stop working.
- `pending_cancellation` banner lives only in Settings → Billing; vanishes when the user closes that tab.
- `payment_recovered` has no email — silent recovery.

### Silent-fail surfaces

- **Stripe `cancellation_details.feedback` is silently discarded on ingest.** When a user selects a reason in the Portal, Stripe stores it; our handler does not deserialize it. The data never enters our system. No warning, no error log — just lost. **[RESOLVED 2026-06]** — now captured.
- Voluntary cancel, dunning failure, trial lapse, and admin cancel all funnel through `customer.subscription.deleted` → same `subscription_cancelled` event. They are distinguishable in principle (trial-lapse via the separate `trial_ended converted=false` event firing close in time), but there's no `cancel_type` field on `subscription_cancelled` itself — joining events is required.
- `payment_recovered` event fires but neither customer nor internal-ops gets a recovery notification.

### Measurement gaps

- **Cannot answer**: "Top cancel reasons by plan and tenure." Reason isn't on the event, isn't in DB, Stripe Portal feedback is silently dropped.
- **Cannot answer cleanly**: "Voluntary vs involuntary churn split." Possible by joining `subscription_cancelled` with `payment_failed` proximity, but no direct discriminant on the cancel event.
- **Cannot answer simply**: "Time-to-cancel distribution." No `cancelled_at` column. `subscription_cancelled` event timestamp is webhook-arrival, not user-click — usable proxy if joined to `created_at`, but requires PostHog joins not SQL.
- **Cannot answer simply**: "Did cancellers re-subscribe within 30/60/90d?" No durable link between a cancel and a subsequent `subscription_created` / `checkout_completed` for the same org. Possible by org_id join in PostHog but with no flag indicating "previously paying."
- **Cannot answer**: "How much runway did cancellers have left in their billing cycle?" No stored `period_end`.
- **Cannot answer**: "Save-offer acceptance rate." Offers don't exist in product; Stripe Portal config is dashboard-side and not synced.
- `subscription_cancelled` payload is the thinnest of all 11 backend events — three fields. A `was_trialing` flag exists locally in `process_subscription_deleted_side_effects` (`service.rs:1606`) to choose between cancellation vs trial-expired email but isn't attached to the event.

### Severity

- **HIGH** — Stripe `cancellation_details` silently discarded on webhook ingest. **[RESOLVED 2026-06]**
- **HIGH** — `subscription_cancelled` event lacks reason / tenure / `period_end` / `was_trialing` — primary slicing blocker for the 80% number. **[RESOLVED 2026-06]** — cancel events enriched.
- **HIGH** — No `cancelled_at` / `period_end` columns — most cohort analyses require event-stream joins instead of simple SQL. (Partial — only `next_renewal_at` was added; `cancelled_at` still open.)
- **HIGH** — Cancel flow 100% off-platform; no Scanopy-controlled reason capture or save surface. **[RESOLVED 2026-06]** — in-app cancel flow with reason capture shipped.
- **MED** — `payment_recovered` has no customer email and no internal alert. **[RESOLVED 2026-06]** — `payment_recovered` email shipped.
- **MED** — Voluntary vs involuntary cancels not directly discriminable on `subscription_cancelled`.
- **MED** — Post-cancel email omits `period_end`, retention policy, re-activation data preservation.
- **LOW** — `pending_cancellation` banner scoped to Settings only.

---

## 3c. Downgrade moment (trial-without-card OR cancel → free)

### Current state — Free feature flags + actual enforcement

`billing/types/base.rs:465–489` instantiates Free; enforcement traced via `auth/middleware/features.rs` and `auth/middleware/auth.rs`.

| Flag | Free | Server enforcement |
|---|---|---|
| `network_mapping` | true | core, no gate |
| `png_export` | true | metadata only — frontend renders (`shares/handlers.rs:523–531`) |
| `csv_export` | true | metadata only |
| `discovery_integrations` | **true** | always-on across all plans (Free has it true; flag is not a paid discriminator) |
| `share_views` | false | `RequireFeature<ShareViewsFeature>` on `create_share` (`features.rs:157–170`) — blocks creating new shares; **render route does NOT check this flag** (only `embeds`) — existing shares keep working post-downgrade |
| `embeds` | false | inline check in `get_share_topology` returns 402 "Embed access requires a plan with embeds feature" (`shares/handlers.rs:393–401`) |
| `api_access` | false | `auth.rs:426–444` returns 402 "Your plan does not include api access" at API-key auth time — previously-issued API keys on a downgraded org are blocked at auth, not at the route. Plus `RequireFeature<ApiKeyFeature>` on key CRUD handlers. |
| `webhooks` | false | placeholder (`features.rs` `is_coming_soon()` true); not implemented |
| `audit_logs` | false | placeholder; not implemented |
| `scheduled_discovery` | false | scheduler early-return `discovery/service.rs:619–636` — silently skips and logs |
| `svg_export` / `pdf_export` / `mermaid_export` / `html_export` / `confluence_export` | false | metadata only — frontend-rendered; backend returns the flags in `ExportFeatures` (`shares/handlers.rs:523–531`) and the UI handles disabled state |
| `remove_created_with` | false | metadata only — frontend-rendered watermark |
| `invite_users` (computed) | gated | `RequireFeature<InviteUsersFeature>` on `create_invite` (`features.rs:127–140`) |
| Network creation quota | 1 included | `RequireFeature<CreateNetworkFeature>` (`features.rs:172–222`) — denies create at quota and emits `feature_limit_hit` |
| Host creation cap | 25 included | inline check in `hosts/handlers.rs:373–410` — emits `feature_limit_hit` and returns 403 `BillingHostLimitReached` |
| Seat invite cap | 1 included | check in `invites/handlers.rs:113` — emits `feature_limit_hit` |
| DaemonPoll mode | gated by error message | `error_codes.rs:300–301` defines `DaemonStandby` "Your plan does not support DaemonPoll mode. The daemon is on standby. Upgrade your plan and restart the daemon to resume." `daemons/handlers.rs:563–574` returns this error if `daemon.base.standby` is true. **No code path was located that sets `daemon.base.standby = true` based on a plan check.** Implementation appears incomplete. |

The billing middleware (`billing.rs:36–118`) **exempts Free entirely** (lines 97–104, returns `next.run(request).await`). Free-tier orgs pass through the billing middleware without any plan-status check; per-feature gating happens at the auth/extractor layer instead.

### Current state — In-product upgrade affordances

| Surface | Component | Behavior |
|---|---|---|
| Export dropdown badges | `ExportModal.svelte:172–189` | Gated formats show yellow "Upgrade" tag; clicking a disabled option calls `triggerUpgrade(feature, source:'export_modal')` which fires `upgrade_button_clicked` PostHog event + opens BillingPlanModal |
| Discovery card schedule field | `DiscoveryScheduledCard.svelte:134–138` | On Free, schedule string replaced with `discovery_schedulePausedFreePlan`: "Paused — upgrade to resume scheduled runs" |
| Discovery form "Scheduled" option | `DiscoveryDetailsForm.svelte:71–87, 174–198` | Yellow "Upgrade" badge + `ArrowUpCircle`; `onDisabledClick` → `triggerUpgrade(feature:'scheduled_discovery', source:'discovery_form')` |
| Embed code | `ShareConfigPanel.svelte:81–83, 199–212` | Hides embed code when `embeds: false`; renders `InlineInfo` ("Embeds require an upgraded plan" / "Upgrade your plan to embed this share on external websites.") + `UpgradeButton feature="embeds"` |
| Hosts tab at-limit | `HostTab.svelte:81–141, 361–382` | Shows `count/limit` in amber; at limit hides "Create" and shows `UpgradeButton feature="hosts"`; near-limit (within 5) shows both |
| Plan Usage home card | `PlanUsage.svelte:66–89` | Progress bars; `UpgradeButton feature="plan_usage"` in header when any resource ≥80% and not overage-priced |
| Sidebar upgrade button | `Sidebar.svelte:748–762` | Persistent amber "Upgrade" item at sidebar bottom when `isFreePlan && isOwner && isBillingEnabled`; `triggerUpgrade(source:'sidebar')` |
| BillingTab status banners | `BillingTab.svelte:345–353` | `InlineInfo` (trialing) / `InlineDanger` (past_due) / `InlineWarning` (canceled / pending_cancellation) |
| View Plans CTA | `BillingTab.svelte:368–392` | Copy shifts: `settings_billing_upgradePlan` ("Upgrade your plan" / "Get scheduled discovery, DaemonPoll mode, and more hosts") vs `settings_billing_changePlan` |
| API error toast | `client.ts:302` | Translates 402 error code via i18n (`errors_billing_subscription_required`, `errors_billing_host_limit_reached`, `errors_billing_feature_not_available`); 10s amber toast |

**Surfaces with no UI affordance** (gated on backend, no proactive UI signal):
- Network limit at-limit / near-limit state on the Networks tab — no badge or counter (only the BillingTab progress bar in settings)
- Seat invite at-limit state on Settings > Members — no badge
- API key creation when `api_access: false` — backend returns 402 from `RequireFeature<ApiKeyFeature>`, but no proactive disabled state on the create-key form was located
- DaemonPoll mode — copy mentions it but no in-product affordance, and the gating logic itself is incomplete (see above)

### Current state — Limit-warning emails

`templates.rs:411–447`, `traits.rs:974–995`, fired from `subscriber.rs:79` on host/network/user create events:
- `PLAN_LIMIT_APPROACHING_BODY` at 80%
- `PLAN_LIMIT_REACHED_BODY` at 100%

Triggered on entity-create events; **not** on the downgrade transition itself. Implication: an org that downgrades and instantly becomes 100% over-cap on an existing resource gets no email at the moment of downgrade — only at the next attempted creation.

### Current state — Communication at the transition itself

- **Email**: `PLAN_CHANGED_BODY` (`templates.rs:206–221`) fires on every plan transition — paid→paid, paid→Free, even Free→paid. Body: "Your Scanopy plan has been changed to {plan_name}. The change takes effect immediately." Single sentence. Does not enumerate the features that stop or start working.
- **Subscription cancelled→Free** also fires `SUBSCRIPTION_CANCELLED_BODY` (see 3b).
- **In-app**: `pending_cancellation` `InlineWarning` in BillingTab + persistent sidebar "Upgrade" button once on Free. No dedicated "welcome to Free" / "here's what changed" surface.

### Current state — Telemetry on downgrade

- `plan_changed` PostHog event with `old_plan`, `new_plan`, `is_downgrade: bool`, `org_id`, `plan_status` (`subscriber.rs:223–231`, fired from `service.rs:1262, 1638`). Allows direct query "rows where `is_downgrade=true`" — answers "how many downgrades and from what plan."
- `feature_limit_hit` event with `limit_type` ("hosts"/"networks"/"seats"), `current_count`, `limit`, `plan_type`, `source` (fired from `hosts/handlers.rs:390`, `features.rs:198`, `invites/handlers.rs:113`, `daemons/service.rs:1144`)
- Frontend `upgrade_button_clicked` with `feature` and `source` — fires on every UI upgrade CTA click

### What actually breaks on downgrade — verified

| Surface | Behavior | Verified at |
|---|---|---|
| Scheduled discovery | scheduler early-returns; UI shows "Paused — upgrade to resume" in card | `discovery/service.rs:619–636`, `DiscoveryScheduledCard.svelte:134–138` |
| Existing shares | **continue working** — render route does not check `share_views`; only embed render is gated | `shares/handlers.rs:374–586` (no `share_views` check at render) |
| Creating new shares | blocked at handler — 402 "Your plan does not include sharing. Upgrade to share live network diagrams." | `RequireFeature<ShareViewsFeature>` on `create_share`, `features.rs:157–170` |
| Embeds (existing share) | 402 "Embed access requires a plan with embeds feature" when `?embed=true` | `shares/handlers.rs:393–401` |
| API keys (existing) | **fail at auth time** — 402 "Your plan does not include api access" | `auth.rs:426–444` |
| Creating new API keys | 402 from `RequireFeature<ApiKeyFeature>` | `features.rs:142–154` |
| Inviting members | 402 "Your plan does not include inviting users" | `RequireFeature<InviteUsersFeature>` on `create_invite`, `features.rs:127–140` |
| Creating networks at quota | 403 "Network limit reached…" + emits `feature_limit_hit` | `features.rs:172–222` |
| Creating hosts at cap | 403 `BillingHostLimitReached` + emits `feature_limit_hit` | `hosts/handlers.rs:373–410` |
| Updating / deleting existing hosts (over-cap or under) | **allowed** | `hosts/handlers.rs:521–591` (no cap check), `hosts/handlers.rs:790–815` (only daemon-association guard) |
| Listing hosts (over-cap org) | **all hosts returned** — no list-side filter | `hosts/handlers.rs:205–263` |
| Exports (svg/pdf/mermaid/html/confluence) | **server returns success; client gates** | `topology/handlers.rs:852, 899` exports take only `Authorized<Viewer>`; `shares/handlers.rs:523–531` returns `ExportFeatures` to client; UI uses these to disable / badge |
| Watermark | client-rendered based on `remove_created_with` flag in metadata | `shares/handlers.rs:530` |
| DaemonPoll mode | **enforcement code path not located** — `DaemonStandby` error exists but no code sets `standby=true` based on plan | `error_codes.rs:300`; gating gap |

### Friction points

- `PLAN_CHANGED_BODY` is one sentence and lists nothing. The downgraded user has to discover what stopped working by trying things.
- Rich set of UI upgrade CTAs (badges, disabled controls, persistent sidebar button), but they all open the same `BillingPlanModal` regardless of which gate triggered them. No deep-link from "you hit the host cap" to "here's the cheapest plan that includes more hosts."
- Network and seat caps have no tab-level UI affordance (only BillingTab progress bars and the toast on the failed action).
- DaemonPoll: copy advertises it; gating in code is incomplete; user could install a daemon expecting standby behavior that never actually triggers.

### Silent-fail surfaces

- `PaymentRecovered` event fires; no customer email.
- `PLAN_CHANGED_BODY` is the main user-facing downgrade signal and lists nothing concrete.
- **Existing share-render does not check `share_views`** — depending on intent, this is either a thoughtful "don't break links" choice or an enforcement gap. Worth flagging because it's not declared anywhere; a stakeholder reading the feature flags would expect existing shares to fail too.
- **DaemonPoll gating is incomplete** — error message exists but no code sets `daemon.standby` from a plan check.
- Plan-limit-approaching/reached emails fire on entity-create events, not on downgrade. An org pushed from over-cap by a downgrade gets no immediate email.

### Measurement gaps

- No `plan_downgraded` event distinct from `plan_changed`, but `plan_changed` carries `is_downgrade: bool` so this is filterable. **The actual gap is in dwell time and reactivation cohorts**: there's no `downgraded_at` column on `organizations`, so "how long did orgs stay on Free before re-upgrading?" requires PostHog event-stream joins instead of a simple SQL query.
- `feature_limit_hit` covers hosts/networks/seats — the three quantitative caps. There is **no event for clicking a paywalled UI element** (e.g. clicking a disabled SVG export, hitting the embed gate, clicking the sidebar Upgrade button generally). The frontend `upgrade_button_clicked` event is the closest signal, but it fires only after the user interacts with an upgrade CTA, not on the underlying gate hit. So we can answer "which features did users click upgrade on?" but not "which features did users try and bounce off without clicking upgrade?"
- `payment_failed` / `payment_recovered` events carry only `org_id` — no plan, no invoice ID, no amount. Can't size the dunning funnel by plan or amount.
- Trial-lapse (`trial_ended converted=false`) and paid-cancel (`subscription_cancelled` for non-trialing) are distinguishable by event-stream join, but `subscription_cancelled` itself has no `was_trialing` flag — joining is required.

### Severity

- **HIGH** — `PLAN_CHANGED_BODY` is a one-sentence template with no feature delta — primary downgrade signal is opaque.
- **HIGH** — No `downgraded_at` column on org — dwell time / reactivation cohorts are PostHog-only, not SQL.
- **HIGH** — DaemonPoll gating advertised in copy but not actually enforced in code (located).
- **MED** — Existing share-render does not check `share_views` — possibly intentional, but undeclared.
- **MED** — Network / seat / API key creation has no proactive UI affordance — user discovers the limit via 402 toast.
- **MED** — No "user clicked a paywalled UI element" event distinct from the post-click `upgrade_button_clicked` — can't measure passive-bounce on gates.
- **MED** — `PaymentRecovered` event has no customer email. **[RESOLVED 2026-06]**
- **MED** — All upgrade CTAs route to the same generic `BillingPlanModal` — no per-gate plan recommendation.
- **LOW** — `payment_failed` / `payment_recovered` events carry only `org_id`.

---

## Open items (absorbed from conversion-side-remediation-brief, 2026-06)

These residual gaps were carried over from the now-deleted `conversion-side-remediation-brief.md` and remain unaddressed:

- **Free-LANDED downgrade banner + contextual recovery affordances.** The existing no-payment banner targets the pre-downgrade card-missing window, NOT an org that has already landed on Free. There is no "restore access by adding a card" copy on the surfaces that just stopped working (the gated controls themselves), only the generic upgrade CTAs.
- **Authoritative Stripe upcoming-invoice preview before the redirect.** Only a client-side estimate shipped (`BillingPlanForm.svelte`); the backend still does not return a Stripe-confirmed upcoming-invoice total before the Checkout redirect. The trial value recap was built then reverted (`76c748e8a`) and remains unshipped.

---

## Cross-cutting summary — top 5 findings

1. **The 80% paid churn rate is not directly sliceable today.** `subscription_cancelled` is a 3-field event (`org_id`, `plan_name`, `subscription_status`). Stripe Portal `cancellation_details.feedback` is silently discarded by `handle_subscription_deleted` (`service.rs:1425`). No `cancelled_at` / `period_end` / `cancel_reason` columns. Trial-lapse vs paid-cancel is recoverable by joining `trial_ended converted=false` to `subscription_cancelled`; voluntary vs involuntary requires joining to `payment_failed` proximity. Largest measurement gap of the three moments. [3b]

2. **The downgrade email is one sentence; the transition has no in-product enumeration of what stops working.** `PLAN_CHANGED_BODY` ("Your Scanopy plan has been changed to {plan_name}. The change takes effect immediately.") is the primary signal. The UI then surfaces the consequences piecemeal, when the user happens to encounter a gated control. The system has the data to render a "here's what changes" surface (it knows old plan, new plan, all feature flags) and doesn't. [3c]

3. **DaemonPoll gating is advertised in copy but not enforced in code.** `error_codes.rs:300` defines `DaemonStandby` ("Your plan does not support DaemonPoll mode…"); `settings_billing_upgradePlanDescription` advertises DaemonPoll as an upgrade benefit. No code path was located that sets `daemon.base.standby = true` based on a plan feature check. Implementation appears incomplete. [3c]

4. **Between T-14d and T+0, the in-app trial signal is static.** Email coverage at T+0/T-3d/T+0 is reasonable. In-app, the only time-varying element is the day count in `BillingTab.svelte:171–197`. No dashboard banner, no sidebar pill, no pre-expiry modal, no tone shift. A user who skips email gets no in-product escalation. [3a]

5. **First-invoice amount is hidden until Stripe Checkout, and `checkout_completed` doesn't capture the actual charge amount either.** `BillingPlanForm.svelte:132–136` computes a local estimate; the backend never returns a confirmed pre-checkout total. The PostHog `checkout_completed` event carries `included_networks`, `included_seats`, `plan_name` but not the cents-amount charged. Both the user-facing pricing reveal and the analytics-side conversion-by-price are absent. [3a]
