import type { Node } from '@xyflow/svelte';
import type { LayoutGraph } from '../layout/layout-graph';
import type { RenderableTopology, TopologyNode } from '../types/base';
import { getTopologyIndex } from '../entity-index';

export interface BuildFlowNodesParams {
	visibleNodes: TopologyNode[];
	collapsed: Set<string>;
	layoutGraph: LayoutGraph | null;
	topology: RenderableTopology;
	isNewStructure: boolean;
	useGraph: boolean;
	liveNodes: Node[];
	infraRuleId: string | null;
	editMode: boolean;
}

/** Count elements recursively within a container from raw topology nodes. */
function countChildElements(containerId: string, nodes: TopologyNode[]): number {
	let count = 0;
	for (const n of nodes) {
		if (n.node_type === 'Element' && (n as Record<string, unknown>).container_id === containerId) {
			count++;
		}
		if (
			n.node_type === 'Container' &&
			(n as Record<string, unknown>).parent_container_id === containerId
		) {
			count += countChildElements(n.id, nodes);
		}
	}
	return count;
}

/** Build subgroup summaries from raw topology nodes (fallback when layoutGraph is unavailable). */
function fallbackSubgroupSummaries(
	containerId: string,
	nodes: TopologyNode[]
): { groupId: string; childCount: number }[] {
	const summaries: { groupId: string; childCount: number }[] = [];
	for (const n of nodes) {
		if (
			n.node_type === 'Container' &&
			(n as Record<string, unknown>).parent_container_id === containerId
		) {
			summaries.push({
				groupId: n.id,
				childCount: countChildElements(n.id, nodes)
			});
		}
	}
	return summaries;
}

export function buildFlowNodes(params: BuildFlowNodesParams): Node[] {
	const {
		visibleNodes,
		collapsed,
		layoutGraph,
		topology,
		isNewStructure,
		useGraph,
		liveNodes,
		infraRuleId,
		editMode
	} = params;

	const currentPositions = new Map(liveNodes.map((n) => [n.id, n.position]));
	const currentSizes = new Map(
		// eslint-disable-next-line @typescript-eslint/no-explicit-any -- @xyflow Node has runtime .computed not in type defs
		(liveNodes as Record<string, any>[]).map((n) => [
			n.id,
			{
				width: n.computed?.width ?? n.width,
				height: n.computed?.height ?? n.height
			}
		])
	);

	return visibleNodes.map((node) => {
		const isNodeCollapsed = collapsed.has(node.id);
		let position: { x: number; y: number };
		let width: number | undefined;
		let height: number | undefined;

		const isElement = node.node_type === 'Element';

		// Container size from layout graph (collapsed = metadata size, expanded = ELK size)
		const containerSize =
			!isElement && layoutGraph ? layoutGraph.getContainerSize(node.id) : undefined;

		if (useGraph && layoutGraph) {
			const graphPos = layoutGraph.getPosition(node.id);
			position = graphPos ?? { x: node.position.x, y: node.position.y };
			width = isNodeCollapsed
				? (containerSize?.width ?? undefined)
				: isElement
					? 250
					: (containerSize?.width ?? undefined);
			height = isNodeCollapsed
				? (containerSize?.height ?? undefined)
				: isElement
					? undefined
					: (containerSize?.height ?? undefined);
		} else if (!isNewStructure) {
			const curPos = currentPositions.get(node.id);
			const curSize = currentSizes.get(node.id);
			position = curPos ?? { x: node.position.x, y: node.position.y };
			width = isNodeCollapsed
				? (containerSize?.width ?? undefined)
				: isElement
					? 250
					: (curSize?.width ?? undefined);
			height = isNodeCollapsed
				? (containerSize?.height ?? undefined)
				: isElement
					? undefined
					: (curSize?.height ?? undefined);
		} else {
			// Measurement pass: place at origin, let content determine size
			position = { x: 0, y: 0 };
			width = isElement ? 250 : undefined;
			height = undefined;
		}

		return {
			id: node.id,
			type: node.node_type,
			position,
			...(width !== undefined && { width }),
			...(height !== undefined && { height }),
			expandParent: true,
			deletable: false,
			selectable: node.node_type !== 'Container',
			// `container_id` is a non-optional Uuid on the backend's NodeType::Element,
			// so it is always present. This used to fall back to the resolved
			// `subnetId`, which is L3-specific (only the IPAddress resolver ever
			// returns one) — a view-specific lookup in a shared builder, and an O(n)
			// resolver call per node, that could never actually fire.
			parentId:
				node.node_type == 'Element'
					? node.container_id
					: node.node_type == 'Container' && node.parent_container_id
						? (node.parent_container_id as string)
						: undefined,
			extent:
				editMode && (node.node_type == 'Element' || node.parent_container_id)
					? 'parent'
					: undefined,
			data: isNodeCollapsed
				? (() => {
						const totalCount =
							layoutGraph?.getChildCount(node.id) ?? countChildElements(node.id, topology.nodes);
						const summaries =
							layoutGraph?.getSubgroupSummaries(node.id) ??
							fallbackSubgroupSummaries(node.id, topology.nodes);
						// Exclude infrastructure services subgroup from workload count
						let excludedCount = 0;
						if (infraRuleId) {
							const { nodesById } = getTopologyIndex(topology);
							for (const s of summaries) {
								const groupNode = nodesById.get(s.groupId);
								if (
									groupNode &&
									(groupNode as Record<string, unknown>).element_rule_id === infraRuleId
								) {
									excludedCount += s.childCount;
								}
							}
						}
						return {
							...node,
							isCollapsed: true,
							childCount: totalCount - excludedCount,
							subgroupSummaries: summaries
						};
					})()
				: node
		};
	});
}

/** Sort parents before children (SvelteFlow requirement). */
export function sortFlowNodes(flowNodes: Node[]): Node[] {
	const depthOf = (n: Node) => {
		if (!n.parentId) return 0;
		if (n.type === 'Container') return 1;
		return 2;
	};
	return flowNodes.sort((a, b) => depthOf(a) - depthOf(b));
}
