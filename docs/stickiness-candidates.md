# Stickiness Mechanism Candidates

**Date:** 2026-04-28
**Purpose:** Scoping shortlist of mechanisms that pull paid users back week-over-week. Ranked by hypothesized churn-cause × feasibility. Not designs — this tees up one or two for detailed design.

## Inputs synthesized

- Avenue 1 competitive audit (`competitive-recurring-value-audit.md`): "your network changed" diff and change-window audit trail are the two highest-consensus return triggers, both compatible with Scanopy's auto-discover positioning.
- Avenue 2a follow-up memo: cancels are 37 voluntary paid + 12 mid-trial. Most are short-tenure (0–30 days). 51% of orgs that completed checkout ran zero discoveries. Reweighted hypotheses: dormancy/value-perception explains larger volume; pricing-intent (Bob-type deliberate cancel) gained slightly but is a smaller cohort.
- Avenue 3 transition-moment audit: trial-side mechanisms (value recap, pause/extend, in-app cancel with reason capture) are conversion-side, not stickiness — moved to Defer.
- Avenue 2b telemetry-gap backlog: `topology_viewed` / `share_link_viewed` (P2-1, P2-2) and email send/open ingestion (P1-9) are needed to measure most candidates here. New stickiness-specific events flagged below feed back into 2b.
- Existing primitives: `topologies` table already has `parent_id UUID` (branching scaffolded but not load-bearing) — usable substrate for snapshot history without a new versioning schema. Scheduled discovery works today (paid-gated). **No digest email infrastructure** — building it is part of any digest feature's cost.

---

## Candidates (ranked)

### 1. Change-detection digest with audit-trail facet — "Your network changed since last scan"

Diff each scheduled scan against the prior. When something material changes (new host, vanished host, new service, interface flap, new edge), surface it via (a) a digest email, (b) an in-app changelog timeline, and (c) a retrievable "what did the network look like on date X" surface — the audit-trail facet originally split out as #2 is folded in here as a design facet, not a separate candidate. The compliance use case (PCI-DSS Req 1.1.3, ISO 27001 A.8.20 want diagram currentness as of a date) is satisfied by the same underlying data the digest is computed from.

- **Hypothesis:** Primarily value-perception / dormancy. The dominant cohort in the corrected data (51% of paid orgs ran zero discoveries; short-tenure cancels) likely paid, set up scheduled discovery, then never had a reason to come back because the topology never visibly changed between visits. A push that says "3 hosts appeared, 1 service vanished" creates the missing return trigger. Compliance/audit retrieval addresses Bob-type product-quality concerns and the S&C persona simultaneously.
- **Feasibility — needs a design pass before any code:**
  - **Snapshot/diff storage architecture is the key open question** (founder, 2026-04-28). The Avenue 4 sketch suggested repurposing `topologies.parent_id` by inserting a new row per scheduled scan, but a topology row is a multi-MB JSONB blob — daily scans on per-network basis would explode the table. Alternatives the design pass must evaluate: (i) a dedicated `topology_diffs` table that stores only deltas (added/removed hosts, services, edges, interface flaps), with full snapshots only at user-pinned points; (ii) snapshot retention policy (keep last N + user-pinned); (iii) hybrid — diffs forward, full snapshots at pin or at policy-driven anchors. The right answer depends on diff-replay needs (can we reconstruct intermediate state from deltas, or do we need a full snapshot for any audit query) and storage budget.
  - **Digest email infra:** transactional only today; building a queued digest/cron pipeline is the bulk of the implementation cost regardless of storage choice.
  - **In-app surface:** changelog timeline anchored to the topology view; audit-trail browse view for "what did the network look like on date X."
- **Effort:** **M** (1–3 weeks) — but bracketed by the storage architecture decision. Bias toward delta-only storage with policy-driven full snapshots; revisit estimate after design pass.
- **Measurability:** Send-side: `change_digest_sent`, `change_digest_opened`, `change_digest_clicked`. Audit-trail side: `snapshot_pinned`, `snapshot_diff_viewed`, `snapshot_diff_exported`. All new, feed back to Avenue 2b. Return-side leverages `topology_viewed` (already emitted per founder review). Email open/click attribution depends on P1-9 Brevo ingestion landing.
- **P18 gate fit:** Ship before P18 memo converges. Highest-leverage citable remediation. Bob/Folkert sessions are likely to refine the diff-fidelity decisions ("what counts as material") more than to overturn the build decision.

### 2. Share-engagement notifications ("your share was viewed by N stakeholders")

When a public share link or embed is viewed externally, notify the share owner — in-app and via email digest. Owner pull-back when their stakeholders rely on the artifact. Avenue 1 implicitly via the share-views observation in `POSTHOG_STRATEGY.md` Dashboard 5.4 ("orgs with embed views are extremely unlikely to churn").

- **Hypothesis:** Value-perception. Closes the loop on the sharing feature, which today emits the artifact and goes silent. The "social proof" feedback (your share is being used) reinforces the share owner's reason to keep the underlying topology current → keeps them paying.
- **Feasibility:** (a) No snapshot history. (b) Needs lightweight notification email and/or weekly digest of view counts — much smaller than #1's full digest infra. (c) In-app notification surface or a counter on the share itself. **Critical prerequisite: `share_link_viewed` and `share_embed_viewed` events do not yet fire** (Avenue 2b P2-2). They're already prioritized but unimplemented.
- **Effort:** **S** (~1 week) once the share-view events exist; otherwise tack on the few-hour P2-2 instrumentation first. Effectively gated on P2-2.
- **Measurability:** `share_view_notified`, `share_view_notification_clicked` — **new**. Plus the prerequisite `share_link_viewed` / `share_embed_viewed`. Retention impact: owners of shares with ≥1 external view per week vs owners whose shares are never viewed (the latter cohort is, per `POSTHOG_STRATEGY.md`, the high-churn one).
- **P18 gate fit:** **Ship before** if P2-2 telemetry is treated as in-scope. Otherwise it slides to "after" because the measurement story collapses without view tracking. Recommend bundling P2-2 instrumentation with this candidate's first slice.

### 3. Coverage-completeness nudges (unmanaged hosts, credential-less subnets)

A standing "your fleet has gaps" surface: hosts without daemon coverage, subnets without credentials, services without identification. Differs from a one-time onboarding checklist — these are recurring conditions that re-emerge as networks grow.

- **Hypothesis:** Value-perception / dormancy. Gives users a recurring reason to engage with their operational state. The "never really done" framing (Avenue 1 #7-adjacent) creates a low-grade but persistent return trigger that doesn't require a discovery delta to land.
- **Feasibility:** (a) No snapshot history. (b) No digest email infra (in-app first; digest can layer later). (c) New in-app surface, but the `GettingStartedChecklist` and `FeatureNudges` patterns already exist as templates.
- **Effort:** **S** (<1 week) for the in-app surface against existing data. Email layer adds another S if/when we want push.
- **Measurability:** `coverage_gap_shown`, `coverage_gap_resolved`, `coverage_gap_dismissed` per `gap_type` — **new, gap to feed back to Avenue 2b**. Retention impact harder to attribute cleanly because the surface is passive (impression-led) rather than push-led; a no-coverage-surface control cohort would be needed for clean attribution.
- **P18 gate fit:** **Ship after** P18 narrows the top cause. Lower expected impact than #1–#3 and the measurement story is weaker (passive surface, hard to isolate counterfactual). Worth shipping eventually but not the place to spend pre-memo bandwidth.

---

## Defer

- **Pre-change blast-radius prompt (was candidate #3, founder 2026-04-28).** Moved to future work. The mechanism depends on **editable topology** (the user can name/select a host/service and explore impact), which is out of scope today. Worth revisiting once editable topology lands as a separate workstream — at that point the in-app entry surface is cheap (S) and reuses the shipped Applications view. Tracking under future-work, not stickiness.
- **Scheduled MSP "state of your network" report (Avenue 1 #5).** Higher-effort (PDF/branded layout, digest email infra, MSP-flavored content) for a persona that does not currently route through the cloud funnel — `icp.md` §Persona 2 flags this gap. Persona-narrowing would be premature pre-P18; revisit once Bob session confirms the MSP customer-deliverable framing and P7 (commercial self-hosted trial) opens an MSP path.
- **Trial value recap, pause-as-middle-path, in-app cancel with reason capture, progressive trial banner cadence** (Avenue 3 transition-moment findings). These are conversion-side mechanisms — they reduce trial-to-paid drop and improve cancel attribution, not week-over-week stickiness. Belong in a separate Avenue 3 remediation pass.
- **Rogue / unexpected device detection (Avenue 1 #4).** Layers cleanly on top of #1's diff infrastructure but adds a baseline-management abstraction (approved MAC/vendor sets) that's a separate scope. Defer to a follow-up after #1 ships; the diff substrate makes it cheap then.
- **Dormant port / stale host cleanup (Avenue 1 #7).** Lighter-weight version of rogue detection; same defer reasoning. Cheap once diffing exists.
- **EOL/CVE enrichment (Avenue 1 #6).** Requires a CVE/EOL feed-ingestion pipeline that doesn't exist today — separate workstream of its own size. Real signal for S&C persona but not the place to start.
- **Slack/Teams/Confluence/Hudu push integrations.** signals.md flags MSP demand (Hudu/UniFi thread, MspGeek Discord). High value but each integration is M–L by itself and the integration architecture is a separate decision. Defer pending dedicated integrations roadmap.
- **MSP multi-tenant dashboard (Avenue 1 #8).** Not a stickiness mechanism — it's an acquisition-funnel feature for a persona we don't currently route through cloud signup. Wrong problem.
- **API pull as visit surrogate (Avenue 1 #9, NetBox-style).** Platform/DevOps persona play; not the churning cohort visible in the corrected data. Track separately.

---

## Recommended next-pass design targets

**Primary:** Candidate #1 — the change-detection digest with audit-trail facet. Single design pass with two open architectural questions to resolve before code:
1. **Snapshot/diff storage:** delta-only with policy-driven full-snapshot anchors vs. full snapshots with retention policy vs. hybrid. The Avenue 4 sketch (full snapshot per scan via `topologies.parent_id`) is rejected — topologies are too heavy. Bias toward delta storage; revisit estimate after the design call.
2. **Diff fidelity:** what counts as "material" (host appeared/vanished is obvious; interface state changes, subnet changes, service identification confidence shifts are calls). Audit-trail facet imposes its own requirement — exports need to be human-readable enough to serve as compliance evidence.

**Telemetry feedback to Avenue 2b:** new events flagged for backlog inclusion — `change_digest_{sent,opened,clicked}`, `snapshot_{pinned,diff_viewed,diff_exported}`, `share_view_notified`, `coverage_gap_{shown,resolved,dismissed}`. The previously-flagged `topology_viewed` and `share_*_viewed` events are confirmed already emitted (founder review, 2026-04-28) — they do not need new instrumentation. The `blast_radius_queried` event is no longer in scope here since candidate #3 (blast-radius prompt) moved to future work pending editable topology.
