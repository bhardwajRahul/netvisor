# Phase 5 data model — implementation plan

Implementation reference for the Phase 5 data model: typed-payload `BillingOperation`, trait-based EventBus filter, `subscription_events` ledger, `SubscriptionService` for derived reads, drop plan/status/trial_end_date columns from `organizations`. Output of the design pass; companion to `docs/phase5-spec.md` (which has the feature decisions this data model serves).

## What this is for

Phase 5 ships in-app cancel + save offers (pause, discount), pause-as-feature with 6-month rolling eligibility, trial extend (+7d, once per lifetime), downgrade banner with 14-day persistence, and a "what changed" page. Each of those features needs to record subscription lifecycle events, derive current state from history, and check eligibility from history. Today's billing data model can't support any of that cleanly: `organizations.plan` is a JSONB current-state snapshot with no history, the only durable record of subscription events is what gets emitted to PostHog (forward-only, no DB persistence, ruled out for backfill), and `BillingEvent` carries `metadata: serde_json::Value` that's untyped and would silently grow shape drift as new operations are added.

This plan establishes a `subscription_events` DB ledger, refactors `BillingOperation` to a typed-payload sum type, makes the EventBus filter generic via a trait, and pulls all subscription state derivation into a new `SubscriptionService`. After this lands, every Phase 5 feature can emit a typed event when something happens and read derived state when it needs to know eligibility / current plan / when something last happened.

## Concerns and where they live

1. **Org identity** — `organizations` table.
2. **Stripe customer linkage** — `organizations.stripe_customer_id`, `organizations.has_payment_method`.
3. **Plan catalog** — code-defined (`backend/src/server/billing/plans.rs` + `types/base.rs`). No DB representation.
4. **Subscription state (current plan, status, trial info)** — derived from queries against `subscription_events`. No columns on `organizations` for these.
5. **Subscription history** — `subscription_events` DB table, populated by a new subscriber that consumes `BillingEvent` from the event bus.

## Design

### EventBus refactor — trait-based filter, typed payloads

A new trait constrains every event type's operation to be a sum type with a derivable discriminant:

```rust
pub trait Event {
    type Operation: IntoDiscriminant + Serialize + DeserializeOwned + Clone + Send + Sync;
    fn operation(&self) -> &Self::Operation;
}
```

`IntoDiscriminant` is the strum trait auto-implemented by `#[derive(EnumDiscriminants)]`. It exposes `operation.discriminant() -> Self::Discriminant`.

`EventFilter` becomes generic on the event type, filtering by the discriminant-enum values:

```rust
pub struct EventFilter<E: Event> {
    operations: Option<Vec<<E::Operation as IntoDiscriminant>::Discriminant>>,
    // other filter dimensions (network_id, organization_id) preserved
}
```

`None` = wildcard (today's behavior). `Some(vec![...])` filters to listed discriminants.

**No hand-rolled "Kind" types** — discriminant enum is whatever strum generates from the macro (default name `<EnumName>Discriminants`).

**No `metadata: serde_json::Value` on event structs** — operation payload is the only data carrier. Emission sites attach context by adding fields to the relevant operation variant.

### BillingOperation — typed payload sum type

Refactor today's unit `BillingOperation` (`backend/src/server/shared/events/types.rs:448-460`) to carry per-variant data. Externally-tagged via `#[serde(tag = "type")]` (same pattern as `BillingPlan` at `types/base.rs:30-41`).

```rust
#[derive(Serialize, Deserialize, Clone, Debug, EnumDiscriminants)]
#[serde(tag = "type")]
#[strum_discriminants(derive(Display, Hash, EnumIter, Serialize, Deserialize))]
pub enum BillingOperation {
    // EXISTING — refactored from unit + metadata to typed:
    CheckoutStarted { session_id: String },
    CheckoutCompleted { session_id: String, plan: BillingPlan, included_networks: u64, included_seats: u64 },
    TrialStarted { plan: BillingPlan, trial_end: DateTime<Utc>, trial_days: u32 },
    TrialWillEnd { has_payment_method: bool },
    TrialEnded { converted: bool, plan: BillingPlan },
    PlanChanged { from: BillingPlan, to: BillingPlan, is_downgrade: bool },
    SubscriptionCancelled {                                 // cancel confirmed at period end
        plan: BillingPlan,
        reason_code: Option<CancelReason>,
        stripe_feedback: Option<StripeFeedback>,
        comment: Option<String>,
        period_end: DateTime<Utc>,
    },
    PaymentFailed { invoice_id: String, amount_cents: i64 },
    PaymentActionRequired { invoice_id: String },
    PaymentRecovered { amount_cents: i64 },
    FeatureLimitHit { limit_type: LimitType, current_count: u64, limit: u64 },

    // NEW — added for Phase 5:
    Paused { duration_days: u32, resumes_at: DateTime<Utc>, plan: BillingPlan },
    Resumed { was_early: bool },
    TrialExtended { days_added: u32, new_trial_end: DateTime<Utc> },
    CancellationInitiated {
        reason_code: CancelReason,
        stripe_feedback: Option<StripeFeedback>,
        comment: Option<String>,
        save_offer_shown: Vec<SaveOffer>,
        save_offer_redeemed: Option<SaveOffer>,
        planned_period_end: DateTime<Utc>,
    },
    PaymentMethodAdded,
    PaymentMethodRemoved,
}
```

`BillingEvent` simplifies — drops `metadata` field:

```rust
pub struct BillingEvent {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub operation: BillingOperation,        // typed — carries all per-variant data
    pub timestamp: DateTime<Utc>,
    pub authentication: AuthenticatedEntity,
    // metadata field DROPPED
}

impl Event for BillingEvent {
    type Operation = BillingOperation;
    fn operation(&self) -> &BillingOperation { &self.operation }
}
```

`SqlValue::BillingOperation(BillingOperation)` variant added to `backend/src/server/shared/storage/traits.rs`, mirroring `SqlValue::OptionBillingPlan` precedent (L220).

### Other event types — minimal touch in this phase

`EntityEvent` keeps its existing shape (`Entity` enum at `entities.rs:62` stays — variants carry typed entity structs like `Entity::Host(Host)`).

For the trait-based filter to apply uniformly, `EntityOperation` (today: `Created/Updated/Deleted` unit) trivially satisfies `Event::Operation` — a unit enum's discriminant is itself.

`EntityEvent.metadata` field stays in this phase. The cleanup precedent (drop metadata, refactor to typed payloads) applies but is **out of scope** here. Same for `AuthEvent` and `OnboardingEvent`. Follow-up work can extend the pattern.

### Schema

```
organizations  (drops plan, plan_status, trial_end_date columns)
├── id uuid PK
├── name text, ...
├── stripe_customer_id text NULL
└── has_payment_method bool

subscription_events  (NEW — DB-backed mirror of BillingEvent)
├── id uuid PK                              -- = BillingEvent.id
├── organization_id uuid FK → organizations -- denormalized for index/query
├── operation jsonb NOT NULL                -- typed BillingOperation, externally-tagged via #[serde(tag = "type")]
└── occurred_at timestamptz NOT NULL        -- = BillingEvent.timestamp, denormalized for index/query
```

No separate operation discriminator column — JSONB's `type` tag IS the discriminant.

Indexes:
- `(organization_id, occurred_at DESC)` — "latest event for org" queries.
- Expression index on `(organization_id, (operation->>'type'), occurred_at DESC)` — "latest event of kind X for org" (powers most derivation queries).

### New subscriber: SubscriptionEventStorageSubscriber

Pattern mirrors `posthog/subscriber.rs:94-159`:

- Implements the new generic `EventSubscriber` trait (refactored to work with the typed `Event` trait).
- Filter: `EventFilter::<BillingEvent> { operations: None, ... }` — None = wildcard, all variants present and future flow in automatically.
- `handle_events(...)` writes one row per event:
  ```sql
  INSERT INTO subscription_events (id, organization_id, operation, occurred_at)
  VALUES (event.id, event.organization_id, jsonb(event.operation), event.timestamp)
  ```
- Registered in `backend/src/server/shared/services/factory.rs:382`.

### Existing subscribers — refactor to typed payloads

Every consumer of `BillingEvent.metadata` updates (the field doesn't exist anymore):

- **`backend/src/server/posthog/subscriber.rs:94-268`** — refactor each `Event::Billing(billing_event)` branch to pattern-match on `billing_event.operation` typed variants. Filter list stays explicit (today's 11 operations); team decides whether to extend the filter for new variants. Likely yes for `Paused`, `Resumed`, `TrialExtended`, `CancellationInitiated`, `PaymentMethodAdded`/`Removed` (all useful for funnel/retention analytics).
- **`backend/src/server/logging/subscriber.rs`** — uses `EventFilter::all()`. Refactor to format from typed variants.
- **`backend/src/server/metrics/subscriber.rs`** — same pattern.
- No other subscribers consume billing events today.

Each refactor is mechanical — replace `metadata.get("field").and_then(...)` with typed accessors via pattern matching.

### Read-side derivations — SubscriptionService

New service at `backend/src/server/billing/subscription_service.rs` (or wherever fits the existing service organization conventions). Provides:

```rust
async fn current_plan(org_id: Uuid) -> Option<BillingPlan>
    // Find latest event with operation->>'type' IN
    //   ('TrialStarted', 'CheckoutCompleted', 'PlanChanged', 'SubscriptionCancelled')
    // Deserialize and extract:
    //   PlanChanged → to
    //   TrialStarted | CheckoutCompleted → plan
    //   SubscriptionCancelled → Free (from code catalog)

async fn current_status(org_id: Uuid) -> Option<PlanStatus>
async fn current_trial_end(org_id: Uuid) -> Option<DateTime<Utc>>

async fn last_paused_at(org_id: Uuid) -> Option<DateTime<Utc>>
    // SELECT MAX(occurred_at) WHERE operation->>'type' = 'Paused' AND organization_id = $1

async fn has_used_trial_extend(org_id: Uuid) -> bool
    // SELECT EXISTS(... WHERE operation->>'type' = 'TrialExtended' AND organization_id = $1)

async fn most_recent_downgrade(org_id: Uuid) -> Option<(BillingPlan, DateTime<Utc>)>
    // Latest event where (operation->>'type' = 'PlanChanged' AND operation->>'is_downgrade' = 'true')
    //   OR operation->>'type' = 'SubscriptionCancelled'
```

**Pure-derived per founder direction.** No denormalized columns on `organizations`. If perf becomes an issue during implementation, denormalized cache columns are an explicit escape hatch — but only after measurement.

### Read-site refactor (16 files)

Current readers of `org.base.plan` / `plan_status` / `trial_end_date`:

- **Hot-path middleware** (`auth/middleware/features.rs`, `billing.rs`, `auth.rs`, `demo_mode.rs`): replace `.base.plan` access with `subscription_service.current_plan(org_id).await?`. Likely refactor to populate `FeatureCheckContext.plan` from the service method early in request handling, so per-feature checks don't repeat the query.
- **Per-call** (11 files: `billing/service.rs`, `billing/handlers.rs`, `billing/subscriber.rs`, `dashboard/handlers.rs`, `discovery/service.rs`, `daemons/service.rs`, `email/traits.rs`, `hosts/handlers.rs`, `shares/handlers.rs`, `organizations/handlers.rs`, `auth/handlers.rs`): same pattern.
- **Storage layer** (`organizations/impl/storage.rs:92, 110-130, 142-143`): drop `plan`, `plan_status`, `trial_end_date` SqlValue bindings + serialization.

### Emissions to add

Today's `BillingEvent::new(...)` call sites refactor to construct typed operation variants instead of packing payload into `metadata: json!({...})`. New emissions added for new variants:

- `Paused` — new pause handler (Phase 5; from in-app cancel save offer).
- `Resumed` — new resume handler + auto-resume reflection (when Stripe webhook confirms `pause_collection` cleared).
- `TrialExtended` — new trial-extend handler (Bundle A).
- `CancellationInitiated` — new in-app cancel handler (Bundle B item 7).
- `PaymentMethodAdded` — `handle_payment_method_attached` at `service.rs:1353` (pairs with quick-win item 5).
- `PaymentMethodRemoved` — `handle_payment_method_detached` at `service.rs:1389`.

## Build sequence

1. **Add `Event` trait.** Refactor `EventBus` / `EventFilter` (`backend/src/server/shared/events/bus.rs`) to be generic on the trait.
2. **`BillingOperation` refactor.** Derive `EnumDiscriminants`; convert variants from unit to typed; drop `BillingEvent.metadata` field.
3. **Refactor every `BillingEvent::new(...)` call site** (in `billing/service.rs`, `billing/handlers.rs`) to construct typed operation variants.
4. **Refactor existing subscribers** (PostHog, Logging, Metrics) to consume typed `BillingOperation` variants instead of `metadata`.
5. **Add new BillingOperation variants** (`Paused`, `Resumed`, `TrialExtended`, `CancellationInitiated`, `PaymentMethodAdded`, `PaymentMethodRemoved`).
6. **Add `SqlValue::BillingOperation` variant** + storage layer additions.
7. **Create `subscription_events` table** + migration with the index list above.
8. **Add `SubscriptionEventStorageSubscriber`**; register in factory.
9. **Backfill seed events** for existing orgs:
   - For each org with `plan` set: insert a `TrialStarted` or `CheckoutCompleted` event at `occurred_at = org.created_at` with current plan + trial info captured as the typed BillingOperation variant.
   - For orgs whose `plan_status` is non-default: insert the corresponding event so derived queries return the right current state.
   - Pre-existing downgrade history is unrecoverable — derived `last_paused_at`, `most_recent_downgrade`, `has_used_trial_extend` return null/false for pre-migration data. Acceptable: 14-day banner window has passed; eligibility starts fresh.
10. **Add `SubscriptionService`** with derivation methods.
11. **Refactor 16 read sites** to call `SubscriptionService`.
12. **Drop `organizations.plan`, `plan_status`, `trial_end_date` columns.**

**Ordering constraints:** Steps 1–4 must ship together (otherwise compilation breaks for consumers of the dropped `metadata` field). Steps 7–9 must precede 10–11. Step 12 is last.

## Critical files

- `backend/src/server/shared/events/types.rs:446-547` — `BillingEvent` + `BillingOperation` (refactor to typed sum, drop metadata).
- `backend/src/server/shared/events/bus.rs:24, 130-202` — `EventSubscriber` + `EventFilter` (refactor to be generic on `Event` trait).
- `backend/src/server/shared/services/factory.rs:382` — subscriber registration.
- `backend/src/server/posthog/subscriber.rs:94-268` — refactor to consume typed BillingOperation; extend filter list for new variants.
- `backend/src/server/logging/subscriber.rs`, `metrics/subscriber.rs` — refactor to consume typed payloads.
- `backend/src/server/shared/entities.rs:62` — `Entity` enum; precedent for typed-payload sum + EnumDiscriminants derive.
- `backend/src/server/shared/storage/traits.rs:220` — `SqlValue::OptionBillingPlan` (precedent for SqlValue variant).
- `backend/src/server/billing/types/base.rs:30-41` — `BillingPlan` (precedent for externally-tagged enum-as-JSONB).
- `backend/src/server/organizations/impl/storage.rs:92, 110-130, 142-143` — drops plan/plan_status/trial_end_date bindings.
- `backend/src/server/auth/middleware/{features,billing,auth,demo_mode}.rs` — hot-path middleware refactors.
- `backend/src/server/billing/service.rs` — all `BillingEvent::new()` emissions refactored; reads switched to `SubscriptionService`; `handle_payment_method_attached:1353` + `handle_payment_method_detached:1389` get new emissions.
- `backend/src/server/billing/handlers.rs` — emission sites refactored.
- `backend/migrations/` — new migration: create `subscription_events` (with required boilerplate per CLAUDE.md migration rules: `SET lock_timeout = '5s';` first; `CREATE INDEX CONCURRENTLY` with sqlx `-- no-transaction` header); backfill seed events; drop columns.
- `backend/Cargo.toml` — verify `strum` already provides `EnumDiscriminants` + `IntoDiscriminant` (it should; `Entity` already uses `EnumDiscriminants`).

## Verification

- `cd backend && cargo test --lib` — existing 310 tests pass; add tests for `SubscriptionService` derivation methods, the new persistence subscriber's INSERT behavior, round-trip serialization of typed BillingOperation variants.
- Manual on dev: trigger a pause → verify `subscription_events` row inserted with typed `Paused` operation and correct fields; verify `current_plan(org_id)` returns the same value as the pre-refactor `org.plan` did; trigger a trial-extend → verify `has_used_trial_extend(org_id)` returns true.
- `make lint-migrations` passes.
- PostHog still receives the operations it filtered on before (verify no analytics regression).
- New `subscription_events` table appears in `get_entity_deserializers()` in `backend/src/server/shared/storage/tests.rs` per CLAUDE.md requirements.

## Out of scope (explicit)

- Frontend work (in-app cancel modal, save offer panels, downgrade banner, recovery affordances, trial countdown, value recap card, first-invoice display, post-Stripe confirmation) — separate worktree(s); see `docs/phase5-spec.md`.
- New backend endpoints (`POST /api/billing/cancel`, `/pause`, `/resume`, `/extend-trial`, `/checkout-preview`) — separate worktree(s) that depend on this data model landing first.
- Stripe SDK integration for pause/extend/cancellation_details/discounts — separate worktree(s).
- Email work (quick wins items 5/8/9, downgrade email rewrite, trial value recap email) — separate worktree(s).
- Refactoring `EntityEvent` / `AuthEvent` / `OnboardingEvent` to typed payloads — same architectural pattern applies; deliberate follow-up scope, not Phase 5.
