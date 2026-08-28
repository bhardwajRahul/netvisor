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
export function cullingDisabledForTooling(): boolean {
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

/**
 * Zoom at or below which a node draws as its box rather than its contents.
 *
 * At 0.25 a 12px label renders at 3px and a 20px icon at 5px, so nothing inside a card is legible
 * — and on a large estate a fitted graph sits far below this anyway: the 5,936-node reproduction
 * fits at 0.0108, where a 250px card is under three pixels wide and every one of the ~5 DOM
 * elements inside it is laid out to draw nothing anyone can see.
 */
/**
 * Zoom below which a node stops rendering its contents.
 *
 * A card's own text is 12px, so this is the point it renders at 6px — the floor of legible. Below
 * here the text is decoration that costs DOM, and on a large estate the fitted zoom is far below
 * it anyway: the 5,936-node reproduction fits at 0.0108, where a 250px card is 2.7px wide.
 *
 * This is deliberately the *only* question answered by zoom alone. What a reduced node then draws
 * depends on how big that particular node is, which `nodeDetail` decides — a switch container and
 * a port-status group are 211px and 2.7px at the same zoom, and treating them alike is what made
 * the first attempt at this render a grid of blank pills with one label floating over them.
 */
export const DETAIL_ZOOM = 0.5;

/** Below this on-screen width a box is too small to be worth drawing structure inside. */
export const BOX_MIN_PX = 12;

export interface SimplifyConditions {
	/** The viewport's current zoom. */
	zoom: number;
	/** A DOM measurement pass is in progress. */
	measuring: boolean;
	/** An export is capturing the flow element. */
	exporting: boolean;
}

/**
 * Whether nodes should draw reduced, given the viewport and what else is happening.
 *
 * The same two suspensions as `shouldCull`, for the same two reasons and with the same
 * consequences if they are missed:
 *
 *  - **`measuring`** — the measure pass mounts every node to read its real card height. A reduced
 *    card measures its pinned height rather than its content's, so the pass would confirm whatever
 *    the last measurement happened to be and ELK would inherit it. Suspending here is also what
 *    makes the height pin correct: measured heights are always full-detail heights, so a reduced
 *    card pinned to `measured` occupies exactly the box the layout was built around and nothing
 *    reflows as the threshold is crossed.
 *  - **`exporting`** — `html-to-image` rasterises the flow element as it stands, so an export taken
 *    below the threshold would be a page of empty rectangles.
 *
 * Tooling shares `__topoNoCull` rather than getting a second flag: anything reading the graph out
 * of the DOM wants it whole and detailed, which is the same thing that switch already means.
 */
export function shouldSimplify({ zoom, measuring, exporting }: SimplifyConditions): boolean {
	if (measuring || exporting || cullingDisabledForTooling()) return false;
	return zoom < DETAIL_ZOOM;
}

/**
 * What a node draws once `shouldSimplify` says the graph is past full detail.
 *
 *  - `full` — contents as normal.
 *  - `boxed` — the box alone, which for an element card means its state colour.
 *  - `hidden` — nothing at all.
 *
 * Keyed on the node's *own* on-screen size rather than on zoom, which is what makes one rule work
 * across the hierarchy: at the zoom a large L2 graph fits at, a host container is 3.8px and the
 * element cards inside it are 2.7px, while zooming in moves them apart by two orders of magnitude.
 *
 * **Only subcontainers are ever `hidden`.** A grouping box a few pixels across is noise inside its
 * parent, and dropping it costs nothing because its children draw at their own absolute positions
 * regardless. An element card that small is still the graph's texture, and its colour is the one
 * thing that survives at that size, so it keeps its box.
 *
 * Nothing is labelled below full detail. A container's name cannot be drawn inside its own box —
 * SvelteFlow renders child nodes as siblings stacked above their parent, so a subcontainer's box
 * paints over anything the parent draws, at any z-index — and drawn outside the box it reads as
 * labelling whatever sits above it. Names return with the real header at `DETAIL_ZOOM`.
 */
export type NodeDetail = 'full' | 'boxed' | 'hidden';

export interface NodeDetailConditions {
	/** False once `shouldSimplify` is true. */
	detail: boolean;
	/** The node's own width in screen pixels — its layout width times the zoom. */
	screenWidth: number;
	/** A grouping container rather than one a user navigates by. Always false for elements. */
	isSubcontainer: boolean;
}

export function nodeDetail({
	detail,
	screenWidth,
	isSubcontainer
}: NodeDetailConditions): NodeDetail {
	if (detail) return 'full';
	if (isSubcontainer && screenWidth < BOX_MIN_PX) return 'hidden';
	return 'boxed';
}
