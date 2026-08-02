/**
 * When SvelteFlow may cull off-screen nodes and edges.
 *
 * At a few hundred hosts the L2 view mounts ~1250 nodes and >20,000 DOM
 * elements, and panning spends most frames over the 50ms jank threshold.
 * Viewport culling is the direct fix, but it cannot simply be switched on:
 *
 *  - **The measure pass needs every node in the DOM.** It mounts all nodes and
 *    reads their heights; culled nodes never mount, so it would time out and
 *    hand ELK fallback sizes.
 *  - **Export rasterises the whole flow element** (`html-to-image`), so a
 *    culled graph exports cropped.
 *
 * Note this helps *interaction*, not the cold load: SvelteFlow force-renders a
 * node until its `handleBounds` exist, and those only appear once a node has
 * mounted and been measured (`@xyflow/system`: `forceInitialRender =
 * !node.internals.handleBounds`). Since the pipeline rebuilds node objects
 * without a `measured` field on every run, everything renders at least once
 * regardless. Panning afterwards is where culling pays.
 */

/**
 * Exported so a diagnostic can report the gate's inputs *and* the line they were
 * measured against — "412 nodes, culling on" is only actionable next to the
 * threshold that made it so.
 */
export const CULLING_THRESHOLD_ELEMENTS = 150;

/**
 * Escape hatch for tooling that reads the graph out of the DOM.
 *
 * The layout-quality eval extracts node positions by querying
 * `.svelte-flow__node`; with culling on it would silently score only the
 * visible subset and report a healthy-looking result for a graph it never saw.
 */
function cullingDisabledForTooling(): boolean {
	return (
		typeof window !== 'undefined' &&
		(window as unknown as { __topoNoCull?: boolean }).__topoNoCull === true
	);
}

export interface CullingConditions {
	/** Nodes currently handed to SvelteFlow. */
	renderedCount: number;
	/** A DOM measurement pass is in progress. */
	measuring: boolean;
	/** An export is capturing the flow element. */
	exporting: boolean;
}

/**
 * Culling is off below the threshold, so small topologies keep exactly the
 * behaviour they have today — the guardrail is that nothing changes at normal
 * scale, and a count-based gate is view-agnostic.
 */
export function shouldCull({ renderedCount, measuring, exporting }: CullingConditions): boolean {
	if (measuring || exporting || cullingDisabledForTooling()) return false;
	return renderedCount >= CULLING_THRESHOLD_ELEMENTS;
}
