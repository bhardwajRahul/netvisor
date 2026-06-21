# Large source-file audit — June 2026

Inventory of source files ≥ 1000 LoC across Rust / Svelte / TypeScript, with the
pattern that drove each one large and a verdict. Generated / vendored code is
excluded: `backend/vendor/`, `ui/src/lib/paraglide/`, `ui/src/lib/api/schema.d.ts`,
`ui/static/openapi*.json`, and build artifacts.

Generated with `find … -name '*.rs|*.svelte|*.ts' | xargs wc -l | sort -rn`.
Churn = commits touching the file since tag `v0.16.2`.

## Verdicts at a glance

| File | LoC | Churn | What it contains | Why it got large | Verdict |
|------|----:|------:|------------------|------------------|---------|
| `backend/src/server/organizations/demo_data.rs` | 5210 | 7 | One `DemoData` builder + ~30 `generate_*` free fns that hand-construct the demo org's entire entity graph | Data-construction file: every demo entity (hosts, services, interfaces, …) is spelled out literally; `generate_hosts_and_services` alone is ~2100 lines | **defer** (split by entity group — layout below) |
| `backend/src/server/hosts/service.rs` | 3072 | 9 | `HostService` CRUD, discovery merge, consolidation, LLDP/FDB resolution | Single `impl HostService` block accreting 24 methods across features | **split now — DONE** |
| `backend/src/server/billing/service.rs` | 3060 | 55 | `BillingService`: Stripe products, checkout, webhooks, subscription lifecycle | Single `impl BillingService` block, 24 methods; highest churn in the repo | **split now — DONE** |
| `backend/src/server/daemons/service.rs` | 2222 | 4 | `DaemonService`: HTTP client, inbound processing, ServerPoll loop, standby | Single `impl DaemonService` block, 36 methods | **split now — DONE** |
| `backend/src/server/discovery/service.rs` | 1808 | 8 | `DiscoveryService`: sessions, scheduling, snapshot coordination, cleanup | Single `impl DiscoveryService` block, 28 methods | **split now — DONE** |
| `backend/src/server/auth/handlers.rs` | 1631 | 3 | Every auth axum endpoint: register, login, password, verification, OIDC | Kitchen-sink handlers file — every endpoint for the auth group as free fns | **split now — DONE** |
| `ui/src/lib/features/topology/layout/elk-layout.ts` | 1573 | — | elkjs layout engine adapter for topology | One module owning the whole layout pipeline | defer (UI, out of scope this branch — layout below) |
| `backend/src/daemon/discovery/integration/snmp/queries.rs` | 1555 | — | SNMP OID query tables + parsers | Cohesive data tables (OID → field mappings) | **acceptable as-is** (data) |
| `backend/src/server/billing/types/base.rs` | 1507 | — | Billing enums/structs: plans, features, status, invoices, cancel flow | Multi-concept type module — 5 unrelated domains in one file | **split now — DONE** |
| `ui/src/lib/shared/components/data/DataControls.svelte` | 1450 | — | Data-table controls (filter/sort/paginate/columns) | One component holding state + handlers + markup for all controls | defer (UI — layout below) |
| `backend/src/server/services/service.rs` | 1358 | 4 | `ServiceService`: binding validation/mutation, upsert, transfer | Single `impl ServiceService` block, 16 methods | **split now — DONE** |
| `backend/src/server/services/impl/patterns.rs` | 1355 | — | `Pattern` enum + `Pattern::matches` service-detection evaluator + tests | One ~530-line `matches()` evaluator over 15 variants; the rest are its support types and a tight test suite | **acceptable as-is** (cohesive algorithm — see note) |
| `backend/src/server/shared/storage/filter.rs` | 1288 | — | `StorableFilter<T>` SQL WHERE-clause builder (~129 methods) | Builder accreting one method per entity/column filter need | **split now — DONE** |
| `backend/src/server/topology/service/workloads_builder.rs` | 1281 | 3 | Topology workloads-view builder | Builder for one perspective; already inside `topology/service/` | acceptable (already modular) |
| `backend/src/daemon/discovery/service/network/scan.rs` | 1210 | — | Network scan orchestration (daemon) | Scan state machine | monitor |
| `backend/src/daemon/utils/scanner.rs` | 1200 | — | Low-level port/host scanner (daemon) | Scanner internals | monitor |
| `backend/src/daemon/discovery/buffer.rs` | 1196 | — | Discovery result buffering/batching (daemon) | Buffer + flush logic | monitor |
| `backend/src/daemon/discovery/integration/docker/scanner.rs` | 1162 | — | Docker container discovery (daemon) | Integration scanner | monitor |
| `ui/src/lib/features/topology/components/panel/inspectors/InspectorMultiSelect.svelte` | 1147 | — | Multi-select inspector panel | Component state + handlers + markup | defer (UI) |
| `ui/src/lib/features/topology/interactions.ts` | 1145 | — | Topology canvas interaction handlers | Pointer/drag/zoom handlers in one module | defer (UI) |
| `backend/src/server/daemons/handlers.rs` | 1119 | — | Daemon axum endpoints | Handlers file (below 1.2k; not yet split) | monitor |
| `backend/src/server/services/impl/tests.rs` | 1038 | — | Service unit tests | Test file (acceptable — tests cluster per module) | acceptable (tests) |
| `backend/src/server/topology/service/application_builder.rs` | 1033 | — | Topology application-view builder | Builder; already inside `topology/service/` | acceptable (already modular) |
| `backend/src/server/hosts/handlers.rs` | 1011 | — | Host axum endpoints | Handlers file (below 1.2k) | monitor |
| `backend/src/server/brevo/service.rs` | 1006 | — | Brevo (email marketing) API client | Single service, near threshold | monitor |
| `ui/src/lib/features/topology/components/visualization/BaseTopologyViewer.svelte` | 1004 | — | Base topology canvas viewer | Component near threshold | monitor |

**Counts:** 26 files ≥ 1000 LoC inventoried; 8 split this branch; 2 deferred backend (demo_data + handlers when they grow); UI offenders deferred to a follow-up branch.

Note on `patterns.rs`: the bulk is `Pattern::matches`, a single expression evaluator that is the core of service detection. Splitting it would fragment one algorithm across modules for no navigational gain; its support types and tests are tightly coupled to it. Left whole deliberately.

## Systemic patterns

Three structural signatures explain every "split now" offender:

1. **The accreting service `impl` block** (5 offenders: hosts, billing, daemons,
   discovery, services). A single `impl <Entity>Service { … }` grows one method
   per feature until it is thousands of lines. This is the dominant pattern.
2. **The kitchen-sink handlers file** (1 offender: `auth/handlers.rs`). Every
   axum endpoint for an entity group lives as free functions in one file.
3. **The multi-concept type module** (2 offenders: `billing/types/base.rs`,
   `shared/storage/filter.rs`). Many unrelated types, or a builder with one
   method per need, pile into one module.

### Recommended split shape

Turn the file `foo.rs` into a directory `foo/`:

- `mod.rs` keeps, **byte-for-byte**: the `use` block, the struct/enum
  *definitions*, all trait `impl`s (`EventBusService`, `CrudService`, …), any
  free helper fns shared across seams, and the `#[cfg(test)]` module.
- Each responsibility becomes a submodule that **re-opens the same `impl`**
  (`impl HostService { … }` / `impl<T: Storable> StorableFilter<T> { … }`) and
  pulls shared imports with `use super::*;`. For free-function handlers, the
  submodules hold the fns and `mod.rs` re-exports them (`use oidc::*;`) so the
  router/`routes!()` paths are unchanged. For pure type modules, `mod.rs`
  re-exports with `pub use plans::*;` so external paths (`billing::types::base::*`)
  don't move.
- The only mechanical change splitting an `impl` forces: a private helper called
  from a sibling submodule must widen to `pub(crate)`. This is the minimum
  scaffolding the split needs and is the only non-additive edit.

### Worked example — `hosts/service.rs` (3072 → 8 files)

```
hosts/service/
├── mod.rs          # use block, HostLimitContext, HostService,
│                   # impl EventBusService, impl CrudService,
│                   # LldpResolutionStats, 3 free helper fns, #[cfg(test)] tests
├── lifecycle.rs    # new + query/response/children helpers
├── create.rs       # create_from_request, create_with_children
├── update.rs       # update_from_request + sync_{ip_addresses,ports,services}
├── discovery.rs    # discover_host, interface linking, subnet/VLAN reconcile
├── consolidate.rs  # IP/MAC matching, locking, upsert_host, consolidate_hosts
├── topology.rs     # resolve_lldp_links, resolve_fdb_links
└── delete.rs       # delete_host
```

Verification (run per split): reassemble `mod.rs`-head + the single original
`impl …{` opener + every submodule body (scaffolding stripped) + `}` +
`mod.rs`-tail, then `diff` against the pre-split file. For all 8 splits the diff
was empty except a handful of documented `pub(crate)` visibility prefixes
(0 for the two pure-type/`pub use` modules).

## Deferred splits (proposed layouts)

Not done this branch; listed so the next pass can pick them up.

- **`organizations/demo_data.rs` (5210)** → `demo_data/` with `mod.rs` (the
  `DemoData` struct + top-level `build`) + `networks.rs`, `tags.rs`,
  `credentials.rs`, `hosts_services.rs` (the ~2100-line generator, itself a
  candidate for further per-host-archetype splitting), `interfaces.rs`,
  `vlans.rs`, `daemons.rs`, `discoveries.rs`, `misc.rs` (api keys, shares,
  dependencies). Each `generate_*` group is contiguous; same byte-for-byte
  recipe applies.
- **`daemons/handlers.rs` (1119)** and **`hosts/handlers.rs` (1011)** → split
  when they cross 1200, same handlers/-directory recipe as `auth/handlers/`.
- **`ui/.../layout/elk-layout.ts` (1573)** → `layout/` with `elk-layout.ts`
  (orchestrator) + `node-sizing.ts`, `edge-routing.ts`, `port-placement.ts`,
  `options.ts`.
- **`ui/.../data/DataControls.svelte` (1450)** → extract `FilterControls`,
  `SortControls`, `ColumnControls`, `PaginationControls` child components, with
  `DataControls.svelte` as the composing shell.
- **`ui/.../inspectors/InspectorMultiSelect.svelte` (1147)** and
  **`topology/interactions.ts` (1145)** → same component/handler extraction.
