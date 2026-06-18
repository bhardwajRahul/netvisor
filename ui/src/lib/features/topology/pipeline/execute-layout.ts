import { get } from 'svelte/store';
import type { EnrichedTopology, TopologyNode, TopologyEdge } from '../types/base';
import type { LayoutState, PrepareResult, XY } from './types';
import { LayoutGraph } from '../layout/layout-graph';
import { ElkLayoutEngine } from '../layout/engine';
import { computeForceLayout, type ForceNode, type ForceLink } from '../layout/force-layout';
import { collapsedContainers, collapseLevel, inferCurrentLevel } from '../collapse';
import { containerTypes } from '$lib/shared/stores/metadata';

const layoutEngine = new ElkLayoutEngine();

/**
 * Execute layout computation (ELK or force-directed) and apply auto-collapse.
 *
 * Mutates state.layoutGraph, state.sessionStructureKey, state.sessionBaseKey,
 * state.seenAutoCollapseIds, state.viewSizeCache, state.containerSizeCache.
 *
 * @returns Updated visible nodes, or null if stale.
 */
export async function executeLayout(
	topology: EnrichedTopology,
	state: LayoutState,
	prep: PrepareResult,
	elementNodeSizes: Map<string, XY>,
	isStale: () => boolean,
	getInfrastructureRuleId: () => string | null
): Promise<{ visibleNodes: TopologyNode[] } | null> {
	const {
		layoutNodes,
		collapsed,
		elevatedEdges,
		deferCollapse,
		prevExpandedSizes,
		prevChildPositions,
		structureKey,
		baseKey,
		currentView,
		hiddenEdgeTypes
	} = prep;
	let { visibleNodes } = prep;

	// Detect if all root containers are collapsed -> use force layout
	const rootContainerNodes = visibleNodes.filter(
		(n) => n.node_type === 'Container' && !n.parent_container_id
	);
	const allRootCollapsed =
		rootContainerNodes.length > 0 && rootContainerNodes.every((n) => collapsed.has(n.id));

	if (allRootCollapsed && currentView !== 'Workloads') {
		// Force layout for all-collapsed overview mode
		visibleNodes = executeForceLayout(
			state,
			rootContainerNodes,
			elevatedEdges,
			elementNodeSizes,
			layoutNodes,
			collapsed,
			structureKey,
			baseKey
		);
	} else {
		// ELK layout path
		const elkCollapsed = deferCollapse ? new Set<string>() : collapsed;
		const elkNodes = deferCollapse ? layoutNodes : visibleNodes;

		const elkResult = await layoutEngine.compute({
			nodes: elkNodes,
			edges: elevatedEdges,
			topology: topology,
			parentIndex: prep.parentIndex,
			collapsedContainers: elkCollapsed,
			expandedContainerSizes: prevExpandedSizes,
			elementNodeSizes,
			hiddenEdgeTypes
		});
		if (isStale()) return null;

		state.sessionStructureKey = structureKey;
		state.sessionBaseKey = baseKey;

		// Rebuild graph and apply ELK result
		state.layoutGraph = LayoutGraph.fromTopology(layoutNodes);
		if (!deferCollapse) {
			state.layoutGraph.syncCollapseState(collapsed);
			if (prevExpandedSizes) {
				state.layoutGraph.restoreExpandedSizes(prevExpandedSizes);
			}
			if (prevChildPositions) {
				state.layoutGraph.restoreContainerChildPositions(prevChildPositions);
			}
		}
		state.layoutGraph.applyElkResult(
			elkResult.nodePositions,
			elkResult.containerSizes,
			elkResult.elementNodeSizes
		);

		// When collapse was deferred, apply it AFTER ELK result
		if (deferCollapse) {
			state.layoutGraph.syncCollapseState(collapsed);
			visibleNodes = state.layoutGraph.getVisibleNodes(layoutNodes);
		}

		// Cache container sizes from ELK result
		for (const [id, size] of elkResult.containerSizes) {
			if (state.layoutGraph?.containers.has(id)) {
				const entry = state.containerSizeCache.get(id) ?? {};
				if (elkCollapsed.has(id)) {
					entry.collapsed = { x: size.width, y: size.height };
				} else {
					entry.expanded = { x: size.width, y: size.height };
				}
				state.containerSizeCache.set(id, entry);
			}
		}

		// Log size mismatches between DOM-measured and ELK-computed
		{
			const mismatches: string[] = [];
			for (const [id, elkSize] of elkResult.containerSizes) {
				const measured = elementNodeSizes.get(id);
				if (measured) {
					const dw = Math.abs(measured.x - elkSize.width);
					const dh = Math.abs(measured.y - elkSize.height);
					if (dw > 10 || dh > 10) {
						mismatches.push(
							`${id.substring(0, 8)}: DOM=${measured.x}x${measured.y} ELK=${elkSize.width}x${elkSize.height}`
						);
					}
				}
			}
		}
	}

	// Cache measured sizes for this view
	const viewCacheKey = `${currentView}:${topology.id}`;
	const existingViewCache = state.viewSizeCache.get(viewCacheKey);
	if (existingViewCache) {
		for (const [id, size] of elementNodeSizes) {
			existingViewCache.set(id, size);
		}
	} else {
		state.viewSizeCache.set(viewCacheKey, new Map(elementNodeSizes));
	}

	// Auto-collapse containers whose type has collapsed_by_default metadata
	applyAutoCollapse(topology, state, collapsed, getInfrastructureRuleId);

	return { visibleNodes };
}

/**
 * Handle port expansion: re-measure affected nodes without full ELK re-layout.
 * @returns true if ports changed and layout was updated.
 */
export async function handlePortExpansion(
	state: LayoutState,
	currentExpandedPorts: Set<string>,
	containerElement: HTMLDivElement,
	buildMeasureNodes: () => import('@xyflow/svelte').Node[],
	setNodes: (nodes: import('@xyflow/svelte').Node[]) => void,
	isStale: () => boolean,
	needsElk: boolean
): Promise<boolean> {
	const portsChanged =
		currentExpandedPorts.size !== state.prevExpandedPortIds.size ||
		[...currentExpandedPorts].some((id) => !state.prevExpandedPortIds.has(id)) ||
		[...state.prevExpandedPortIds].some((id) => !currentExpandedPorts.has(id));

	if (portsChanged && !needsElk && state.layoutGraph) {
		// Render with current positions to let DOM update port content
		setNodes(buildMeasureNodes());
		const { tick } = await import('svelte');
		await tick();
		await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
		if (isStale()) return false;

		// Re-measure affected nodes and update graph
		if (containerElement) {
			const changedIds = new Set([...currentExpandedPorts, ...state.prevExpandedPortIds]);
			for (const nodeId of changedIds) {
				const el = containerElement.querySelector(`[data-id="${nodeId}"]`) as HTMLElement;
				if (el) {
					state.layoutGraph.updateElementSize(nodeId, {
						x: el.offsetWidth || 250,
						y: el.offsetHeight || 100
					});
				}
			}
		}
		state.prevExpandedPortIds = new Set(currentExpandedPorts);
		return true;
	} else if (needsElk) {
		state.prevExpandedPortIds = new Set(currentExpandedPorts);
	}

	return false;
}

function executeForceLayout(
	state: LayoutState,
	rootContainerNodes: TopologyNode[],
	elevatedEdges: TopologyEdge[],
	elementNodeSizes: Map<string, XY>,
	layoutNodes: TopologyNode[],
	collapsed: Set<string>,
	structureKey: string,
	baseKey: string
): TopologyNode[] {
	const forceNodes: ForceNode[] = rootContainerNodes.map((n) => {
		const measured = elementNodeSizes.get(n.id);
		const meta = containerTypes.getMetadata(
			((n as Record<string, unknown>).container_type as string) ?? 'Subnet'
		);
		return {
			id: n.id,
			width: measured?.x ?? meta.collapsed_size.width,
			height: measured?.y ?? meta.collapsed_size.height
		};
	});

	// Build deduplicated links from elevated edges between root containers
	const rootIds = new Set(rootContainerNodes.map((n) => n.id));
	const forceLinks: ForceLink[] = [];
	const seenLinks = new Set<string>();
	for (const edge of elevatedEdges) {
		const src = edge.source as string;
		const tgt = edge.target as string;
		if (rootIds.has(src) && rootIds.has(tgt) && src !== tgt) {
			const key = `${src}->${tgt}`;
			if (!seenLinks.has(key)) {
				seenLinks.add(key);
				forceLinks.push({ source: src, target: tgt });
			}
		}
	}

	const forceResult = computeForceLayout(forceNodes, forceLinks);

	state.sessionStructureKey = structureKey;
	state.sessionBaseKey = baseKey;
	state.layoutGraph = LayoutGraph.fromTopology(layoutNodes);
	state.layoutGraph.syncCollapseState(collapsed);
	state.layoutGraph.applyForceResult(forceResult.nodePositions, elementNodeSizes);

	// Recompute visible nodes after force layout rebuilds the graph
	return state.layoutGraph.getVisibleNodes(layoutNodes);
}

function applyAutoCollapse(
	topology: EnrichedTopology,
	state: LayoutState,
	collapsed: Set<string>,
	getInfrastructureRuleId: () => string | null
) {
	const currentLevel = get(collapseLevel);
	const infraRuleId = getInfrastructureRuleId();

	const allCandidates = topology.nodes.filter((n) => {
		if (n.node_type !== 'Container') return false;
		const data = n as Record<string, unknown>;
		const ct = data.container_type as string | undefined;
		return (
			(ct && containerTypes.getMetadata(ct).collapsed_by_default === true) ||
			(infraRuleId && data.element_rule_id === infraRuleId)
		);
	});

	const userExplicitlyExpandedAll = currentLevel === 4 && state.collapseLevelInferred;
	const autoCollapseIds = userExplicitlyExpandedAll
		? []
		: allCandidates
				.filter((n) => !collapsed.has(n.id) && !state.seenAutoCollapseIds.has(n.id))
				.map((n) => n.id);

	if (autoCollapseIds.length > 0) {
		for (const id of autoCollapseIds) state.seenAutoCollapseIds.add(id);
		const next = new Set(collapsed);
		for (const id of autoCollapseIds) next.add(id);
		collapsedContainers.set(next);
	}

	// Re-infer level after auto-collapse
	if (!state.collapseLevelInferred) {
		state.collapseLevelInferred = true;
		const newCollapsed = get(collapsedContainers);
		const inferred = inferCurrentLevel(
			newCollapsed,
			topology.nodes,
			containerTypes,
			getInfrastructureRuleId()
		);
		collapseLevel.set(inferred);
	}
}
