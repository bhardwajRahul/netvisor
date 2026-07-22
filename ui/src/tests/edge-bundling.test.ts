import { describe, it, expect } from 'vitest';
import { bundleEdges } from '$lib/features/topology/layout/edge-bundling';
import type { components } from '$lib/api/schema';

type TopologyEdge = components['schemas']['Edge'];

const PROXY_BOX = 'box-proxy';
const DB_BOX = 'box-db';

function edge(
	source: string,
	target: string,
	edgeType: string,
	extra: Record<string, unknown> = {}
): TopologyEdge {
	return {
		id: `${edgeType}-${source}-${target}`,
		edge_type: edgeType,
		source,
		target,
		source_handle: 'Bottom',
		target_handle: 'Top',
		is_multi_hop: false,
		label: null,
		...extra
	} as unknown as TopologyEdge;
}

/** Both containers have a card in each of the two subnet boxes. */
const elementToContainer = new Map<string, string>([
	['api-proxy', PROXY_BOX],
	['api-db', DB_BOX],
	['worker-proxy', PROXY_BOX],
	['worker-db', DB_BOX]
]);

describe('edge bundling', () => {
	it('keeps each container’s SameContainer edge separate', () => {
		// Two different containers each spanning the same pair of subnets. Merging them would
		// produce a bundle claiming the subnets are linked while dropping which container links
		// them — and bundles render without a label and highlight only as a group.
		const { bundles, unbundled } = bundleEdges(
			[
				edge('api-proxy', 'api-db', 'SameContainer', { service_id: 'svc-api' }),
				edge('worker-proxy', 'worker-db', 'SameContainer', { service_id: 'svc-worker' })
			],
			elementToContainer
		);

		expect(bundles).toHaveLength(0);
		expect(unbundled).toHaveLength(2);
	});

	it('still bundles interchangeable edges of the same type between the same containers', () => {
		// Edges carrying no relationship identity remain bundleable — the clutter reduction that
		// bundling exists for.
		const { bundles } = bundleEdges(
			[edge('api-proxy', 'api-db', 'SameHost'), edge('worker-proxy', 'worker-db', 'SameHost')],
			elementToContainer
		);

		expect(bundles).toHaveLength(1);
		expect(bundles[0].count).toBe(2);
	});

	it('keeps separate dependencies apart', () => {
		const { bundles, unbundled } = bundleEdges(
			[
				edge('api-proxy', 'api-db', 'RequestPath', { dependency_id: 'dep-1' }),
				edge('worker-proxy', 'worker-db', 'RequestPath', { dependency_id: 'dep-2' })
			],
			elementToContainer
		);

		expect(bundles).toHaveLength(0);
		expect(unbundled).toHaveLength(2);
	});
});
