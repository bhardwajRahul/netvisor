# Proposal: node aggregation for large topologies

**Status: proposal only. Not implemented. Needs a coordinator/product decision
before any code is written.**

This is the structural lever behind L2 Physical's scale problem. It is not a
perf tweak — it changes what the user sees by default — which is why it is
written up rather than shipped alongside the rendering fixes.

## The problem it addresses

In L2 Physical the backend emits one `Container` per host and one `Element` per
interface (`backend/src/server/topology/service/l2_builder.rs:159-211`). A
customer with a few hundred hosts therefore gets **hundreds of containers plus
thousands of elements**, laid out as tall columns of cards that are unreadably
small at the zoom level needed to see the whole graph.

The rendering fixes in this branch reduce the *cost per node*. They do not
reduce the *node count*. Aggregation is the only lever that changes the order of
magnitude, and it is also the only one that makes the view legible rather than
merely faster — at that zoom the cards convey nothing anyway.

## Why this is mostly assembly, not new machinery

Three of the four pieces already exist:

1. **Collapsible containers.** `ui/src/lib/features/topology/collapse.ts` already
   implements leveled collapse (levels 1–4), persistence, and edge aggregation
   into `AggregatedEdge` for collapsed containers.
2. **Auto-collapse by container type.** `pipeline/execute-layout.ts:251-296`
   already collapses containers whose type metadata sets
   `collapsed_by_default`, and infers the user's level so an explicit expand is
   respected.
3. **Composable grouping rules.** `GroupingConfig` already expresses nested
   structural grouping generically, which is where the new parent level belongs.

What is missing is (4): **a grouping rule that parents host containers under the
switch they are physically attached to**, and a policy for when it engages.

## The shape

Introduce a grouping rule that nests a container under another container derived
from a relationship, rather than from a field on the node. For L2 that
relationship is "this host's uplink interface has a `neighbor_interface_id` on
that host". Composed with the existing collapse machinery:

- Each switch becomes a parent container holding its attached hosts.
- Those parents collapse by default above a size threshold.
- The 400-host column collapses to ~8 switch nodes with a child count each,
  expandable on click.

Crucially the rule must be expressed in terms the platform already has —
"group by a relationship to another node" — **not** as an L2/switch-specific
concept. A `ByUplinkNeighbour` rule baked into shared primitives would be
exactly the domain leakage the topology abstraction is meant to prevent. The
same generic rule should be able to express "group VMs under their hypervisor"
or "group services under their host" without modification.

## The decisions that are not ours

1. **Should collapsed-by-default be the default at scale, or opt-in?**
   Collapsing by default makes a large topology legible immediately, but a user
   who lands on a collapsed view may not realise there is detail underneath.
   Opt-in preserves today's behaviour but leaves the default experience slow.
2. **What threshold, and threshold of what?** Node count, host count, or
   "doesn't fit on screen at readable zoom"? A count is simple and
   view-agnostic; a legibility-based trigger is better UX but harder to reason
   about and to keep stable.
3. **Is a switch the right grouping for L2 specifically?** Racks, sites or
   subnets may match how customers actually think about their estate. This
   overlaps with the planned physical-infrastructure view, and picking the wrong
   grouping is worse than picking none.
4. **Does grouping change the data model?** If switch-parenting is derived at
   render time it is cheap and reversible. If it needs to be persisted or
   server-computed, that is a schema conversation and should not be settled as a
   side effect of a performance task.

## Sizing

Not estimated deliberately: the rendering fixes on this branch should be
measured against the seeded large dataset first. If they bring a 400-host graph
to an acceptable interaction cost, aggregation becomes a legibility feature to
schedule on its own merits rather than a performance necessity — and that
changes both its priority and its design.

## Recommendation

Decide (1) and (3) before any implementation. If the answer to "should we do
this" is yes, the work is small enough to fold into the existing grouping-rule
system and should be scoped as a UX change with a perf benefit, not the reverse.
