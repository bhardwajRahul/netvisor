# Phase 5 design brief — conversion-side remediation

Working doc for the Phase 5 design pass. Kickoff input for the founder + Claude Code session that will produce the spec(s). Eight product changes from the Avenue 3 transition-moment audit, clustered into three bundles by surface area. Each bundle can ship independently or together — bundle choice is itself a design-pass decision.

---

## The problem

Avenue 3's audit (`docs/transition-moment-audit.md`) surfaced HIGH-severity friction at the three moments where users exit paid status or fail to enter it: add-a-card (trial → paid), cancel, and downgrade-to-free. The friction is documented; the mechanisms below address it. These reduce trial-to-paid drop and improve cancel attribution; they do **not** make paid users return week-over-week (that's stickiness, Phase 2).

This phase was originally framed as "open after Phase 2 design lands." That framing was wrong — Phase 1 telemetry measures impact, doesn't justify the work, and the audit already documented these as broken. Most items here have no hard dependency on Phase 1. Phase 5 can run in parallel with Phase 2.

## The mechanism — three bundles, eight items

### Bundle A — Trial-side (cluster: trial → paid conversion friction)

1. **In-app urgency ramp** through the trial timeline (T-7d / T-3d / T-1d / T+0). Today the only in-app trial signal is a static amber InfoCard in Settings → Billing, regardless of where in the trial timeline the user is. No dashboard countdown, no sidebar surface, no escalating tone, no pre-expiry modal. Email handles the cadence variation; the in-app surface is flat.
2. **First-invoice amount displayed before Stripe redirect.** Today the user sees range-based plan cards in `BillingPlanForm.svelte`, then jumps to Stripe Checkout to find the actual total (base + seats + networks). The user's seat/host/network counts are known to us — show the confirmed total on the Scanopy side.
3. **Trial value recap surface** — "Scanopy discovered X hosts, Y services during your trial; you'd lose this on {date}." Doesn't exist today. Could live in the BillingTab card, in a T-3d email, or both.
4. **Pause / extend / soft-downgrade middle path** beyond binary pay-or-lose. Three sub-options that don't have to all ship together: pause (data preserved, billing suspended, resume later), trial extend (+N days, self-serve, eligibility-gated), soft-downgrade-with-restore (drop to Free with a clear "restore full access by adding a card" affordance — also addresses Bundle C).
5. **Wire the orphaned `PAYMENT_METHOD_ADDED_BODY` email.** Template exists at `templates.rs:265`, send function never called. Adding a card mid-trial currently produces zero acknowledgement (no email, no toast, no in-product confirmation surface).
6. **Post-Stripe confirmation moment** in product. Today `plan_status` silently flips `trialing → active` after Stripe redirect; `billing_completed` event fires but no UI surface confirms the conversion to the user.

### Bundle B — Cancel-side (cluster: cancel-flow migration)

7. **In-app cancel flow** with reason capture + save offers. Today the "Manage Subscription" button hands off 100% to Stripe Portal — no Scanopy-side reason capture, no save offers, no flow control. Bundle B's biggest item; introduces a new modal/page surface and reason taxonomy.
8. **Name `period_end` and data retention** in the post-cancel email. Currently the email is generic ("Your account has been moved to the Free plan"); doesn't tell the user when access ends or what's retained.
9. **Recovery email for `payment_recovered`.** Event fires today (`service.rs:2089`), no email is sent. Silent recovery means a user who fixes their card never knows it worked until the next charge cycle.

### Bundle C — Downgrade-side (cluster: post-downgrade communication)

10. **Communicate the downgrade transition.** When trial ends without a card, or a paid plan cancels, scheduled discovery silently pauses, shares go dark, exports return 402, embeds and API tokens stop working. None of it is communicated. Add banner / modal / email at the transition moment — at minimum, an email; ideally an in-product recovery banner persistent until the user dismisses or upgrades.
11. **In-product recovery affordance** on the Free tier — "Restore full access by adding a card." Today the only path back is a generic pricing page link. Should be a contextual button on the surfaces that just stopped working (the disabled scheduled-discovery toggle, the disabled share-create button, etc.) plus a top-level banner for the first N days post-downgrade.

## Roles

- **Founder** owns architectural decisions (cancel flow shape, reason taxonomy, save offer types, pause/extend mechanics, recovery affordance placement) and UX direction.
- **Claude Code agents** do code investigations, draft option memos, implement once decisions are made. Worker agents spawn off worktrees per the project's coordinator/worker pattern.
- **End-user audience:** trialing users (Bundle A), paying customers contemplating cancel (Bundle B), downgraded users (Bundle C).

---

## Open architectural questions

### Q1. Cancel flow architecture (Bundle B item 7 — biggest decision in this phase)

The cancel flow is currently a single button → Stripe Portal redirect. Moving it in-app introduces several decisions that compound:

- **Modal vs page surface.** Modal (faster to ship, lower commitment, can layer over Settings → Billing) or dedicated page (richer flow, supports multi-step better, easier to A/B). Modal is probably right for v1.
- **Reason taxonomy.** Enum, free-text, or hybrid? Stripe has its own enum we *could* mirror but it's UX-shaped for them, not us. Candidate enum from the prior draft plan: `too_expensive` / `missing_feature` / `not_using_enough` / `better_alternative` / `tech_issues` / `pausing` / `other`. Free-text supplemental field captures detail.
- **Save offer types.** Which subset to ship initially? Candidates:
  - Discount (% off for N months) — Stripe coupon application
  - Pause (suspends billing, preserves data, resumes later) — couples with Bundle A item 4
  - Support handoff (open chat, schedule call, file ticket)
  - Plan downgrade-as-alternative (drop to a cheaper paid tier instead of canceling)
  - "Just pausing" → hand off to pause flow directly
- **Save offer triggering.** Always-shown vs reason-dependent? Reason-dependent is more respectful of explicit signals (don't show a discount to someone canceling because of a missing feature) but adds branching complexity.
- **Persistence of cancel state.** Phase 1's P0-1 + P0-2 already enrich the `subscription_cancelled` event with reason, plan, MRR, tenure — that handles analytics needs. The deferred typed `cancellations` table (P1-6) is *not* a prerequisite here unless product code needs to read cancel state (admin UI, automated comeback flows). Decide if save offer redemption logging needs a separate table.
- **Confirmation step.** Separate modal page or last step of same flow? Last step keeps it tight; separate page allows a richer "data retention / what stops working" disclosure surface.

### Q2. Pause / extend / soft-downgrade mechanics (Bundle A item 4)

Three sub-features that share a question shape but have different implementations:

- **Pause:** Stripe-native `pause_collection` (suspends invoice generation, keeps subscription "active") vs a custom Scanopy-side `plan_status` state. Stripe-native is simpler but inherits Stripe's behavior; custom gives flexibility (e.g., what features stay accessible during pause?).
- **Trial extend:** new trial issuance via Stripe API (push `trial_end`), or Scanopy-side override? Stripe-native is cleaner and keeps invoice generation consistent.
- **Soft-downgrade with restore:** is this distinct from existing free-tier downgrade, or just a UX overlay on top of it? If just UX, this is mostly a banner + recovery affordance (Bundle C item 11).

Plus eligibility rules for each — once per org? Once per N months? Plan-tier-gated? Founder call.

### Q3. Trial value recap mechanism (Bundle A item 3)

- **Surface placement:** in-app card on the BillingTab? T-3d email? Both? In-app dashboard widget?
- **What counts as recap-worthy:** `host_count`, `service_count`, scans run, shares created, networks discovered, daemons registered, time invested? Probably a small curated set (3–5 items) chosen for emotional weight, not a metric dump.
- **Computation:** lazy on render (cheap) or pre-computed at trial-end-2-days as part of the email-send job (more reliable for the email)?
- **Empty-state handling.** A user who barely used Scanopy during trial will see an embarrassing recap. Suppress? Show a different surface? Use this as an aha-moment intervention?

### Q4. Downgrade communication architecture (Bundle C)

- **Channel mix.** Email at downgrade is non-negotiable (the user may not be in-app when it happens). Beyond that: in-app banner, modal on next login, both?
- **Persistence model.** Time-bound (banner shows for N days post-downgrade) or until-dismissed? Until-dismissed is more aggressive, time-bound feels more respectful.
- **Per-feature-loss messaging vs single summary.** Listing every disabled feature is heavy but precise; a single summary with a "see what changed" link is lighter.
- **Recovery affordance placement (item 11).** Top-level banner only, or also contextual buttons on the surfaces that just disabled (scheduled-discovery toggle, share-create button, etc.)?

---

## Open UX questions

- Bundle A's urgency ramp: is the InfoCard the surface that escalates, or do new surfaces appear at T-3d / T-1d (e.g., a modal at T-1d)?
- Cancel modal step count: single step with all save offers in one view, or multi-step flow (reason → save offer → confirm)?
- Pause flow placement: standalone Settings option, or only available as a save offer during cancel?
- Trial extend: founder-discretion granted (manually triggered) vs self-serve via "pause for N more days" button?
- Recovery banner copy/tone — apologetic, neutral, opportunity-framing?

---

## Dependencies and prerequisites

**Phase 1 (telemetry) is NOT a hard prerequisite.** Telemetry measures impact, doesn't justify the work. Most Phase 5 items have no Phase 1 dependency at all. Bundles A, B, C can each ship parallel to Phase 1.

**Light dependencies that exist:**
- Bundle B item 8 (`period_end` in post-cancel email) needs `period_end` stored somewhere readable at email-send time. Phase 1's P0-2 puts it on the `subscription_cancelled` event payload (PostHog-only). Phase 3's P1-5 adds it to `organizations` table (DB-readable). Item 8 can either: (a) read from Stripe webhook on the fly during email composition, or (b) wait for P1-5. Option (a) is preferred — no waiting.
- Save offer redemption tracking (Bundle B item 7) ships with its own co-shipping events; does not depend on existing telemetry.
- Bundle C item 11 (recovery affordance) overlaps in code with Bundle A item 4 (soft-downgrade with restore) — design pass should decide if they share an implementation.

**No blockers from any other phase.**

---

## Success metrics

These are measured via Phase 1 + Phase 3 telemetry once those land — but the features can ship without waiting:

- **Trial → paid conversion rate (true)** — using `payment_method_added` (Phase 3 P0-4) as the conversion event. Compare cohorts before/after Bundle A ships.
- **Card-add rate during trial** — Phase 3 P0-4 enables this directly.
- **Cancel reason mix** — captured at cancel time (Phase 1 P0-1 covers Stripe-Portal cancels; Bundle B item 7 captures in-app cancels with our own taxonomy).
- **Save offer acceptance rate** — new metric, ships with Bundle B item 7. Per save-offer-type breakdown.
- **Downgrade-to-reupgrade rate** — currently 7.5% (Avenue 2a Q5). Bundle C ships and the metric improves (or doesn't).
- **`payment_recovered` recovery email click-through** — Bundle B item 9 ships and we track clicks.

---

## Out of scope (explicit)

- **Stickiness mechanisms** (change-detection digest, audit trail) — Phase 2.
- **Broader in-app billing UX migration** — invoice viewing, plan changes outside the cancel context, payment method updates outside trial, usage display, tax/billing-address management. These are candidates for a hypothetical Phase 7 if you want a strategic pass on "where billing UX should live"; not in this brief's scope.
- **Team management / seat assignment UX** — separate workstream.
- **Tax handling and compliance flows** — out of scope.
- **A/B testing infrastructure** — if A/B is a goal for any of these surfaces, that's its own scoping conversation.

---

## Source references

- `/Users/maya/dev/scanopy/docs/transition-moment-audit.md` — the audit driving every item; findings 3a / 3b / 3c map directly to Bundles A / B / C
- `/Users/maya/dev/scanopy/docs/telemetry-gap-backlog.md` — Phase 1 telemetry items that measure impact (run in parallel, not blocking)
- `/Users/maya/dev/scanopy-content/avenue-2a-followup-memo.md` — 37-voluntary-cancel cohort + corrected diagnosis context
- `/Users/maya/dev/scanopy-content/churn-remediation-phases.md` — Phase 5 in context with the other phases
- `/Users/maya/dev/scanopy/docs/stickiness-candidates.md` — what's *not* in scope here (Phase 2 stickiness work)
- `/Users/maya/dev/scanopy/docs/change-digest-design-brief.md` — Phase 2 brief, for shape comparison
