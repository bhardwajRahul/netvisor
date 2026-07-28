import type { Node, Edge } from '@xyflow/svelte';
import type { LayoutState, PrepareResult, XY } from './types';
import type { RenderableTopology } from '../types/base';
import * as perf from '../perf';
import { reportShapeVerification, shapeVerifyEnabled } from './shape-verify';

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
	topology: RenderableTopology,
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

	// A cache hit is only usable if it actually covers the visible nodes. The
	// `{ x: 250, y: 100 }` fallback below is a placeholder, not a measurement:
	// handing it to ELK for a card whose real height differs lays the graph out
	// wrongly, which surfaces as overlapping nodes. This bites when containers
	// start collapsed at scale, because their element cards have never been
	// mounted and so were never cached — expanding one found nothing.
	//
	// So: fill from cache, and if anything was missing, discard the lot and take
	// the full measurement path instead of laying out against placeholders.
	const fillFromCache = (cache: Map<string, XY>): boolean => {
		let complete = true;
		for (const node of visibleNodes) {
			const cached = cache.get(node.id);
			if (cached) {
				elementNodeSizes.set(node.id, cached);
			} else {
				complete = false;
				break;
			}
		}
		if (!complete) elementNodeSizes.clear();
		return complete;
	};

	if (isViewTransition && cachedSizes) {
		if (!fillFromCache(cachedSizes)) perf.count('measure.cache-incomplete:view');
	} else if (expandCachedSizes) {
		if (!fillFromCache(expandCachedSizes)) perf.count('measure.cache-incomplete:expand');
	} else if (state.containerSizeCache.size > 0) {
		// Use cached container sizes + SvelteFlow computed element sizes
		// Read element sizes from SvelteFlow computed state.
		// Skip containers — handled below via collapsed size cache.
		const liveNodes = getNodes();
		for (const n of liveNodes) {
			if (state.layoutGraph?.containers.has(n.id)) continue;
			// `measured` is where @xyflow/svelte v1 puts the rendered size.
			// This previously read `computed`, which was the v0 name and does not
			// exist in v1 — so `w`/`h` always fell back to the literal node props
			// (a hardcoded 250 for elements, undefined for heights), the guard
			// below was false for essentially every node, and this whole
			// cached-size fast path silently did nothing.
			const w = n.measured?.width ?? n.width;
			const h = n.measured?.height ?? n.height;
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
		const buildDone = perf.stage('measure.build-nodes');
		const measureNodes = callbacks.buildMeasureNodes();
		callbacks.setNodes(measureNodes);
		buildDone();
		// Wait for SvelteFlow to render every measure-pass node in the DOM.
		// Waiting only for "any node present" returns stale matches from the
		// previous render and lets newly-added nodes (fresh SSE hosts during
		// discovery) miss measurement — ELK then falls back to metadata
		// defaults and positions the new container's siblings too close.
		const expectedIds = new Set(measureNodes.map((n) => n.id));
		const renderWaitDone = perf.stage('measure.render-wait');
		await callbacks.waitForNodesRendered(expectedIds);
		renderWaitDone();
		if (isStale()) {
			callbacks.setMeasuring(false);
			return null;
		}

		const readDone = perf.stage('measure.dom-read');
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

		readDone();

		// Validate the shape key against this full measurement — every element
		// sharing a key must have measured to the same height. Runs here, on the
		// unsampled path, so it checks the key rather than the sampling.
		if (shapeVerifyEnabled()) {
			reportShapeVerification(visibleNodes, topology, elementNodeSizes);
		}

		// Populate container size cache from this measurement.
		// During deferred collapse, everything was measured EXPANDED
		// regardless of the collapsed store — categorize accordingly.
		//
		// Containers are identified from the nodes being laid out, not from
		// `state.layoutGraph`: the graph is built later, in executeLayout, so on a
		// cold load it is still null here. Gating on it meant nothing was cached on
		// the very pass that measures everything — and the post-render self-heal
		// then saw every collapsed container as new and triggered a full corrective
		// re-layout (two more elk.layout() calls) on every first render.
		const containerIds = new Set(
			visibleNodes.filter((n) => n.node_type === 'Container').map((n) => n.id)
		);
		for (const [id, size] of elementNodeSizes) {
			if (containerIds.has(id)) {
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
