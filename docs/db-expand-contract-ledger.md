# DB Expand/Contract Ledger

Tracks destructive schema changes (**contracts**) that cannot ship in the same release as their additive **expand**, because during a rolling deploy an older container still references the column/table. A `DROP COLUMN` / `NOT NULL` / rename / type-change / enum-variant removal is only safe once **no deployed container references it** — typically the release *after* the code that stops using it has fully rolled out.

The coordinator maintains this file (see CLAUDE.md → *Database Expand/Contract Sequencing*). Every worktree doing DB-affecting work records its entries here and mirrors them in its TASK.md. At review/merge, scan for contracts whose precondition release has shipped and schedule them.

**Deploy model reminder:** old and new server containers run simultaneously during a deploy, and daemons lag further. An expand is safe in one deploy; a contract needs the expand (and any code that reads the column) fully retired from all running containers first.

## Status legend
`code-removal pending` → the release that stops referencing the column hasn't shipped · `drop scheduled` → precondition met, DROP queued for the named release · `done` → contracted.

## Pending

| Change | Expand | Code-removal release | Contract (DROP) release | Precondition | Status | Owner |
|---|---|---|---|---|---|---|
| `discovery.pending_credential_ids` (UUID[]) | superseded by `discovery.integration_targets` (0.17.2) | already unreferenced at 0.17.2 | **0.17.3** | none — no deployed container reads it | drop scheduled (0.17.3) | feat/credential-daemon-compat (Part C1) |
| `credentials.target_ips` (INET[]) | superseded by `integration_targets` / `host_credentials` | **0.17.3** (remove storage r/w, `base.rs` field, dedup + loopback readers) | **0.17.4** | 0.17.3 fully deployed — no container reads `target_ips` | code-removal pending | feat/credential-daemon-compat (Part C2) |
| `daemons.capabilities` (JSONB) | interfaced subnets move to `daemon_interfaced_subnets` junction (0.17.3) | **0.17.3** (delete `DaemonCapabilities`, stop reading/writing the column) | **0.17.4** | 0.17.3 fully deployed — no container reads/writes `capabilities` | code-removal pending | feat/credential-daemon-compat (Part B3) |

## Deferred diagnostic instrumentation (non-schema)

Temporary tracing added to triage a live issue, to remove or downgrade once the issue is
confirmed resolved. Not a schema concern, tracked here so the coordinator sees it at release time.

| Instrumentation | Added | Remove/downgrade when | Recommendation | Owner |
|---|---|---|---|---|
| GH #649 L2 diagnostics (commit `04097c3af`, all at **debug** level — off unless `SCANOPY_LOG_LEVEL=debug`): daemon `"SNMP host collection summary"` + `"SNMP ifTable walk finished"` + `"Bridge FDB walk finished"` split (`snmp/mod.rs`, `snmp/queries.rs`); server `"Discovery host received"` (`daemons/service/processing.rs`); prune-decision lines + `"L2 topology summary after discovery"` + FDB-resolution breakdown (`hosts/service/create.rs`, `hosts/service/topology.rs`, `hosts/subscriber.rs`) | 0.17.4 | #649 confirmed fixed on the reporter's setup (switches stay on the L2 map across scheduled scans) | Remove the debug lines added purely for #649 triage. The `should_prune_interfaces` gate + its behavior stay; only the tracing goes. | fix/issue-649-l2-topology-missing-devices |

## Notes
- **No native Postgres enums exist** in this schema — all "enums" (credential type, subnet/host virtualization, integration targets) are serde-tagged JSON stored in JSONB columns. Adding/removing a variant is a code + JSONB-data concern, not a DDL contract, but old-server *deserialization* of a new variant is still a coexistence risk (mind `#[serde(other)]` fallbacks and `Deploy-Mode: downtime` when a new variant can reach an old binary).
- Squawk flags every `DROP COLUMN` as unsafe; contract migrations need the documented exception annotation + `SET lock_timeout`/`statement_timeout` guards (pattern: `backend/migrations/20260502120004_drop_legacy_topology_columns.sql`).

## Done
_(none yet)_
