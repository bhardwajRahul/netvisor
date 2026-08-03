import type { Node, Edge } from '@xyflow/svelte';
import type { LayoutState, PrepareResult, XY } from './types';
import type { RenderableTopology } from '../types/base';
import * as perf from '../perf';
import { noteFullMeasurePass } from '../diagnostics';
import { containerTypes } from '$lib/shared/stores/metadata';
import {
	fillMissingSizesByShapeKey,
	reportShapeVerification,
	shapeVerifyEnabled
} from './shape-verify';

/**
 * Visible-node count above which the full DOM measurement pass is avoided where possible.
 *
 * The pass mounts every node in the graph at once. That is affordable at a few hundred nodes and
 * is what makes the layout exact; at several thousand it is the dominant memory cost, and at the
 * scale that produced this ceiling — 17,236 nodes — it exhausts the browser outright.
 *
 * Set above the culling threshold so the two do not interact at the boundary: a graph small
 * enough to render uncalled is also small enough to measure in full.
 */
const FULL_MEASURE_NODE_CEILING = 2000;

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
	containerElement: HTMLDivElement,
	isStale: () => boolean,
	callbacks: MeasureCallbacks
): Promise<Map<string, XY> | null> {
	const { collapsed, visibleNodes, isViewTransition, needsElkForExpand, isNewStructure } = prep;
	const viewCacheKey = `${prep.currentView}:${prep.topologyId}`;

	const elementNodeSizes = new Map<string, XY>();

	// Above this many visible nodes, an uncached collapsed container is estimated from its type
	// metadata instead of forcing the full measurement pass. Below it, behaviour is exactly as
	// before: the graph is small enough that mounting all of it to measure is cheap and exact.
	const estimateCollapsedSizes = visibleNodes.length >= FULL_MEASURE_NODE_CEILING;
	let estimatedCollapsed = 0;

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
		// Use cached container sizes + previously measured element sizes.
		// Skip containers — handled below via collapsed size cache.
		//
		// Element sizes come from `viewSizeCache`, which the pipeline populates from its own DOM
		// measurement (`execute-layout.ts`), not from SvelteFlow's node array.
		//
		// This previously read `n.measured` off `getNodes()`. Two successive bugs kept that dead:
		// first the field was named `computed` (the v0 name, absent in v1); then, once renamed,
		// it was still read from the wrong object — SvelteFlow writes `measured` into its internal
		// `nodeLookup` and never back onto the user nodes `getNodes()` returns. So `w`/`h` always
		// fell back to the literal props (a hardcoded 250 for elements, undefined for heights),
		// the guard below was false for essentially every node, and this entire fast path had
		// never once produced a size — every expand fell through to the full measurement pass,
		// which mounts every node in the graph. Do not reintroduce either read.
		const viewCache = state.viewSizeCache.get(viewCacheKey);
		if (viewCache) {
			for (const [id, size] of viewCache) {
				if (state.layoutGraph?.containers.has(id)) continue;
				if (size.x && size.y) elementNodeSizes.set(id, { ...size });
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
					if (estimateCollapsedSizes) {
						// Fall back to the container type's declared collapsed size rather than
						// counting a miss. A miss clears the whole map below and takes the full
						// measurement pass, which mounts every node in the graph — at this scale
						// that pass *is* the out-of-memory failure, so paying it to refine a
						// collapsed box that is off screen anyway is the wrong trade.
						//
						// This became reachable when culling started working: `cacheCollapsedSizes`
						// reads the DOM, so it only ever sees collapsed containers that are
						// mounted, and every off-screen one is now a permanent miss. The estimate
						// is corrected as the user pans — `cacheCollapsedSizes` runs on move-end
						// and records the real size once the container comes into view.
						const meta = containerTypes.getMetadata(
							((node as Record<string, unknown>).container_type as string) ?? 'Subnet'
						);
						elementNodeSizes.set(node.id, {
							x: meta.collapsed_size.width,
							y: meta.collapsed_size.height
						});
						estimatedCollapsed++;
					} else {
						cacheMisses++;
					}
				}
				// Expanded containers without cached collapsed size: omit,
				// ELK uses metadata for minimum
			}
		}
		if (estimatedCollapsed > 0) perf.count('measure.collapsed-estimate');

		// Fill any visible node still missing a size — chiefly containers, which the element pass
		// above skips and which the collapsed-size pass only covers when `containerSizeCache`
		// holds an entry.
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

		// Elements must have a size too, not just collapsed containers.
		//
		// This counted containers only, so an element card missing from both the measured sizes
		// and the view cache left the map *partial* — which is worse than empty, because the
		// `size === 0` guard below only takes the full-measurement path when the map is entirely
		// empty. ELK then fell back to `node.size`, which the server leaves at `Uxy::default()`
		// — literally 0x0 — so it packed zero-sized children a spacing apart while the DOM
		// rendered them at their real size, overlapping by ~80%.
		//
		// Reachable exactly when containers start collapsed at scale: their cards are never
		// mounted, so never measured, so absent from the cache when one is expanded.
		//
		// Filled from a measured card of the same shape key rather than by re-measuring
		// everything. Discarding the map would be correct but costs a full pass — mounting every
		// card in the graph to learn sizes most of them already have — which at a few hundred
		// hosts is the cold-load cost the sampling exists to avoid. Only a key with no measured
		// representative at all forces the full path.
		const unresolved = fillMissingSizesByShapeKey(visibleNodes, topology, elementNodeSizes);
		if (unresolved > 0) {
			perf.count('measure.cache-incomplete:collapse');
			elementNodeSizes.clear();
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
		// Also counted in the always-on diagnostic: `perf` records nothing in a customer's build,
		// and how often this path runs is the difference between a graph mounted once and a graph
		// mounted repeatedly.
		noteFullMeasurePass();
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
