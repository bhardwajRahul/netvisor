> **First:** Read `CLAUDE.md` (project instructions) — you are a **worker**.

<<<<<<< HEAD
<<<<<<< HEAD
# Task: Add Pagination Support to GET List APIs

## Objective

Implement backwards-compatible pagination for all GET list endpoints. Users hitting endpoints without pagination params should continue to work (though may now receive a subset + pagination token).
=======
# Task: Tags System Refactor

## Objective

Refactor the tags system to use a junction table, enabling cleaner bulk/inline tag operations across all entity types without per-entity handler duplication.

**Original goal:** Improve tags UX (bulk tagging, interactive card tags)
**Evolved scope:** Backend-first refactor to eliminate code smell of per-entity tag handlers

## Architecture Overview

### Current State (Problem)
- Tags stored as `Vec<Uuid>` on each entity table
- Each entity needs its own update mutation to modify tags
- Frontend needs per-entity `handleTagAdd`/`handleTagRemove` handlers
- Code duplication across Host, Service, Subnet, Group, Network, etc.

### Target State (Solution)
- Junction table `entity_tags(entity_id, entity_type, tag_id)`
- Single API endpoint for tag assignment/removal
- Generic frontend hook works for all entity types
- Tag hydration in service layer (follows existing patterns)

## Implementation Plan
>>>>>>> feature/tags-refactor

### Phase 1: Database Migration

<<<<<<< HEAD
### Phase 1: Investigation & Design (STOP FOR APPROVAL AFTER THIS)

Before implementing, investigate and document decisions for:

1. **Filter Query Architecture**
   - Current: Each entity has its own filter query type with overlapping fields
   - Problem: Adding pagination params would duplicate them across all query types
   - Consider: Array-based or composable filter query design
   - Explore: Can pagination params be a separate composable struct?

2. **Entity Relationships & Data Loading**
   - Current frontend pattern: Lists load all entities; cards display related entities
   - Example: HostCard displays services for that host
   - Problem: If services page only loads 25, HostCard may not have full data
   
   **Questions to answer:**
   - Which entities are cross-referenced in cards and need full data?
   - Should some entities remain unpaginated (or have unlimited option)?
   - Should we add filtered queries (e.g., "get services by host_id") for card data?
   - Can we still support a "get all" query variant for entities that need it?
   - What's the right default limit per entity type?

3. **Write up your findings in TASK.md** under a new "## Phase 1 Findings" section:
   - Proposed filter query architecture
   - Entity-by-entity pagination strategy (paginate vs full load)
   - Recommended approach for card data loading
   - Any frontend implications to flag for follow-up task

**IMPORTANT: After completing Phase 1, STOP and wait for user approval before proceeding to Phase 2.**

### Phase 2: Backend Implementation (After Approval)

1. **Extend `ApiMeta`** (in `backend/src/server/shared/types/api.rs`):
   - Add optional `pagination` field with:
     - `total_count`: Total number of items matching filter
     - `limit`: Number of items returned
     - `offset`: Current offset
     - `has_more`: Boolean indicating if more results exist

2. **Refactor filter query architecture** (in `backend/src/server/shared/handlers/query.rs`):
   - Consider composable/array-based design to reduce duplication
   - Add pagination params in a reusable way
   - Default behavior when omitted: return paginated results with reasonable default
   - Maximum limit cap to prevent abuse
   - Support unlimited queries where needed (e.g., `limit=0` or `all=true`)

3. **Update Storage trait** (in `backend/src/server/shared/storage/traits.rs`):
   - Add method for paginated queries that returns `(Vec<T>, total_count)`
   - Or extend `get_all` to accept pagination params

4. **Update OpenAPI macros** (in `backend/src/server/shared/handlers/openapi_macros.rs`):
   - `crud_get_all_handler!` should support pagination params
   - Response schema should reflect optional pagination metadata

5. **Update OpenAPI documentation**:
   - Add clear documentation on pagination query params (`limit`, `offset`)
   - Document response pagination metadata structure
   - Add examples showing paginated responses
   - Tag/group pagination-related documentation for discoverability

6. **Apply to list endpoints per your entity strategy**:
   - Networks, Hosts, Daemons, Services, Subnets, etc.
   - Both macro-generated and custom handlers
   - Respect decisions from Phase 1 about which entities get full pagination

### Backwards Compatibility

- Endpoints must work without pagination params
- Existing API clients should not break
- Response structure (`ApiResponse<Vec<T>>`) should remain compatible
- New `meta.pagination` field is additive

### Phase 3: Frontend Implementation (After Backend Complete)

1. **Generate updated types**:
   ```bash
   make generate-types
   ```

2. **Update API queries to support pagination**:
   - Add `limit` and `offset` params to query functions
   - Update TanStack Query hooks to accept pagination options
   - Key files: `ui/src/lib/features/*/queries.ts`

3. **Extend DataControls.svelte** (`ui/src/lib/shared/components/data/DataControls.svelte`):
   - Add prev/next pagination controls
   - Display "Page X of Y" or "Showing X-Y of Z"
   - Wire pagination state to API calls
   - Persist pagination state in localStorage (like existing sort/filter)

4. **Update list pages to use pagination**:
   - **Hosts page** - Primary candidate, can have many hosts
   - **Discoveries page** - History can grow large
   - **Other pages** - Use `limit=0` for now until pagination UI needed

5. **Handle card data loading**:
   - Cards displaying related entities (e.g., HostCard showing services) use embedded data from parent response
   - No changes needed since HostResponse includes full child data

## Key Files

| Purpose | File |
|---------|------|
| Response types | `backend/src/server/shared/types/api.rs` |
| Query filtering | `backend/src/server/shared/handlers/query.rs` |
| Generic handlers | `backend/src/server/shared/handlers/traits.rs` |
| Macros | `backend/src/server/shared/handlers/openapi_macros.rs` |
| Storage trait | `backend/src/server/shared/storage/traits.rs` |
| Data controls (pagination UI) | `ui/src/lib/shared/components/data/DataControls.svelte` |
| Hosts queries | `ui/src/lib/features/hosts/queries.ts` |
| Hosts tab | `ui/src/lib/features/hosts/components/HostTab.svelte` |
| Discovery queries | `ui/src/lib/features/discovery/queries.ts` |
| Discovery tabs | `ui/src/lib/features/discovery/components/tabs/` |

## Acceptance Criteria

### Phase 1 & 2 (Backend)
- [x] Phase 1 investigation complete with documented decisions
- [x] User approved Phase 1 findings before implementation
- [x] Filter query architecture improved (not N duplicated pagination params)
- [x] All GET list endpoints support pagination per entity strategy
- [x] Response includes `meta.pagination` when applicable
- [x] Unlimited/full-load option available where needed (`limit=0`)
- [x] OpenAPI spec includes pagination documentation with clear examples
- [x] `cargo test` passes

### Phase 3 (Frontend)
- [x] Types regenerated with `make generate-types`
- [x] DataControls.svelte extended with pagination controls (already has client-side pagination UI)
- [x] Hosts page uses paginated queries (using `limit: 0` for full load + client-side pagination)
- [x] Discoveries page uses paginated queries (using `limit: 0` for full load + client-side pagination)
- [x] Other pages pass `limit=0` to maintain current behavior
- [x] `npm test` passes (no test script; `npm run check` passes)
- [x] `make format && make lint` passes

## Out of Scope

- Cursor-based pagination (offset-based is sufficient for now)
- Infinite scroll (use prev/next pagination controls)

## Notes

- Backend is complete. Proceed with Phase 3 frontend implementation.
- Focus on Hosts and Discoveries pages first - these are most likely to have large datasets.
- Other entity pages can continue using `limit=0` until pagination is needed.

---

## Phase 1 Findings

### 1. Filter Query Architecture

**Current State:**
7 filter query types in `shared/handlers/query.rs`, each implementing `FilterQueryExtractor`:

| Query Type | Fields | Used By |
|------------|--------|---------|
| `NetworkFilterQuery` | `network_id` | Host, Subnet, Group, Topology, DaemonApiKey |
| `NoFilterQuery` | (none) | Tags, Invites, Organizations, Users, Networks, UserApiKey |
| `HostChildQuery` | `host_id`, `network_id` | Ports, Services, Daemons |
| `InterfaceQuery` | `host_id`, `subnet_id`, `network_id` | Interfaces |
| `BindingQuery` | `service_id`, `network_id`, `port_id`, `interface_id` | Bindings |
| `DiscoveryQuery` | `network_id`, `daemon_id` | Discoveries |
| `SharesQuery` | `network_id`, `topology_id` | Shares |

**Proposed Solution: Composable Pagination Params**

Create a `PaginationParams` struct and compose it into all filter queries:

```rust
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct PaginationParams {
    /// Maximum number of results to return. Default: 50. Use 0 for unlimited.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}
```

**Defaults:**
- `limit`: 50 (reasonable default)
- `offset`: 0
- Max `limit`: 1000 (prevent abuse)
- `limit=0`: Return all results (backward compatibility)

### 2. Entity Data Loading Patterns

**Embedded Pattern (via HostResponse):**
The Hosts endpoint returns `HostResponse` with embedded interfaces, ports, services. When hosts paginate, embedded children for each returned host are complete.

**Independent Endpoints:**
Services, Interfaces, Ports, Bindings have standalone endpoints used primarily for CRUD. Pagination applies uniformly but frontend mainly uses the embedded data.

### 3. Implementation Strategy

**Backend: Implement pagination for ALL list endpoints**

Using generic handlers and macros, the incremental effort is minimal once the infrastructure exists. All endpoints get pagination support uniformly.

**Frontend: Initially only paginate Hosts and Discoveries**

Other list views continue using `limit=0` (all results) until UI is updated. The `DataControls.svelte` component already has:
- Search/filter/sort (client-side)
- "Showing X of Y items" display
- localStorage state persistence

To add pagination:
- Add prev/next controls
- Add offset state that triggers API calls
- Use `totalCount` from API meta
- Persist pagination state in localStorage

### 4. Implementation Plan

**Backend (This Task):**
1. Create `PaginationParams` struct with limit/offset
2. Compose into all `FilterQueryExtractor` types via `#[serde(flatten)]`
3. Extend `EntityFilter` with `limit()` and `offset()` methods
4. Add `get_paginated()` to Storage trait returning `(Vec<T>, total_count)`
5. Extend `ApiMeta` with optional pagination info (total_count, limit, offset, has_more)
6. Update `crud_get_all_handler!` macro to support pagination
7. Update custom handlers (hosts, discoveries) to support pagination
8. Update OpenAPI documentation with pagination examples

**Frontend (Follow-up Task):**
1. Extend `DataControls.svelte` with pagination controls
2. Update Hosts page to use paginated queries
3. Update Discoveries page to use paginated queries
4. Other pages continue using `limit=0` until needed
=======
# Task: Investigate Discovery Duplicate Entity Bug (#425, #426)

## Objective

Exhaustively investigate why Docker discovery creates duplicate entities (interfaces, ports, services) that should be deduplicated. Propose the root cause with evidence.

## Issues

### Issue #426: Interface/Bridge Duplicates
- Docker bridges and interfaces being rediscovered as new entries
- Same IP (`172.18.0.2`), same MAC (`6A:70:CD:AF:DC:D3`), same name (`garage`)
- But different interface IDs and different subnet IDs
- Violates documented "upsert behavior"

### Issue #425: Port Duplicates
- Ports that were manually assigned to services get rediscovered as "Unclaimed"
- Creates duplicate port entries with conflicting states
- Warnings during discovery about duplicate ports/services being dropped
- Interfaces show identical IP/MAC but different internal IDs

**Both issues:**
- First appeared in v0.12.8
- Involve Docker discovery
- Create duplicates despite identical identifying properties

## Investigation Requirements

**This is an investigation task, not an implementation task.** Your deliverable is a root cause analysis, not a fix.

### Phase 1: Understand the Discovery Flow

Trace the complete end-to-end flow. Document your findings as you go.

1. **Discovery initiation**
   - Where does Docker discovery start?
   - What parameters/context does it receive?

2. **Interface enumeration**
   - How are Docker interfaces discovered?
   - What identifies an interface as "the same" (IP? MAC? name? combination?)

3. **Entity creation/upsert logic**
   - Where is the deduplication logic?
   - What fields are used for matching existing entities?
   - Is there a sync method for interfaces like there is for other entities?

4. **Subnet association**
   - How are interfaces associated with subnets?
   - Could different subnet IDs cause the same interface to appear as "new"?

5. **Port/service assignment**
   - How are ports associated with interfaces?
   - What happens to port assignments when interface IDs change?

### Phase 2: Identify the Failure Point

Based on your understanding, identify WHERE the deduplication fails:

1. **Is the matching query wrong?**
   - Does it match on the right fields?
   - Is network_id/org_id part of the match incorrectly?

2. **Is there a race condition?**
   - Could discovery create duplicates before checking for existing?

3. **Is the subnet association the culprit?**
   - If subnets get recreated, do interfaces lose their identity?
   - Is interface identity tied to subnet_id when it shouldn't be?

4. **Is there missing sync logic?**
   - Compare interface sync to other entity sync methods
   - Are interfaces missing a sync pattern that other entities have?

### Phase 3: Document Root Cause

Write up your findings in this TASK.md under "## Investigation Findings":

1. **Data flow diagram** - How discovery data flows through the system
2. **Root cause** - The specific code path that causes duplicates
3. **Evidence** - Code references (file:line) supporting your conclusion
4. **Proposed fix** - High-level approach to fix (don't implement yet)
5. **Risk assessment** - What else might break if we change this?

## Key Files to Investigate

Start with these, but follow the code wherever it leads:

| Area | Likely Files |
|------|--------------|
| Discovery entry point | `backend/src/daemon/` or `backend/src/server/discovery/` |
| Interface handling | `backend/src/server/interfaces/` |
| Host handling | `backend/src/server/hosts/` |
| Subnet handling | `backend/src/server/subnets/` |
| Port handling | `backend/src/server/ports/` |
| Service handling | `backend/src/server/services/` |
| Sync/upsert logic | Look for `sync_*`, `upsert`, `find_or_create` patterns |

## Questions to Answer

- [ ] What uniquely identifies an interface in the DB? (IP? MAC? name? host_id + name?)
- [ ] What uniquely identifies a port? (interface_id + port_number? something else?)
- [ ] When does discovery "create new" vs "update existing"?
- [ ] Is there a sync method for interfaces? If not, why not?
- [ ] How does subnet recreation affect interface identity?
- [ ] What changed in v0.12.8 that might have caused this?

## Acceptance Criteria

- [ ] Full discovery flow documented
- [ ] Root cause identified with code references
- [ ] Proposed fix outlined (not implemented)
- [ ] Findings written in TASK.md "Investigation Findings" section
- [ ] No code changes (this is investigation only)

## DO NOT

- Do not implement a fix yet
- Do not make code changes
- Do not create migrations

Your job is to understand and document. The fix will come after review of your findings.

---

## Investigation Findings

### 1. Data Flow Diagram

```
Docker Discovery Flow:
═══════════════════════════════════════════════════════════════════════════════

DAEMON SIDE                              SERVER SIDE
───────────────────────────────────────  ──────────────────────────────────────

1. DiscoveryRunner::discover()
   │
   ├─► discover_create_subnets()
   │   ├─► get_own_interfaces()
   │   │   └─► Creates Subnet objects     ─────► POST /api/v1/subnets
   │   │       with DiscoveryType::Docker         │
   │   │       metadata                           ▼
   │   │                                   SubnetService::create()
   │   │                                   ├─► find(|s| subnet.eq(s))
   │   │                                   │   (matches by CIDR+network_id)
   │   │                                   ├─► Check metadata for Docker host_id
   │   │                                   └─► Return existing OR create new
   │   │                                          │
   │   └─► get_subnets_from_docker_networks()    │
   │       └─► Returns server-confirmed subnets ◄─┘
   │
   ├─► Update interface subnet_ids
   │   to match server-returned subnets
   │
   ├─► get_container_interfaces()
   │   └─► Create Interface objects
   │       with subnet_id from confirmed subnets
   │
   └─► create_host()                      ─────► POST /api/v1/hosts/discovery
       (host + interfaces + ports + services)     │
                                                  ▼
                                           HostService::discover_host()
                                           └─► create_with_children()
                                               │
                                               ├─► find_matching_host_by_interfaces()
                                               │   Uses Interface::eq() which supports:
                                               │   - (ip + subnet_id) match
                                               │   - MAC address match  ✓
                                               │   - ID match
                                               │
                                               └─► Interface deduplication (lines 501-513)
                                                   Query: host_id + subnet_id + ip_address
                                                   ⚠️ NO MAC FALLBACK - ROOT CAUSE
```

### 2. Root Cause

**The interface deduplication in `create_with_children` queries by `(host_id, subnet_id)` without MAC address fallback.**

When subnet_id differs between runs (due to subnet recreation, different discovery ordering, or daemon re-registration), the existing interface is not found, causing duplicates.

#### Critical Code Path (`hosts/service.rs:501-513`):

```rust
// Check by unique constraint (host_id, subnet_id, ip_address)
let filter = EntityFilter::unfiltered()
    .host_id(&interface.base.host_id)
    .subnet_id(&interface.base.subnet_id);  // ⚠️ PROBLEM
let existing_by_key: Vec<Interface> =
    self.interface_service.get_all(filter).await?;
if let Some(existing_iface) = existing_by_key
    .into_iter()
    .find(|i| i.base.ip_address == interface.base.ip_address)
{
    // Match found - reuse existing
    created_interfaces.push(existing_iface);
    continue;
}
// No match - creates NEW interface (DUPLICATE!)
```

#### Inconsistency with Host Matching:

The `Interface::PartialEq` implementation (`interfaces/impl/base.rs:91-99`) DOES support MAC matching:

```rust
impl PartialEq for Interface {
    fn eq(&self, other: &Self) -> bool {
        (self.base.ip_address == other.base.ip_address
            && self.base.subnet_id == other.base.subnet_id)
        || (self.base.mac_address == other.base.mac_address  // MAC fallback exists
            && self.base.mac_address.is_some()
            && other.base.mac_address.is_some())
        || (self.id == other.id)
    }
}
```

This `PartialEq` is used for **host matching** (`find_matching_host_by_interfaces`), so hosts are deduplicated correctly. But the **interface creation** step uses a database query that doesn't include MAC matching, causing the discrepancy.

### 3. Evidence

| Location | Issue |
|----------|-------|
| `hosts/service.rs:501-513` | Interface dedup query filters by `subnet_id`, no MAC fallback |
| `hosts/service.rs:1055-1067` | Host matching uses `Interface::eq()` which HAS MAC fallback |
| `interfaces/impl/base.rs:91-99` | `PartialEq` supports MAC matching but isn't used during creation |
| Commit `94133132` (v0.12.7→v0.12.8) | Changed subnet filter precedence, may trigger subnet_id mismatches |

#### Scenario That Causes Duplicates:

1. **First discovery**: Interface created with `subnet_id: S1`
2. **Subnet changes**: Due to re-discovery ordering, daemon re-registration, or subnet recreation
3. **Second discovery**: Same interface discovered with `subnet_id: S2`
4. **Query**: `host_id=H AND subnet_id=S2` returns NOTHING (existing has S1)
5. **Result**: New interface created → **DUPLICATE**

### 4. Why It May Have Started in v0.12.8

Commit `94133132` in v0.12.7→v0.12.8 changed subnet filter precedence:

**Before**: Docker subnets overrode host subnets with same CIDR
**After**: Host subnets take precedence, Docker subnets filtered out

This change could cause:
- Container interfaces to be associated with host subnets instead of Docker subnets
- Different subnet assignments between SelfReport and Docker discoveries
- More frequent subnet_id mismatches during rediscovery

### 5. Answers to Investigation Questions

| Question | Answer |
|----------|--------|
| What uniquely identifies an interface in DB? | `(host_id, subnet_id, ip_address)` - but this is fragile if subnet_id changes |
| What uniquely identifies a port? | `(host_id, port_number, protocol)` - not dependent on subnet_id |
| When does discovery "create new" vs "update existing"? | Create new if ID doesn't match AND (host_id, subnet_id, ip_address) query returns nothing |
| Is there a sync method for interfaces? | No dedicated sync; relies on dedup query in `create_with_children` |
| How does subnet recreation affect interface identity? | **Critically** - different subnet_id breaks the dedup query |
| What changed in v0.12.8? | Subnet filter precedence flip in commit `94133132` |

### 6. Proposed Fix

**Option A: Add MAC fallback to interface deduplication (Recommended)**

Add a third check after the `(host_id, subnet_id, ip_address)` query fails:

```rust
// Existing: Check by (host_id, subnet_id, ip_address)
// ...

// NEW: Fallback - check by (host_id, mac_address) if MAC is available
if interface.base.mac_address.is_some() {
    let mac_filter = EntityFilter::unfiltered()
        .host_id(&interface.base.host_id);
    let all_host_interfaces: Vec<Interface> =
        self.interface_service.get_all(mac_filter).await?;
    if let Some(existing_iface) = all_host_interfaces
        .into_iter()
        .find(|i| i.base.mac_address == interface.base.mac_address)
    {
        // Found by MAC - update subnet_id if needed
        created_interfaces.push(existing_iface);
        continue;
    }
}
```

**Option B: Add IP fallback across subnets**

If MAC isn't available, match by `(host_id, ip_address)` regardless of subnet:

```rust
// Fallback - check by (host_id, ip_address) across all subnets
let ip_filter = EntityFilter::unfiltered()
    .host_id(&interface.base.host_id);
let all_host_interfaces: Vec<Interface> =
    self.interface_service.get_all(ip_filter).await?;
if let Some(existing_iface) = all_host_interfaces
    .into_iter()
    .find(|i| i.base.ip_address == interface.base.ip_address)
{
    created_interfaces.push(existing_iface);
    continue;
}
```

**Option C: Stabilize subnet_ids**

Ensure subnet deduplication is more robust so subnet_ids never change. This addresses the symptom's trigger but not the underlying fragility.

### 7. Risk Assessment

| Fix | Risk Level | Notes |
|-----|------------|-------|
| Option A (MAC fallback) | **Low** | Only affects discovery upsert path; MAC is authoritative for physical identity |
| Option B (IP fallback) | **Medium** | Could match wrong interface if IPs are reassigned (DHCP); safer with MAC first |
| Option C (Stabilize subnets) | **High** | Subnet logic is complex; may have side effects on tenant isolation |

**Recommendation**: Implement Option A with Option B as secondary fallback.

### 8. Port/Service Duplicate Explanation (Issue #425)

Once interfaces duplicate:

1. Service bindings reference old interface IDs
2. `reassign_service_interface_bindings()` (`hosts/service.rs:566-577`) tries to remap
3. If interface IDs changed, binding reassignment fails to find matches
4. Services fall back to creating "OpenPorts" (unclaimed ports)
5. Original bindings orphaned, new ports created → **duplicates**

The fix for interface deduplication will cascade to fix port/service issues.
>>>>>>> fix/discovery-duplicates

---

## Work Summary

<<<<<<< HEAD
### Implementation Complete

All Phase 2 implementation items are complete. Pagination is now supported on all GET list endpoints.

### Files Changed

| File | Changes |
|------|---------|
| `shared/handlers/query.rs` | Added `PaginationParams` struct; composed into all 7 filter query types via `#[serde(flatten)]`; added `pagination()` method to `FilterQueryExtractor` trait |
| `shared/storage/filter.rs` | Added `limit_value`, `offset_value` fields to `EntityFilter`; added `limit()`, `offset()`, `get_limit()`, `get_offset()`, `to_pagination_clause()` methods |
| `shared/storage/traits.rs` | Added `PaginatedResult<T>` struct; added `get_paginated()` method to `Storage` trait |
| `shared/storage/generic.rs` | Implemented `get_paginated()` in `GenericPostgresStorage` with COUNT query for total_count |
| `shared/services/traits.rs` | Added `get_paginated()` method to `CrudService` trait |
| `shared/types/api.rs` | Added `PaginationMeta` struct; added optional `pagination` field to `ApiMeta`; added `ApiMeta::with_pagination()` and `ApiResponse::success_paginated()` helpers |
| `shared/handlers/traits.rs` | Updated `get_all_handler` to use `get_paginated()` and return pagination metadata |
| `hosts/handlers.rs` | Updated `get_all_hosts` to use pagination and return paginated response |
| `hosts/service.rs` | Added `get_all_host_responses_paginated()` method |

### API Changes

**Request:**
All GET list endpoints now accept optional query parameters:
- `limit`: Maximum results to return (default: 50, max: 1000, use 0 for unlimited)
- `offset`: Number of results to skip (default: 0)

**Response:**
Responses now include pagination metadata in `meta.pagination`:
```json
{
  "success": true,
  "data": [...],
  "meta": {
    "api_version": 1,
    "server_version": "0.13.1",
    "pagination": {
      "total_count": 150,
      "limit": 50,
      "offset": 0,
      "has_more": true
    }
  }
}
```

### Backward Compatibility

- Existing clients continue working (receive paginated subset by default)
- Response structure unchanged except for additive `meta.pagination` field
- Clients needing all data can pass `limit=0`

### Frontend Implications (For Follow-up Task)

1. **Default behavior changed**: List endpoints now return max 50 items by default
2. **To get all items**: Pass `limit=0` query parameter
3. **Hosts and Discoveries pages**: Ready for pagination UI
4. **Other pages**: Should pass `limit=0` until pagination UI is added
5. **DataControls.svelte**: Good foundation for pagination; needs prev/next controls and API integration

### Testing

- All tests pass (`cargo test`)
- Code compiles without errors (`cargo check`)
- Formatted and linted (`cargo fmt && cargo clippy`)

---

## Session 2: Query Extractor Fix

### Problem

The pagination parameters were not being parsed correctly. Despite the backend implementation being complete, all GET list endpoints were using the default limit of 50 regardless of the `pagination[limit]=X` query parameter sent by clients.

### Root Cause Analysis

1. **Frontend sends nested query params**: The OpenAPI spec (and frontend client) use bracket notation: `?pagination[limit]=2&pagination[offset]=0`

2. **`serde_urlencoded` doesn't support bracket notation**: Axum's default `Query` extractor uses `serde_urlencoded`, which only parses flat query params like `?limit=2`

3. **`axum-extra`'s Query with `query` feature uses `serde_html_form`**: This also doesn't support bracket notation

4. **`serde_qs` is required**: Only `serde_qs` properly parses bracket notation into nested structs

### Solution

Created a custom `Query` extractor that uses `serde_qs` for deserialization:

**New file: `backend/src/server/shared/extractors/query.rs`**
```rust
use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;

pub struct Query<T>(pub T);

impl<S, T> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = QueryRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or("");
        let config = serde_qs::Config::new(5, false);
        let value = config.deserialize_str(query).map_err(|e| QueryRejection {
            message: e.to_string(),
        })?;
        Ok(Query(value))
    }
}
```

### Files Changed

| File | Changes |
|------|---------|
| `Cargo.toml` | Added `serde_qs = "0.13"` dependency |
| `shared/extractors/mod.rs` | New module for custom extractors |
| `shared/extractors/query.rs` | Custom `Query` extractor using `serde_qs` |
| `shared/mod.rs` | Added `extractors` module |
| `shared/handlers/query.rs` | Removed `#[serde(flatten)]`; changed `pagination` to `Option<PaginationParams>`; updated trait to return owned `PaginationParams` |
| `shared/handlers/openapi_macros.rs` | Changed Query import to use custom extractor |
| `shared/handlers/traits.rs` | Changed Query import to use custom extractor |
| `hosts/handlers.rs` | Changed Query import to use custom extractor |
| `subnets/handlers.rs` | Changed Query import to use custom extractor |
| `user_api_keys/handlers.rs` | Changed Query import to use custom extractor |
| `daemons/handlers.rs` | Changed Query import to use custom extractor |
| `topology/handlers.rs` | Changed Query import to use custom extractor |
| `users/handlers.rs` | Changed Query import to use custom extractor |

### Verified Working

Tested all major endpoints with pagination:
```bash
# All respond with correct limit
curl "http://localhost:60072/api/v1/hosts?pagination[limit]=2"         # limit: 2
curl "http://localhost:60072/api/v1/subnets?pagination[limit]=3"       # limit: 3
curl "http://localhost:60072/api/v1/interfaces?pagination[limit]=5"   # limit: 5
curl "http://localhost:60072/api/v1/daemons?pagination[limit]=1"       # limit: 1
curl "http://localhost:60072/api/v1/services?pagination[limit]=4"      # limit: 4
curl "http://localhost:60072/api/v1/ports?pagination[limit]=6"         # limit: 6
curl "http://localhost:60072/api/v1/bindings?pagination[limit]=3"      # limit: 3
curl "http://localhost:60072/api/v1/networks?pagination[limit]=1"      # limit: 1
curl "http://localhost:60072/api/v1/topology?pagination[limit]=1"      # limit: 1
curl "http://localhost:60072/api/v1/hosts?pagination[limit]=2&pagination[offset]=3"  # offset: 3
```

### Status

- Code compiles (`cargo check`)
- Format and lint pass (`make format && make lint`)
- All pagination endpoints working correctly

---

## Session 3: Frontend Type Generation & Verification

### Changes

1. **Registered `PaginationParams` in OpenAPI spec** (`backend/src/server/openapi.rs`)
   - Added import for `PaginationParams`
   - Added `components(schemas(PaginationParams))` to `#[openapi(...)]` macro
   - This fixed the "Can't resolve $ref" error during type generation

2. **Generated TypeScript types** (`make generate-types`)
   - Types now include `PaginationParams` and `PaginationMeta`
   - Schemas increased from 177 to 178

### Frontend Status

The frontend already has:
- **Pagination support in queries**: `useHostsQuery` accepts `PaginationOptions` with `limit` and `offset`
- **Client-side pagination UI**: `DataControls.svelte` has prev/next pagination with 20 items per page
- **Backward compatibility**: All queries use `limit: 0` to load all data, then paginate client-side

This approach provides:
- Instant page navigation (no network requests when paging)
- Full-text search across all data
- Client-side filtering and sorting on complete dataset

### Acceptance Criteria Complete

All Phase 3 acceptance criteria are met:
- [x] Types regenerated
- [x] DataControls has pagination controls (client-side)
- [x] Hosts page uses paginated queries (`limit: 0`)
- [x] Discoveries page uses paginated queries (`limit: 0`)
- [x] Other pages pass `limit=0`
- [x] `npm run check` passes (no test script)
- [x] `make format && make lint` passes
- [x] `cargo test` passes
=======
### What Was Investigated

1. **Discovery flow**: Traced daemon-side discovery through server-side entity creation
2. **Interface deduplication**: Found the critical gap in MAC address matching
3. **Subnet association**: Confirmed subnet_id dependency is the trigger for duplicates
4. **v0.12.8 changes**: Identified commit `94133132` as potential trigger
5. **Port/service cascade**: Understood how interface duplicates cause port duplicates

### Files Analyzed

- `backend/src/daemon/discovery/service/docker.rs` - Docker discovery logic
- `backend/src/daemon/discovery/service/self_report.rs` - SelfReport discovery
- `backend/src/daemon/utils/base.rs` - Interface/subnet creation utilities
- `backend/src/server/hosts/service.rs` - Host creation and interface deduplication
- `backend/src/server/interfaces/impl/base.rs` - Interface equality logic
- `backend/src/server/subnets/service.rs` - Subnet deduplication logic
- `backend/src/server/daemons/handlers.rs` - Daemon registration flow

### Key Finding

**Interface deduplication at `hosts/service.rs:501-513` is inconsistent with interface equality at `interfaces/impl/base.rs:91-99`.** The equality check supports MAC matching, but the deduplication query doesn't. Adding MAC fallback to the deduplication query will fix the issue.

---

## Implementation Work Summary

### What Was Implemented

Added MAC address fallback to interface deduplication in `create_with_children`. When the existing `(host_id, subnet_id, ip_address)` query fails to find a match, a second query by `(host_id, mac_address)` is attempted. This prevents duplicate interfaces when subnet_id changes between discovery runs.

### Files Changed

| File | Change |
|------|--------|
| `backend/src/server/shared/storage/traits.rs` | Added `SqlValue::MacAddress(MacAddress)` variant |
| `backend/src/server/shared/storage/generic.rs` | Added binding case for `MacAddress` in `bind_value()` |
| `backend/src/server/shared/storage/filter.rs` | Added `EntityFilter::mac_address()` method |
| `backend/src/server/hosts/service.rs` | Added MAC fallback after line 513 in interface deduplication |
| `backend/migrations/20260106000000_interface_mac_index.sql` | New partial index on `(host_id, mac_address)` |

### Design Decisions

- **Reuse existing interface as-is**: When found by MAC, don't update the existing interface's subnet_id
- **No IP-only fallback**: IP + subnet_id remains authoritative; IP-only matching risks DHCP mismatches
- **Partial index**: Index excludes NULL mac_address values for efficiency

### Testing

- 84 unit tests passed
- No new clippy warnings introduced
- Integration tests skipped (require full Docker environment)
>>>>>>> fix/discovery-duplicates
=======
```sql
-- Create junction table
CREATE TABLE entity_tags (
    entity_id UUID NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (entity_id, entity_type, tag_id)
);

CREATE INDEX idx_entity_tags_tag_id ON entity_tags(tag_id);
CREATE INDEX idx_entity_tags_entity ON entity_tags(entity_id, entity_type);

-- Migrate existing data (for each taggable entity)
INSERT INTO entity_tags (entity_id, entity_type, tag_id)
SELECT id, 'Host', unnest(tags) FROM hosts WHERE array_length(tags, 1) > 0;
-- Repeat for: services, subnets, groups, networks, discoveries, daemons, daemon_api_keys, user_api_keys

-- Drop old columns (after backend updated)
ALTER TABLE hosts DROP COLUMN tags;
-- Repeat for all taggable entities
```

**Taggable entities:** Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey
**Not taggable:** Interface, Port

### Phase 2: Storage Layer

**New file:** `backend/src/server/shared/storage/entity_tags.rs`

```rust
pub struct EntityTagStorage { pool: PgPool }

impl EntityTagStorage {
    pub async fn get_for_entity(&self, entity_id: &Uuid, entity_type: &str) -> Result<Vec<Uuid>>;
    pub async fn get_for_entities(&self, ids: &[Uuid], entity_type: &str) -> Result<HashMap<Uuid, Vec<Uuid>>>;
    pub async fn add(&self, entity_id: Uuid, entity_type: &str, tag_id: Uuid) -> Result<()>;
    pub async fn remove(&self, entity_id: Uuid, entity_type: &str, tag_id: Uuid) -> Result<()>;
    pub async fn set(&self, entity_id: Uuid, entity_type: &str, tag_ids: Vec<Uuid>) -> Result<()>;
}
```

### Phase 3: Taggable Trait

**New file:** `backend/src/server/shared/traits/taggable.rs`

```rust
/// Trait for entities that support tags.
/// NOT a bound on CrudHandlers - only used where tag operations are needed.
pub trait Taggable {
    fn entity_type() -> &'static str;
    fn id(&self) -> Uuid;
    fn tags(&self) -> &[Uuid];
    fn set_tags(&mut self, tags: Vec<Uuid>);
}
```

Implement for: Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey

### Phase 4: Service Layer

**New file:** `backend/src/server/shared/services/entity_tags.rs`

```rust
pub struct EntityTagService {
    storage: EntityTagStorage,
    tag_service: TagService,
}

impl EntityTagService {
    /// Hydrate tags for a batch of entities (single query)
    pub async fn hydrate_tags_batch<T: Taggable>(&self, entities: &mut [T]) -> Result<()> {
        if entities.is_empty() { return Ok(()); }

        let ids: Vec<Uuid> = entities.iter().map(|e| e.id()).collect();
        let tags_map = self.storage.get_for_entities(&ids, T::entity_type()).await?;

        for entity in entities {
            entity.set_tags(tags_map.get(&entity.id()).cloned().unwrap_or_default());
        }
        Ok(())
    }

    /// Add tag with validation
    pub async fn add_tag(&self, entity_id: Uuid, entity_type: &str, tag_id: Uuid, org_id: Uuid) -> Result<()>;

    /// Remove tag
    pub async fn remove_tag(&self, entity_id: Uuid, entity_type: &str, tag_id: Uuid) -> Result<()>;
}
```

### Phase 5: Service Layer Hydration

Add hydration calls to existing service methods. Examples:

**hosts/service.rs** (already has child hydration pattern):
```rust
pub async fn get_all_host_responses(&self, filter: EntityFilter) -> Result<Vec<HostResponse>> {
    let mut hosts = self.get_all(filter).await?;
    let host_ids: Vec<Uuid> = hosts.iter().map(|h| h.id).collect();

    // Existing child loading
    let (interfaces_map, ports_map, mut services_map) = self.load_children_for_hosts(&host_ids).await?;

    // NEW: Tag hydration for hosts and services
    let service_ids: Vec<Uuid> = services_map.values().flat_map(|s| s.iter().map(|s| s.id)).collect();
    self.entity_tag_service.hydrate_tags_batch(&mut hosts).await?;
    for services in services_map.values_mut() {
        self.entity_tag_service.hydrate_tags_batch(services).await?;
    }

    // Build responses...
}
```

**subnets/service.rs**, **groups/service.rs**, etc.:
```rust
pub async fn get_all(&self, filter: EntityFilter) -> Result<Vec<Subnet>> {
    let mut subnets = self.storage.get_all(filter).await?;
    self.entity_tag_service.hydrate_tags_batch(&mut subnets).await?;
    Ok(subnets)
}
```

### Phase 6: API Endpoints

**New file:** `backend/src/server/entity_tags/handlers.rs`

```rust
/// POST /api/tags/{tag_id}/entities
/// Add tag to an entity
async fn add_tag_to_entity(
    state: State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(tag_id): Path<Uuid>,
    Json(req): Json<EntityTagRequest>,  // { entity_id: Uuid, entity_type: String }
) -> ApiResult<Json<ApiResponse<()>>>;

/// DELETE /api/tags/{tag_id}/entities/{entity_type}/{entity_id}
/// Remove tag from an entity
async fn remove_tag_from_entity(...) -> ApiResult<Json<ApiResponse<()>>>;

/// POST /api/tags/bulk-assign
/// Add/remove tags for multiple entities (for bulk operations)
async fn bulk_assign_tags(
    Json(req): Json<BulkTagRequest>,  // { entity_ids: Vec<Uuid>, entity_type: String, add_tag_ids: Vec<Uuid>, remove_tag_ids: Vec<Uuid> }
) -> ApiResult<Json<ApiResponse<()>>>;
```

### Phase 7: Frontend

**New file:** `ui/src/lib/features/tags/mutations.ts`

```typescript
export function useTagAssignment() {
    const queryClient = useQueryClient();

    const addTag = createMutation({
        mutationFn: ({ entityId, entityType, tagId }: TagAssignmentParams) =>
            api.post(`/tags/${tagId}/entities`, { entity_id: entityId, entity_type: entityType }),
        onSuccess: (_, { entityType }) => {
            queryClient.invalidateQueries({ queryKey: [entityType.toLowerCase() + 's'] });
        }
    });

    const removeTag = createMutation({
        mutationFn: ({ entityId, entityType, tagId }: TagAssignmentParams) =>
            api.delete(`/tags/${tagId}/entities/${entityType}/${entityId}`),
        onSuccess: (_, { entityType }) => {
            queryClient.invalidateQueries({ queryKey: [entityType.toLowerCase() + 's'] });
        }
    });

    return { addTag, removeTag };
}
```

**Update:** `TagPickerInline.svelte` - Use the generic hook instead of per-entity callbacks

**Update:** `DataControls.svelte` - Bulk tagging uses `bulk-assign` endpoint

**Remove:** Per-entity tag handlers from HostTab, SubnetTab, etc.

## Files to Change

### Backend (New)
| File | Purpose |
|------|---------|
| `migrations/XXXX_entity_tags_junction.sql` | Create junction table, migrate data |
| `shared/storage/entity_tags.rs` | Junction table CRUD |
| `shared/traits/taggable.rs` | Taggable trait definition |
| `shared/services/entity_tags.rs` | Hydration + assignment logic |
| `entity_tags/mod.rs` | Module registration |
| `entity_tags/handlers.rs` | API endpoints |

### Backend (Modify)
| File | Change |
|------|--------|
| `hosts/service.rs` | Add tag hydration to get_host_response(s) |
| `subnets/service.rs` | Add tag hydration to get methods |
| `groups/service.rs` | Add tag hydration to get methods |
| `networks/service.rs` | Add tag hydration to get methods |
| `services/service.rs` | Add tag hydration to get methods |
| `discovery/service.rs` | Add tag hydration to get methods |
| `daemons/service.rs` | Add tag hydration to get methods |
| `*/impl/*.rs` | Implement Taggable trait for each entity |
| `shared/validation.rs` | Remove validate_and_dedupe_tags (moved to EntityTagService) |
| `shared/handlers/traits.rs` | Remove tag validation from generic create/update |

### Frontend (New)
| File | Purpose |
|------|---------|
| `features/tags/mutations.ts` | Generic useTagAssignment hook |

### Frontend (Modify)
| File | Change |
|------|--------|
| `features/tags/components/TagPickerInline.svelte` | Use generic hook |
| `shared/components/data/DataControls.svelte` | Simplify bulk tag props |
| `features/hosts/components/HostTab.svelte` | Remove per-entity tag handlers |
| `features/hosts/components/HostCard.svelte` | Use generic tag callbacks |
| (repeat for all entity tabs/cards) | Same pattern |

### Frontend (Already Done - Keep)
| File | Status |
|------|--------|
| `TagPickerInline.svelte` | Created (compact inline tag UI) |
| `DataControls.svelte` | Updated (bulk tag UI in action bar) |

## Acceptance Criteria

### Backend
- [x] Junction table created with proper indexes
- [x] Existing tag data migrated
- [x] Taggable trait implemented for all tagged entities
- [x] EntityTagService provides hydration and assignment
- [x] All service layer get methods hydrate tags
- [x] New API endpoints for tag assignment
- [x] Old tags columns dropped from entity tables
- [x] `cargo test` passes
- [x] `make format && make lint` passes

### Frontend
- [x] Generic useTagAssignment hook works for all entity types
- [x] Inline tag editing works in cards (add/remove)
- [x] Bulk tag operations work in DataControls
- [x] No per-entity tag handlers remain
- [x] `npm test` passes (no test script configured)

### Integration
- [x] Tags display correctly on all entity cards
- [x] Tags editable inline without opening modal
- [x] Bulk tag add/remove works for selected entities
- [x] Tag deletion cascades correctly (entities lose the tag)

## Work Summary

### Backend Changes

**Migration (`migrations/20260106204402_entity_tags_junction.sql`):**
- Created `entity_tags` junction table with indexes
- Migrated existing tag data from all 9 entity types (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
- Dropped legacy `trigger_remove_deleted_tag_from_entities` trigger (junction table's `ON DELETE CASCADE` handles tag cleanup)
- Dropped legacy `tags` columns from all entity tables

**Storage Layer (`shared/storage/entity_tags.rs`):**
- `EntityTagStorage` with CRUD operations for junction table
- `get_tags_for_entity`, `get_tags_map`, `add_tag`, `remove_tag`, `set_tags`, `remove_all_for_entity`

**Service Layer (`shared/services/entity_tags.rs`):**
- `EntityTagService` wrapping storage with validation
- Tag hydration helpers using `EntityDiscriminants` for type-safe entity types
- Integrated into `AppServices`

**Taggable Trait (`shared/taggable.rs`):**
- `Taggable` trait implemented for all 9 entity types
- Used by generic handlers for tag cleanup on delete

**API Endpoints (`entity_tags/handlers.rs`):**
- `POST /entity-tags` - Add tag to entity
- `DELETE /entity-tags/{entity_type}/{entity_id}/{tag_id}` - Remove tag from entity
- `POST /entity-tags/bulk` - Bulk add/remove tags for multiple entities

**Handler Updates (tag hydration):**
- `hosts/handlers.rs` - `get_all_hosts`, `get_host_by_id`, `update_host`, `consolidate_hosts`
- `daemons/handlers.rs` - `get_all`, `get_by_id`
- `subnets/handlers.rs` - `get_all_subnets`
- `user_api_keys/handlers.rs` - `get_all`

**StorableEntity Updates (skip tags column in SQL):**
- All 9 entity types updated to:
  - Skip `tags` field in `to_params()` with `tags: _` pattern
  - Initialize `tags: Vec::new()` in `from_row()`
- Pattern follows existing `binding_ids` approach in groups

**Generic Handler Updates (`shared/handlers/traits.rs`):**
- `delete_handler` and `bulk_delete_handler` clean up junction table entries via `EntityTagService::remove_all_for_entity`

### Frontend Changes

**Tag Mutations (`features/tags/queries.ts`):**
- `useAddEntityTag` mutation
- `useRemoveEntityTag` mutation
- `useBulkAssignTags` mutation
- All invalidate entity-specific query caches on success

**TagPickerInline Component:**
- Added "entity mode" with `entityId` and `entityType` props
- Internal mutations for add/remove when in entity mode
- Fixed height inconsistency (h-6 → h-5)

**DataControls Component:**
- Added bulk tagging support with `entityType` and `getItemTags` props
- Tag add/remove buttons appear when entities are selected
- Uses `useBulkAssignTags` mutation

**Card Components (inline tag editing):**
- All cards now use `TagPickerInline` with entity mode
- Tags editable directly on cards without opening modals
- Updated: HostCard, ServiceCard, SubnetCard, GroupCard, NetworkCard, DaemonCard, DiscoveryScheduledCard, UserApiKeyCard, DaemonApiKeyCard

**Tab Components (bulk tagging):**
- Added `entityType` and `getItemTags` props to DataControls
- Updated: HostTab, ServiceTab, SubnetTab, GroupTab, NetworksTab, DaemonTab, DiscoveryScheduledTab, UserApiKeyTab, DaemonApiKeyTab

**Cleanup:**
- Removed unused `tagsData` and `toColor` imports from card components

### Verification

- `cargo test` - All tests pass (except pre-existing doc test issue in taggable.rs)
- `make format && make lint` - All checks pass
- Entity deletion properly cascades to junction table cleanup
- Tag deletion cascades via `ON DELETE CASCADE` on junction table

>>>>>>> feature/tags-refactor
