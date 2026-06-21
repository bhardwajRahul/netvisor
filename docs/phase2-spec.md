# Phase 2 spec — change-detection digest + audit-trail facet

> **Status as of 2026-06.** This spec is part shipped, part never-built. Read this header before trusting any section below.
>
> **Shipped and load-bearing:** the SCD2 substrate (`valid_from` / `valid_to` / `last_seen_at` time-tracked entity rows), first-class snapshots, plan-gated retention, the unified topology read path (current-state and as-of filters), and the per-discovery-session digest email. References: `backend/src/server/shared/storage/snapshot.rs`, `backend/src/server/snapshots/`, `backend/src/server/digest/`, migration `20260502000000_scd2_add_columns.sql`, commits `1b6745eec` / `c3fdae9b4` / `d78428a7f` / `3a71e944f`.
>
> **Kept as accurate reference:** the **Data model**, **Schema additions**, **Daemon-server protocol**, and daemon/server reconciliation sections (plus the **Investigation** appendix) describe what shipped and remain trustworthy.
>
> **Never built (no planned-work successor):**
> - Scheduled snapshots (cron-cadence snapshot firing).
> - Snapshot pinning, naming, and the `kind` (manual/scheduled) distinction.
> - Per-org digest cadence configuration.
> - `digest_send_empty` / the "nothing changed" digest email.
> - Paid-gating of digest opt-out.
> - Discovery-failure digest variant.
> - "Presumed vanished" coverage-gap surface (the stale-`last_seen_at` read query / nudge UI).
> - In-app changelog: the Activity page and the topology-anchored timeline pane.
> - PDF / HTML diff export.
>
> **Two factual divergences from this spec as written:**
> - The digest is **per-discovery-session and per-user**, not weekly and per-org.
> - Snapshot reads are discrete `snapshot_id`-stamped copies, not as-of-arbitrary-`T` temporal queries.
>
> **Provenance:** this spec's architecture choice (Option C / SCD2) originated in the now-deleted `change-digest-design-brief.md`; that brief's options analysis is superseded by what shipped. Full audit in `DOCS_AUDIT_2026-06.md`.

---

Output of the founder + Claude Code interactive design pass. This is the spec workers should build against. The pre-pass investigation that fed the design pass is folded in as the **Investigation** appendix below — every claim about current code carries `file:line` citations.

The originating brief (since deleted; see provenance note above) framed the architectural choice as option C (per-entity SCD Type 2 temporal tables); the design pass confirmed it and refined it to a **snapshot-driven** SCD2 — continuous scans only update live rows in place; `valid_from` / `valid_to` are touched only when a snapshot fires. The model below is the spec.

This spec settles direction. Implementation specifics that the founder did not weigh in on (migration backfill mechanics, snapshot close-and-clone timing, exact index columns, plan-tier-specific defaults beyond directional examples) are flagged as **Open for implementer** within each section. The implementer surfaces those for review before building.

---

## Decisions at a glance

1. **Architecture: snapshot-driven per-entity time-tracked rows.** Each entity row carries `valid_from`, `valid_to`, and `last_seen_at`. Continuous scans only update live rows in place — no row inserts on field changes. Snapshots (manual or scheduled) are the only mechanism that closes rows and the only mechanism that produces audit-trail content.
2. **Identity model:** unchanged from today (server-assigned surrogate, natural-key matching). The daemon-state-loss fingerprint tier is **deferred** to v2; v1 ships with current IP+MAC matching and accepts that NIC-swap-during-daemon-outage produces a duplicate host.
3. **Vanish: implicit.** A row whose `last_seen_at` is stale stays live (`valid_to IS NULL`). It is surfaced as "presumed vanished" via a read query (`last_seen_at < NOW() - threshold AND valid_to IS NULL`) which drives the coverage-gap nudge surface. There is no row-close sweep — `valid_to` is only set by snapshot creation.
4. **Retention:** snapshot-window-based, paid-tier lever. Daily background job deletes snapshots past the org's window and the closed entity rows they anchor. Pinned snapshots exempt. Founder's directional example: free 1 week, pro 1 month; higher tiers longer.
5. **Diff fidelity:** the audit trail is the closed-row sets across snapshots — natural-key changes manifest as vanish+appear pairs across consecutive snapshots; descriptor changes (hostname, oper_status, vendor refinement) update in place and are not in the audit trail by design. Per-org configurability deferred.
6. **UX:** > NOT BUILT as of 2026-06 — see status header. (Scheduled snapshots, per-org digest cadence, pinning/naming, changelog/Activity page, PDF/HTML export, and empty-digest behavior were never built. The shipped digest is per-discovery-session and per-user.)
7. **Branching:** no `branch_id` column on the v1 schema. Existing `topologies.parent_id`-anchored locks migrate into the snapshot mechanism (manual + pinned). The planning-edit feature, when it lands, adds `branch_id` via expand-and-contract.
8. **S4 — branching/editable-topology for planning edits:** out of scope for Phase 2 per TASK.md.

---

## Data model

The per-entity tables (`hosts`, `ip_addresses`, `interfaces`, `ports`, `services`, `bindings`, `subnets`) become time-aware. Each row represents one version of one entity:

- **Live (`valid_to IS NULL`):** the entity exists right now.
- **Closed (`valid_to IS NOT NULL`):** the entity existed in this state from `valid_from` to `valid_to`; superseded by the next live row created at the next snapshot.

There is at most **one live row per natural key** per entity at any time. The natural-key matching logic in S1.1 of the Investigation determines uniqueness; the existing logic is unchanged.

### Three columns, three drivers

- **`valid_from`** — set by snapshots and by initial discovery. On initial-discovery insert, `valid_from = NOW()`. On snapshot-clone, `valid_from = snapshot_timestamp`.
- **`valid_to`** — set only by snapshot creation. Initial value is `NULL` (live). When a snapshot fires, every then-live row's `valid_to = snapshot_timestamp`.
- **`last_seen_at`** — discovery-driven. Refreshed on every successful natural-key match against a live row. Never reset by snapshots.

### What continuous scans do

For each entity in an incoming `DiscoveryHostRequest`:

1. Match the entity by natural key against live rows on the network (existing logic).
2. **Match found** → `UPDATE` the live row. Refresh `last_seen_at = NOW()`. Apply any field changes in place. No new row inserted; `valid_from` / `valid_to` untouched.
3. **No match** → `INSERT` a new live row. `valid_from = NOW()`, `last_seen_at = NOW()`, `valid_to = NULL`.

There is no insert-on-field-change branch. There is no mark-vanished branch. Continuous scans are read-mostly with at-most one row write per matched entity (the in-place UPDATE) and one row write per new entity (the INSERT).

### What snapshots do

A snapshot at timestamp T:

1. Selects every currently-live row on the snapshot's network (`valid_to IS NULL`).
2. Sets `valid_to = T` on each — these become the closed historical record.
3. Inserts fresh clones with `valid_from = T`, `valid_to = NULL`, copies of the previous values (including `last_seen_at`). These become the new live rows that subsequent scans mutate.

Manual snapshots fire on user action. Scheduled snapshots fire on cron-like cadence configured per-org (paid feature).

**Open for implementer:** whether the close-and-clone happens immediately at snapshot time or lazily on next scan via copy-on-write. The user-visible semantics are identical; the transactional cost trades off differently. The implementer should pick after sizing live-row counts on representative orgs.

### What the audit trail is

Browsing the audit trail = browsing snapshots. Each snapshot is a queryable point in time:

- "What was live at snapshot T?" → `SELECT * FROM <entity> WHERE network_id = ? AND valid_from <= T AND (valid_to IS NULL OR valid_to > T)`.
- "What changed between snapshot A and snapshot B?" → diff the two row sets.

Between snapshots, the row history is `valid_from`-driven: a host that was discovered after snapshot A but before snapshot B has `valid_from > A.timestamp AND valid_from <= B.timestamp`. It appears in B's snapshot view, not A's.

Descriptor changes (in-place UPDATEs) leave no trace in the audit trail — only the latest value is queryable. This is by design; the natural-key-matching layer determines what counts as a structural transition.

### Stale-entity surface

> NOT BUILT as of 2026-06 — see status header.

### Topology read path

Topology assembly (`backend/src/server/topology/service/main.rs:295`) keeps the same eight-`get_all` shape; only the filter changes:

- Current state: `network_id = ? AND valid_to IS NULL`.
- As-of state at T: `network_id = ? AND valid_from <= T AND (valid_to IS NULL OR valid_to > T)`.

The existing `topologies` JSONB cache continues as a current-state render cache. Snapshot views render from the entity tables directly using the as-of query. **Open for implementer:** whether snapshot views also get a per-snapshot render cache.

---

## Schema additions

Direction-level — column lists, types, and migration mechanics are the implementer's call.

**Per-entity-tracked tables** (`hosts`, `ip_addresses`, `interfaces`, `ports`, `services`, `bindings`, `subnets`) get the three columns: `valid_from`, `valid_to`, `last_seen_at`. The existing UNIQUE constraints (e.g., `(host_id, port_number, protocol)` on `ports`) hold under the snapshot-driven model because there is at most one live row per natural key — but the implementer needs to verify the migration leaves them intact and decide whether closed rows participate (likely partial UNIQUE on live row only, but that's a confirmation pass).

**`dependencies`** is user-managed and not time-tracked.

**New table: snapshots metadata** — first-class records carrying at minimum: id, network_id, name (nullable for unnamed scheduled snapshots), `taken_at_timestamp`, `kind` (manual / scheduled), `pinned` (bool), `created_by_user_id`, created/updated timestamps. Implementer picks the table name and exact column list.

**Org-level settings additions** (extend the existing org settings model): snapshot cadence, digest cadence, digest send-empty, digest opt-out, snapshot retention window. Plan-tier-gated defaults; specific values per tier at implementation time.

**Indexes for time-aware queries:** the live-state filter (`valid_to IS NULL`) and the as-of filter (`valid_from <= T AND (valid_to IS NULL OR valid_to > T)`) need to be backed by indexes that don't degrade under historical row growth. **Open for implementer:** the specific index columns (likely partial indexes filtered on `valid_to IS NULL` for the current-state path, plus history-friendly indexes for the as-of path; the row-count projections in the Investigation appendix S6.3 inform the choice).

**Migration backfill: open for implementer.** The migration adds the three columns to existing tables with sensible defaults that capture the current state as the first live row per natural key. Backfill column-by-column is the implementer's call; the spec does not prescribe values.

**Existing `topologies.parent_id`-anchored locks: migration converts them into the snapshot mechanism** as manual + pinned snapshots whose `taken_at_timestamp` corresponds to the existing locked moment. **Open for implementer:** the exact migration shape and how to map the legacy JSONB into the entity-table-backed snapshot view (or whether legacy locked topologies stay rendered from JSONB and only new snapshots use the entity-table path; both work).

---

## Daemon-server protocol

**Daemon: zero changes.** The daemon already sends wholesale topology per host (`DiscoveryHostRequest` at `backend/src/server/hosts/impl/api.rs:62-74`) in both DaemonPoll and ServerPoll modes. The wholesale-send + server-side reconciliation model from the brief is what the spec keeps.

**Server: two additions to the per-entity submission path** (under the existing per-host `Mutex` lock at `backend/src/server/hosts/service.rs:116-117`):

1. **`last_seen_at` refresh.** Every successful match against a live row refreshes `last_seen_at = NOW()`, regardless of whether anything else changed.
2. **In-place UPDATE for field changes.** Field changes on matched live rows update in place. No row close, no clone, no insert.

New-entity INSERT follows the existing path with `valid_from = NOW()`, `valid_to = NULL`, `last_seen_at = NOW()`.

**No row-close logic at submission time.** Closes are exclusively triggered by snapshot creation, which runs on a separate code path (manual user action or scheduled job).

The server does not need any discovery-session-complete signal from the daemon. `last_seen_at` carries the necessary information for the coverage-gap surface; snapshots are user-driven or scheduled and don't depend on daemon-side semantics.

---

## Snapshots

The only mechanism that produces audit-trail content. Two kinds:

### Manual snapshots — free + paid

Any user with the right permission level (Member and above per the UX section) can take a manual snapshot at any time. The user gives it a name (or leaves it unnamed; the implementer decides whether the name is required). The snapshot fires immediately, closing the network's live rows at that instant and creating fresh clones.

Manual snapshots are available on free and paid tiers. They are the free-tier mechanism for capturing change history.

### Scheduled snapshots — paid only

> NOT BUILT as of 2026-06 — see status header.

### Pinning — paid only

> NOT BUILT as of 2026-06 — see status header.

### Stale-entity surface

> NOT BUILT as of 2026-06 — see status header.

---

## Retention

**Direction:** daily background job deletes snapshots older than the org's retention window and the closed entity rows they anchor. Per-org single-window (no per-network or per-entity-type knobs in v1). (Plan-gated retention shipped; pin-exemption did not — pinning was never built, see status header.)

**Per-tier window is the paid lever** — founder's directional example: free 1 week, pro 1 month. Higher tiers get longer windows.

**Open for implementer:**

- Specific per-tier defaults (and the upgrade-back lever positioning of those defaults).
- The deletion mechanic: delete snapshots first then orphan-prune closed rows, or delete closed rows whose `valid_to` is past the window (relying on snapshot exemption to keep pin-anchored rows alive). Both work; the implementer picks based on transactional cost.
- Whether self-hosted users can override the per-tier default (founder previously said yes for self-hosted; verify at implementation).

No archive table. If the cloud-tier marketing position later wants "your history is preserved across downgrade" as an upgrade-back lever, that's a separate expand-and-contract migration.

---

## Diff fidelity

The audit trail is structurally defined by the data-model shape:

- **Natural-key changes** (a host's IPs all changing such that the matching layer no longer recognizes it; a service's `service_definition` changing; an interface's `if_name` changing) manifest as **vanish+appear pairs** at the next snapshot. The old entity becomes a closed row; a new entity inserts as a new live row. Captured in the snapshot diff.
- **Descriptor changes** (hostname, oper_status, vendor refinement, SNMP descriptor drift) update in place. They are **not** in the audit trail by design — only the latest value is queryable.

The line between natural-key and descriptor is settled by the existing identity-matching logic (S1.1 of the Investigation appendix). No new tracked-vs-untracked classification is needed.

### Digest filter

The digest renders from snapshot diffs. It applies a **filter** at the rendering layer to decide which classes of events surface to the user.

**Open for implementer:** the digest's default content rule. Direction: surface entity-presence transitions (hosts appeared/vanished, services added/removed, subnets appeared) at the host- and service-level granularities; finer-grained transitions (port-level, interface-level) probably belong in the in-app activity view but not in the email. The implementer makes the call after sizing typical diffs on representative networks.

Per-org configurability of the digest filter is deferred. v1 is a fixed default.

### Audit-trail readability

> NOT BUILT as of 2026-06 — see status header. (PDF / HTML diff export was never built.)

---

## UX

> NOT BUILT as of 2026-06 — see status header. The entire UX surface described here (per-org/weekly digest cadence, paid-gated opt-out, in-app changelog / Activity page / topology timeline pane, snapshot pinning permissions, PDF/HTML diff export, `digest_send_empty` / empty-digest behavior, discovery-failure digest variant) was never built. What shipped: manual snapshot creation, in-app snapshot browse/diff, plan-gated retention, and a per-discovery-session **per-user** digest email (not the per-org weekly model described above).

---

## Out of scope / deferred

- **S4 — branching for planning edits.** Out of scope for Phase 2 per TASK.md. The v1 schema does not reserve `branch_id`.
- **Daemon-state-loss host identity fingerprint tier.** Deferred to v2. v1 ships with current IP+MAC matching. Trigger to revisit: field reports of duplicate hosts following NIC-swap-during-daemon-outage events.
- **Cold-storage retention archive.** Deferred. Build only when a "your history is preserved across downgrade" upgrade-back lever has demonstrated demand.
- **Per-network / per-entity-type retention windows.** Deferred. v1 is per-org-uniform.
- **Per-org diff-fidelity configurability.** Deferred. v1 is fixed default rule.
- **Real-time digest cadence.** Out of scope (alerting, not digest).
- **Share-link / markdown / Confluence / Hudu diff exports.** Out of scope for v1 export formats.
- **Pre-change blast-radius prompt** (Avenue 4 candidate #3). Out of scope; depends on planning-edit workstream.
- **Rogue / unexpected device detection.** Out of scope; layers on top of this feature's snapshot diff substrate later.
- **Migration of legacy `topologies.parent_id` locks** to the new snapshot mechanism: in scope for v1, but the exact migration shape is open for the implementer.

---

# Investigation (appendix — pre-pass background)

The pre-pass investigation that fed the design pass. Every claim cites `file:line`. The decisions above were made against this background; the appendix is preserved for citational reference. Forward-looking annotations that referenced the earlier continuous-SCD2 framing have been trimmed to keep the appendix purely backward-looking on current code state.

## S1 — Identity resolution today

### 1.1 Per-entity identity model

All entity tables use a UUID surrogate as primary key. Identity *resolution* across scans uses a natural key matched in application code (occasionally enforced at the DB level as a UNIQUE constraint). Across the board, daemons send entities with daemon-generated UUIDs; the server resolves identity from the natural key and **either reuses an existing UUID (overwriting the daemon-supplied one) or accepts the daemon's UUID for a brand-new entity**. This is the **(B) server-assigned surrogate via natural key** shape from the brief, end-to-end.

| Entity | Table PK | Natural key (matching) | Where matched | Enforcement |
|---|---|---|---|---|
| Host | `id UUID` (`backend/migrations/20251006215151_create_hosts.sql:2`) | IPs + MACs on the network, with per-batch MAC-count guard for VLANs and explicit filters for loopback / VRRP / HSRP virtual MACs | `find_matching_host_by_ip_addresses()` `backend/src/server/hosts/service.rs:1939` | App-layer only — no UNIQUE constraint |
| IP address (table `ip_addresses`, originally named `interfaces` then renamed) | `id UUID` (`backend/migrations/20251221040000_interfaces_table.sql:4`) | `(host_id, subnet_id, ip_address)` | App-layer in `backend/src/server/hosts/service.rs:802-842`; falls back to `(host_id, mac_address)` when MAC is unique on the host | UNIQUE `(host_id, subnet_id, ip_address)` (`20251221040000_interfaces_table.sql:14`); table renamed at `backend/migrations/20260410000000_rename_interfaces_and_if_entries.sql:7` |
| Port | `id UUID` (`backend/migrations/20251221050000_ports_table.sql:4`) | `(host_id, port_number, protocol)` | App-layer in `backend/src/server/hosts/service.rs:691-709` | UNIQUE `(host_id, port_number, protocol)` (`20251221050000_ports_table.sql:12`) |
| Service | `id UUID` (`backend/migrations/20251006215212_create_services.sql:2`) | Tiered: `(host_id, service_definition_id)` for non-generic; container_id (Docker) → container_name → port-binding overlap for generic; gateway/open-ports treated as singletons per host | `Service::eq` (`backend/src/server/services/impl/base.rs:125-269`) used by `ServiceService::create()` | App-layer only — no UNIQUE constraint |
| Interface (SNMP `ifTable`, table renamed from `if_entries` to `interfaces`) | `id UUID` (`backend/migrations/20260116030000_if_entries.sql:5`) | Tiered: `(host_id, if_name)` → `(host_id, if_index)` → `(host_id, mac_address)` with single-MAC guard | `InterfaceService::find_matching_existing()` (`backend/src/server/interfaces/service.rs:217-290`) | Partial UNIQUE `(host_id, if_name) WHERE if_name IS NOT NULL` (`backend/migrations/20260417000000_reindex_interfaces_identity.sql:41`); old non-partial UNIQUE `(host_id, if_index)` was dropped at `:38` |
| Binding | `id UUID` (`backend/migrations/20251221060000_bindings_table.sql:4`) | Implicit `(service_id, binding_type, ip_address_id?, port_id?)`; bindings are derived from service definition + port/interface detection | Re-derived per discovery; deduped during service upsert in `ServiceService` | CHECK constraint `valid_binding` (`20251221060000_bindings_table.sql:13-16`); no UNIQUE on the natural key |
| Subnet | `id UUID` (`backend/migrations/20251006215155_create_subnets.sql:2`) | `(network_id, cidr)` plus `virtualization.service_id` for Docker bridges so the same CIDR on different hosts is distinct | `Subnet::eq` (`backend/src/server/subnets/impl/base.rs:180-187`) | App-layer only — no UNIQUE constraint |
| Dependency (formerly `groups`, renamed at `backend/migrations/20260405120000_rename_groups_to_dependencies.sql:4`) | `id UUID` (`backend/migrations/20251006215201_create_groups.sql:2`) | User-managed; not produced by discovery | n/a | n/a |

Notes worth carrying into implementation:

- **The host upsert path takes a per-host `Mutex` lock** (`backend/src/server/hosts/service.rs:116-117`, lock map at `:260`). Any in-place UPDATE on host rows under daemon submission runs under this lock.
- **The match warning at `backend/src/server/hosts/service.rs:140-144`** explicitly tells operators to write the resolved `host_id` back into the daemon's config file — i.e., the daemon **does** persist *its own* host_id across restarts. It does NOT persist the IDs of the hosts it discovers; those are regenerated each session from `EntityBuffer` (`backend/src/daemon/discovery/buffer.rs:46-58`). This asymmetry is what creates the daemon-state-loss host-duplicate failure mode that the deferred fingerprint tier is meant to address.
- **The IP-match scan is O(all_hosts × incoming_ips × ips_per_host)** (`backend/src/server/hosts/service.rs:1992-2014`). It loads every host on the network into memory each time. The cost is non-trivial on busy networks; the snapshot-driven model doesn't change the asymptotic shape.
- **There is no soft-delete or "vanished" marking today.** Hosts not seen in a scan stay in the table unchanged; deletion is manual via `delete_host` handler (`backend/src/server/hosts/handlers.rs:790`). Snapshots fill this gap under the new model.

### 1.2 Daemon→server reconciliation

The daemon discovers a network and submits in one of two modes (`backend/src/daemon/discovery/service/ops.rs:633`):

- **DaemonPoll (default):** the daemon issues an HTTP POST per entity as it's discovered — hosts to `/api/v1/hosts/discovery` with full children attached (`backend/src/daemon/discovery/service/ops.rs:634-649`), subnets to `/api/v1/subnets` (`:694-706`). Retries up to `ENTITY_CREATION_MAX_RETRIES` with exponential backoff.
- **ServerPoll:** the daemon buffers all discovered entities in `EntityBuffer` (`backend/src/daemon/discovery/buffer.rs`) and the server polls `GET /api/discovery`, receiving a `DiscoveryPollResponse` containing `BufferedEntities { hosts, subnets }` (`backend/src/daemon/runtime/state.rs:65-90`). Each `host` in that buffer is itself a `DiscoveryHostRequest` with the full child set attached — wire shape identical, transport differs.

Either way, the unit of submission is **one host with its full child set** (`DiscoveryHostRequest` at `backend/src/server/hosts/impl/api.rs:62-74`). The daemon never sends "host X removed" or "service Y changed" events. The daemon also has no cross-session memory of what it discovered last time (`EntityBuffer` is rebuilt each discovery session).

Server-side, each `DiscoveryHostRequest` flows through `HostService::create()` (`backend/src/server/hosts/service.rs:109-196`): match by IP/MAC, adopt existing UUID if matched, upsert children in place. The natural-key matching logic is unchanged under the new model — it's the substrate the snapshot mechanism builds on.

### 1.3 Identity-shape readiness

Every discovery-managed entity is **(B) server-assigned surrogate via natural key**. The server is authoritative for IDs; the daemon supplies natural keys.

| Entity | Ready for time-tracking? | Notes |
|---|---|---|
| Host | Yes, with the daemon-state-loss caveat (deferred fingerprint tier) | The IP/MAC-only natural key cannot survive simultaneous IP and MAC change with daemon state loss. v1 ships with this risk. |
| IP address | Yes | UNIQUE `(host_id, subnet_id, ip_address)` on live rows. |
| Port | Yes | UNIQUE `(host_id, port_number, protocol)` on live rows. |
| Service | Yes | Generic-service port-overlap matching is a known soft-spot today (occasional duplicate generic services from cross-subnet rediscovery — `backend/src/server/services/impl/base.rs:255-265` notes this). Time-tracking doesn't make it worse. |
| Interface | Yes | Tiered match landed in `backend/migrations/20260417000000_reindex_interfaces_identity.sql` is exactly the time-tracking-friendly shape: strong key when present, controlled fallbacks when not. |
| Binding | Yes (derivative) | Bindings follow their parent service's lifecycle. |
| Subnet | Yes | `(network_id, cidr)` is stable; virtualization-context handles Docker-bridge per-host scoping. |
| Dependency | n/a | User-curated; user controls lifecycle. |

### 1.4 Failure-mode trace

| Failure mode | Behavior today | File:line |
|---|---|---|
| **NIC swap (MAC changes, same host)** | Identity preserved if any IP+subnet still matches an existing IP row on the host. Lost when MAC changes on a host previously matched only via unique-MAC fallback. | `backend/src/server/hosts/service.rs:1992-2014`; `:802-842` |
| **DHCP rotation (IP changes, same host)** | Preserved when MAC is unique on the host (`backend/src/server/hosts/service.rs:1979-1990` handles "Docker container whose IP changed via DHCP"). Lost when host shares MACs across interfaces (VLAN sub-interfaces, bonds) — the per-batch MAC-count guard at `:1984-1990` refuses to use shared MACs for matching. | `backend/src/server/hosts/service.rs:1984-1990`, `:2003` |
| **Daemon state loss** | Daemon's *own* host record survives (`host_id` is in daemon config file, `:140-144`). Discovered hosts on the scanned network do NOT survive: `EntityBuffer` (`backend/src/daemon/discovery/buffer.rs:46-58`) is per-session. Server-side IP-match prevents duplicates *as long as* IP or MAC carries over. | `backend/src/daemon/discovery/buffer.rs:46-58`; `backend/src/server/hosts/service.rs:140-144` |
| **Hostname change** | Preserved. Hostname is not part of any matching path. | `backend/src/server/hosts/service.rs:2029` |
| **Multi-homed host** | Preserved when at least one IP+subnet pair matches. Multiple MACs become multiple `ip_addresses` rows under the same host. The MAC-count guard at `:1984` correctly handles this case. | `backend/src/server/hosts/service.rs:1984-2014` |
| **Two scans of overlapping subnets** | Same network: dedup works. Different networks: two distinct host rows are created — there is no cross-network matching anywhere in the host service. | `backend/src/server/hosts/service.rs:1948-1949`; no cross-network match logic exists |

The deferred fingerprint-tier work would close the daemon-state-loss + simultaneous-IP-MAC-change path. The `idx_hosts_chassis_id` column is already there from earlier work, so the v2 implementation is small.

## S3 — Daemon-server protocol assessment

### 3.1 What the daemon sends today

**Wholesale per-host with full child set, not per-entity events, not per-network deltas.** The unit of submission is `DiscoveryHostRequest { host, ip_addresses, ports, services, interfaces, subnets }` (`backend/src/server/hosts/impl/api.rs:62-74`). One submission per discovered host. Subnets are also submitted independently.

Two transport variants from the same wire shape (`backend/src/daemon/discovery/service/ops.rs:619-678`):

- **DaemonPoll:** `POST /api/v1/hosts/discovery` (`:638`) and `POST /api/v1/subnets` (`:698`) per entity, with retries and exponential backoff (`:641-646`). On 200 OK, the daemon updates `EntityBuffer` to mark the entity as `Created` with the server's authoritative ID (`:651-653`).
- **ServerPoll:** the daemon accumulates entities in `EntityBuffer`. Server fetches via `GET /api/discovery`, gets `DiscoveryPollResponse { progress, entities: BufferedEntities { hosts, subnets } }` (`backend/src/daemon/runtime/state.rs:84-90`), and posts `CreatedEntitiesPayload { subnets, hosts: Vec<(Uuid, HostResponse)> }` back to confirm (`:94-103`).

The daemon does not maintain previous-scan state. `EntityBuffer` is created fresh per discovery session (`backend/src/daemon/discovery/buffer.rs:46-58`). There is no on-disk delta cache or "last seen" record on the daemon side.

### 3.2 What the server does on receive (today)

**Merge by natural key, no diff, no vanish marking.** Walked above in S1.2. The server upserts each entity into the existing row via the per-entity natural-key match, leaves rows that aren't in the incoming submission untouched, and never tracks which previously-known children weren't repeated. The new model adds `last_seen_at` refresh and in-place field UPDATEs to this path; new-entity INSERT follows the existing path. No row-close happens at submission time — closes are exclusively snapshot-driven.

### 3.3 Why the daemon needs no changes

The daemon already sends complete per-host topology snapshots, retries on failure, and tracks discovery sessions in ServerPoll mode. The wholesale-send + server-side reconciliation model from the brief is what the spec keeps; the snapshot mechanism layers on top of it as a server-side concern, with no daemon-side participation required.

## S6 — Read-path performance survey

### 6.1 Existing index inventory

Pulled from `backend/migrations/`. "Live indexes" are what's there now after the renames in `20260410000000_rename_interfaces_and_if_entries.sql`.

| Live table | Index | Columns | Source migration |
|---|---|---|---|
| `hosts` | PK | `(id)` | `20251006215151_create_hosts.sql:2` |
| | `idx_hosts_network` | `(network_id)` | `20251006215151_create_hosts.sql:17` |
| | `idx_hosts_chassis_id` | `(chassis_id)` | added later for SNMP discovery dedup |
| `ip_addresses` (was `interfaces`) | PK | `(id)` | `20251221040000_interfaces_table.sql:4` |
| | UNIQUE | `(host_id, subnet_id, ip_address)` | `20251221040000_interfaces_table.sql:14` |
| | `idx_ip_addresses_network` | `(network_id)` | `20251221040000_interfaces_table.sql:36` (renamed `20260410000000:8`) |
| | `idx_ip_addresses_host` | `(host_id)` | `20251221040000_interfaces_table.sql:37` (renamed `20260410000000:9`) |
| | `idx_ip_addresses_subnet` | `(subnet_id)` | `20251221040000_interfaces_table.sql:38` (renamed `20260410000000:10`) |
| | `idx_ip_addresses_host_mac` | partial `(host_id, mac_address) WHERE mac_address IS NOT NULL` | `20260106000000_interface_mac_index.sql` (renamed `20260410000000:11`) |
| `interfaces` (was `if_entries`) | PK | `(id)` | `20260116030000_if_entries.sql:5` |
| | partial UNIQUE | `(host_id, if_name) WHERE if_name IS NOT NULL` | `20260417000000_reindex_interfaces_identity.sql:41` |
| | `idx_interfaces_host_if_index` | `(host_id, if_index)` | `20260417000000_reindex_interfaces_identity.sql:47` |
| | `idx_interfaces_host` | `(host_id)` | `20260116030000_if_entries.sql:60` (renamed `20260410000000:23`) |
| | `idx_interfaces_network` | `(network_id)` | `20260116030000_if_entries.sql:61` (renamed `20260410000000:24`) |
| | `idx_interfaces_ip_address` | `(ip_address_id)` | `20260116030000_if_entries.sql:62` (renamed `20260410000000:25`) |
| | `idx_interfaces_mac_address` | `(mac_address)` | `20260116030000_if_entries.sql:63` (renamed `20260410000000:26`) |
| | `idx_interfaces_neighbor_interface` | `(neighbor_interface_id)` | renamed at `20260410000000:27` |
| | `idx_interfaces_neighbor_host` | `(neighbor_host_id)` | renamed at `20260410000000:28` |
| `ports` | PK | `(id)` | `20251221050000_ports_table.sql:4` |
| | UNIQUE | `(host_id, port_number, protocol)` | `20251221050000_ports_table.sql:12` |
| | `idx_ports_network` | `(network_id)` | `20251221050000_ports_table.sql:31` |
| | `idx_ports_host` | `(host_id)` | `20251221050000_ports_table.sql:32` |
| | `idx_ports_number` | `(port_number)` | `20251221050000_ports_table.sql:33` |
| `services` | PK | `(id)` | `20251006215212_create_services.sql:2` |
| | `idx_services_host_id` | `(host_id)` | `20251006215212_create_services.sql:15` |
| | `idx_services_network` | `(network_id)` | `20251006215212_create_services.sql:16` |
| `bindings` | PK | `(id)` | `20251221060000_bindings_table.sql:4` |
| | `idx_bindings_network` | `(network_id)` | `20251221060000_bindings_table.sql:54` |
| | `idx_bindings_service` | `(service_id)` | `20251221060000_bindings_table.sql:55` |
| | `idx_bindings_ip_address` | `(ip_address_id)` (renamed from `idx_bindings_interface`) | `20251221060000_bindings_table.sql:56`; renamed `20260410000000:51` |
| | `idx_bindings_port` | `(port_id)` | `20251221060000_bindings_table.sql:57` |
| `subnets` | PK | `(id)` | `20251006215155_create_subnets.sql:2` |
| | `idx_subnets_network` | `(network_id)` | `20251006215155_create_subnets.sql:13` |
| `dependencies` (was `groups`) | PK | `(id)` | `20251006215201_create_groups.sql:2` |
| | `idx_groups_network` | `(network_id)` | `20251006215201_create_groups.sql:13` (table renamed at `20260405120000:4` — index kept its old name) |
| `dependency_members` (was `group_bindings`) | PK + UNIQUE on `(dependency_id, service_id)` | — | `20260405120000_rename_groups_to_dependencies.sql:29` |

Every entity table has `(network_id)` covering and FK-targeted indexes covering the join paths topology assembly walks today. Nothing partial-on-currency, nothing range-friendly for as-of queries — that's the gap the implementer's index additions fill.

### 6.2 Current topology read-path

Current state is materialized as a JSONB blob in the `topologies` table (per `20251118225043_*` and the columns referenced in the rename migration `20260410000000:56-59`). The blob is rebuilt on demand and cached.

The rebuild pulls each entity table separately, filtered by `network_id`, and joins in memory. From `backend/src/server/topology/service/main.rs:295-345`:

```
hosts        ← StorableFilter::<Host>::new_from_network_ids(&[network_id]).hidden_is(false)
ip_addresses ← StorableFilter::<IPAddress>::new_from_network_ids(&[network_id])
subnets      ← StorableFilter::<Subnet>::new_from_network_ids(&[network_id])
dependencies ← StorableFilter::<Dependency>::new_from_network_ids(&[network_id])
ports        ← StorableFilter::<Port>::new_from_network_ids(&[network_id])
bindings     ← StorableFilter::<Binding>::new_from_network_ids(&[network_id])
interfaces   ← StorableFilter::<Interface>::new_from_network_ids(&[network_id])
```

Plus `services` via `get_service_data` at `:348-354`. Eight per-entity reads, each scanning `idx_<entity>_network` and returning every row for the network. Joins (`Service.host_id` → `Host.id`, `Binding.service_id` → `Service.id`, `Interface.host_id` → `Host.id`, etc.) happen in Rust against the loaded vectors.

Under the snapshot-driven model the per-entity read shape is unchanged; the filter changes from `network_id = ?` to `network_id = ? AND valid_to IS NULL` (current state) or to the as-of filter for snapshot views. Index coverage for those filters is open for the implementer.

### 6.3 Row-count and growth shape

No production-sizing references in the codebase. Test fixtures in `backend/src/server/shared/storage/seed_data.rs` instantiate trivial counts. Reasoning from first principles:

- Representative paid org carries 100–500 hosts per network, 300–2000 services, similar order of IP addresses, 500–5000 ports.
- Under the snapshot-driven model, **growth is bounded by snapshot frequency × network size**. A weekly cadence on a 200-host / 800-service network produces ~1,000 closed rows per snapshot (per entity type aggregated). Hourly cadence on the same network produces ~24× that. Stable networks at coarse cadence stay small for years; chatty networks at fine cadence accumulate faster.
- Storage envelope and trim cadence are the implementer's call, informed by typical org tier defaults. The retention window is the dominant control; per-tier retention defaults bound the steady-state row count.

The existing `topologies` JSONB cache continues as a current-state render cache. Snapshot views render from the entity tables. Whether snapshot views also get a per-snapshot render cache is open for the implementer — the row-count projection above informs the call.
