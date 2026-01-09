> **First:** Read `CLAUDE.md` (project instructions) — you are a **worker**.

<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
<<<<<<< HEAD
# Task: Fix HTTP 413 Error When Rebuilding Topology (Issue #451)

## Objective

Fix the HTTP 413 (Payload Too Large) error that occurs when rebuilding topology.
=======
# Task: Storable/Entity Trait Refactor

## Objective

Refactor the `StorableEntity` trait into two separate traits:
1. **`Storable`** - Base trait for anything stored in the database (including junction tables)
2. **`Entity`** - Extended trait for user-facing domain entities (excludes junction tables)

Additionally, consolidate entity naming and add taggability validation.
>>>>>>> trait-refactor

## Issue Summary

<<<<<<< HEAD
**GitHub Issue:** #451

**Reported Behavior:**
- Navigate to Topology section
- Click "Auto" then "Rebuild"
- Red error alert displays HTTP 413

**Environment:**
- v0.13.3
- Debian Trixie (Proxmox VM)
- Firefox 146.0.1
- Caddy reverse proxy
=======
Currently, `StorableEntity` is used for both domain entities (Host, Service, Network) and junction tables (GroupBinding, EntityTag, UserNetworkAccess). This leads to:
- Junction tables implementing stub methods (`network_id() -> None`, `set_updated_at()` as no-op)
- Junction tables being technically taggable via the tag API (even though they shouldn't be)
- Inconsistent entity naming across `table_name()`, OpenAPI macros, and `EntityDiscriminants`
>>>>>>> trait-refactor

**User's Troubleshooting:**
- Configured Caddy's `request_body` directive with 100MB limit
- Temporarily resolved the issue but errors recurred
- No errors in Docker server logs when failure occurred

<<<<<<< HEAD
## Investigation Approach

1. **Understand the topology rebuild flow:**
   - What endpoint is called?
   - What data is sent in the request body?
   - How large can this payload get?

2. **Check server-side limits:**
   - Axum/Tower body size limits
   - Any middleware limiting request size

3. **Check the payload:**
   - Is the full topology being sent unnecessarily?
   - Can we reduce payload size?
   - Should this be a streaming/chunked request?

4. **Consider solutions:**
   - Increase server body size limit
   - Optimize the payload (send only what's needed)
   - Document proxy configuration requirements
   - Add better error messaging

## Files Likely Involved

- `backend/src/server/topology/handlers.rs` - Topology endpoint handlers
- `backend/src/bin/server.rs` - Server configuration, body limits
- `ui/src/lib/features/topology/` - Frontend topology components
- `ui/src/lib/api/` - API client for topology endpoints

## Acceptance Criteria

- [ ] Topology rebuild works without 413 error for reasonably-sized networks
- [ ] Server body size limits are appropriately configured
- [ ] If payload optimization is possible, implement it
- [ ] If proxy configuration is required, document it clearly
- [ ] `cargo test` passes
- [ ] Error message is helpful if limit is exceeded

## Notes

- The issue may be in the reverse proxy (Caddy), but we should also ensure server-side limits are reasonable
- Consider if the topology rebuild really needs to send/receive large payloads
- Check if there's a way to make this operation more efficient
=======
# Task: Fix Service Binding Text Search in Groups (Issue #452)
=======
# Task: Fix Host Icon from Best Service (Issue #449)

## Objective

Fix the regression where host icons no longer display the icon from the "best" or "top" service.

## Issue Summary

**GitHub Issue:** #449

**Reported Behavior:**
- Navigate to Hosts section
- Observe host icons
- Question marks appear instead of service icons

**Expected Behavior:**
- Icons should display for the top-performing/best service
- Matches behavior from v0.12.x

**Additional Context:**
- In v0.12.x, a dropdown existed on host details page to select icon display strategy
- This configuration option is no longer available in current version
- Reporter unsure if removal was intentional

**Environment:** v0.13.3, regression since v0.13.2
>>>>>>> fix/449-host-icon

## Objective

Fix the broken text search functionality when selecting service bindings while creating/editing Groups.

## Issue Summary

**GitHub Issue:** #452

**Reported Behavior:**
- Navigate to Groups section
- Create or edit a Group
- Scroll to "Select a binding to add"
- Attempt to search for bindings by text
- Nothing shows up, despite bindings existing

**Expected Behavior:**
- Users should be able to search for substrings matching interface or service names

**Environment:** v0.13.3, suspected regression from v0.13.1 or later

## Investigation Approach

1. **Find the Group creation/edit modal** - Look in `ui/src/lib/features/groups/`
2. **Locate the binding selector component** - Likely a searchable dropdown or combobox
3. **Check the search/filter logic** - May be filtering on wrong field, case sensitivity issue, or empty results
4. **Compare with similar selectors** - Other entity selectors that work correctly
5. **Check for recent changes** - What changed in v0.13.1+ that could have broken this?

## Files Likely Involved

- `ui/src/lib/features/groups/` - Group-related components
- `ui/src/lib/components/` - Shared selector/search components
- Look for components like `BindingSelector`, `SearchableSelect`, `Combobox`

## Acceptance Criteria

- [ ] Text search in binding selector filters results correctly
- [ ] Search matches interface names
- [ ] Search matches service names
- [ ] Case-insensitive search works
- [ ] Empty search shows all available bindings
- [ ] `npm test` passes (if relevant tests exist)

## Notes

- This is a frontend bug - focus on the UI components
- May be related to how bindings are being filtered/displayed
- Check if the search is client-side or server-side
>>>>>>> fix/452-group-binding-search

## Work Summary

### Root Cause
<<<<<<< HEAD

<<<<<<< HEAD
The `rebuild` and `refresh` endpoints accepted `Json<Topology>` containing the **full topology** (hosts, interfaces, services, subnets, groups, ports, bindings, nodes, edges, etc.) but only actually used a few fields. Combined with Axum's default 2MB body limit, large networks would exceed this limit and trigger HTTP 413 errors.

### Solution Implemented

Created a lightweight `TopologyRebuildRequest` type that only includes fields the server actually needs:
- `network_id` - for authorization
- `options` - for graph building configuration
- `nodes` - for position preservation during rebuild
- `edges` - for edge reference during rebuild

This reduces payload size from potentially megabytes to kilobytes.

### Files Changed

**Backend:**
- `backend/src/server/topology/types/base.rs` - Added `TopologyRebuildRequest` type
- `backend/src/server/topology/handlers.rs` - Updated `rebuild` and `refresh` handlers to use new type

**Frontend:**
- `ui/src/lib/features/topology/queries.ts` - Updated `useRebuildTopologyMutation`, `useRefreshTopologyMutation`, and SSE auto-rebuild to send minimal payload
- `ui/static/openapi.json` - Regenerated with new type
- `ui/src/lib/api/schema.d.ts` - Regenerated TypeScript types

### Payload Size Comparison

| Scenario | Before | After |
|----------|--------|-------|
| Small network (10 hosts) | ~50KB | ~5KB |
| Medium network (100 hosts) | ~500KB | ~20KB |
| Large network (1000+ hosts) | ~5MB+ (413 error) | ~100KB |

### Authorization

- Permission requirement: `Member` (unchanged)
- Tenant isolation: Validated via `network_id` in request against user's `network_ids()`

### Testing

- `cargo test` - All tests pass
- `make format && make lint` - All checks pass
- Type generation successful
=======
The `RichSelect` component's search filter was not passing the `context` parameter to `displayComponent.getLabel()`. For `BindingWithServiceDisplay`, this context is required to look up service names from the services array. Without context, `getLabel` always returned "Unknown Service", making text search ineffective.

Additionally, the search was only checking `label` and `description` fields, but for bindings, the interface/port info is displayed via `getTags()`, which wasn't being searched.

### Changes Made

**File: `ui/src/lib/shared/components/forms/selection/RichSelect.svelte`**

1. **Line 67**: Added `context` parameter to `getLabel()` call:
   - Before: `displayComponent.getLabel(option)`
   - After: `displayComponent.getLabel(option, context)`

2. **Lines 69-70**: Added tag searching - now also searches the labels from `getTags()`:
   ```javascript
   const tags = displayComponent.getTags?.(option, context) ?? [];
   const tagLabels = tags.map((tag) => tag.label.toLowerCase()).join(' ');
   ```

3. **Lines 72-76**: Updated return to include tag matches:
   ```javascript
   return (
       label.includes(searchTerm) ||
       description.includes(searchTerm) ||
       tagLabels.includes(searchTerm)
   );
   ```

**File: `ui/src/lib/features/groups/components/GroupEditModal/GroupEditModal.svelte`**
=======

The issue was a race condition combined with incorrect fallback logic in `HostCard.svelte`.

**The problematic code:**
```javascript
Icon:
    serviceDefinitions.getIconComponent(hostServices[0]?.service_definition) ||
    entities.getIconComponent('Host'),
```

**What happened:**
1. On initial render, services haven't loaded yet → `hostServices` is `[]`
2. `hostServices[0]?.service_definition` evaluates to `undefined`
3. `getIconComponent(undefined)` returns `HelpCircle` (question mark icon)
4. `HelpCircle` is truthy, so the `|| entities.getIconComponent('Host')` fallback never triggers
5. When services load, the derived block should re-run, but the initial `HelpCircle` was being shown inconsistently

The inconsistency occurred because:
- Sometimes TanStack Query had cached data → services available immediately → correct icon
- Sometimes cache miss → initial render shows `HelpCircle` → re-render timing issues

### Fix

Changed the fallback logic to explicitly check if services exist:

```javascript
Icon:
    hostServices.length > 0
        ? serviceDefinitions.getIconComponent(hostServices[0].service_definition)
        : entities.getIconComponent('Host'),
```

This ensures:
- If no services (yet or ever) → Host icon is shown (not HelpCircle)
- If services exist → first service's icon is shown

### Files Changed

1. **`ui/src/lib/features/hosts/components/HostCard.svelte`** (lines 94-97)
   - Changed from `||` fallback to explicit ternary with `hostServices.length > 0` check

### Regarding Icon Strategy Dropdown

No evidence of an "icon strategy" dropdown exists in the current codebase. The implementation uses the first service (sorted by position) to determine the host icon. This appears to be the intended behavior.
>>>>>>> fix/449-host-icon

4. **Line 132**: Added filter to exclude "Unclaimed Open Ports" services from binding dropdown:
   ```javascript
   .filter((s) => s.service_definition !== 'Unclaimed Open Ports')
   ```

<<<<<<< HEAD
### Acceptance Criteria Status

- [x] Text search in binding selector filters results correctly
- [x] Search matches service names (via `getLabel` with context)
- [x] Search matches interface names (via `getTags` search)
- [x] Case-insensitive search works (all comparisons use `.toLowerCase()`)
- [x] Empty search shows all available bindings (early return on empty filterText)
- [x] `make format && make lint` passes
>>>>>>> fix/452-group-binding-search
=======
- `npm run check` (svelte-check): 0 errors, 0 warnings
- `npm run format && npm run lint`: Passes
>>>>>>> fix/449-host-icon
=======
# Task: Fix Grouped Hosts Pagination UX (Issue #450)

## Objective

Fix the unintuitive pagination behavior when hosts are grouped (e.g., by "Virtualized by").

## Issue Summary

**GitHub Issue:** #450

**Reported Behavior:**
- Navigate to Hosts section
- Apply grouping by "Virtualized by"
- Change the page number
- Groups with no additional pages disappear from the display
- Pagination applies globally to all groups simultaneously

**Expected Behavior (per reporter):**
- Individual paginators for each group
- Option to view all pages simultaneously
- Groups shouldn't vanish when paginating

**Environment:** v0.13.3

## Investigation Approach

1. **Understand current pagination implementation:**
   - Is pagination server-side or client-side?
   - How does grouping interact with pagination?
   - What determines which groups are visible?

2. **Analyze the UX problem:**
   - Global pagination + grouping = confusing results
   - Items from one group may be on page 1, items from another on page 2

3. **Consider solutions:**
   - **Option A:** Per-group pagination (more complex, best UX)
   - **Option B:** When grouped, show all items (no pagination)
   - **Option C:** When grouped, increase page size significantly
   - **Option D:** "Expand all" option within groups

4. **Check similar implementations:**
   - How do other tables/lists handle grouped pagination in this codebase?

## Files Likely Involved

- `ui/src/lib/features/hosts/` - Host list components
- `ui/src/lib/components/` - Shared table/list/pagination components
- Look for `DataTable`, `Pagination`, `GroupedList` type components

## Acceptance Criteria

- [ ] Grouped hosts display has intuitive pagination behavior
- [ ] Groups don't disappear unexpectedly when paginating
- [ ] Users can navigate through grouped data effectively
- [ ] Solution is consistent with other grouped displays in the app
- [ ] `npm test` passes

## Design Considerations

This is a UX improvement, so consider:
- What's the simplest solution that fixes the confusion?
- What do users actually need when viewing grouped hosts?
- Is per-group pagination worth the complexity, or is disabling pagination when grouped sufficient?

## Notes

- This is a frontend UX bug/improvement
- May require changes to shared pagination components
- Consider performance implications of showing all items when grouped

## Work Summary

### Solution Implemented

Added **server-side ordering** support to the hosts and services list endpoints. When grouping is active, the server returns items sorted by the group field, keeping groups contiguous across pages. This prevents groups from appearing/disappearing as users paginate.

**Example:** When grouped by "Virtualized By":
- Page 1: All VMware hosts (sorted by group field)
- Page 2: Remaining VMware hosts + start of KVM hosts
- Page 3: Remaining KVM hosts + Bare Metal hosts

### Backend Changes

**`backend/src/server/shared/storage/filter.rs`**
- Extended `EntityFilter` with `joins` field and `join()`, `to_join_clause()`, `has_joins()` methods

**`backend/src/server/shared/storage/generic.rs`**
- Updated `get_all_ordered` and `get_paginated` to use JOIN clauses from filter
- Uses table-qualified SELECT when JOINs are present to avoid column conflicts

**`backend/src/server/shared/handlers/query.rs`**
- Added `OrderDirection` enum (asc/desc) with `to_sql()` method

**`backend/src/server/hosts/handlers.rs`**
- Added `HostOrderField` enum with `to_sql()` and `join_sql()` methods
- Added `HostFilterQuery` struct with `group_by`, `order_by`, `order_direction` params
- Updated `get_all_hosts` handler to use new query struct

**`backend/src/server/hosts/service.rs`**
- Updated `get_all_host_responses_paginated` to accept `order_by` parameter

**`backend/src/server/services/handlers.rs`**
- Added `ServiceOrderField` enum with `to_sql()` and `join_sql()` methods
- Added `ServiceFilterQuery` struct (same pattern as hosts)
- Replaced generated handler with custom `get_all_services` handler

**`backend/src/server/openapi.rs`**
- Registered `OrderDirection`, `HostOrderField`, `ServiceOrderField` schemas

### Frontend Changes

**`ui/src/lib/shared/components/data/DataControls.svelte`**
- Added `onOrderChange` callback prop that exposes group/sort state changes
- Effect that tracks ordering changes and resets pagination to page 1

**`ui/src/lib/features/hosts/queries.ts`**
- Updated `HostQueryOptions` to include `group_by`, `order_by`, `order_direction`
- Updated `useHostsQuery` to pass ordering params to API

**`ui/src/lib/features/hosts/components/HostTab.svelte`**
- Added ordering state (`groupBy`, `orderBy`, `orderDirection`)
- Added `handleOrderChange` handler with field key to backend enum mapping
- Wired up `onOrderChange` to DataControls

**`ui/src/lib/features/services/queries.ts`**
- Updated `ServicesQueryParams` to include ordering parameters
- Updated `useServicesQuery` to pass ordering params to API

**`ui/src/lib/features/services/components/ServiceTab.svelte`**
- Same pattern as HostTab (ordering state, handler, mapping)

### Key Design Decisions

1. **Rust enums as source of truth:** `HostOrderField` and `ServiceOrderField` define orderable fields, generating TypeScript union types via OpenAPI
2. **EntityFilter handles JOINs:** JOINs flow through the existing query builder via `filter.join()` method
3. **Consolidated `join_sql()` method:** Returns `Option<&str>` - if Some, the JOIN is needed
4. **Separate `group_by` and `order_by` params:** Enables compound ORDER BY (group first ASC, then sort with configurable direction)
5. **Reset to page 1 on order change:** Per user preference

### Verification

- [x] Backend unit tests pass (84 passed)
- [x] Frontend type checking passes (svelte-check: 0 errors)
- [x] `make format && make lint` passes
- [x] TypeScript types generated correctly with new enum types

### Files Changed

| File | Change Type |
|------|-------------|
| `backend/src/server/shared/storage/filter.rs` | Modified |
| `backend/src/server/shared/storage/generic.rs` | Modified |
| `backend/src/server/shared/handlers/query.rs` | Modified |
| `backend/src/server/hosts/handlers.rs` | Modified |
| `backend/src/server/hosts/service.rs` | Modified |
| `backend/src/server/services/handlers.rs` | Modified |
| `backend/src/server/openapi.rs` | Modified |
| `ui/src/lib/shared/components/data/DataControls.svelte` | Modified |
| `ui/src/lib/features/hosts/queries.ts` | Modified |
| `ui/src/lib/features/hosts/components/HostTab.svelte` | Modified |
| `ui/src/lib/features/services/queries.ts` | Modified |
| `ui/src/lib/features/services/components/ServiceTab.svelte` | Modified |
| `ui/src/lib/api/schema.d.ts` | Auto-generated |
>>>>>>> fix/450-grouped-pagination
=======
### 1. Split StorableEntity into Storable + Entity

**`Storable` trait** (base, for all DB-stored types):
```rust
pub trait Storable: Sized + Clone + Send + Sync + 'static + Default {
    type BaseData;

    fn new(base: Self::BaseData) -> Self;
    fn get_base(&self) -> Self::BaseData;

    fn table_name() -> &'static str;
    fn id(&self) -> Uuid;
    fn created_at(&self) -> DateTime<Utc>;
    fn set_id(&mut self, id: Uuid);
    fn set_created_at(&mut self, time: DateTime<Utc>);

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error>;
    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error>;
}
```

**`Entity` trait** (extends Storable, for domain entities only):
```rust
pub trait Entity: Storable {
    fn entity_type() -> EntityDiscriminants;
    fn entity_name_singular() -> &'static str;
    fn entity_name_plural() -> &'static str;

    fn network_id(&self) -> Option<Uuid>;
    fn organization_id(&self) -> Option<Uuid>;
    fn is_network_keyed() -> bool;
    fn is_organization_keyed() -> bool;

    fn updated_at(&self) -> DateTime<Utc>;
    fn set_updated_at(&mut self, time: DateTime<Utc>);

    // Tags - default implementations
    fn is_taggable() -> bool { is_entity_taggable(Self::entity_type()) }
    fn get_tags(&self) -> Option<&Vec<Uuid>> { None }
    fn set_tags(&mut self, _tags: Vec<Uuid>) {}

    // Optional overrides
    fn set_source(&mut self, _source: EntitySource) {}
    fn preserve_immutable_fields(&mut self, _existing: &Self) {}
}
```

### 2. Implement Taggability as Single Source of Truth

Create a centralized function in `backend/src/server/shared/entities.rs`:

```rust
/// Single source of truth for which entity types support tagging
pub fn is_entity_taggable(entity_type: EntityDiscriminants) -> bool {
    matches!(entity_type,
        EntityDiscriminants::Host |
        EntityDiscriminants::Service |
        EntityDiscriminants::Subnet |
        EntityDiscriminants::Group |
        EntityDiscriminants::Network |
        EntityDiscriminants::Discovery |
        EntityDiscriminants::Daemon |
        EntityDiscriminants::DaemonApiKey |
        EntityDiscriminants::UserApiKey
    )
}
```

- `Entity::is_taggable()` has a default impl that calls `is_entity_taggable(Self::entity_type())`
- Tag API handlers (`/tags/assign/*`) must validate `is_entity_taggable(request.entity_type)` before processing
- ServiceFactory injects `entity_tag_service` only for entities where `T::is_taggable()` is true

### 3. Add Entity Naming Methods

Add to `Entity` trait:
- `fn entity_name_singular() -> &'static str` (e.g., "host")
- `fn entity_name_plural() -> &'static str` (e.g., "hosts")

Update OpenAPI macros to use these instead of string parameters where possible.

**Fix Topology inconsistency:** Currently uses `"topology"` for both singular and plural in OpenAPI macros - should use `"topologies"` for plural.

### 4. Update All Implementations

**Junction tables (impl Storable only):**
- `GroupBinding`
- `EntityTag`
- `UserNetworkAccess`
- `UserApiKeyNetworkAccess`

**Domain entities (impl Entity, which requires Storable):**
- All other entities: Host, Subnet, Service, Interface, Port, Binding, Network, Organization, User, Tag, Group, Discovery, Daemon, Topology, Invite, Share, UserApiKey, DaemonApiKey

## Files Likely Involved

- `backend/src/server/shared/storage/traits.rs` - Main trait definitions
- `backend/src/server/shared/entities.rs` - EntityDiscriminants, add `is_entity_taggable()`
- `backend/src/server/shared/storage/generic.rs` - GenericPostgresStorage (update trait bounds)
- `backend/src/server/shared/handlers/traits.rs` - Handler traits (update bounds)
- `backend/src/server/shared/handlers/openapi_macros.rs` - Consider using trait methods
- `backend/src/server/shared/services/traits.rs` - CrudService (update bounds)
- `backend/src/server/shared/services/factory.rs` - ServiceFactory (taggable injection logic)
- `backend/src/server/tags/handlers.rs` - Add taggability validation
- `backend/src/server/*/impl/*.rs` - All entity implementations (split trait impls)
- `backend/src/server/group_bindings/impl/base.rs` - Junction table impl
- `backend/src/server/shared/storage/entity_tags.rs` - Junction table impl
- `backend/src/server/topology/handlers.rs` - Fix naming inconsistency

## Acceptance Criteria

- [ ] `Storable` trait defined with base storage methods
- [ ] `Entity` trait extends `Storable` with domain-specific methods
- [ ] Junction tables implement only `Storable`
- [ ] Domain entities implement `Entity` (and thus `Storable`)
- [ ] `is_entity_taggable()` function is single source of truth
- [ ] Tag API handlers validate taggability before operations
- [ ] `entity_name_singular()` and `entity_name_plural()` added to Entity
- [ ] Topology naming fixed to use "topologies" plural
- [ ] All existing tests pass
- [ ] `cargo test` passes
- [ ] `make format && make lint` passes

## Notes

- This is a large refactor touching many files - work incrementally
- Ensure backward compatibility - no behavior changes, just better organization
- The `ChildStorableEntity` trait in `storage/child.rs` may need similar treatment
- Watch for trait bounds in generic functions - update `StorableEntity` to `Entity` or `Storable` as appropriate

## Work Summary

### Completed Tasks

1. **Split `StorableEntity` into `Storable` + `Entity` traits** (`storage/traits.rs`)
   - `Storable`: Base trait with `new()`, `get_base()`, `table_name()`, `id()`, `created_at()`, `set_id()`, `set_created_at()`, `to_params()`, `from_row()`
   - `Entity`: Extends `Storable` with `entity_type()`, `entity_name_singular()`, `entity_name_plural()`, `network_id()`, `organization_id()`, `updated_at()`, `set_updated_at()`, tagging methods, and optional overrides
   - Removed `StorableEntity` entirely (no backwards compatibility alias)

2. **Added `is_entity_taggable()` function** (`entities.rs`)
   - Single source of truth for taggable entity types
   - Used by `Entity::is_taggable()` default implementation

3. **Updated storage layer bounds** to use `Storable` (`generic.rs`, `child.rs`)

4. **Converted junction tables to `Storable`-only** (4 files):
   - `group_bindings/impl/base.rs`
   - `shared/storage/entity_tags.rs`
   - `users/impl/network_access.rs`
   - `user_api_keys/impl/network_access.rs`

5. **Converted domain entities to `Storable` + `Entity`** (18 files):
   - Added `entity_name_singular()` and `entity_name_plural()` methods to all domain entities
   - Split existing `impl StorableEntity` into separate `impl Storable` and `impl Entity` blocks

6. **Updated service/handler layer bounds**:
   - `CrudService<T: Entity>`
   - `ChildCrudService<T: ChildStorableEntity + Entity>`
   - `EventBusService<T: Into<EntityEnum>>`
   - `CrudHandlers: Entity`
   - Used `Entity as EntityEnum` aliasing to resolve naming conflict between the enum and trait

7. **Fixed Topology OpenAPI tag** (`topology/handlers.rs`)
   - Changed from "topology" to "topologies" for plural

8. **Added taggability validation** (`tags/handlers.rs`)
   - Added validation to `bulk_add_tag`, `bulk_remove_tag`, and `set_entity_tags` handlers
   - Returns 400 Bad Request for non-taggable entity types

9. **Removed junction table variants from Entity enum** (`entities.rs`)
   - Removed `GroupBinding`, `EntityTag`, `UserApiKeyNetworkAccess`, `UserNetworkAccess` variants
   - Removed associated imports, `From` implementations, and color/icon mappings

10. **Fixed test imports** across unit and integration tests
    - Updated `StorableEntity` imports to `Storable` or `Entity` as appropriate

### Files Changed

**Core trait definitions:**
- `backend/src/server/shared/storage/traits.rs`
- `backend/src/server/shared/entities.rs`
- `backend/src/server/shared/storage/child.rs`
- `backend/src/server/shared/storage/generic.rs`

**Service/Handler layer:**
- `backend/src/server/shared/services/traits.rs`
- `backend/src/server/shared/services/entity_tags.rs`
- `backend/src/server/shared/handlers/traits.rs`
- `backend/src/server/tags/handlers.rs`
- `backend/src/server/topology/handlers.rs`

**Junction tables (Storable-only):**
- `backend/src/server/group_bindings/impl/base.rs`
- `backend/src/server/shared/storage/entity_tags.rs`
- `backend/src/server/users/impl/network_access.rs`
- `backend/src/server/user_api_keys/impl/network_access.rs`

**Domain entities (Storable + Entity):**
- All 18 storage implementation files in `backend/src/server/*/impl/storage.rs`

**Other service/handler files with import updates:**
- `backend/src/server/auth/service.rs`, `auth/handlers.rs`
- `backend/src/server/daemons/handlers.rs`, `daemon_api_keys/handlers.rs`
- `backend/src/server/discovery/service.rs`, `groups/service.rs`, `hosts/service.rs`
- `backend/src/server/organizations/handlers.rs`, `services/service.rs`
- `backend/src/server/subnets/service.rs`, `subnets/impl/base.rs`
- `backend/src/server/topology/service/main.rs`, `users/service.rs`

**Test files:**
- `backend/src/tests/mod.rs`
- `backend/src/server/services/tests.rs`
- `backend/src/server/shared/storage/tests.rs`
- `backend/tests/integration/*.rs` (6 files)

### 11. File Reorganization (completed)

Moved junction table implementations into their parent entity directories:

**entity_tags → tags/entity_tags.rs:**
- Combined `shared/storage/entity_tags.rs` (EntityTag, EntityTagBase, EntityTagStorage) and `shared/services/entity_tags.rs` (EntityTagService) into single file `tags/entity_tags.rs`
- Updated exports in `tags/mod.rs`
- Updated 15+ files with new import paths

**group_bindings → groups/group_bindings.rs:**
- Combined `group_bindings/impl/base.rs` (GroupBinding, GroupBindingBase) and `group_bindings/impl/storage.rs` (GroupBindingStorage) into single file `groups/group_bindings.rs`
- Deleted entire `group_bindings/` directory
- Removed `pub mod group_bindings` from `server/mod.rs`
- Updated exports in `groups/mod.rs`
- Updated 3 files with new import paths

**Files deleted:**
- `backend/src/server/shared/storage/entity_tags.rs`
- `backend/src/server/shared/services/entity_tags.rs`
- `backend/src/server/group_bindings/` (entire directory)

**Files created:**
- `backend/src/server/tags/entity_tags.rs`
- `backend/src/server/groups/group_bindings.rs`

### Verification

- `cargo test --lib`: 84 passed, 0 failed
- `cargo fmt`: No changes needed
- `cargo clippy -- -D warnings`: No warnings
>>>>>>> trait-refactor
