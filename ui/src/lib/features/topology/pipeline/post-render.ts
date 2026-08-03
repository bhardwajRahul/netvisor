import type { LayoutGraph } from '../layout/layout-graph';
import type { XY } from './types';

/** Height difference, in px, below which a card is considered unchanged. */
const SIZE_DRIFT_TOLERANCE_PX = 2;

/**
 * Correct cached sizes against what the mounted cards actually measure.
 *
 * Nodes are built carrying `measured` and `handles` so SvelteFlow can cull them before they have
 * ever rendered. The cost of that is `NodeWrapper` treating them as initialised and never
 * attaching a ResizeObserver — so nothing else notices if a card's real height drifts from the
 * cached value. A pipeline run re-measures after any topology change, and port expansion
 * re-measures explicitly, but a change with no pipeline run behind it (a font finally loading, a
 * theme switch, a container resize) would otherwise leave a wrong height cached indefinitely,
 * laying the graph out around a card size that no longer exists.
 *
 * Bounded by the mounted set — a few hundred nodes with culling on, not the whole graph — and it
 * converges: the cache is updated to what was measured, so the next build seeds the correct value
 * and the following pass finds no drift.
 *
 * @returns How many cached sizes were corrected.
 */
export function reconcileMeasuredSizes(
	containerElement: HTMLDivElement,
	viewSizeCache: Map<string, XY>
): number {
	let corrected = 0;

	for (const el of containerElement.querySelectorAll('.svelte-flow__node')) {
		const htmlEl = el as HTMLElement;
		const id = htmlEl.dataset.id;
		if (!id) continue;

		const cached = viewSizeCache.get(id);
		if (!cached) continue;

		const height = htmlEl.offsetHeight;
		// A zero height is a node mid-mount, not a card that shrank to nothing.
		if (!height) continue;

		if (Math.abs(height - cached.y) > SIZE_DRIFT_TOLERANCE_PX) {
			viewSizeCache.set(id, { x: cached.x, y: height });
			corrected += 1;
		}
	}

	return corrected;
}

/**
 * Cache collapsed container sizes after render.
 * Unconstrain width to read natural content size, then restore.
 * Synchronous — no paint between write-read-restore.
 *
 * @returns Number of new collapsed cache entries added.
 */
export function cacheCollapsedSizes(
	containerElement: HTMLDivElement,
	layoutGraph: LayoutGraph,
	collapsed: Set<string>,
	containerSizeCache: Map<string, { collapsed?: XY; expanded?: XY }>
): number {
	let newCollapsedCacheEntries = 0;

	const saved = new Map<HTMLElement, { w: string; h: string }>();
	const nodeEls = containerElement.querySelectorAll('.svelte-flow__node');

	for (const el of nodeEls) {
		const htmlEl = el as HTMLElement;
		const id = htmlEl.dataset.id;
		if (id && layoutGraph.containers.has(id) && collapsed.has(id)) {
			if (!containerSizeCache.get(id)?.collapsed) {
				saved.set(htmlEl, { w: htmlEl.style.width, h: htmlEl.style.height });
				htmlEl.style.width = 'auto';
				htmlEl.style.height = 'auto';
				const inner = htmlEl.querySelector(':scope > .relative') as HTMLElement;
				if (inner) {
					saved.set(inner, { w: inner.style.width, h: inner.style.height });
					inner.style.width = 'auto';
					inner.style.height = 'auto';
				}
			}
		}
	}

	if (saved.size > 0) {
		for (const el of nodeEls) {
			const htmlEl = el as HTMLElement;
			const id = htmlEl.dataset.id;
			if (id && saved.has(htmlEl)) {
				const w = htmlEl.offsetWidth || 250;
				const h = htmlEl.offsetHeight || 100;
				const entry = containerSizeCache.get(id) ?? {};
				entry.collapsed = { x: w, y: h };
				containerSizeCache.set(id, entry);
				newCollapsedCacheEntries++;
			}
		}
		for (const [el, { w, h }] of saved) {
			el.style.width = w;
			el.style.height = h;
		}
	}

	return newCollapsedCacheEntries;
}
