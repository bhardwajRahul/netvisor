import { describe, it, expect } from 'vitest';
import { computeCollapsedForLevel } from '$lib/features/topology/collapse';
import { LayoutGraph } from '$lib/features/topology/layout/layout-graph';
import type { components } from '$lib/api/schema';

type TopologyNode = components['schemas']['Node'];
type ContainerTypeMetadata = import('$lib/shared/stores/metadata').ContainerTypeMetadata;

const container = (
	id: string,
	container_type: string,
	parent_container_id?: string
): TopologyNode =>
	({
		id,
		node_type: 'Container',
		container_type,
		...(parent_container_id ? { parent_container_id } : {})
	}) as unknown as TopologyNode;

const element = (id: string, container_id: string): TopologyNode =>
	({
		id,
		node_type: 'Element',
		container_id,
		host_id: 'h',
		element_type: 'Service'
	}) as unknown as TopologyNode;

// Mock the container-type metadata the collapse logic reads.
const META: Record<string, Partial<ContainerTypeMetadata>> = {
	Subnet: { is_subcontainer: false, collapsed_by_default: false },
	NestedTag: { is_subcontainer: true, collapsed_by_default: false },
	ApplicationUngrouped: { is_subcontainer: false, collapsed_by_default: true }
};
const containerTypes = {
	getMetadata: (ct: string | null) => (META[ct ?? ''] ?? {}) as ContainerTypeMetadata
};

describe('computeCollapsedForLevel — collapsed_by_default root', () => {
	const nodes = [
		container('root', 'Subnet'),
		container('sub', 'NestedTag', 'root'),
		container('ung', 'ApplicationUngrouped')
	];

	it('collapses a collapsed_by_default root at every level except 4', () => {
		expect(computeCollapsedForLevel(1, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(2, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(3, nodes, containerTypes, null).has('ung')).toBe(true);
		expect(computeCollapsedForLevel(4, nodes, containerTypes, null).has('ung')).toBe(false);
	});

	it('still expands a plain root at level 2 and collapses subcontainers', () => {
		const c2 = computeCollapsedForLevel(2, nodes, containerTypes, null);
		expect(c2.has('root')).toBe(false);
		expect(c2.has('sub')).toBe(true);
	});
});

describe('getVisibleNodes — transitive ancestor collapse', () => {
	// root → subcontainer → element. Only the root is in the collapsed set
	// (the level-3 / auto-collapse case where the subcontainer is left expanded).
	const nodes = [
		container('ung', 'ApplicationUngrouped'),
		container('sub', 'NestedTag', 'ung'),
		element('svc', 'sub')
	];

	it('hides a grandchild when only its root ancestor is collapsed', () => {
		const graph = LayoutGraph.fromTopology(nodes);
		graph.containers.get('ung')!.collapsed = true; // root only; sub stays expanded
		const visible = graph.getVisibleNodes(nodes).map((n) => n.id);
		expect(visible).toContain('ung'); // collapsed root itself still renders
		expect(visible).not.toContain('sub'); // direct child hidden
		expect(visible).not.toContain('svc'); // grandchild hidden (the bug)
	});

	it('shows everything when nothing is collapsed', () => {
		const graph = LayoutGraph.fromTopology(nodes);
		const visible = graph.getVisibleNodes(nodes).map((n) => n.id);
		expect(visible).toEqual(expect.arrayContaining(['ung', 'sub', 'svc']));
	});
});
