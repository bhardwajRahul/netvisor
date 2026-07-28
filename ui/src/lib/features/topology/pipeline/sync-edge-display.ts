import type { Node, Edge } from '@xyflow/svelte';
import { getEdgeDisplayState } from '../interactions';

/**
 * Compute updated edge display state (animation, highlight, selection)
 * based on current selection and filter stores.
 *
 * **Preserves object identity for edges whose display state did not change**,
 * and returns the input array itself when none changed.
 *
 * This runs on every selection change *and every edge pointer-enter/leave*. It
 * used to spread a fresh object (and a fresh `data` object) for every edge
 * unconditionally, which meant hovering one edge re-ran every `$derived` in
 * every `CustomEdge` — and also broke SvelteFlow's `edge === previous.edge`
 * memo, so it recomputed every edge's path geometry too. On a graph with
 * hundreds of edges that turned a pointer move into O(edges) work.
 *
 * The previous flags are already stored on `e.data`, so the comparison needs no
 * side state: an edge is rebuilt only when one of its flags actually differs.
 */
export function computeEdgeDisplayUpdates(
	currentEdges: Edge[],
	selectedNode: Node | null,
	selectedEdge: Edge | null,
	searchHidden: Set<string>,
	tagHidden: Set<string>
): Edge[] {
	const hasActiveSelection = !!(selectedNode || selectedEdge);
	let anyChanged = false;

	const next = currentEdges.map((e) => {
		const { shouldAnimate, shouldShowFull, isEndpointSearchHidden, isEndpointTagHidden } =
			getEdgeDisplayState(e, selectedNode, selectedEdge, searchHidden, tagHidden);
		const isSelected = selectedEdge?.id === e.id;

		const previous = e.data as Record<string, unknown> | undefined;
		const unchanged =
			e.animated === false &&
			previous !== undefined &&
			previous.shouldShowFull === shouldShowFull &&
			previous.shouldAnimate === shouldAnimate &&
			previous.isSelected === isSelected &&
			previous.hasActiveSelection === hasActiveSelection &&
			previous.isEndpointSearchHidden === isEndpointSearchHidden &&
			previous.isEndpointTagHidden === isEndpointTagHidden;

		if (unchanged) return e;

		anyChanged = true;
		return {
			...e,
			data: {
				...e.data,
				shouldShowFull,
				shouldAnimate,
				isSelected,
				hasActiveSelection,
				isEndpointSearchHidden,
				isEndpointTagHidden
			},
			animated: false
		};
	});

	// Returning the same array reference lets callers skip the store write, which
	// in turn stops the downstream merge effect reallocating the rendered edge
	// array on every pointer event.
	return anyChanged ? next : currentEdges;
}
