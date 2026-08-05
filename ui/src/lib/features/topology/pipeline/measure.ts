import type { Node, Edge } from '@xyflow/svelte';
import type { LayoutState, PrepareResult, XY } from './types';
import type { RenderableTopology } from '../types/base';
import * as perf from '../perf';
import { noteFullMeasurePass } from '../diagnostics';
import {
	fillMissingSizesByShapeKey,
	reportShapeVerification,
	shapeVerifyEnabled
} from './shape-verify';

export interface MeasureCallbacks {
	setMeasuring: (v: boolean) => void;
	setNodes: (n: Node[]) => void;
	setEdges: (e: Edge[]) => void;
	/**
	 * Build the node set for a measurement pass.
	 *
	 * `onlyIds` restricts it to those nodes plus whatever ancestors SvelteFlow needs to place
	 * them, so learning one container's collapsed size does not require handing the whole graph
	 * over again. Omitted, it returns everything.
	 */
	buildMeasureNodes: (onlyIds?: Set<string>) => Node[];
	/**
	 * Wait for SvelteFlow to render the given set of node IDs into the DOM.
	 * When `expectedIds` is provided, the callback should poll until every
	 * expected `data-id` is present (capped by its internal timeout); when
	 * omitted it falls back to "any node present."
	 */
	waitForNodesRendered: (expectedIds?: Set<string>) => Promise<void>;
}

/**
 * Mount a set of nodes, wait for them, and read their rendered sizes back out of the DOM.
 *
 * Shared by the full pass and the scoped one so there is a single description of what measuring
 * means. `onlyIds` narrows what gets handed to SvelteFlow — the cost being avoided is the
 * *adoption*, not the mounting: a headless comparison showed mounting 2,890 nodes rather than 320
 * moved peak heap by 8MB, while each full `setNodes` costs 130-230MB downstream of the call as
 * SvelteFlow builds internal state for every node handed to it.
 *
 * @returns Measured sizes, or null if the pipeline went stale mid-measurement.
 */
async function runMeasurePass(
	callbacks: MeasureCallbacks,
	containerElement: HTMLElement | null | undefined,
	isStale: () => boolean,
	onlyIds?: Set<string>
): Promise<Map<string, XY> | null> {
	const sizes = new Map<string, XY>();
	callbacks.setMeasuring(true);
	callbacks.setEdges([]);
	const buildDone = perf.stage(onlyIds ? 'measure.build-nodes.scoped' : 'measure.build-nodes');
	const measureNodes = callbacks.buildMeasureNodes(onlyIds);
	callbacks.setNodes(measureNodes);
	buildDone();

	// Wait for every node in this pass, not just "any node present": waiting on the latter returns
	// stale matches from the previous render and lets newly-added nodes miss measurement, after
	// which ELK falls back to metadata defaults and packs their siblings too close.
	const expectedIds = new Set(measureNodes.map((n) => n.id));
	const renderWaitDone = perf.stage(onlyIds ? 'measure.render-wait.scoped' : 'measure.render-wait');
	await callbacks.waitForNodesRendered(expectedIds);
	renderWaitDone();
	if (isStale()) {
		callbacks.setMeasuring(false);
		return null;
	}

	const readDone = perf.stage(onlyIds ? 'measure.dom-read.scoped' : 'measure.dom-read');
	if (containerElement) {
		for (const el of containerElement.querySelectorAll('.svelte-flow__node')) {
			const htmlEl = el as HTMLElement;
			const id = htmlEl.dataset.id;
			// A scoped pass leaves the rest of the graph mounted, so restrict what is recorded to
			// what this pass asked for — otherwise it would overwrite good cached sizes with
			// whatever happens to be on screen.
			if (!id || (onlyIds && !onlyIds.has(id))) continue;
			sizes.set(id, { x: htmlEl.offsetWidth || 250, y: htmlEl.offsetHeight || 100 });
		}
	}
	readDone();
	return sizes;
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
		const missingContainerIds = new Set<string>();
		for (const node of visibleNodes) {
			if (node.node_type === 'Container') {
				const cached = state.containerSizeCache.get(node.id)?.collapsed;
				if (cached) {
					elementNodeSizes.set(node.id, cached);
				} else if (collapsed.has(node.id)) {
					// A miss clears the whole map below and takes the full measurement pass.
					//
					// That is expensive — it mounts every node in the graph — and an earlier
					// attempt on this branch substituted the container type's declared
					// `collapsed_size` above a node-count ceiling to avoid it. That was wrong:
					// the declared size is a placeholder, ELK laid the parents out around it, and
					// containers came back with no real expanded size at all. They then rendered
					// at `{0, 0}` plus borders — a 2px sliver with its contents spilling outside
					// — and 47 of 72 mounted nodes sat outside their parent. Guessing a size for
					// something ELK will size other things against is not a safe trade; take the
					// measurement.
					missingContainerIds.add(node.id);
				}
				// Expanded containers without cached collapsed size: omit,
				// ELK uses metadata for minimum
			}
		}

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
		const unresolvedElementIds = new Set<string>();
		if (
			fillMissingSizesByShapeKey(visibleNodes, topology, elementNodeSizes, unresolvedElementIds) > 0
		) {
			perf.count('measure.cache-incomplete:collapse');
		}

		// Containers missing a collapsed size are measured on their own, not by re-measuring
		// everything.
		//
		// This was the dominant reason the full pass ran — five passes in one capture, only one of
		// them otherwise attributable. Collapsing a single container reaches it every time, because
		// that container's *collapsed* size has never been needed and so cannot be cached, and the
		// old behaviour discarded every element size to learn it: a full re-measure of 2,890 nodes,
		// ~136MB, for one number.
		//
		// The size still has to be *measured*. Substituting the container type's declared
		// `collapsed_size` was tried on this branch and reverted — see the note above; it is a
		// placeholder ELK sizes parents against, and containers came back with no real expanded size
		// and rendered as 2px slivers. What changes here is the scope of the measurement, not
		// whether one happens.
		// Everything still missing a size gets measured together, once.
		//
		// Both gaps used to end in `elementNodeSizes.clear()`, which forces the full pass below:
		// every node in the graph mounted to learn a handful of sizes. At 19,095 nodes that pass
		// costs ~665MB and 5.5s, and a capture showed three of them where one was warranted — the
		// cold load — with the other two triggered by unresolved element shape keys.
		//
		// Measuring the gap instead of the whole graph is the same trade already made for
		// containers, now applied to both. Merged into one pass rather than two: each pass swaps
		// the node store and waits a frame, so two scoped passes cost twice the churn for no
		// additional information.
		const needsMeasuring = new Set([...unresolvedElementIds, ...missingContainerIds]);
		if (needsMeasuring.size > 0) {
			if (missingContainerIds.size > 0) perf.count('measure.cache-incomplete:container');
			// With nothing worth preserving the full pass below is already the cheapest option.
			if (elementNodeSizes.size > 0) {
				const scoped = await runMeasurePass(callbacks, containerElement, isStale, needsMeasuring);
				if (scoped === null) return null;
				if (scoped.size === 0) {
					// Nothing came back measurable, so fall through rather than lay out against a gap.
					perf.count('measure.scoped-empty');
					elementNodeSizes.clear();
				} else {
					for (const [id, size] of scoped) {
						elementNodeSizes.set(id, size);
						// A container in this set is one that is collapsed and had no cached collapsed
						// entry, so record the measurement as its collapsed size. Without this the
						// same miss recurs on every press and the scoped pass runs forever — the trap
						// the full pass avoided only by measuring everything.
						if (missingContainerIds.has(id)) {
							const entry = state.containerSizeCache.get(id) ?? {};
							entry.collapsed = { ...size };
							state.containerSizeCache.set(id, entry);
						}
					}
				}
			}
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
		const measured = await runMeasurePass(callbacks, containerElement, isStale);
		if (measured === null) return null;
		for (const [id, size] of measured) elementNodeSizes.set(id, size);

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

	// Persist the measurement here, not after the layout.
	//
	// This used to live at the end of `executeLayout`, which meant a run that went stale on the way
	// there threw its measurement away — and since the staleness check now bails *before* ELK, that
	// became the common case rather than a rare one. At 19,095 nodes a full pass costs ~665MB and
	// 5.5s, and one capture showed eight of them for eight runs: five superseded runs each measured
	// the entire graph and discarded the result, so the next run found an empty cache and measured
	// it again.
	//
	// A measurement is valid whether or not the layout that prompted it completes. Nothing below
	// this point can invalidate it.
	const existingViewCache = state.viewSizeCache.get(viewCacheKey);
	if (existingViewCache) {
		for (const [id, size] of elementNodeSizes) {
			existingViewCache.set(id, size);
		}
	} else {
		state.viewSizeCache.set(viewCacheKey, new Map(elementNodeSizes));
	}

	return elementNodeSizes;
}
