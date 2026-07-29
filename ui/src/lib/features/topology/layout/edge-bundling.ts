import type { TopologyEdge } from '../types/base';
import { getRelationIdentity } from './edge-classification';

export interface BundledEdge {
	/** Unique ID for the bundle */
	id: string;
	/** Container IDs (or element IDs if same container) */
	sourceContainerId: string;
	targetContainerId: string;
	/** The individual edges in this bundle */
	edges: TopologyEdge[];
	/** Count of edges in the bundle */
	count: number;
	/** Representative source/target for the bundle edge (first edge's endpoints) */
	source: string;
	target: string;
	/** Edge type discriminant (all edges in bundle share this) */
	edgeType: string;
}

/**
 * Group edges by their container pair and edge type.
 * Edges between the same two containers of the same type are bundled together.
 *
 * - Intra-container edges (source and target in same container) are never bundled
 * - Edges with source/target not in the element-to-container map are not bundled
 * - Groups with only 1 edge are treated as unbundled
 */
export function bundleEdges(
	edges: TopologyEdge[],
	elementToContainer: Map<string, string>
): { bundles: BundledEdge[]; unbundled: TopologyEdge[] } {
	const groups = new Map<string, TopologyEdge[]>();
	const unbundled: TopologyEdge[] = [];

	for (const edge of edges) {
		const sourceContainer = elementToContainer.get(edge.source);
		const targetContainer = elementToContainer.get(edge.target);

		// Can't determine containers → unbundled
		if (!sourceContainer || !targetContainer) {
			unbundled.push(edge);
			continue;
		}

		// Intra-container edges → never bundled
		if (sourceContainer === targetContainer) {
			unbundled.push(edge);
			continue;
		}

		// Sort container IDs for consistent key regardless of direction
		const [c1, c2] =
			sourceContainer < targetContainer
				? [sourceContainer, targetContainer]
				: [targetContainer, sourceContainer];

		// Edges that stand for one specific relationship carry that relationship's identity in
		// the key, so two instances between the same containers never merge: a cable is the pair
		// of ports it joins, a dependency its dependency_id, a SameHost edge the host whose
		// addresses it ties together. Merging those would leave a bundle asserting "these subnets
		// are linked" while dropping *which* cable, host or dependency links them — and bundles
		// also lose their label and highlight only as a group.
		//
		// Only edges elevated onto their containers have no identity: those land on identical
		// endpoints, so merging them is the only way to show there is more than one. The backend
		// decides which is which — see `EdgeType::relation_key`.
		const relationIdentity = getRelationIdentity(edge);
		const key = relationIdentity
			? `${c1}:${c2}:${relationIdentity}`
			: `${c1}:${c2}:${edge.edge_type}`;

		const group = groups.get(key);
		if (group) {
			group.push(edge);
		} else {
			groups.set(key, [edge]);
		}
	}

	const bundles: BundledEdge[] = [];
	for (const [key, group] of groups) {
		if (group.length < 2) {
			unbundled.push(...group);
			continue;
		}

		const first = group[0];
		const sourceContainer = elementToContainer.get(first.source)!;
		const targetContainer = elementToContainer.get(first.target)!;
		const [c1, c2] =
			sourceContainer < targetContainer
				? [sourceContainer, targetContainer]
				: [targetContainer, sourceContainer];

		bundles.push({
			// The grouping key, which is what makes this bundle distinct — expansion state is
			// tracked by this id, so two bundles between one container pair must not share one.
			id: `bundle-${key}`,
			sourceContainerId: c1,
			targetContainerId: c2,
			edges: group,
			count: group.length,
			source: first.source,
			target: first.target,
			edgeType: first.edge_type
		});
	}

	return { bundles, unbundled };
}
