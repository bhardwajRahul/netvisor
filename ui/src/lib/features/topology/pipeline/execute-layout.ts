import type { RenderableTopology, TopologyNode, TopologyEdge } from '../types/base';
import type { LayoutState, PrepareResult, XY } from './types';
import { LayoutGraph } from '../layout/layout-graph';
import { ElkLayoutEngine } from '../layout/engine';
import { computeForceLayout, type ForceNode, type ForceLink } from '../layout/force-layout';
import { containerTypes } from '$lib/shared/stores/metadata';
import * as perf from '../perf';
import { noteRunDetail } from '../diagnostics';

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
	topology: RenderableTopology,
	state: LayoutState,
	prep: PrepareResult,
	elementNodeSizes: Map<string, XY>,
	isStale: () => boolean
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
	// Force-directed "overview mode" is for a handful of collapsed roots, where a
	// cloud reads better than a column. It is also affordable only there: it runs
	// 300 synchronous simulation ticks.
	const FORCE_LAYOUT_MAX_ROOTS = 50;
	const allRootCollapsed =
		rootContainerNodes.length > 0 &&
		rootContainerNodes.length <= FORCE_LAYOUT_MAX_ROOTS &&
		rootContainerNodes.every((n) => collapsed.has(n.id));

	// L2 Physical always lays out with ELK, at every collapse level. Its identity
	// is the structured column of hosts under a switch; swapping to a force
	// simulation when the last container collapses makes the fully-collapsed view
	// a different diagram rather than a denser one, and reads worse. Workloads is
	// excluded for the same reason.
	const useForceLayout =
		allRootCollapsed && currentView !== 'Workloads' && currentView !== 'L2Physical';

	if (useForceLayout) {
		// Force layout for all-collapsed overview mode
		visibleNodes = executeForceLayout(
			state,
			rootContainerNodes,
			elevatedEdges,
			elementNodeSizes,
			layoutNodes,
			collapsed,
			structureKey,
			baseKey,
			prevExpandedSizes
		);
	} else {
		// ELK layout path
		const elkCollapsed = deferCollapse ? new Set<string>() : collapsed;
		const elkNodes = deferCollapse ? layoutNodes : visibleNodes;

		const elkComputeDone = perf.stage('layout.elk-compute');
		const elkResult = await layoutEngine.compute({
			nodes: elkNodes,
			edges: elevatedEdges,
			topology: topology,
			view: currentView,
			parentIndex: prep.parentIndex,
			collapsedContainers: elkCollapsed,
			expandedContainerSizes: prevExpandedSizes,
			elementNodeSizes,
			hiddenEdgeTypes
		});
		elkComputeDone();
		if (isStale()) return null;

		state.sessionStructureKey = structureKey;
		state.sessionBaseKey = baseKey;

		// Rebuild graph and apply ELK result
		const graphBuildDone = perf.stage('layout.graph-build');
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
		graphBuildDone();
		const applyDone = perf.stage('layout.apply-elk');
		state.layoutGraph.applyElkResult(
			elkResult.nodePositions,
			elkResult.containerSizes,
			elkResult.elementNodeSizes
		);
		applyDone();
		noteRunDetail({ elkSizedContainers: elkResult.containerSizes.size });

		// When collapse was deferred, apply it AFTER ELK result
		if (deferCollapse) {
			const visibleDone = perf.stage('layout.visible-nodes');
			state.layoutGraph.syncCollapseState(collapsed);
			visibleNodes = state.layoutGraph.getVisibleNodes(layoutNodes);
			visibleDone();
		}

		// Cache container sizes from ELK result
		const cacheDone = perf.stage('layout.cache-sizes');
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
		cacheDone();
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
	needsElk: boolean,
	viewCacheKey: string
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
			const viewCache = state.viewSizeCache.get(viewCacheKey);
			for (const nodeId of changedIds) {
				const el = containerElement.querySelector(`[data-id="${nodeId}"]`) as HTMLElement;
				if (el) {
					const size = { x: el.offsetWidth || 250, y: el.offsetHeight || 100 };
					state.layoutGraph.updateElementSize(nodeId, size);
					viewCache?.set(nodeId, size);
				} else {
					// Not mounted, so not measurable — it is off screen and culled. Drop its cached
					// size rather than leaving the pre-toggle height in place: without a cached size
					// the node is built without a size seed, which makes SvelteFlow render it once
					// and measure it for real the next time it comes into view. Keeping the stale
					// height would leave a card-sized gap in the layout that nothing corrects.
					viewCache?.delete(nodeId);
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
	baseKey: string,
	prevExpandedSizes: Map<string, { width: number; height: number }> | undefined
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
	// Same restore as the ELK path above, and for the same reason: rebuilding recreates every
	// container with `expandedSize` at {0, 0}, and the force layout only sizes the root containers
	// it was given. Without this, anything it does not touch is left at zero, `getContainerSize`
	// returns that rather than undefined, and the container renders as its borders with its
	// contents outside — persistently, until something forces a fresh ELK run.
	if (prevExpandedSizes) {
		state.layoutGraph.restoreExpandedSizes(prevExpandedSizes);
	}
	state.layoutGraph.applyForceResult(forceResult.nodePositions, elementNodeSizes);

	// Recompute visible nodes after force layout rebuilds the graph
	return state.layoutGraph.getVisibleNodes(layoutNodes);
}
