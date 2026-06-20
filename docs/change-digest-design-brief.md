# Phase 2 design brief — change-detection digest + audit-trail facet

Working doc for the Phase 2 design pass. Kickoff input for the founder + Claude Code session that will produce the spec. Not a spec itself — the spec is the output of working through the open questions below.

---

## The problem

Scanopy's value peaks at first-topology-generation and then has no structural mechanism pulling users back. From the corrected diagnostic data:

- 51% of paid orgs ran zero discoveries during their tenure
- Most cancels are short-tenure (0–30 days)
- 85% of orgs that downgrade to free become permanently dormant (re-upgrade rate 7.5%)
- The "dominant churn pattern is dormancy" hypothesis was the strongest survivor of the follow-up's event-semantics corrections

The competitive audit confirmed this is a category gap. Auvik returns users via alerts. SlurpIT via snapshot diffs. Lansweeper via accumulating ITAM data. NetBox via change-management gravity. Scanopy generates a topology and goes silent until a user manually checks back.

## The mechanism

A scheduled-scan diff with two surfaces, sharing the same underlying snapshot/diff data:

1. **Push surface — digest.** Email digest (and in-app changelog) surfaces material changes since the prior scan: hosts appeared/vanished, services added/removed, edges changed, interfaces flapped.
2. **Pull surface — audit-trail facet.** Users can pin a snapshot ("post-Tuesday-maintenance"), browse historical state, and export a before/after diff as a PDF/share artifact for compliance evidence.

Both surfaces share the snapshot/diff data, which is why this is one design pass and one feature, not two.

## Roles

- **Founder** owns architectural decisions (data model, fidelity, UX direction, what counts as material).
- **Claude Code agents** do investigations, draft options, write code, and produce design artifacts. Worker agents spawn off worktrees per the project's coordinator/worker pattern.
- **End-user audience for the digest:** primarily IT Ops persona (the dominant dormant cohort).
- **End-user audience for the audit-trail facet:** S&C compliance persona — PCI-DSS Req 1.1.3 and ISO 27001 A.8.20 explicitly want diagram-currentness as of a date. Also MSP persona for the customer-deliverable use case.
- **External input running in parallel:** Bob/Folkert qualitative interviews. Their findings may refine diff fidelity decisions but won't change the build decision.

---

## Open architectural questions (decisions needed before code)

### 1. Data model — temporal granularity and source-of-truth shape

The fundamental question is **where the temporal dimension lives** in the data model. Today, `topologies` is a JSONB blob containing all hosts/services/interfaces/ports/bindings/subnets/groups, overwritten on every rebuild. This works for "show me current state" and nothing else. The change-digest feature needs *something* over time, but "snapshot the JSONB blob per scan" isn't the only shape that something can take — and may not be the best one.

#### Founder framework answers — inputs to evaluate, not a pre-commitment

Founder's answers to the ranking framework, recorded for the design pass to weigh against the options below:
- Editable topology is coming eventually
- More change-aware features will follow (rogue detection, scheduled reports, alerting, blast-radius)
- Migration appetite is good
- History retention: self-hosted user-configurable; cloud per-plan tiering

Use these as inputs to the evaluation. They favor some options over others — per-entity temporal models tend to score higher against "editable topology coming" and "more change-aware features"; snapshot-based options score higher when migration cost or implementation simplicity dominates. **The design pass should weigh the seven options interactively against these criteria, surface trade-offs the founder may not have considered, and reach the architectural decision in the conversation. Do not walk into the design pass with the choice already made.**

The agent driving the design pass should expect to:
- Present pros and cons of each option honestly, including trade-offs that work against options the framework answers seem to favor
- Surface second-order considerations: read-path performance under realistic load, daemon-server protocol implications, what happens to the existing `parent_id` branching primitive, soft-delete semantics, identity resolution
- Push back if the founder's instinct in the conversation is heading somewhere that contradicts the data or the audit findings
- Produce a single decision with explicit rationale at the end of the pass — not three options in priority order

The design space, from "least change to current model" to "most change":

#### A. Topology snapshots with retention (smallest change)
New `topology_snapshots` row per scheduled scan or per material-change. Retention policy prunes old ones (keep last N + user-pinned). Diffs are computed by JSONB-comparing two snapshot rows.
- **Pros:** Smallest schema change. Topology read path unchanged.
- **Cons:** Massive storage redundancy — every snapshot duplicates every unchanged entity. Storage cost scales with scan frequency × estate size. Diff computation is JSONB-heavy and not indexable.

#### B. Topology diffs only
Compute the delta between scans server-side, store only the deltas in a `topology_diffs` table. Replay deltas from a known anchor to render any historical point.
- **Pros:** Cheap storage.
- **Cons:** Replay is O(history) — rendering "what did the topology look like on date X" walks the diff stream from the most recent anchor. Painful for distant queries, fine for recent ones. Diff schema itself becomes complex (additions/removals/modifications across many entity types).

#### C. Per-entity temporal tables (SCD Type 2) — the option you're pointing at
Each entity table (`hosts`, `services`, `interfaces`, `ports`, `bindings`, `subnets`, `groups`) gains `valid_from`, `valid_to`, and a stable identity key. Entity rows are insert-only — when something changes, close out the old row (`valid_to = now`) and insert a new row. "Topology as of time T" becomes `SELECT * WHERE valid_from <= T AND (valid_to IS NULL OR valid_to > T)` against each entity type, joined into a topology projection.
- **Pros:** No JSONB blob storage at all — topology is a derived view. Each entity carries its own history natively. "When did this host first appear?" is a trivial query. Diff between any two times is a join. Aligns naturally with compliance evidence (per-asset history). No per-scan overhead — only writes for actually-changed state.
- **Cons:** Bigger architectural change. Topology assembly becomes a query rather than a fetch — index design matters a lot. Application code reading topology must specify "as of when" (or default to current). Soft-delete handling (entity vanished from network — but is "vanished" the same as "we didn't see it this scan"?) needs careful semantics.
- **Open sub-decisions if we go this route:** identity resolution (when is this scan's host the same as last scan's host? MAC? IP? hostname? composite?). Daemon-server protocol may need to change — daemons today send wholesale topology, but a temporal model wants per-entity updates with stable IDs.

#### D. Discovery-anchored entities
Each `discovery` row gets the entities it discovered foreign-keyed to it. Entity identity is resolved post-hoc by joining across discoveries (host with same MAC across discovery 1, 2, 3 = same host). Topology over time is "the union of entities seen in discoveries within window W."
- **Pros:** Aligns naturally with how scans actually work. Audit trail is "what did we see in this scan" which is concrete.
- **Cons:** Identity resolution is implicit in queries rather than the data model. Same identity-resolution complexity as C, just deferred.

#### E. Event sourcing
Store discovery *events* as the source of truth (`host_seen`, `service_added`, `interface_state_changed`, etc.). Current topology and historical state are both projections of the event stream.
- **Pros:** Complete audit trail by construction. Highly analytics-friendly. Diff is just "the events between two points."
- **Cons:** Largest architectural shift. Projection materialization strategy is its own design problem. Query performance depends on snapshot cadence within the projection layer. Probably overkill for this feature alone, but might be worth it if you're planning to add more change-aware features later.

#### F. Hybrid — temporal entities + cached topology projection
Entities have temporal rows (option C). The `topologies` table becomes an explicit cache of the current projection, rebuilt on each scan. Optionally: keep the cache for periodic snapshot rows (every Nth scan or at pins) for fast historical queries.
- **Pros:** Fast queries for current state (cache); full history available (entities); fast historical queries at common anchor points (cached snapshots).
- **Cons:** Cache invalidation complexity. Two sources of truth that have to agree.

#### G. Bitemporal (transaction time + valid time)
Each entity row has both *when we recorded it* (transaction time) and *when it was actually true* (valid time). Compliance audits can distinguish "what we believed on date X" from "what was actually true on date X" — which matters when discovery is delayed or backdated corrections happen.
- **Pros:** Most accurate model for compliance use cases.
- **Cons:** Most complex; probably overkill unless audit defensibility is a load-bearing requirement.

#### Cross-cutting sub-decisions

These questions come into play during the evaluation of the temporal-aware options (C, D, E, F, G). They mostly *don't* apply to A or B (snapshot or diff-only models — those don't need entity-level identity, soft-delete semantics, etc.). The design pass should weigh how each option answers these — that's a major part of the option's downstream burden.

The **identity resolution** question (S1) is the highest-priority pre-pass investigation regardless of which option ends up chosen, because for any temporal-aware option, current Scanopy identity-resolution behavior either supports or constrains the architecture. Worth running a worker-agent investigation before the design pass so the conversation has ground truth.

##### S1. Identity resolution (worker-agent investigation before the design pass)

When is "this scan's host" the same as "last scan's host"? Per-entity temporal modeling is only viable if entity identity persists reliably across scans. Three shapes:

- **Daemon-assigned stable ID** — daemon remembers what it called each host. Fragile (daemon state loss = identity loss).
- **Server-assigned surrogate ID, reconciled via natural key** — daemon sends a natural key (e.g., `(mac_address, network_id)`); server matches against existing rows and assigns/reuses the surrogate UUID. Most flexible, handles daemon state loss.
- **Composite natural key as identity** — no surrogate; the natural key *is* the row's PK. Simplest, but breaks when the natural key changes (NIC swap → looks like a new host).

**Worker-agent task before the design pass:** spawn an Explore-style worker to read `backend/src/server/hosts/service.rs` and `daemon/discovery/service/` and report on how host/service/interface identity is established today and what daemon-server reconciliation looks like. The current logic likely commits Scanopy to one of the three shapes already — if it's working well, any temporal-aware option that adopts the existing shape pays a smaller migration cost; if it's brittle, the design pass needs to weigh fixing it as part of the architectural decision rather than after.

##### S2. Soft-delete semantics

"Host not seen in this scan" is not necessarily "host vanished." Could be: daemon down, scan timed out, host was offline, network partition.

Three rule shapes to consider:

- **Immediate close.** Missed once → close `valid_to`. Aggressive. Will produce flapping if any scan ever misses something.
- **Consecutive-miss threshold.** Missed N consecutive scans → close. Conservative; tolerates transient gaps.
- **Time-based threshold.** Not seen for N days → close. Robust to variable scan intervals; works regardless of scan frequency.

A hybrid is probably right: time-based threshold for closing `valid_to`, plus a "presumed vanished" intermediate state surfaced as a coverage-gap nudge ("these hosts haven't been seen in 7 days; are they really gone?"). That intermediate state actually feeds back into Avenue 4 candidate #3 (coverage-completeness nudges) — so this decision has stickiness leverage too.

##### S3. Daemon-server protocol

Today daemons send wholesale topology output. Two options for the temporal model:

- **Keep wholesale, do reconciliation server-side.** Daemon code changes minimally; server compares incoming wholesale state against current temporal rows and generates inserts/updates/closes. Simplest transition.
- **Shift to per-entity deltas.** Daemon sends "host X added/changed/removed since last scan." More efficient on the wire, but requires daemons to track previous-scan state.

**Recommendation:** keep wholesale-send semantics with server-side reconciliation. Daemon code stays simple; the temporal model is server-side complexity. Revisit per-entity deltas later if bandwidth becomes the bottleneck.

##### S4. Branching and the editable-topology unification

The existing `topologies.parent_id` is scaffolded for branch-from-parent. Different options handle branching differently:
- **Snapshot-based options (A, B):** branching means forking the snapshot or diff stream into a parallel namespace. The current `parent_id` model already does this at the topology level.
- **Per-entity temporal options (C, F):** branching becomes "add a `branch_id` column (NULL = main branch) to entity tables; forks copy or reference entity rows into a branch namespace; editable topology is 'edit rows where `branch_id = your_branch`.'" Unifies branching with editable-topology under one mechanism.
- **Event-sourcing (E):** branching is forking the event stream and projecting both branches separately.

**Open question:** does this design pass commit to a branching model now, or just leave room for it? If editable-topology is months out, the change-digest feature can ignore branching entirely. If editable-topology is closer, the schema should be branch-aware from day one to avoid a second migration. This question's answer interacts with the option choice — pick the option first, then settle this.

##### S5. Per-plan retention enforcement

Self-hosted: user-configurable. Cloud: tier-gated.

Retention shape varies by option:
- **Snapshot-based (A, B):** retention is "keep last N snapshots" or "keep snapshots within window W." Simple to implement; coarse — every entity within a snapshot lives or dies together.
- **Per-entity temporal (C, F):** retention is a background trim — drop entity rows where `valid_to < now() - retention_window`. Granular; can apply different policies to different entity types if the use case demands.
- **Event-sourcing (E):** retention is event-stream truncation, with checkpointing to preserve current-state queryability.

**Open implementation questions:**
- Hard-delete vs. cold-storage archive table? Hard-delete is simpler; archive supports "we'll restore your history if you upgrade" reactivation flows.
- Is the retention boundary per-org, per-network, or per-entity-type? Likely per-org for v1; compliance use cases sometimes want longer retention on specific entity types but defer to demonstrated demand.

##### S6. Read-path performance

Read-path shape varies by option:
- **Snapshot-based (A):** "current state" is a single row fetch — fastest possible. "Historical state" is a single row fetch. No index design problem.
- **Diff-based (B):** "current state" is fetching the latest anchor + applying recent diffs. "Historical state" is replay from anchor.
- **Per-entity temporal (C):** "current state" becomes a multi-table join filtered by `valid_to IS NULL`; "historical state" filters on `valid_from <= T AND (valid_to IS NULL OR valid_to > T)`. Index design matters: `(network_id, valid_to)` partial index where `valid_to IS NULL`, `(entity_id, valid_from)` for entity history, `(network_id, valid_from, valid_to)` for range queries.
- **Hybrid (F):** "current state" reads the cache (cheap); "historical state" reads temporal entities or cached anchor snapshots.
- **Event-sourcing (E):** all reads go through projections; performance depends on materialization strategy.

The design pass should weigh expected read patterns against each option's performance shape — heavy current-state-read traffic with rare historical queries argues differently than balanced traffic. If a chosen option has a known read-path bottleneck, the cache approach (option F's pattern) can layer on as a deferred optimization rather than being pre-built.

### 2. Diff fidelity ("what counts as material")

Three buckets to settle:

- **Definitely material.** Host appeared, host vanished, new service detected on a host, new edge in topology, new subnet detected.
- **Probably material, signal/noise calibration needed.** Interface state changes (up→down), subnet membership changes, service identification confidence shifts (port previously identified as HTTP, now SSH), tag/group reassignments.
- **Probably noise.** Temporary host offline that recovers in the next scan, MAC vendor refinement, OS version-string detection delta, transient port flaps.

**Two open questions:**
- The default rule for the digest (which buckets get included)
- Whether users can configure it (per-org rules: "ignore VLAN changes," "only alert me on new hosts in subnet X")

The audit-trail facet imposes a stricter constraint than the digest: diff exports must be human-readable enough to serve as compliance evidence, in concrete network terms — not low-level state codes.

---

## Open UX questions

- **Digest cadence.** Weekly default? Configurable per org? Real-time on material-change?
- **Digest opt-in vs opt-out.** Opt-out by default for paid orgs (assume value, accept some unsubscribe rate as the price) or opt-in (no noise but slower adoption)? The Dashboard 10 unsubscribe-rate insight is what calibrates this *after* the fact — initial choice is a judgment call.
- **In-app changelog placement.** Timeline view anchored to the topology view, or a separate page? How does it relate to the existing topology consumption flow?
- **Snapshot pinning permissions.** Any seat / admin only / owner only? Naming conventions enforced or freeform?
- **Diff export format.** PDF, HTML, share-link, or all three? PDF likely required for the compliance use case.
- **Free tier behavior.** Do free orgs get any of this? Digest paid-gated is consistent with current scheduled-discovery gating. Audit-trail browse on free is more debatable — the compliance use case argues for at least read-only diff visibility.
- **Empty-state UX.** Orgs whose networks rarely change will get no-op digests. Surface this differently (skip the digest? send a "no changes this week" digest? show a coverage-gap nudge instead?).

---

## Dependencies and prerequisites

**Telemetry events shipping with the feature** (already pre-flagged in `telemetry-gap-backlog.md` co-shipping section CS-1, CS-2):
- `change_digest_sent`, `change_digest_opened`, `change_digest_clicked`
- `snapshot_pinned`, `snapshot_diff_viewed`, `snapshot_diff_exported`

**Soft prerequisite:** P1-9 (Brevo email open/click ingestion) for full digest engagement attribution. Without it, only click-through landings are measurable. Either ship P1-9 in Phase 3/4 in time, or accept partial measurability for the first weeks of Phase 2 rollout.

**No other engineering blockers.** Scheduled discovery infrastructure already works today (paid-gated). Email subsystem exists but is transactional only — building the queued digest pipeline is part of this feature's cost.

---

## Success metric

**Headline outcome:** D7 / D30 retention curve for orgs receiving ≥1 digest with material changes vs. orgs whose scheduled scans were no-ops. Lives in Dashboard 10 (Stickiness & Re-engagement, new in Phase 2). This is the experiment that justifies or kills the digest investment.

**Secondary metrics:**
- Digest engagement funnel (sent → opened → clicked → `topology_viewed` within 24h)
- Audit-trail usage by plan (`snapshot_pinned`, `snapshot_diff_exported` rates per org per week)
- Material-change frequency distribution (identifies orgs the digest can't help — they need a different mechanism)

**Anti-goal:** unsubscribe rate. Tracked but not optimized for — high unsubscribe is a signal that diff fidelity is wrong, not that digests should stop.

---

## Out of scope (explicit)

- **Pre-change blast-radius prompt** — depends on editable topology, which is a separate workstream
- **Rogue / unexpected device detection** — defers until diff infrastructure ships, then layers cheaply
- **Slack / Teams / Confluence / Hudu push integrations** — separate integrations roadmap
- **Trial-side mechanisms** (value recap, pause, in-app cancel, progressive banners) — these live in the Phase 5 conversion-side remediation workstream
- **Real-time alerting** — this is digest-cadence, not alert-cadence. Alert mechanisms could layer on later.
- **MSP-specific scheduled "state of your network" report** — gated on the MSP funnel actually existing

---

## Source references

- `/scanopy/docs/stickiness-candidates.md` (candidate #1 — full hypothesis, feasibility, P18 framing)
- `/scanopy-content/competitive-recurring-value-audit.md` (Auvik / SlurpIT / NetBox return-trigger context)
- `/scanopy-content/avenue-2a-followup-memo.md` (corrected diagnosis: dormancy is the dominant pattern; activation hypothesis lost direct support but underlying observation stands)
- `/scanopy/docs/transition-moment-audit.md` (Avenue 3 friction findings — for what this feature does *not* address)
- `/scanopy-content/icp.md` (S&C compliance persona for the audit-trail facet)
- `/scanopy-content/churn-remediation-phases.md` (Phase 2 in context with the other phases)
- `/scanopy/docs/telemetry-gap-backlog.md` (co-shipping events CS-1, CS-2; soft prerequisite P1-9)
