# Entity Lock Audit — In-Memory Locks → DB-Level Lock Design

Audit of every in-memory synchronization primitive that serializes entity operations in the backend, a cross-process verdict for each, and a design for one reusable DB-level lock primitive. All citations verified against source at `refactor/entity-lock-audit`.

> **STATUS: IMPLEMENTED (2026-07-13).** The design below has been built and all audited call sites migrated on this branch: `shared/storage/lock.rs` (typed `LockKey` over sqlx's built-in `PgAdvisoryLock`), `StorageTransaction::{lock, get_by_id_for_update, get_all}`, and migrations of L1–L4 and G1–G4. The in-memory `host_locks`, `service_locks`, and `dependency_update_lock` are deleted. **Cross-process re-validation** (two backend instances sharing one Postgres, concurrent requests split across them): host dedup now yields exactly 1 host + 9 validation errors on the user path and 1 host + 1 service on the daemon discovery path; IP and service positions come back unique 0..9. One design deviation discovered during validation: locking `create()` alone did NOT fix host dedup (the in-lock check is ID-based; the IP/MAC natural-key match runs earlier in `create_with_children`) — the `HostDedup` lock now spans `create_with_children` from the natural-key match through child persistence, and `create()` splits into a locked wrapper + `create_unlocked` (same shape as `delete_host`/`delete_host_inner` for consolidate re-entrancy). Line numbers in the audit text below refer to the pre-implementation code.

**Live validation (2026-07-10, local dev server, single backend process):** the highest-value races were exercised with concurrent API requests. **Confirmed live:** L1 host dedup — 10 parallel creates of the same new IP/MAC produced **2 duplicate hosts** on the user-API path (`ConflictBehavior::Error`) *and* 2 duplicate hosts on the daemon discovery path (`ConflictBehavior::Upsert`), each duplicate carrying its own copy of the submitted service. G1 IP positions — 10 parallel IP creates on one host yielded positions `[0, 0, 2, 3, …]` (duplicate + gap). L2 service positions — 10 parallel creates of different services on one host yielded positions `[0, 0, 0, 0, 0, 0, 1, 3, 3, 4]`. **Reasoned, not reproduced:** direct same-host service dedup (6 rounds × 10 concurrent discovery submissions produced no same-host duplicate) — the shared per-host lock upstream in `create()` staggers the requests, so each reaches service creation after the previous insert is visible; the window is real but narrow, and in practice the corruption surfaces via the duplicate-host path instead. All other races (host lost-update, consolidate, dependency members, scan_count, close_and_clone, junction syncs) are reasoned from code. Note these reproductions were **single-process** — every remaining site fails a fortiori across multiple backends, where the in-memory locks don't exist at all.

**Headline:** there is **zero DB-level locking anywhere in the backend today** — no advisory locks, no `SELECT … FOR UPDATE`, no `SKIP LOCKED`. The only DB-level concurrency control is `ON CONFLICT` in migrations (`shared/storage/migration_runner.rs:197`) and unique-violation → `ValidationError` mapping in `shared/storage/generic.rs:274`. Every serialization guarantee below evaporates the moment a second backend instance shares the database.

---

## Part 1 — Lock inventory

### 1A. Real cross-process races: in-memory locks guarding DB read-modify-write

These are the locks whose job a DB lock must take over.

#### L1. Per-host lock — `HostService.host_locks`

- **Primitive:** `Arc<tokio::sync::Mutex<HashMap<Uuid, Arc<Mutex<()>>>>>` — `backend/src/server/hosts/service/mod.rs:71`; accessor `get_host_lock()` at `hosts/service/consolidate.rs:97-103`.
- **Guards:** Host entity — discovery/API create with dedup, update, delete, consolidation. Acquisition sites, all keyed by `host.id`:
  - `create()` — `hosts/service/mod.rs:116-117`. Under the lock: `get_all(network).live()` (`mod.rs:122-123`), find match by `Host::eq` (**ID-only comparison**, `hosts/impl/base.rs:155-158`), then `upsert_host()` or `storage().create()`. Check-then-insert against the DB.
  - `update()` — `hosts/service/mod.rs:213-214`: `get_by_id` → `storage().update` (read-modify-write).
  - `delete_host()` — `hosts/service/delete.rs:25-26`: tag cleanup → `storage().delete`.
  - `consolidate_hosts()` — `hosts/service/consolidate.rs:266-267`: merges IPs/ports/services from `other_host` into `destination_host`. **Only the destination is locked; the source host is not**, so a concurrent update/delete of the source interleaves with the merge even single-process.
- **Race prevented:** two concurrent discovery submissions (or API create + discovery) for the same known host both read the host list, both miss each other's insert/update, and either create a duplicate row or overwrite each other's field merges (lost update in `upsert_host`).
- **Additional finding — the dedup race is open even WITH this lock (single-process):** the IP/MAC natural-key match runs in `create_with_children` (`hosts/service/create.rs:174-176`) **before any lock is held**; matching rewrites `host.id` to the existing host's ID (`create.rs:202`). For a genuinely **new** host, two concurrent submitters both find no match, keep **distinct fresh UUIDs**, acquire **distinct locks**, and the in-lock check (`Host::eq`, ID-only) can't catch the collision → both insert → duplicate hosts. There is no DB unique-constraint backstop because the natural key (IP/MAC) lives in the `ip_addresses` table, not on `hosts`. This is why the replacement dedup lock must be **scope-keyed** (per network), not ID-keyed.
- **Cross-process verdict: REAL RACE — CONFIRMED LIVE** (see header note: duplicates reproduced on both the API-create and daemon-discovery paths). Needs a DB-level lock. The dedup path additionally needs a natural-scope key to fix the fresh-UUID gap.

#### L2. Per-service lock — `ServiceService.service_locks`

- **Primitive:** `Arc<futures::lock::Mutex<HashMap<Uuid, Arc<Mutex<()>>>>>` — `backend/src/server/services/service/mod.rs:41`; accessor `get_service_lock()` at `services/service/lifecycle.rs:75-81`.
- **Guards:** Service entity — create with dedup + position assignment, update, delete, binding reassignment. Acquisition sites, keyed by `service.id`:
  - `create()` — `services/service/mod.rs:158-159`. Under the lock: `get_all(host).live()` (`mod.rs:162-163`), `next_position()` (`mod.rs:166`, `shared/position.rs:310`), dedup via `Service::eq` (a genuine natural-key comparison: host + network + service definition + container id, `services/impl/base.rs:145-176`) → `upsert_service()` or `storage.create()` + bindings + tags.
  - `update()` — `services/service/mod.rs:283-284`: `get_by_id` → validations → `update_dependency_members` → `storage.update` + binding/tag re-save.
  - `delete()` — `services/service/mod.rs:372-373`: `get_by_id` → `update_dependency_members` → tag cleanup → `storage.delete`.
  - `reassign_service_interface_bindings()` — `services/service/transfer.rs:20-21` (in-memory binding remap during host transfer, held for consistency with create/update).
- **Race prevented:** duplicate service rows from concurrent identical discovery submissions; lost updates on concurrent update.
- **Additional findings:**
  - Same fresh-UUID gap as hosts: unlike hosts, the in-lock dedup check IS natural-key (`Service::eq`), but the lock is keyed on `service.id` — a fresh UUID per new service — so two concurrent identical creates hold different locks and both pass the check before either inserts.
  - **Position assignment mis-keyed:** `next_position(&existing_services)` (`mod.rs:166`) computes MAX+1 across the host's services under a lock keyed by `service.id`. Two concurrent creates of *different* services on the same host never contend → duplicate `position` values.
- **Cross-process verdict: REAL RACE.** The position race is **CONFIRMED LIVE** (six services at position 0 from ten concurrent creates); the same-host dedup race is reasoned (narrow window, see header note). Dedup and position need host-scoped keys; update/delete need the ID key.

#### L3. Global dependency-members lock — `ServiceService.dependency_update_lock`

- **Primitive:** `Arc<futures::lock::Mutex<()>>` — `backend/src/server/services/service/mod.rs:40`, constructed at `lifecycle.rs:17`.
- **Guards:** Dependency entity `members` lists. `update_dependency_members()` (`services/service/dependencies.rs:5-80`) removes deleted services/stale bindings from every affected dependency and calls `dependency_service.update()` in a loop (`dependencies.rs:72-76`).
- **Race prevented:** two concurrent service deletes/updates both read the same dependency, each removes its own service from *its stale copy* of `members`, and the second write resurrects the first delete's member (lost update).
- **Additional finding:** the read that feeds the RMW (`dependency_service.get_all`, `dependencies.rs:13-15`) executes **before** the lock is acquired at `dependencies.rs:17` — so even single-process, two callers can both read, then serialize their conflicting writes through the lock without re-reading. The lock narrows the window; it does not close it.
- **Cross-process verdict: REAL RACE** (and partially broken as-is). Needs a DB lock scoped per network (the filter granularity), with the read moved inside the critical section.

#### L4. Discovery `scan_count` increment under session RwLock

- **Primitive:** `sessions: RwLock<HashMap<Uuid, DiscoveryUpdatePayload>>` — `backend/src/server/discovery/service/mod.rs:41`. In `update_session()` the write guard acquired at `dispatch.rs:255` is held for the whole body, incidentally covering a DB RMW at `dispatch.rs:387-414`: on session completion, `discovery_storage.get_by_id` (`:399`) → `scan_count += 1`, `force_full_scan = false` (`:401-406`) → `discovery_storage.update` (`:407`), plus `create` of the historical discovery row (`:418`).
- **Race prevented:** concurrent session-finalizations for the same parent discovery reading the same `scan_count` and both writing N+1 (lost increment; also lost `force_full_scan` reset).
- **Cross-process verdict: REAL RACE** — the in-memory RwLock serializes finalizations within one process only. (The RwLock's *primary* job — session state — is process-local and fine; only this embedded DB RMW leaks. The fix is to take the RMW out from under the RwLock into a locked transaction, which also stops holding an in-memory lock across DB I/O.)

### 1B. Real cross-process races with NO lock at all (gaps the same primitive should cover)

#### G1. IP address position assignment
`create_ip_address` handler (`backend/src/server/ip_addresses/handlers.rs:106-122`) calls `get_next_position_for_host()` (`ip_addresses/service.rs:104-107` — returns `count(existing)` as next position) then creates. No lock anywhere. Concurrent IP creates on one host get the same position. **REAL RACE — CONFIRMED LIVE** (positions `[0, 0, 2, 3, …]` from ten concurrent creates; duplicate positions violate the `validate_position_for_update` invariant the API enforces elsewhere).

#### G2. Snapshot close-and-clone
`SnapshotMutator::close_and_clone` (`backend/src/server/shared/services/traits.rs:708-724`), blanket-implemented for every `Snapshotable` entity: `begin_transaction` → `get_by_id` (plain SELECT, no `FOR UPDATE`) → create closed copy → update live row → commit. Two concurrent snapshot mutations of the same entity both read the same live row and create **two closed historical copies** for one lineage interval. **REAL RACE** — but the fix is row-level `FOR UPDATE`, not an advisory lock (the row exists and the operation is already transactional).

#### G3. Junction-table read-diff-write syncs
Pattern: read current junction rows (often *outside* the transaction), diff against the desired set, then insert/close/delete inside a transaction — no lock, and the inserted rows don't exist yet so `FOR UPDATE` on them is impossible. Sites (all `begin_transaction` calls verified):
- `tags/entity_tags.rs:341` (`set_tags` — read+diff at `:330-339` precedes the tx)
- `dependencies/dependency_members.rs:338` (`save_for_dependency` — delete-all + reinsert in tx; two concurrent savers can interleave into duplicate member rows)
- `vlans/impl/subnet_vlans.rs:275`, `daemons/impl/interfaced_subnets.rs:135`, `users/impl/network_access.rs:157`, `user_api_keys/impl/network_access.rs:157`, `credentials/impl/junction.rs:196,261,346,418`
**REAL RACE** (duplicate/conflicting junction rows), low frequency (mostly user-driven writes to the same parent entity). Advisory lock keyed on the parent row is the fit.

#### G4. API key rotation
`rotate_key` (`backend/src/server/shared/api_key_common.rs:195-235`, used by `user_api_keys/handlers.rs` and `daemon_api_keys/handlers.rs`): `get_by_id` → `set_key(new hash)` → `update`. Two concurrent rotations of the same key: last write wins; the loser is handed a plaintext key that authenticates nothing. **REAL RACE, low severity** (self-inflicted, no corruption — one caller gets a dead key and can retry). Verdict: cover with the ID-keyed advisory lock if/when the service adopts the primitive; not worth bespoke work before that.

### 1C. Process-local primitives — DB lock is the wrong tool

Honest verdicts: these guard in-memory state, singleton init, or throughput caps. A DB lock would be wrong or useless for all of them. Multi-instance caveats noted where behavior (not data integrity) degrades.

| # | Primitive | Location | Guards | Verdict |
|---|---|---|---|---|
| P1 | `login_attempts: Arc<RwLock<HashMap<Email,(u32,Instant)>>>` | `server/auth/service.rs:56` | Brute-force lockout counters (in-memory only) | Process-local. Multi-instance: attacker gets N× the attempt budget — an availability/security-hardening concern, not data corruption. If it ever matters, move the counter to DB/Redis; not this primitive's job. |
| P2 | `verification_resend_cooldown: Arc<RwLock<HashMap<Email,Instant>>>` | `server/auth/service.rs:58` | 60s resend rate limit | Process-local rate limiting. Same multi-instance note as P1. |
| P3 | `status: Arc<RwLock<LicenseStatus>>` | `server/license/service.rs:8` | Cached license validation, refreshed by background task | Process-local cache; each instance validates independently. Correct as-is. |
| P4 | Discovery session cluster: `sessions`, `daemon_sessions`, `discovery_sessions`, `daemon_pull_cancellations`, `running_snapshots`, `session_last_updated`, `job_ids` (all `RwLock`) | `server/discovery/service/mod.rs:41-54` | In-flight discovery session state, per-daemon queues, per-network snapshot exclusion | Process-local session bookkeeping — the sessions exist only in this process's memory, so in-memory locks are the right tool. **Multi-instance caveat (out of scope):** the state itself doesn't replicate — running multiple backends needs session state externalized (DB/queue), a much larger change than a lock swap. The one DB RMW hiding here is L4, extracted above. |
| P5 | `poll_semaphore: Arc<Semaphore>` (`MAX_CONCURRENT_POLLS`) | `server/daemons/service/mod.rs:120` | Caps concurrent ServerPoll daemon polls | Throughput cap per instance, not entity serialization. Correct as-is. |
| P6 | Event bus: `subscribers: Arc<RwLock<Vec<…>>>`, per-subscriber `pending: Arc<RwLock<Vec<Event>>>` | `server/shared/events/traits.rs:561-570, 501-507` | Subscriber registry + debounce buffers | Process-local pub/sub. Correct as-is. |
| P7 | Late-binding singletons: `OnceLock<Arc<HostService>>` etc. | `credentials/service.rs:123`, `daemons/service/mod.rs:117`, `services/service/mod.rs:38`, `discovery/service/mod.rs:61` | Circular-dependency injection, set once at wiring | Init-once. Correct as-is. |
| P8 | Config/data singletons: `plans: OnceLock<Vec<BillingPlan>>`, `OUI_DB`, `PROMETHEUS_HANDLE`, `RESOLVER`, `DUMMY_PASSWORD_HASH`, brevo domain tables | `billing/service/mod.rs:95`, `shared/oui.rs:26`, `shared/services/factory.rs:46`, `auth/email_domain.rs:33`, `auth/service.rs:468`, `brevo/domain_classification.rs:295-366` | Lazy-loaded constants | Init-once/read-only. Correct as-is. |
| P9 | Rate limiters: `RATE_LIMITERS: OnceLock<…DashMapStateStore…>`, per-share `LIMITER` | `auth/middleware/rate_limit.rs:44`, `shares/handlers.rs:53` | Request rate limiting (governor) | Process-local by design (per-instance quotas). Correct as-is. |
| P10 | `AppCache` (moka TTL cache) | `shared/handlers/cache.rs:6-13` | Response caching | Process-local cache. Correct as-is. |
| P11 | Daemon runtime: `EntityBuffer` RwLocks (`daemon/discovery/buffer.rs:53-60`), session manager (`daemon/discovery/manager.rs:15-16`), `ConfigStore` (`daemon/shared/config.rs:530-539`), progress atomics (`daemon/discovery/service/base.rs:70-88`, `ops.rs`), ARP thread coordination (`network/arp/broadcast.rs:183-189`), adaptive batch controller (`daemon/utils/scanner.rs:76-97`), `api_client OnceCell` (`daemon/shared/api_client.rs:83-90`) | daemon tree | Daemon-local scan state, buffers, config | The daemon has **no database** — it persists via HTTP to the server. Nothing here can use a DB lock; all races funnel into the server's API handlers, which is exactly where L1/L2 sit. Correct as-is. |
| P12 | `ExposeSecretsGuard` | `credentials/impl/types/secrets.rs:22-44` | Thread-local secret-serialization flag (RAII, not a mutex) | Not a concurrency primitive. Correct as-is. |
| P13 | Fixture-capture mutexes | `auth/middleware/fixture_capture.rs:38`, `daemon/shared/middleware.rs:13-30` | Test instrumentation | Test-only. Correct as-is. |

---

## Part 2 — Reusable DB-level lock design

### 2.1 Recommendation

**One primitive: Postgres advisory locks behind a typed `LockKey` enum**, living in a new `backend/src/server/shared/storage/lock.rs` (inside the module that is allowed to write SQL — services/handlers never see a SQL string). Exposed in two scopes from the same key type:

- **Session-scoped RAII guard** (`pg_advisory_lock` on a dedicated pool connection) — the workhorse. The big critical sections (host create/update/delete/consolidate, service create/update/delete, dependency members) are multi-service orchestrations making pool-based storage calls across entity types (bindings, tags, dependencies, child entities). They cannot be collapsed into one transaction without violating the no-cross-entity-storage rule, so transaction-scoped locks can't cover them.
- **Xact-scoped** (`pg_advisory_xact_lock` on an existing `sqlx::Transaction`) — for storage-local RMWs that are already in, or trivially movable into, a transaction (positions, junction syncs). Auto-released at commit/rollback; no guard management.

Complementary (not a second primitive): one generic method `StorageTransaction::get_by_id_for_update` (`SELECT … FOR UPDATE`) for the sites that mutate an **existing single row inside a transaction** — `scan_count` (L4) and `close_and_clone` (G2). Where the row exists, `FOR UPDATE` is strictly better than an advisory lock: zero false contention, self-cleaning, and the waiter re-evaluates the row after the winner commits.

**No schema, no migrations, no new dependencies** — advisory locks are a connection-level Postgres feature; `sqlx` and `sha2` are already in the tree.

### 2.2 Typed key API

```rust
// backend/src/server/shared/storage/lock.rs
use crate::server::shared::entities::EntityDiscriminants;

/// Closed registry of every DB-level lock in the system. Adding a lock site
/// means adding a variant here — keys are never built from ad-hoc strings,
/// so all lock scopes are auditable in one place and two sites can't
/// accidentally mint overlapping keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum LockKey {
    /// Serialize update/delete/consolidate of one host (replaces L1's id-keyed sites).
    Host(Uuid),
    /// Serialize host discovery dedup within a network (fixes the fresh-UUID gap).
    HostDedup { network_id: Uuid },
    /// Serialize update/delete of one service.
    Service(Uuid),
    /// Serialize service create-dedup + position assignment. Host-scoped:
    /// both the natural-key match (Service::eq is host-scoped) and
    /// next_position read the per-host service list.
    ServiceDedup { host_id: Uuid },
    /// Replaces the global in-memory dependency_update_lock, scoped per network.
    DependencyMembers { network_id: Uuid },
    /// MAX+1 position assignment for IPs on a host (no row exists to FOR UPDATE).
    IpPositions { host_id: Uuid },
    /// Junction-table read-diff-write sync, keyed by the parent entity row.
    JunctionSync { parent: EntityDiscriminants, parent_id: Uuid },
}

impl LockKey {
    /// Stable 64-bit advisory-lock key: first 8 bytes (big-endian i64) of
    /// SHA-256 over (variant name ++ 0x00 ++ payload bytes). Stable across
    /// processes and releases — required for rolling deploys where old and
    /// new instances must contend on the same keys. (std's DefaultHasher is
    /// explicitly NOT stable across releases; sha2 is already a dependency.)
    /// A collision only causes false contention, never lost mutual
    /// exclusion — acceptable at 64 bits.
    fn to_pg_key(self) -> i64 { /* hash as described */ }
}
```

Why a closed enum over a generic `LockKey::of(discriminant, bytes)` builder: every lock scope is declared and documented in one greppable place, payload shape is compile-enforced per variant, and a new site costs one variant. This matches the project's compile-enforced-invariants preference.

### 2.3 Functions, guard, errors

```rust
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("timed out after {0:?} waiting for lock {1:?}")]
    Timeout(Duration, LockKey),
    #[error("deadlock detected acquiring {0:?}")] // pg SQLSTATE 40P01
    Deadlock(LockKey),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(30);

// -- xact-scoped: composes with StorageTransaction / *_in_tx flows ----------
pub async fn xact_lock(
    tx: &mut sqlx::Transaction<'_, Postgres>, key: LockKey, timeout: Duration,
) -> Result<(), LockError>;   // SET LOCAL lock_timeout='…'; SELECT pg_advisory_xact_lock($1)

// -- session-scoped: for pool-based multi-service critical sections ---------
pub struct SessionLockGuard { conn: Option<PoolConnection<Postgres>>, key: LockKey }

pub async fn session_lock(pool: &PgPool, key: LockKey, timeout: Duration)
    -> Result<SessionLockGuard, LockError>;
/// Multi-lock acquire: sorts keys by to_pg_key ascending before acquiring —
/// the process-wide deadlock-avoidance rule. Callers never hand-order.
pub async fn session_lock_many(pool: &PgPool, keys: &[LockKey], timeout: Duration)
    -> Result<Vec<SessionLockGuard>, LockError>;

impl SessionLockGuard {
    /// Happy path: SELECT pg_advisory_unlock($1); connection returns to the pool.
    pub async fn release(self) -> Result<(), LockError>;
}
impl Drop for SessionLockGuard {
    /// Error/panic/cancel path: detach the connection from the pool and drop
    /// it — closing the session releases its advisory locks server-side.
    /// A connection holding a session lock must never return to the pool.
    fn drop(&mut self) { if let Some(c) = self.conn.take() { drop(c.detach()); } }
}
```

Timeout strategy differs per scope, deliberately:
- **Xact scope:** `SET LOCAL lock_timeout` — the server aborts the wait (SQLSTATE `55P03` → `LockError::Timeout`); `SET LOCAL` resets at transaction end, so no pooled-connection state pollution.
- **Session scope:** `tokio::time::timeout` around plain `SELECT pg_advisory_lock($1)`; on expiry, detach-and-drop the connection to kill the server-side wait. No `SET` on pooled connections at all. Cost: one dedicated pool connection for the duration of the critical section — the same serialization the in-memory lock imposed, now correct across instances.

Convenience surface so call sites never touch sqlx types:

```rust
impl<T: Storable> GenericPostgresStorage<T> {
    pub async fn session_lock(&self, key: LockKey, timeout: Duration)
        -> Result<SessionLockGuard, LockError>;          // delegates to self.pool
}
impl<'a, T: Storable> StorageTransaction<'a, T> {
    pub async fn lock(&mut self, key: LockKey, timeout: Duration) -> Result<(), LockError>;
    /// SELECT * FROM {table} WHERE id=$1 [AND valid_to IS NULL when T::HAS_SCD2] FOR UPDATE
    pub async fn get_by_id_for_update(&mut self, id: &Uuid) -> Result<Option<T>, anyhow::Error>;
}
```

Placement respects every project boundary: raw SQL only inside `shared/storage/`; services acquire locks through their own storage handle (no cross-entity storage access); keys are typed (`EntityDiscriminants` + `Uuid`), not strings.

### 2.4 Advisory vs `FOR UPDATE` — which fits which site

`FOR UPDATE` fits when (a) the contended row already exists and (b) the whole critical section is one transaction. Advisory locks fit dedup races (the row doesn't exist yet), gap problems (MAX+1 / set-diff inserts), and pool-based multi-service sections that can't become one transaction.

| Audited site | Primitive | Key | Semantics |
|---|---|---|---|
| L1 host create dedup (`hosts/service/mod.rs:116`) | Advisory, session guard | `HostDedup { network_id }` | Block, 30s (discoveries queue) |
| L1 host update / delete | Advisory, session guard | `Host(id)` | Block, 30s |
| L1 consolidate (`consolidate.rs:266`) | Advisory, `session_lock_many` | `[Host(dest), Host(other)]`, sorted | Block, 10s; timeout → validation error to the user. Also fixes the unlocked-source gap. |
| L2 service create dedup + position (`services/service/mod.rs:158`) | Advisory, session guard | `ServiceDedup { host_id }` | Block, 30s. One key covers both dedup and `next_position`, fixing the mis-keyed position race. |
| L2 service update / delete / transfer | Advisory, session guard | `Service(id)` | Block, 30s |
| L3 dependency members (`dependencies.rs:17`) | Advisory, session guard | `DependencyMembers { network_id }` | Block, 30s. Acquire **before** the `get_all` read (fixes the read-outside-lock bug). Strictly better than today's global mutex: per-network and cross-instance. |
| L4 scan_count (`dispatch.rs:387-414`) | **FOR UPDATE** | — (row lock on the discovery row) | Move the RMW out from under the sessions RwLock: resolve `discovery_id`, drop the guard, then `begin_transaction` → `get_by_id_for_update` → increment → `update` → commit. |
| G1 IP positions (`ip_addresses/service.rs:104`) | Advisory, xact-scoped | `IpPositions { host_id }` | Move count + create into one `StorageTransaction`; `tx.lock(...)` first. Block, 30s. |
| G2 close_and_clone (`shared/services/traits.rs:710`) | **FOR UPDATE** | — | Replace the plain `get_by_id` with `get_by_id_for_update`. Under READ COMMITTED a second waiter re-evaluates `valid_to IS NULL` after the winner commits, sees the row closed, gets zero rows → correct "already closed" outcome. |
| G3 junction syncs (sites in §1B) | Advisory, xact-scoped | `JunctionSync { parent, parent_id }` | One `tx.lock(...)` at tx start; move the pre-read inside the tx. Block, 30s. |
| G4 rotate_key (`api_key_common.rs:195`) | Advisory, session guard (when adopted) | `Service`-style ID variant added at adoption time | Block, short. Low priority. |

All sites block-and-wait; `try_` variants (`pg_try_advisory_xact_lock`) can be added to the same module later if a skip-work site appears — none exists today.

### 2.5 Deadlock avoidance

One rule: **a site acquiring more than one lock uses `session_lock_many`, which sorts by `to_pg_key`**; never acquire a second guard while holding one outside that helper. Session guards are acquired before opening any transaction that takes xact locks (the categories above never interleave the other way). Postgres's deadlock detector is the backstop: `40P01` maps to `LockError::Deadlock` and fails loudly instead of hanging.

### 2.6 Migration / deployment note

**None required.** Advisory locks are schema-free, transaction/session-scoped, auto-released on disconnect, and work across any number of backend instances sharing the database. No leases table is proposed. One operational note: lock keys must remain stable across a rolling deploy (old + new instances contending on the same entities) — hence the release-stable hash and the key-stability regression test below.

### 2.7 Testing the primitive

Unit tests in `lock.rs` against the existing testcontainer harness (`src/tests/mod.rs:45` `setup_test_db()` — plain `PgPool`, no fixtures needed since advisory locks are schema-free):

1. **Key stability regression:** `LockKey::Host(known_uuid).to_pg_key() == <hardcoded i64>` — catches a hash swap that would silently break rolling-deploy compatibility. Plus: distinct variants over the same UUID → distinct keys.
2. **Mutual exclusion:** guard held → second acquire on same key times out; different key succeeds; after `release()` succeeds.
3. **Xact auto-release:** lock in tx1 → tx2 blocked; commit/rollback/drop tx1 → tx2 proceeds.
4. **Guard drop releases:** drop without `release()` → short-timeout reacquire succeeds (retry loop; server-side close is async).
5. **Timeout mapping:** both scopes produce `LockError::Timeout` (tokio elapse / SQLSTATE 55P03).
6. **Ordering under contention:** two tasks loop acquiring `{A,B}` and `{B,A}` via `session_lock_many` — zero `40P01`s (fails reliably if sorting is removed).
7. **Cross-process semantics:** two separate `PgPool`s to the same container — proves exclusion is DB-level, not in-process.

The races themselves are validated by `TEST_PLAN.json` (worktree root): each test fires concurrent API requests that fail (produce corruption) without the lock and pass with it.

### 2.8 Adoption shape (follow-up work, needs coordinator sign-off)

- **Category A — service orchestration (L1, L2, L3):** replace the in-memory lock-acquisition lines with a `session_lock` guard; the body is untouched. Delete `host_locks`, `service_locks`, `dependency_update_lock`, `get_host_lock`, `get_service_lock`. For L3, move the `get_all` read after the acquire.
- **Category B — pool-based single-entity RMW (L4, G1):** wrap in a `StorageTransaction` with `tx.lock(...)` or `get_by_id_for_update`. L4 additionally moves the DB work out from under the sessions RwLock.
- **Category C — already-in-tx flows (G2, G3):** one added lock/`FOR UPDATE` line at transaction start; junction pre-reads move inside the tx.

No call sites were migrated in this branch — audit, design, and test plan only, per task scope.
