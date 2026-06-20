/**
 * Shared topology context resolver.
 *
 * Components need the current topology. In the **app** it comes from an
 * authenticated TanStack Query keyed by `selectedTopologyId`. In **share/embed**
 * pages it comes from a Svelte context store (no auth needed).
 *
 * `useTopology()` detects the mode and returns the right pieces. Usage:
 *
 * ```svelte
 * const topo = useTopology();
 * let topology = $derived(topo.fromContext ? $topo.store : topo.query.data?.find(t => t.id === $selectedTopologyId));
 * ```
 *
 * Or use the shorthand that works for most consumers:
 *
 * ```svelte
 * const { topology: topo$, isReadonly } = useTopologyDerived();
 * // topo$ is a Writable<Topology> in share context — use $topo$
 * // In app context, topo$ is undefined — use query.data instead
 * ```
 */

import { getContext } from 'svelte';
import type { Writable } from 'svelte/store';
import { useTopologiesQuery } from './queries';
import type { RenderableTopology } from './types/base';

interface TopologyFromContext {
	fromContext: true;
	store: Writable<RenderableTopology | null>;
	query: undefined;
	isReadonly: boolean;
}

interface TopologyFromQuery {
	fromContext: false;
	store: undefined;
	query: ReturnType<typeof useTopologiesQuery>;
	isReadonly: false;
}

export type TopologyHandle = TopologyFromContext | TopologyFromQuery;

/**
 * Returns the topology source and readonly flag.
 *
 * Must be called during component initialization (uses `getContext`).
 *
 * Consumers resolve the topology reactively using `$derived`:
 * ```
 * const topo = useTopology();
 * const topoStore = topo.fromContext ? topo.store : null;
 * let topology = $derived(
 *   topoStore ? $topoStore : topo.query.data?.find(t => t.id === $selectedTopologyId)
 * );
 * ```
 */
export function useTopology(): TopologyHandle {
	const ctx = getContext<Writable<RenderableTopology | null> | undefined>('topology');

	if (ctx) {
		// `isReadonly` is exposed as a hint for the app vs share contexts:
		// shares set it true, the app's editable view leaves it false.
		const isReadonly = getContext<boolean>('topology:isReadonly') ?? false;
		return { fromContext: true, store: ctx, query: undefined, isReadonly };
	}

	const query = useTopologiesQuery();
	return { fromContext: false, store: undefined, query, isReadonly: false };
}

// Re-export for convenience so consumers don't need a separate import
export { selectedTopologyId } from './queries';
