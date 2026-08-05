/**
 * Blank-canvas diagnostics for the topology viewer.
 *
 * A customer on a ~300-host network reports the L2 canvas going blank at the upper collapse
 * levels — minimap still drawing, `F` doing nothing, intermittent and browser-dependent. It did
 * not reproduce on the 444-host seed here, so the next move is evidence from their browser rather
 * than more guessing.
 *
 * # Why this is not in `perf.ts`
 *
 * `perf.ts` is gated on `import.meta.env.DEV || window.__topoPerf`, so it records nothing in a
 * customer's build — and asking someone to set a global and reproduce an intermittent fault is
 * asking them to do the hard part twice. This is always on. It costs one
 * `querySelectorAll().length` per pipeline run plus a throttled handler, which is what makes that
 * affordable.
 *
 * # What it records
 *
 * A ring buffer, not a single snapshot. The interesting part of an intermittent blank is the
 * *transition into* it — what the culling gate, the viewport and the measured-size coverage were
 * doing on the runs before. One sample of a broken state rarely identifies a cause.
 *
 * Blankness has three distinguishable causes and the record has to separate them, since they lead
 * to completely different fixes:
 *
 * - **Culled** — the store holds nodes, none are mounted. The viewport is somewhere the graph is
 *   not, or the gate is wrong.
 * - **Empty** — the store holds nothing. A pipeline or data problem, nothing to do with rendering.
 * - **Hidden** — the container is `visibility: hidden` for the measure pass. Transient by design;
 *   only a fault if it sticks.
 */

import throttle from 'just-throttle';
import { get } from 'svelte/store';
import {
	CULLING_THRESHOLD_ELEMENTS,
	cullingDisabledForTooling as cullingSuppressedForTooling
} from './pipeline/render-mode';
import { collapseLevel, collapsedContainers } from './collapse';
import { activeView } from './queries';

/** How many samples to keep. Enough to cover the runs leading into a blank, small enough to mail. */
const HISTORY_LIMIT = 30;

/**
 * Size at or below which a mounted node counts as degenerate, in either dimension.
 *
 * A node rendered with `width: 0` or `height: 0` still occupies its 1px borders, so the floor is a
 * couple of pixels rather than zero. Nothing legitimate comes close: element cards are a fixed 250
 * wide, and a container is at least its type's declared collapsed size.
 */
const DEGENERATE_SIZE_PX = 4;

/** Minimum gap between viewport-driven samples. Panning fires continuously. */
const VIEWPORT_SAMPLE_INTERVAL_MS = 500;

export type BlankReason = 'culled' | 'empty' | 'hidden' | null;

/**
 * How many of the store's nodes SvelteFlow is *able* to cull.
 *
 * The quantity missing from the first version of this report, and the one that would have named
 * the fault immediately. Culling is opt-out until a node can be tested: `getNodesInside` skips the
 * viewport check entirely while `internals.handleBounds` is undefined (`forceInitialRender`), and
 * computes `area` from `measured`, treating an unknown height as zero — which passes
 * `overlappingArea >= area` wherever the viewport is. A node missing either is mounted no matter
 * where it sits, so a report can show `culling: on` beside `mounted == store.nodes` and nothing in
 * it explains why. `forceRendered` is that number directly.
 */
export interface CullabilitySummary {
	/** Internal nodes inspected. Less than `store.nodes` when the viewport path stride-samples. */
	total: number;
	withMeasured: number;
	withHandleBounds: number;
	/** Nodes SvelteFlow will mount regardless of the viewport. */
	forceRendered: number;
}

/** The shape `summariseCullability` needs. Structural, so a test needs no SvelteFlow. */
export interface CullableNode {
	measured?: { width?: number; height?: number };
	internals?: { handleBounds?: unknown };
}

/**
 * Reduce internal nodes to the counts above. Pure, so it can be unit-tested without a DOM.
 *
 * A node counts as measured only with both dimensions: a width alone still yields `area === 0`,
 * which is the case that made every element card unconditionally visible.
 */
export function summariseCullability(nodes: (CullableNode | undefined)[]): CullabilitySummary {
	let total = 0;
	let withMeasured = 0;
	let withHandleBounds = 0;
	let forceRendered = 0;

	for (const node of nodes) {
		if (!node) continue;
		total += 1;
		const measured = Boolean(node.measured?.width && node.measured?.height);
		const handleBounds = Boolean(node.internals?.handleBounds);
		if (measured) withMeasured += 1;
		if (handleBounds) withHandleBounds += 1;
		if (!handleBounds || !measured) forceRendered += 1;
	}

	return { total, withMeasured, withHandleBounds, forceRendered };
}

/**
 * Counters that accumulate over the session, rather than describing one moment.
 *
 * The customer's console held 247 `out of memory` throws, and the ring buffer could not say
 * whether that was one graph too large to mount or the same graph mounted over and over. These
 * separate the two: a handful of node-store writes with a high `peakMounted` is a single
 * allocation, while hundreds of writes is a remount loop. Deliberately not in `perf.ts`, which
 * records nothing unless the build is a dev build.
 *
 * `usedJSHeapSize` would answer it directly but is Chrome-only, and the report that prompted this
 * came from Firefox — so these are counters rather than a memory reading, and the heap figure is
 * included only when the browser happens to offer it.
 */
export interface CumulativeCounters {
	pipelineRuns: number;
	nodeStoreWrites: number;
	fullMeasurePasses: number;
	peakStoreNodes: number;
	peakMounted: number;
	/** Chrome-only; absent in Firefox and Safari. */
	usedJSHeapMb?: number;
}

const counters: CumulativeCounters = {
	pipelineRuns: 0,
	nodeStoreWrites: 0,
	fullMeasurePasses: 0,
	peakStoreNodes: 0,
	peakMounted: 0
};

/** Called on every write to the node store, so a remount loop is visible as a count. */
export function noteNodeStoreWrite(nodeCount: number): void {
	counters.nodeStoreWrites += 1;
	counters.peakStoreNodes = Math.max(counters.peakStoreNodes, nodeCount);
}

/**
 * What one pipeline run did, recorded as it happens.
 *
 * The per-sample ring says *that* containers went zero-sized; it cannot say which run did it or
 * why. Every capture so far shows the same shape — a `pipeline` sample with degenerate containers
 * after a clean one — and the last one placed the onset 66 seconds after the user stopped
 * interacting, so the run was data-driven rather than a click. These fields separate the remaining
 * candidates: what started the run, whether it rebuilt the layout graph (which resets every
 * container's `expandedSize` to zero), whether ELK ran to put sizes back, and how many containers
 * the graph still believed were zero-sized when it finished.
 *
 * `containersZeroSizedAfter` is the one that matters: it reads the layout *model*, not the DOM, so
 * it distinguishes "the graph lost the sizes" from "the render is behind".
 */
export interface RunRecord {
	at: number;
	/** What triggered it — `collapsed`, `topology`, `pending`, and so on. */
	source: string;
	isNewStructure?: boolean;
	needsElk?: boolean;
	/** The layout graph was rebuilt, resetting every `expandedSize` to `{0, 0}`. */
	graphRebuilt?: boolean;
	/** Containers ELK returned a size for, when it ran. */
	elkSizedContainers?: number;
	/**
	 * Expanded containers whose `expandedSize` is still zero once the run finished.
	 *
	 * The number to read first. Anything above zero here means the layout model itself lost the
	 * sizes, and every one of those containers will draw as its borders with its contents outside.
	 * Collapsed containers are excluded — they have never been laid out expanded, so a zero is
	 * expected and would swamp the signal.
	 */
	containersZeroSizedAfter?: number;
}

/** Last few runs, so the transition into a bad state is visible rather than just its aftermath. */
const RUN_HISTORY_LIMIT = 12;
const runs: RunRecord[] = [];

export function noteRunStart(source: string): void {
	runs.push({ at: Math.round(performance.now()), source });
	if (runs.length > RUN_HISTORY_LIMIT) runs.shift();
}

/** Fill in detail on the run currently in flight. */
export function noteRunDetail(patch: Partial<RunRecord>): void {
	const current = runs.at(-1);
	if (current) Object.assign(current, patch);
}

/** Called when the pipeline takes the full measurement path, which mounts every node. */
export function noteFullMeasurePass(): void {
	counters.fullMeasurePasses += 1;
}

export interface ViewerSample {
	/** Milliseconds since page load, so a reader can see the spacing between samples. */
	at: number;
	/** What prompted this sample. */
	trigger: 'pipeline' | 'viewport' | 'manual';
	view: string;
	/** Nodes and edges the store holds — what *should* be drawn. */
	store: { nodes: number; edges: number };
	/** Nodes SvelteFlow actually mounted. The gap between this and `store.nodes` is culling. */
	mounted: number;
	/** How many mounted nodes report a non-zero size. Zero here with a non-zero mount count is
	 *  the measure pass having produced nothing, which starves both layout and fit-view. */
	withSize: number;
	/**
	 * Mounted nodes rendering at essentially no size in either dimension, split by node type.
	 *
	 * `withSize` cannot see this fault and never could: it asks whether a node's bounding box is
	 * non-zero, and a container collapsed to its own borders is 250x2 — non-zero, and counted as
	 * healthy. But a container at 2px has its contents drawn outside it, which is what an operator
	 * reports as "the nodes didn't finish resizing". Width and height are both tested because they
	 * collapse independently — the same fault produced 250x2 and 2x2 nodes in the same graph.
	 *
	 * Split by type because the two causes are different. Containers go degenerate when ELK never
	 * sized them — `LayoutContainer.expandedSize` starts at `{0, 0}` and `getContainerSize` returns
	 * that rather than `undefined`, so a container it never laid out is built with `height: 0`.
	 * Elements go degenerate when a measurement pass is mid-flight, since it deliberately builds
	 * them unsized.
	 */
	degenerate: { containers: number; elements: number };
	culling: {
		/** The value actually handed to SvelteFlow, not a re-derivation of it. */
		on: boolean;
		threshold: number;
		measuring: boolean;
		exporting: boolean;
		/** `window.__topoNoCull` — tooling suppressing culling entirely. */
		suppressedForTooling: boolean;
	};
	/** How much of the store SvelteFlow could cull even in principle. */
	cullable: CullabilitySummary | null;
	/** The pane's own size. Zero either dimension and *everything* is off-screen by definition. */
	pane: { width: number; height: number };
	/** SvelteFlow's transform, verbatim. */
	transform: string | null;
	/** Bounding box of the mounted nodes in screen space, and whether it meets the pane at all. */
	bounds: { left: number; top: number; right: number; bottom: number } | null;
	intersectsPane: boolean;
	collapse: { level: number | null; collapsedContainers: number };
	blank: BlankReason;
}

const history: ViewerSample[] = [];

interface SampleInputs {
	storeNodes: number;
	storeEdges: number;
	/** A DOM measurement pass is running, on any load. Suspends culling. */
	measuring: boolean;
	/** The cold load is hiding the pane while it measures. Only ever true on the first render. */
	coldLoadMeasure: boolean;
	exporting: boolean;
	/** The value the viewer passed to `onlyRenderVisibleElements`. */
	culling: boolean;
	/**
	 * This viewer's own element. Scopes the DOM reads below.
	 *
	 * The counts used to come from `document`, which also finds any other `<SvelteFlow>` mounted
	 * at the time — the dependency tutorial and the read-only viewer both mount one — so a report
	 * could attribute another canvas's nodes to this one.
	 */
	container: HTMLElement | null;
	/** SvelteFlow's internal nodes, for the cullability summary. */
	internalNodes: () => (CullableNode | undefined)[];
	trigger: ViewerSample['trigger'];
}

/**
 * Inspect at most this many internal nodes on the throttled viewport path.
 *
 * A pan samples up to twice a second and the walk is O(nodes); at 17,000 nodes doing it in full
 * would be a cost the diagnostic itself imposes on the case it exists to diagnose. Pipeline and
 * manual samples count everything, so the exact figure is always available when it matters.
 */
const CULLABILITY_SAMPLE_LIMIT = 500;

function sampleCullability(nodes: (CullableNode | undefined)[], full: boolean): CullabilitySummary {
	if (full || nodes.length <= CULLABILITY_SAMPLE_LIMIT) return summariseCullability(nodes);
	const stride = Math.ceil(nodes.length / CULLABILITY_SAMPLE_LIMIT);
	const sampled: (CullableNode | undefined)[] = [];
	for (let i = 0; i < nodes.length; i += stride) sampled.push(nodes[i]);
	return summariseCullability(sampled);
}

/**
 * The viewer's own pane, or the first one in the document if it hasn't bound yet.
 *
 * `container` is a `bind:this`, so it is null for the first sample or two after mount; the
 * document-wide fallback keeps those samples useful. Everywhere else it matters: another
 * `<SvelteFlow>` can be mounted at the same time (`DependencyTutorial`, `ReadOnlyTopologyViewer`),
 * and its pane would otherwise be measured against this viewer's node counts.
 */
function readPane(container: HTMLElement | null): { el: HTMLElement | null; rect: DOMRect | null } {
	const root: ParentNode = container ?? document;
	const el = root.querySelector('.svelte-flow') as HTMLElement | null;
	return { el, rect: el?.getBoundingClientRect() ?? null };
}

/**
 * One cheap snapshot of everything that decides whether the graph is visible.
 *
 * Reads the DOM rather than SvelteFlow's internals deliberately: the question is what the user is
 * looking at, and the store's opinion of that is exactly what is in doubt.
 */
export function sampleViewerState(inputs: SampleInputs): ViewerSample {
	const root: ParentNode = inputs.container ?? document;
	const { el: pane, rect: paneRect } = readPane(inputs.container);
	const viewport = root.querySelector('.svelte-flow__viewport') as HTMLElement | null;
	const nodeEls = Array.from(root.querySelectorAll('.svelte-flow__node')) as HTMLElement[];

	let bounds: ViewerSample['bounds'] = null;
	let withSize = 0;
	const degenerate = { containers: 0, elements: 0 };
	for (const el of nodeEls) {
		const r = el.getBoundingClientRect();
		if (r.width > 0 && r.height > 0) withSize += 1;
		// Both dimensions, because they fail independently: a container ELK never sized was
		// measured here at 250x2 in one case and 2x2 in another, the first keeping a width from
		// CSS while the second lost both. Layout size, not the bounding rect — the rect is scaled
		// by the viewport transform, so at low zoom every node looks tiny.
		if (el.offsetWidth <= DEGENERATE_SIZE_PX || el.offsetHeight <= DEGENERATE_SIZE_PX) {
			if (el.classList.contains('svelte-flow__node-Container')) degenerate.containers += 1;
			else degenerate.elements += 1;
		}
		bounds = bounds
			? {
					left: Math.min(bounds.left, r.left),
					top: Math.min(bounds.top, r.top),
					right: Math.max(bounds.right, r.right),
					bottom: Math.max(bounds.bottom, r.bottom)
				}
			: { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
	}

	const intersectsPane = Boolean(
		bounds &&
			paneRect &&
			bounds.right > paneRect.left &&
			bounds.left < paneRect.right &&
			bounds.bottom > paneRect.top &&
			bounds.top < paneRect.bottom
	);

	const paneHidden = pane ? getComputedStyle(pane).visibility === 'hidden' : false;
	let blank: BlankReason = null;
	if (inputs.storeNodes === 0) {
		blank = 'empty';
	} else if (paneHidden && !inputs.coldLoadMeasure) {
		// Hidden *while the cold load measures* is that pass doing its job — it mounts every node
		// at full size behind `visibility: hidden` to read their heights, and a viewport sample
		// landing in that window sees an intentionally invisible canvas. Only a pane still hidden
		// with no cold load running is a fault, and it is the one an operator would describe
		// the same way as the others.
		//
		// Gated on `coldLoadMeasure`, not `measuring`: the cold load is the only pass that hides
		// the pane, so a later measure pass must not excuse a pane that is stuck hidden.
		blank = 'hidden';
	} else if (!paneHidden && (nodeEls.length === 0 || !intersectsPane)) {
		blank = 'culled';
	}

	counters.peakMounted = Math.max(counters.peakMounted, nodeEls.length);

	const round = (n: number) => Math.round(n);
	return {
		at: round(performance.now()),
		trigger: inputs.trigger,
		view: get(activeView),
		store: { nodes: inputs.storeNodes, edges: inputs.storeEdges },
		mounted: nodeEls.length,
		withSize,
		degenerate,
		culling: {
			// The value the viewer actually handed to SvelteFlow. This used to be re-derived from
			// the node count instead — on the reasoning that a sample stays meaningful even if the
			// two disagree — which is backwards for the fault it was built to catch: the report
			// that mattered said `on: true` beside `mounted == store.nodes`, and the re-derivation
			// is exactly what could not be trusted. It also silently omitted the `__topoNoCull`
			// term, so a tooling run would have been reported as culling when it was not.
			on: inputs.culling,
			threshold: CULLING_THRESHOLD_ELEMENTS,
			measuring: inputs.measuring,
			exporting: inputs.exporting,
			suppressedForTooling: cullingSuppressedForTooling()
		},
		// Full count on the two triggers that are taken rarely; stride-sampled on the throttled
		// viewport path, which fires throughout a pan.
		cullable: sampleCullability(inputs.internalNodes(), inputs.trigger !== 'viewport'),
		pane: { width: round(pane?.clientWidth ?? 0), height: round(pane?.clientHeight ?? 0) },
		transform: viewport?.style.transform ?? null,
		bounds: bounds
			? {
					left: round(bounds.left),
					top: round(bounds.top),
					right: round(bounds.right),
					bottom: round(bounds.bottom)
				}
			: null,
		intersectsPane,
		collapse: {
			level: get(collapseLevel) ?? null,
			collapsedContainers: get(collapsedContainers).size
		},
		blank
	};
}

/**
 * The buffer as it stood the first time the canvas went blank, kept verbatim.
 *
 * The ring alone is not enough. Someone who has just lost the diagram pans, zooms, clicks the
 * minimap and presses F before thinking to run anything — every one of which records a sample, at
 * up to two a second. Fifteen seconds of that evicts the runs that led *into* the blank, which is
 * the only part that identifies a cause. So the moment blankness is first seen, the buffer is
 * copied somewhere nothing overwrites, and the report carries it however long they take.
 */
let firstBlankCapture: ViewerSample[] | null = null;

function push(sample: ViewerSample): void {
	history.push(sample);
	if (history.length > HISTORY_LIMIT) history.shift();
}

/** The last sample's blank reason, so a transition is announced once rather than every frame. */
let previousBlank: BlankReason = null;

function record(inputs: SampleInputs): void {
	const sample = sampleViewerState(inputs);
	push(sample);

	// Announce the edge, not the state. A blank canvas that persists across a pan would otherwise
	// fill the console with the same line and bury whatever else is there.
	if (sample.blank && sample.blank !== previousBlank) {
		// Before anything else can push it out of the ring.
		firstBlankCapture ??= [...history];
		console.warn(
			`[scanopy] topology canvas is blank (${sample.blank}): ` +
				`${sample.store.nodes} nodes in the graph, ${sample.mounted} drawn, ` +
				`culling ${sample.culling.on ? 'on' : 'off'}, level ${sample.collapse.level}. ` +
				`Run scanopyTopologyDiagnostics() to save a report.`
		);
	}
	previousBlank = sample.blank;
}

/** Called after each pipeline run, once the viewport has settled. */
export function recordAfterRun(inputs: Omit<SampleInputs, 'trigger'>): void {
	counters.pipelineRuns += 1;
	record({ ...inputs, trigger: 'pipeline' });
}

/**
 * Called on viewport movement, throttled.
 *
 * `leading: false, trailing: true` matches the convention used elsewhere: a pan's *final* position
 * is the one worth recording, and sampling its first frame would capture the state being left.
 */
export const recordAfterViewportMove = throttle(
	(inputs: Omit<SampleInputs, 'trigger'>) => record({ ...inputs, trigger: 'viewport' }),
	VIEWPORT_SAMPLE_INTERVAL_MS,
	{ leading: false, trailing: true }
);

export interface DiagnosticsReport {
	generatedAt: string;
	userAgent: string;
	screen: { width: number; height: number; devicePixelRatio: number };
	/**
	 * The buffer as it stood when the canvas first went blank, or `null` if it never did.
	 *
	 * Read this first: it holds the runs leading into the fault. `samples` below is whatever the
	 * live ring happens to hold by the time the report was taken, which after any amount of
	 * panning is the aftermath rather than the cause.
	 */
	firstBlank: ViewerSample[] | null;
	samples: ViewerSample[];
	/** What each recent pipeline run did — see `RunRecord`. */
	runs: RunRecord[];
	/**
	 * Totals for the session, which the per-sample ring cannot express.
	 *
	 * Read these against `samples`: they are what distinguishes one graph too large to mount from
	 * the same graph being mounted repeatedly, which the ring buffer alone cannot separate.
	 */
	cumulative: CumulativeCounters;
}

/** Chrome-only. Absent in Firefox and Safari, which is where this report tends to come from. */
function usedJSHeapMb(): number | undefined {
	const memory = (performance as unknown as { memory?: { usedJSHeapSize?: number } }).memory;
	const bytes = memory?.usedJSHeapSize;
	return typeof bytes === 'number' ? Math.round(bytes / 1024 / 1024) : undefined;
}

function buildReport(current: ViewerSample): DiagnosticsReport {
	const heap = usedJSHeapMb();
	return {
		generatedAt: new Date().toISOString(),
		// The fault is reported as browser-dependent, so the browser is part of the evidence.
		userAgent: navigator.userAgent,
		screen: {
			width: window.innerWidth,
			height: window.innerHeight,
			devicePixelRatio: window.devicePixelRatio
		},
		firstBlank: firstBlankCapture,
		runs: [...runs],
		samples: [...history, current],
		cumulative: { ...counters, ...(heap !== undefined && { usedJSHeapMb: heap }) }
	};
}

/**
 * Publish `scanopyTopologyDiagnostics()` on `window`.
 *
 * One command for a customer to run and one file to send back. It takes a fresh sample first, so
 * it works when called *while* the canvas is blank — which is when someone will reach for it.
 */
export interface DiagnosticsOptions {
	/**
	 * Save the report to a file. On by default — the point of the command is one file to send
	 * back. Tests pass `false`: they want the object, and a download triggers a browser prompt.
	 */
	download?: boolean;
}

export function installDiagnostics(read: () => Omit<SampleInputs, 'trigger'>): void {
	if (typeof window === 'undefined') return;
	(
		window as unknown as {
			scanopyTopologyDiagnostics?: (options?: DiagnosticsOptions) => DiagnosticsReport;
		}
	).scanopyTopologyDiagnostics = (options?: DiagnosticsOptions) => {
		const current = sampleViewerState({ ...read(), trigger: 'manual' });
		const report = buildReport(current);

		if (options?.download ?? true) {
			const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
			const url = URL.createObjectURL(blob);
			const link = document.createElement('a');
			link.href = url;
			link.download = `scanopy-topology-diagnostics-${Date.now()}.json`;
			link.click();
			URL.revokeObjectURL(url);
		}

		// Returned as well as downloaded: a browser that blocks the download still leaves the
		// object in the console to copy.
		return report;
	};
}

/** Test seam — the ring buffer is module state and specs need a clean one. */
export function resetDiagnostics(): void {
	history.length = 0;
	previousBlank = null;
	firstBlankCapture = null;
	counters.pipelineRuns = 0;
	counters.nodeStoreWrites = 0;
	counters.fullMeasurePasses = 0;
	counters.peakStoreNodes = 0;
	counters.peakMounted = 0;
	runs.length = 0;
}

/** Test seam. */
export function diagnosticsHistory(): ViewerSample[] {
	return [...history];
}
