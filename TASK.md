> **First:** Read `CLAUDE.md` (project instructions) — you are a **worker**.

# Task: CSV Export Endpoints Implementation

## Overview
Add CSV export capability to all entities with list endpoints, reusing existing query parameters (filtering, ordering) with minimal code duplication.

## Design Decisions (Pre-Approved)
- **Endpoint style:** Separate `/export/csv` endpoints (e.g., `GET /api/hosts/export/csv`)
- **Pagination:** Export ALL matching records (ignore limit/offset)
- **Filtering/ordering:** Reuse existing FilterQuery types from list endpoints
- **Children:** Base entity only (interfaces, services, etc. have their own export endpoints)
- **Trait location:** Extend existing Entity trait (no separate trait)

## Implementation Steps

### Step 1: Add csv crate
**File:** `backend/Cargo.toml`
```toml
csv = "1.3"
```

### Step 2: Extend Entity trait with CSV methods
**File:** `backend/src/server/shared/storage/traits.rs`

Add to Entity trait:
```rust
type CsvRow: Serialize;
fn csv_headers() -> Vec<&'static str>;
fn to_csv_row(&self) -> Self::CsvRow;
```

### Step 3: Create CSV export handler module
**File:** `backend/src/server/shared/handlers/csv.rs` (NEW)

Generic handler that:
- Accepts FilterQuery (same as list endpoint)
- Fetches ALL matching records (no pagination)
- Writes CSV with headers
- Returns with download headers

### Step 4: Add OpenAPI macro
**File:** `backend/src/server/shared/handlers/openapi_macros.rs`

Add `crud_export_csv_handler!` macro similar to existing CRUD macros.

### Step 5: Wire up per entity
For each entity, add macro invocation and route.

## Entities to Implement

| Entity | FilterQuery | Notes |
|--------|-------------|-------|
| Host | HostFilterQuery | - |
| Service | ServiceFilterQuery | Exclude nested bindings in CsvRow |
| Subnet | SubnetFilterQuery | - |
| Interface | InterfaceQuery | - |
| Port | HostChildQuery | - |
| Binding | BindingQuery | Serialize enum as string |
| Group | GroupFilterQuery | Serialize binding_ids as comma-separated |
| Tag | TagFilterQuery | - |
| Daemon | DaemonFilterQuery | Exclude API key - use custom CsvRow |
| User | UserFilterQuery | Check for sensitive fields |
| Share | SharesQuery | - |
| Network | NetworkFilterQuery | - |

## Security Considerations
- Use `Authorized<Viewer>` - same as list endpoints
- Exclude sensitive fields from CSV (API keys, passwords)
- Tenant isolation via same filter logic as list endpoints

## Files to Create/Modify

| File | Change |
|------|--------|
| `backend/Cargo.toml` | Add `csv = "1.3"` |
| `backend/src/server/shared/storage/traits.rs` | Add CsvRow, csv_headers(), to_csv_row() |
| `backend/src/server/shared/handlers/mod.rs` | Add `pub mod csv;` |
| `backend/src/server/shared/handlers/csv.rs` | NEW: export_csv_handler |
| `backend/src/server/shared/handlers/openapi_macros.rs` | Add crud_export_csv_handler! macro |
| `backend/src/server/<entity>/impl/*.rs` | Implement new Entity methods |
| `backend/src/server/<entity>/handlers.rs` | Add macro invocation + route |

## Verification
```bash
cd backend && cargo build
cd backend && cargo test
curl -H "X-API-Key: <KEY>" "http://localhost:60072/api/hosts/export/csv" -o hosts.csv
curl -H "X-API-Key: <KEY>" "http://localhost:60072/api/hosts/export/csv?network_id=<ID>" -o filtered.csv
```

## API Testing
```
API Key: scp_u_YANq5G2OLn7zir5ixPydwe3WrXOsaWyw
Network ID: b19b9406-8e6e-44ed-a68e-c65e7738ff09
```

## Acceptance Criteria
- [ ] All entities have `/export/csv` endpoints
- [ ] Filtering works (same as list endpoints)
- [ ] Sensitive fields excluded (API keys, etc.)
- [ ] Downloaded files are valid CSV
- [ ] OpenAPI schema updated (`make generate-types`)
- [ ] Tests added
- [ ] `cd backend && cargo test` passes
- [ ] `make format && make lint` passes

## Work Summary

### Implemented

**Core Infrastructure:**
- Added `csv = "1.3"` to `Cargo.toml`
- Extended Entity trait with `CsvRow`, `csv_headers()`, and `to_csv_row()` methods in `shared/storage/traits.rs`
- Created generic CSV export handler at `shared/handlers/csv.rs`
- Added `crud_export_csv_handler!` macro to `shared/handlers/openapi_macros.rs`

**Entities with CSV Export (16 total):**
| Entity | CsvRow Type | Sensitive Field Handling |
|--------|-------------|--------------------------|
| Host | HostCsvRow | - |
| Subnet | SubnetCsvRow | - |
| Interface | InterfaceCsvRow | - |
| Port | PortCsvRow | - |
| Service | ServiceCsvRow | Excludes nested bindings |
| Binding | BindingCsvRow | - |
| Group | GroupCsvRow | binding_ids as comma-separated |
| Tag | TagCsvRow | - |
| Daemon | DaemonCsvRow | Excludes `url` field (contains connection secrets) |
| User | UserCsvRow | Excludes `password_hash` |
| Share | ShareCsvRow | Excludes `password_hash`, adds `has_password` bool |
| Network | NetworkCsvRow | - |
| Discovery | DiscoveryCsvRow | - |
| Topology | TopologyCsvRow | Metadata only (excludes graph data) |
| UserApiKey | UserApiKeyCsvRow | Excludes `key` (hash) |
| DaemonApiKey | DaemonApiKeyCsvRow | Excludes `key` (hash) |

**Entities NOT Implemented (by design):**
- `Organization`: Single-tenant access pattern (users only see their own org); no list endpoint
- `Invite`: Custom filtering via `list_active_invites`, doesn't fit standard CRUD pattern

### Files Modified

| File | Change |
|------|--------|
| `backend/Cargo.toml` | Added `csv = "1.3"` |
| `backend/src/server/shared/storage/traits.rs` | Extended Entity trait |
| `backend/src/server/shared/handlers/mod.rs` | Added `pub mod csv;` |
| `backend/src/server/shared/handlers/csv.rs` | NEW: generic CSV handler |
| `backend/src/server/shared/handlers/openapi_macros.rs` | Added macro |
| `backend/src/server/hosts/impl/storage.rs` | HostCsvRow impl |
| `backend/src/server/subnets/impl/storage.rs` | SubnetCsvRow impl |
| `backend/src/server/interfaces/impl/storage.rs` | InterfaceCsvRow impl |
| `backend/src/server/ports/impl/storage.rs` | PortCsvRow impl |
| `backend/src/server/services/impl/storage.rs` | ServiceCsvRow impl |
| `backend/src/server/bindings/impl/storage.rs` | BindingCsvRow impl |
| `backend/src/server/groups/impl/storage.rs` | GroupCsvRow impl |
| `backend/src/server/tags/impl/storage.rs` | TagCsvRow impl |
| `backend/src/server/daemons/impl/storage.rs` | DaemonCsvRow impl |
| `backend/src/server/users/impl/base.rs` | UserCsvRow impl |
| `backend/src/server/shares/impl/base.rs` | ShareCsvRow impl |
| `backend/src/server/networks/impl.rs` | NetworkCsvRow impl |
| `backend/src/server/discovery/impl/storage.rs` | DiscoveryCsvRow impl |
| `backend/src/server/topology/types/storage.rs` | TopologyCsvRow impl |
| `backend/src/server/user_api_keys/impl/storage.rs` | UserApiKeyCsvRow impl |
| `backend/src/server/daemon_api_keys/impl/storage.rs` | DaemonApiKeyCsvRow impl |
| `backend/src/server/organizations/impl/storage.rs` | OrganizationCsvRow impl (trait compliance) |
| `backend/src/server/invites/impl/base.rs` | InviteCsvRow impl (trait compliance) |
| All entity `handlers.rs` files | Added macro + route wiring |

### Security Notes
- All CSV exports require `Authorized<Viewer>` permission
- Tenant isolation enforced via same filter logic as list endpoints
- Sensitive fields excluded from all CsvRow types

### Verification
- `cargo test` - All 89 tests pass
- `cargo fmt --all` - Clean
- `cargo clippy --all-targets --all-features` - Pre-existing warnings only (no new warnings)

---

## UI Implementation

### Overview
Added CSV export button to all entity tab pages that triggers download using the backend CSV export endpoints.

### Files Created

| File | Purpose |
|------|---------|
| `ui/src/lib/api/entities.ts` | Entity-to-API-path mapping utility |
| `ui/src/lib/shared/utils/csvExport.ts` | CSV download utility with error handling |

### Files Modified

| File | Change |
|------|--------|
| `ui/src/lib/shared/components/data/DataControls.svelte` | Added `onCsvExport` callback prop, `Download` icon, CSV button with loading state |
| `ui/src/lib/features/hosts/components/HostTab.svelte` | Added CSV export with server-side filters (tag_ids, order_by, order_direction) |
| `ui/src/lib/features/services/components/ServiceTab.svelte` | Added CSV export with server-side filters |
| `ui/src/lib/features/subnets/components/SubnetTab.svelte` | Added CSV export (all records) |
| `ui/src/lib/features/groups/components/GroupTab.svelte` | Added CSV export (all records) |
| `ui/src/lib/features/tags/components/TagTab.svelte` | Added CSV export (all records) |
| `ui/src/lib/features/daemons/components/DaemonTab.svelte` | Added CSV export (all records) |
| `ui/src/lib/features/networks/components/NetworksTab.svelte` | Added CSV export (all records) |

### Implementation Details

**Entity Path Mapping (`entities.ts`):**
- Uses `EntityDiscriminants` type from generated schema
- Maps all entity types to their API export paths
- Special handling for API keys: `UserApiKey` → `auth/keys`, `DaemonApiKey` → `auth/daemon`
- Returns `null` for entities without export support (Organization, Invite, Unknown)

**CSV Download Utility (`csvExport.ts`):**
- `downloadCsv(entityType, params)` function
- Constructs URL with filter parameters (tag_ids, order_by, order_direction)
- Uses `credentials: 'include'` for authentication
- Creates blob URL and triggers browser download
- Error handling with toast notifications via `pushError`

**DataControls Component:**
- New `onCsvExport?: (() => void | Promise<void>) | null` prop
- Button only renders when callback is provided
- Loading state shows "Exporting..." during download
- Disabled state prevents double-clicks

**Tab Components:**
- Tabs with server-side filtering (Host, Service) pass current filter state to export
- Tabs with client-side filtering export all records

### UI Verification
- `npm run check` - 0 errors, 0 warnings
- `npm run format` - All files properly formatted
