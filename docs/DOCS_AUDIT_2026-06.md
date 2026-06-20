# `docs/` audit — 2026-06

Audit of the nine internal working documents in `docs/` against everything merged
since tag **`v0.16.2`** (`git log v0.16.2..HEAD`, 244 commits). None of these are
user-facing; they are design briefs, spec snapshots, and backlog grooming notes
from the Phase 2 (change-tracking/digest), Phase 5 (billing/subscription
retention), and conversion/churn arcs.

For each document: **what it described**, **what shipped** (traced to commits and
current file paths), **what hasn't** (deferred vs. silently dropped), and a
**recommendation**. Dispositions are applied in this same branch; see the Work
Summary in `TASK.md` for the final ledger.

Forward-looking successor specs live in `planned-work/`
(`/Users/maya/.claude/projects/-Users-maya-dev-scanopy/planned-work/`). That
directory is entirely topology/discovery/integration work — **nothing** there
covers the Phase 2 digest/snapshot or Phase 5 billing surfaces, so anything
unbuilt in those arcs is silently dropped, not formally deferred, unless noted.

---

## `phase2-spec.md`

### What it described
A snapshot-driven SCD Type-2 architecture making every discovery entity table
time-aware (`valid_from`/`valid_to`/`last_seen_at`), with manual + scheduled
snapshots closing rows and producing an audit trail, plus a change-digest email,
in-app changelog, snapshot pinning, PDF/HTML diff export, a "presumed vanished"
coverage-gap surface, and snapshot-window retention as a paid lever.

### What has been implemented
The architectural core shipped across `feat/phase2-foundation`,
`feat/phase2-topology-snapshots`, and `feat/phase2-session-digest`:
- **SCD2 substrate** — `c3fdae9b4`, `20851a309`, `1b6745eec`, `e866bcb25`.
  `backend/migrations/20260502000000_scd2_add_columns.sql` adds
  `valid_from`/`valid_to`/`lineage_id` to 13 Snapshotable tables and
  `last_seen_at`/`last_discovery_id`/`first_discovery_id` to 9 DiscoveryTracked
  tables (broader than the spec's "seven tables"). Traits + `make_closed_copy`
  in `backend/src/server/shared/storage/snapshot.rs`.
- **Continuous-scan reconciliation** — `.live()` filter, in-place UPDATE +
  `last_seen_at` refresh on match, INSERT on no-match, no row-close at submission
  (`c3fdae9b4`). `HAS_SCD2`/`is_live_row()` auto-404 closed rows (`d78428a7f`).
- **First-class snapshots** —
  `backend/migrations/20260502120000_create_snapshots_table.sql`,
  `SnapshotService::run_close_and_clone` (`backend/src/server/snapshots/service.rs`),
  manual endpoint gated by `TakeSnapshotFeature` (`handlers.rs:118`,
  `Authorized<Member>`).
- **Retention (shipped & wired)** — daily sweep `SnapshotService::run_retention`
  (`service.rs:268`) scheduled in `backend/src/bin/server.rs:175`, per-plan window
  via `BillingPlan::snapshot_retention_days` with a self-host env override.
- **Topology read path** — unified
  `get_topology_data(network_id, snapshot_id: Option<Uuid>)`
  (`backend/src/server/topology/service/main.rs:225`); the old topology
  lock/staleness UX was **replaced** by snapshots (`d78428a7f`).
- **Digest (shipped & active)** — `backend/src/server/digest/` (`3a71e944f` +
  rounds 2–7). Per-discovery-session email, gated per-user via
  `email_settings.discovery_digest`
  (`20260502100000_add_user_email_settings.sql`;
  `ui/src/lib/features/settings/EmailTab.svelte`).

### What hasn't
All silently dropped (no `planned-work/` successor):
- **Scheduled snapshots** — only the manual endpoint exists (`service.rs:81`
  calls scheduled snapshots a "future" caller).
- **Snapshot pinning / naming / `kind`** — the `snapshots` table has no `name`,
  `kind`, or `pinned` columns.
- **Per-org digest cadence** (`{daily,weekly,monthly}`) — the digest is
  per-session and per-user, not weekly/per-org.
- **`digest_send_empty` / "nothing changed" email** — inverted: empty digests
  are unconditionally suppressed (`payload.rs:182 has_changes()`).
- **Paid-gating of the digest opt-out** — it's a free per-user checkbox.
- **Discovery-failure digest variant** — explicitly v1-dropped (`subscriber.rs:3`).
- **"Presumed vanished" coverage-gap surface** — the `last_seen_at` column ships
  but nothing reads it for a staleness nudge.
- **In-app changelog (Activity page + timeline pane)** — only the snapshot
  dropdown on `TopologyTab.svelte` exists.
- **PDF/HTML diff export** — `Feature::PdfExport` exists but is consumed by
  `shares` (`backend/src/server/shares/handlers.rs:497`), not snapshot-diff.
- **As-of-arbitrary-`T` query** — diverged to discrete `snapshot_id`-stamped
  reads.

### Recommendation
**REPHRASE / TRUNCATE.** The data-model and daemon/server reconciliation contract
are load-bearing and accurate — keep them. Prepend a status header and trim the
unbuilt digest/UX surface so the doc stops reading as shipped scope. Correct two
factual divergences: the digest is per-session/per-user (not weekly/per-org), and
snapshot reads are `snapshot_id`-discrete (not as-of-`T`).

---

## `change-digest-design-brief.md`

### What it described
A pre-spec options brief (explicitly "not a spec itself") that framed the
dormancy-churn problem and laid out seven temporal data-model options (A–G) plus
sub-decisions (identity resolution, soft-delete, retention, read-path) to be
resolved interactively.

### What has been implemented
The brief's deliberation concluded at **Option C (per-entity SCD Type 2)**, and
that architecture shipped — same evidence as `phase2-spec.md` above
(`1b6745eec`, `c3fdae9b4`, `e866bcb25`; `backend/src/server/snapshots/`,
`backend/src/server/digest/`, `backend/src/server/shared/storage/snapshot.rs`).
The brief's recommended sub-decisions also landed: server-side identity via
`lineage_id` + discovery FKs; hybrid soft-delete
(`EntityDigestStatus::{New,Unchanged,PossiblyMissing,Missing}`,
`backend/src/server/digest/payload.rs`); per-org hard-delete retention by plan.

### What hasn't
The brief's co-shipping telemetry events (`change_digest_sent/opened/clicked`,
`snapshot_pinned`, `snapshot_diff_viewed/exported`), PDF/HTML diff export, the
in-app changelog page, and explicit snapshot pinning did not ship — same gaps as
`phase2-spec.md`. Options A, B, D–G were rejected by design (the brief's expected
outcome, not abandonment).

### Recommendation
**DELETE.** A disposable pre-spec working doc whose only job — choosing among
options A–G — is done. Option C shipped and is documented by the successor
`phase2-spec.md`. Its six unselected options now read as live design space and are
actively misleading. A one-line provenance pointer is folded into
`phase2-spec.md`.

---

## `phase5-spec.md`

### What it described
A settled implementation spec for the Phase 5 billing/subscription retention arc:
in-app cancel flow with reason capture and reason-dependent save offers
(pause/discount), trial urgency ramp, trial value recap, trial extend, downgrade
communication (banner + email + "what changed" page), and quick-win billing
emails — backed by a `cancellations` table and new `organizations` columns.

### What has been implemented
A large, real implementation across `feat/phase5-subscription-mechanics`,
`feat/phase5-trial-ui`, `feat/phase5-quick-wins`, `feat/billing-telemetry-enrichments`,
`fix/billing-tab-ux-polish`, and the `audit/*` branches:
- **Cancel flow + save offers** — `CancelSubscriptionModal.svelte` (`5e72e0f8c`,
  `540c84e44`, `188185eea`), wired in `BillingTab.svelte`. Backend
  `POST /api/billing/cancel` (`handlers.rs:651`, `Authorized<Owner>`),
  `CancelReason`/`SaveOffer` enums in `billing/types/base.rs`.
- **Pause / discount / extend / resume** — `handlers.rs:520/555/616/684`
  (`service.rs:2159/2355/2629`), with cooldown/once-ever gates.
- **Trial urgency ramp** — `TrialEndingBanner.svelte` (T-3d),
  `TrialExpiryModal.svelte` (T-1d), Sidebar trial pill (T-7d); mounted in
  `ui/src/routes/+page.svelte` (`df2763393`).
- **Quick-win emails** — `send_payment_method_added_email`,
  `send_subscription_cancelled_email` (with `period_end`),
  `send_payment_recovered_email`, plus a `subscription_paused` email
  (`backend/src/server/email/service.rs:295/308/316/333`).
- **Post-Stripe confirmation** — `PostStripeWelcomeBanner.svelte` (`df2763393`).
- **Schema** — `20260501000000_add_organization_billing_flags.sql` (written via
  `OrganizationBillingSubscriber`, Pattern B).
- **Telemetry** — `BillingOperation::CancellationInitiated` +
  `CancellationFeedbackProvided` (`events/types.rs:218/232`, `74ac498cd`), to the
  event bus + Brevo.

### What hasn't
Silently dropped (no successor doc):
- **`cancellations` table — not built.** Persistence collapsed to Stripe metadata
  + org flag columns + bus/Brevo events, the opposite of the spec's emphatic
  decision.
- **Downgrade recovery banner — not built.** `last_downgrade_at` /
  `last_downgrade_from_plan` are written but no `DowngradeRecoveryBanner`
  component exists.
- **"What changed" page — not built.**
- **Trial value recap** — the in-app card shipped then was deliberately removed
  (`76c748e8a`); the recap **email** metrics live (`service.rs:35/711`).
- **Downgrade email enrichment** — `send_plan_changed_email` still takes only
  `plan_name`.
- **Schema/enum naming** diverged from the spec verbatim (e.g.
  `downgraded_at`→`last_downgrade_at`, canonical reason taxonomy replaced with
  Stripe-identity names).

### Recommendation
**REPHRASE / TRUNCATE + absorb the Phase 5 siblings.** The pre-implementation
investigation and UX-deliberation bulk is obsolete. Replace with a concise
"Phase 5 — shipped vs. deferred" ledger that also subsumes `phase5-features.md`
(feature inventory) and the single surviving decision from `phase5-data-model.md`
(event-ledger considered, denormalized columns chosen). The four unbuilt items
(downgrade banner, "what changed" page, trial recap surface, downgrade email
enrichment) are the only forward-looking content and have no `planned-work/` home.

---

## `phase5-data-model.md`

### What it described
An implementation plan proposing an **event-sourced** billing data model: a
`subscription_events` DB ledger, a `SubscriptionService` deriving all current
state from event history, and the **dropping** of `organizations.plan`,
`plan_status`, and `trial_end_date`.

### What has been implemented
The typed-payload `BillingOperation` + trait-based `EventBus` shipped
(`09d314451`; `backend/src/server/shared/events/traits.rs`, `types.rs:120-265`).
But the persistence model is the **inverse** of the proposal:
- **No `subscription_events` ledger** (`grep` returns nothing).
- **No `SubscriptionService`** (symbol does not exist).
- **Org columns kept and added to**, not dropped — migration
  `20260501000000_add_organization_billing_flags.sql`, whose own comment says the
  denormalized columns "power Phase 5 eligibility gates … **without needing an
  event-sourced ledger.**"
- **A mirroring subscriber** (`backend/src/server/organizations/subscriber.rs:64`)
  writes denorm columns per event — the opposite of "write history, derive on
  read." Read sites read columns directly
  (`backend/src/server/auth/middleware/billing.rs:98`).

### What hasn't
The ledger, the `SubscriptionService`, the column-drop, and the backfill of seed
events were all silently abandoned in favor of the denormalized design.

### Recommendation
**DELETE.** A superseded implementation plan whose load-bearing thesis was
rejected during implementation. Following it now would actively mislead. The one
durable sentence — "an event-sourced ledger was considered and denormalized
columns were chosen instead" — is folded into `phase5-spec.md`. The shipped model
is self-documenting via the migration's own comments.

---

## `phase5-features.md`

### What it described
A 14-feature scope memo for the Phase 5 arc — the feature-list companion to
`phase5-spec.md` and `phase5-data-model.md` — enumerating quick-win emails, trial
urgency surfaces, invoice preview, pause, trial extend, the cancel modal,
downgrade-recovery banner, downgrade-email rewrite, "what changed" page, and
recovery affordances, each tagged with dependencies and worker spawn ordering.

### What has been implemented
~70% shipped (same evidence as `phase5-spec.md`): features 1–4, 7, 8, 9, 10, 14
landed. Notable details: the trial value-recap **email** shipped while the
in-app **card** was built-then-removed (`76c748e8a`); recovery affordances
(feature 14) wired `UpgradeButton` on Networks/Members/API-keys
(`NetworksTab.svelte:215`, `UserTab.svelte:190`, `UserApiKeyTab.svelte:196`).

### What hasn't
Features 5 (first-invoice amount pre-redirect — only a client-side estimate
exists), 11 (downgrade-recovery banner), 12 (downgrade email per-feature delta),
and 13 ("what changed" page) were not built. No `planned-work/` successor.

### Recommendation
**CONSOLIDATE into `phase5-spec.md`.** Three docs describe the same arc; the
"worker spawn ordering" and dependency-table framing is spent. Fold the surviving
feature status into the `phase5-spec.md` ledger and delete this file.

---

## `conversion-side-remediation-brief.md`

### What it described
A kickoff/working brief (explicitly "kickoff input … that will produce the
spec(s)") of eleven conversion-funnel remediation items from the transition-moment
audit, in three bundles: trial→paid friction (A), in-app cancel + recovery emails
(B), and post-downgrade communication (C).

### What has been implemented
The design pass ran (producing the `phase5-*` specs) and 9 of 11 items shipped on
`feat/phase5-subscription-mechanics`, `feat/phase5-trial-ui`,
`audit/banner-conditions-and-payment-prompt`, and `fix/billing-tab-ux-polish`:
the trial urgency ramp (`df2763393`), `payment_method_added` email
(`email/subscriber.rs:142`), post-Stripe confirmation banner, pause/extend
(`handlers.rs:520/616`), in-app cancel flow with reason capture + save offers
(`CancelSubscriptionModal.svelte`), `period_end` in the cancel email, and the
`payment_recovered` email. The brief's premises are now mostly false (it calls the
trial ramp "flat," the payment-added email "orphaned," cancel "100% Stripe
Portal").

### What hasn't
Two genuine residual gaps, captured nowhere else:
- **Item 10/11 — Free-landed downgrade banner + contextual recovery
  affordances.** The existing no-payment banner targets the *pre-downgrade*
  card-missing window, not an org that has landed on Free; there is no "restore
  access by adding a card" copy on the surfaces that just stopped working.
- **Item 2 (remainder)** — an authoritative Stripe upcoming-invoice preview (only
  a client-side estimate shipped). **Item 3** — trial value recap (built then
  reverted).

### Recommendation
**DELETE**, lifting the two residual gaps (items 10/11) into
`transition-moment-audit.md`'s open-items list so they don't evaporate. The brief
is a pre-spec kickoff whose purpose is fulfilled; its open questions are resolved
in code and the successor `phase5-spec.md` records the shipped scope.

---

## `transition-moment-audit.md`

### What it described
A purely diagnostic audit (no fixes proposed) of three billing transitions —
trial→paid, cancel, and downgrade-to-free — cataloguing existing
UX/email/banner/telemetry surfaces and the silent-fail and measurement gaps at
each, all cited to `file:line`. It is the upstream diagnosis that fed the
conversion remediation and telemetry backlog.

### What has been implemented
~70% of its HIGH/MED findings shipped (same Phase 5 branches): the in-app trial
urgency ramp, the wired `PAYMENT_METHOD_ADDED` email, the post-Stripe
confirmation banner, Stripe `cancellation_details` capture
(`billing/service.rs:2785`, `74ac498cd`), enriched cancel events, the in-app
cancel flow with reason capture, the `payment_recovered` email, and a stored
`next_renewal_at` column.

### What hasn't
Silently abandoned (gap persists, no successor):
- **DaemonPoll plan-gating** — `daemon.base.standby` is set only by the 30-day
  inactivity sweep (`daemons/service.rs:2001`), not a plan-feature gate; the
  `DaemonStandby` error copy (`error_codes.rs:300-301`) is still misleading.
- **`PLAN_CHANGED_BODY` feature delta** — no enumeration of what stops/starts
  working on downgrade; no in-product "what changed" surface.
- **`downgraded_at` / `cancelled_at` columns** — never added (only
  `next_renewal_at`); tenure cohorts still require event-stream joins.

### Recommendation
**KEEP / TRUNCATE.** The genuine root-cause diagnosis that spawned three
successor docs and most of Phase 5; it documents the pre-remediation baseline and
the `file:line` provenance the successors cite. Prepend a status banner marking
resolved findings, and absorb the two residual conversion-brief gaps (items
10/11) as open items. Do not consolidate it away — the briefs are forward-looking
and would lose this diagnostic value.

---

## `stickiness-candidates.md`

### What it described
A 2026-04-28 ideation shortlist of retention mechanisms — change-detection digest
+ audit trail, share-engagement notifications, coverage-completeness nudges — plus
a Defer list. Grooming notes, not a spec.

### What has been implemented
Candidate #1 (the headline) largely shipped via Phase 2: the digest email
(`backend/src/server/digest/`, `3a71e944f`) and the SCD2 snapshot audit trail
(`backend/src/server/snapshots/`, `c3fdae9b4`, `d78428a7f`) — including the
storage-architecture question the doc agonized over, resolved as SCD2 (a third
option the doc didn't enumerate). Candidate #2's measurement prerequisite shipped:
`TopologyShareViewed`/`TopologyEmbedViewed` (`events/types.rs:438-439`,
`shares/handlers.rs`, PostHog subscriber).

### What hasn't
- **Candidate #1 in-app changelog timeline** — not built (digest is email +
  snapshot-browse only).
- **Candidate #2 feature itself** (persisted view counts, owner notification) —
  not built; only the telemetry prerequisite landed.
- **Candidate #3 coverage-completeness nudges** — not built;
  `GettingStartedChecklist`/`FeatureNudges` (`188185eea`) are onboarding-gated,
  not fleet-state-gated.
- **Defer list** (blast-radius prompt, MSP reports, rogue-device detection,
  EOL/CVE, push integrations) — all still ideation, none in `planned-work/`.

### Recommendation
**REPHRASE / TRUNCATE.** Mostly live ideation, so keep it — but the headline
candidate shipped and the doc still presents the storage architecture as an open
question and asserts "no digest email infrastructure" (both false). Prepend a
status banner, strike the resolved deliberation, mark #2's prerequisite met, and
keep #2/#3/Defer as live ideation.

---

## `telemetry-gap-backlog.md`

### What it described
A prioritized (P0–P2) backlog of missing billing/onboarding/product telemetry —
enriched cancellation/checkout payloads, a paid-conversion signal, org cancel-state
columns, UTM capture, structured discovery errors, Brevo email-engagement
ingestion — consolidated from the strategy memo, the Avenue 2a/3 memos, and
`transition-moment-audit.md`. Telemetry flows event-bus → PostHog subscriber +
Brevo subscriber.

### What has been implemented
~40% closed via `feat/billing-telemetry-enrichments` + `feat/phase5-subscription-mechanics`:
- **P0-1** Stripe `cancellation_details` capture (`billing/service.rs:2785`,
  `74ac498cd`).
- **P0-2** enriched `subscription_cancelled` (`was_trialing`, `period_end`,
  `mrr_amount_cents`, `tenure_days`) — minus the `cancel_type` taxonomy.
- **P0-3** `mrr_amount_cents` + deterministic plan on `checkout_completed`.
- **P1-2** enriched `payment_failed`/`payment_recovered`.
- **P1-4** UTM on email CTAs (`email/messages/mod.rs:198-211`).
- **P2-1/2/3** `topology_viewed`, nudge events, `checklist_dismissed` — confirmed
  emitted.

### What hasn't
- **Genuinely open:** P0-5 `first_invoice_paid`; P1-7a server-side UTM capture;
  P1-8 structured `discovery_failed` codes (still free-text `error_reason`);
  P1-9 Brevo email send/open/click ingestion; P2-6 `402_gate_returned`;
  P2-7 login coverage (web-only `LoginSuccess`).
- **Partial:** P0-4 `payment_method_added` (emitted but **no analytics subscriber
  consumes it**, and it's a property-less unit struct); P1-3 (no
  `trial_card_dismissed`); P1-5 (shipped `next_renewal_at`, not the named
  `cancelled_at`/`period_end`); P1-7 (Brevo-only, no PostHog group prop / no DB
  column); P1-1 `paywall_gate_hit` (fires in lockstep with the click — doesn't
  measure passive bounce).
- **Deliberately scoped-out** (code matches the decision): P1-6 typed
  `cancellations` table (DEFERRED 2026-04-28), CS-3 blast-radius event. Aligned
  with the standing preference against derivable-signal events.

### Recommendation
**REPHRASE / TRUNCATE.** Still the live source of truth for telemetry work
(~60% open), so keep it — but it is stale and self-contradicting against the code.
Mark P0-1/2/3, P1-2/4, P2-1/2/3 closed; rewrite the partials (esp. P0-4 — built
but unconsumed); strip stale line numbers; keep the genuinely-open and
deliberately-deferred items.

---

## Cross-cutting observations

- **The Phase 5 cluster is three views of one arc.** `phase5-spec.md` (UX),
  `phase5-features.md` (inventory), and `phase5-data-model.md` (foundation) were
  written pre-implementation; the data-model thesis was inverted in code and the
  feature inventory is spent. Collapsing to a single status ledger removes the
  drift risk of three docs describing the same shipped system.
- **The conversion/churn docs form a diagnosis→fix→measure chain.**
  `transition-moment-audit.md` (diagnosis) → `conversion-side-remediation-brief.md`
  (fixes) → `telemetry-gap-backlog.md` (measurement). The fix brief is fully
  superseded by shipped code; the diagnosis and the measurement backlog remain
  load-bearing.
- **Unbuilt scope has no home.** Several deliberately-deferred or
  silently-dropped items (downgrade recovery banner, "what changed" page,
  in-app changelog, coverage-gap nudges, Free-landed recovery affordances) are
  not tracked in `planned-work/`. Per task constraints this audit does not create
  `planned-work/` entries; the Work Summary surfaces them for the coordinator.
- **No source code was changed** — this is a docs-only branch. One quality
  observation worth a separate look: the `DaemonStandby` error copy
  (`backend/.../error_codes.rs:300-301`) misleadingly references DaemonPoll plan
  support for what is actually an inactivity-driven standby; flagged, not fixed.
