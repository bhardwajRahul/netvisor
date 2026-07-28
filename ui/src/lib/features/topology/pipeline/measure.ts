import type { Node, Edge } from '@xyflow/svelte';
import type { LayoutState, PrepareResult, XY } from './types';
import * as perf from '../perf';

export interface MeasureCallbacks {
	setMeasuring: (v: boolean) => void;
	setNodes: (n: Node[]) => void;
	setEdges: (e: Edge[]) => void;
	buildMeasureNodes: () => Node[];
	/**
	 * Wait for SvelteFlow to render the given set of node IDs into the DOM.
	 * When `expectedIds` is provided, the callback should poll until every
	 * expected `data-id` is present (capped by its internal timeout); when
	 * omitted it falls back to "any node present."
	 */
	waitForNodesRendered: (expectedIds?: Set<string>) => Promise<void>;
}

/**
 * Resolve element/container sizes for ELK layout. Uses cached sizes when
 * available, falls back to a full DOM measurement pass.
 *
 * @returns Size map, or null if the pipeline became stale during async measurement.
 */
export async function resolveNodeSizes(
	state: LayoutState,
	prep: PrepareResult,
	getNodes: () => Node[],
	containerElement: HTMLDivElement,
	isStale: () => boolean,
	callbacks: MeasureCallbacks
): Promise<Map<string, XY> | null> {
	const { collapsed, visibleNodes, isViewTransition, needsElkForExpand, isNewStructure } = prep;
	const viewCacheKey = `${prep.currentView}:${prep.topologyId}`;

	const elementNodeSizes = new Map<string, XY>();

	// Try cached sizes first
	const cachedSizes = isViewTransition ? state.viewSizeCache.get(viewCacheKey) : undefined;
	const expandCachedSizes =
		needsElkForExpand && !isNewStructure ? state.viewSizeCache.get(viewCacheKey) : undefined;

	if (isViewTransition && cachedSizes) {
		for (const node of visibleNodes) {
			const cached = cachedSizes.get(node.id);
			elementNodeSizes.set(node.id, cached ?? { x: 250, y: 100 });
		}
	} else if (expandCachedSizes) {
		for (const node of visibleNodes) {
			const cached = expandCachedSizes.get(node.id);
			elementNodeSizes.set(node.id, cached ?? { x: 250, y: 100 });
		}
	} else if (state.containerSizeCache.size > 0) {
		// Use cached container sizes + SvelteFlow computed element sizes
		// Read element sizes from SvelteFlow computed state.
		// Skip containers — handled below via collapsed size cache.
		const liveNodes = getNodes();
		for (const n of liveNodes) {
			if (state.layoutGraph?.containers.has(n.id)) continue;
			// eslint-disable-next-line @typescript-eslint/no-explicit-any -- @xyflow Node has runtime .computed not in type defs
			const w = (n as any).computed?.width ?? n.width;
			// eslint-disable-next-line @typescript-eslint/no-explicit-any -- @xyflow Node has runtime .computed not in type defs
			const h = (n as any).computed?.height ?? n.height;
			if (w && h) {
				elementNodeSizes.set(n.id, { x: w, y: h });
			}
		}

		// Put COLLAPSED size for ALL containers. For collapsed containers,
		// ELK uses it as the fixed size. For expanded containers, ELK uses
		// it as elk.nodeSize.minimum (= smallest the container can be).
		// ELK computes the actual expanded size from children (>= minimum).
		let cacheMisses = 0;
		for (const node of visibleNodes) {
			if (node.node_type === 'Container') {
				const cached = state.containerSizeCache.get(node.id)?.collapsed;
				if (cached) {
					elementNodeSizes.set(node.id, cached);
				} else if (collapsed.has(node.id)) {
					cacheMisses++;
				}
				// Expanded containers without cached collapsed size: omit,
				// ELK uses metadata for minimum
			}
		}

		// Fill ALL missing visible nodes from viewSizeCache — not just
		// liveNodes misses. Elements newly visible from collapse changes
		// aren't in getNodes() yet and weren't counted as misses.
		const viewCache = state.viewSizeCache.get(viewCacheKey);
		if (viewCache) {
			for (const node of visibleNodes) {
				if (!elementNodeSizes.has(node.id)) {
					const cached = viewCache.get(node.id);
					if (cached) {
						elementNodeSizes.set(node.id, cached);
					}
				}
			}
		}

		// If any containers are missing from cache, fall through to full measurement
		if (cacheMisses > 0) {
			elementNodeSizes.clear();
		}
	}

	// Full DOM measurement pass if no cache
	if (elementNodeSizes.size === 0) {
		// The expensive path: every node is mounted into the live canvas and
		// measured. Counted separately from the cached paths so the harness can
		// tell a cold load from a cache miss.
		perf.count('full-measure-pass');
		callbacks.setMeasuring(true);
		callbacks.setEdges([]);
		const measureNodes = callbacks.buildMeasureNodes();
		callbacks.setNodes(measureNodes);
		// Wait for SvelteFlow to render every measure-pass node in the DOM.
		// Waiting only for "any node present" returns stale matches from the
		// previous render and lets newly-added nodes (fresh SSE hosts during
		// discovery) miss measurement — ELK then falls back to metadata
		// defaults and positions the new container's siblings too close.
		const expectedIds = new Set(measureNodes.map((n) => n.id));
		await callbacks.waitForNodesRendered(expectedIds);
		if (isStale()) {
			callbacks.setMeasuring(false);
			return null;
		}

		if (containerElement) {
			const nodeEls = containerElement.querySelectorAll('.svelte-flow__node');
			for (const el of nodeEls) {
				const id = (el as HTMLElement).dataset.id;
				if (id) {
					const htmlEl = el as HTMLElement;
					elementNodeSizes.set(id, {
						x: htmlEl.offsetWidth || 250,
						y: htmlEl.offsetHeight || 100
					});
				}
			}
		}

		// Populate container size cache from this measurement.
		// During deferred collapse, everything was measured EXPANDED
		// regardless of the collapsed store — categorize accordingly.
		for (const [id, size] of elementNodeSizes) {
			if (state.layoutGraph?.containers.has(id)) {
				const entry = state.containerSizeCache.get(id) ?? {};
				const wasExpandedInMeasurement = prep.deferCollapse || !collapsed.has(id);
				if (wasExpandedInMeasurement) {
					entry.expanded = { ...size };
				} else {
					entry.collapsed = { ...size };
				}
				state.containerSizeCache.set(id, entry);
			}
		}
	}

	return elementNodeSizes;
}
