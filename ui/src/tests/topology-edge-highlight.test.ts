import { describe, it, expect } from 'vitest';
import { nodesConnectedByEdge } from '$lib/features/topology/interactions';
import type { TopologyEdge } from '$lib/features/topology/types/base';

/**
 * Clicking an edge highlights what the edge's own `selection_scope` says it should — the whole
 * relation it is a segment of, or just its endpoints. These exercise that dispatch through the
 * real edge-type metadata, so an edge type whose scope changes is covered here too.
 */

const HOST_A = '11111111-1111-1111-1111-111111111111';
const HOST_B = '22222222-2222-2222-2222-222222222222';

function sameHost(source: string, target: string, hostId: string): TopologyEdge {
	return {
		id: `${source}-${target}`,
		source,
		target,
		edge_type: 'SameHost',
		host_id: hostId,
		label: null,
		source_handle: 'Bottom',
		target_handle: 'Top',
		is_multi_hop: false
	} as unknown as TopologyEdge;
}

function physicalLink(source: string, target: string): TopologyEdge {
	return {
		id: `${source}-${target}`,
		source,
		target,
		edge_type: 'PhysicalLink',
		source_entity_id: source,
		target_entity_id: target,
		protocol: 'LLDP',
		label: null,
		source_handle: 'Bottom',
		target_handle: 'Top',
		is_multi_hop: false
	} as unknown as TopologyEdge;
}

describe('nodesConnectedByEdge', () => {
	it('lights up every segment of the clicked edge’s relation', () => {
		const first = sameHost('ip-1', 'ip-2', HOST_A);
		const edges = [first, sameHost('ip-1', 'ip-3', HOST_A), sameHost('ip-9', 'ip-8', HOST_B)];

		expect(nodesConnectedByEdge(first, edges)).toEqual(new Set(['ip-1', 'ip-2', 'ip-3']));
	});

	it('keeps separate relations of the same edge type apart', () => {
		const other = sameHost('ip-9', 'ip-8', HOST_B);
		const edges = [sameHost('ip-1', 'ip-2', HOST_A), other];

		expect(nodesConnectedByEdge(other, edges)).toEqual(new Set(['ip-9', 'ip-8']));
	});

	it('lights up only its own endpoints for a segment-scoped edge', () => {
		const link = physicalLink('if-1', 'if-2');
		const edges = [link, physicalLink('if-3', 'if-4')];

		expect(nodesConnectedByEdge(link, edges)).toEqual(new Set(['if-1', 'if-2']));
	});

	it('falls back to its own endpoints when the relation id is missing', () => {
		const orphan = { ...sameHost('ip-1', 'ip-2', HOST_A) } as Record<string, unknown>;
		delete orphan.host_id;
		const edges = [orphan as unknown as TopologyEdge, sameHost('ip-5', 'ip-6', HOST_A)];

		expect(nodesConnectedByEdge(orphan as unknown as TopologyEdge, edges)).toEqual(
			new Set(['ip-1', 'ip-2'])
		);
	});
});
