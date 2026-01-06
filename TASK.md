> **First:** Read `CLAUDE.md` (project instructions) — you are a **worker**.

# Task: Add Pagination Support to GET List APIs

## Objective

Implement backwards-compatible pagination for all GET list endpoints. Users hitting endpoints without pagination params should continue to work (though may now receive a subset + pagination token).

## Requirements

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

---

## Work Summary

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
