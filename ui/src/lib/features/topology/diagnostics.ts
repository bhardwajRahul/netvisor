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
import { CULLING_THRESHOLD_ELEMENTS } from './pipeline/render-mode';
import { collapseLevel, collapsedContainers } from './collapse';
import { activeView } from './queries';

/** How many samples to keep. Enough to cover the runs leading into a blank, small enough to mail. */
const HISTORY_LIMIT = 30;

/** Minimum gap between viewport-driven samples. Panning fires continuously. */
const VIEWPORT_SAMPLE_INTERVAL_MS = 500;

export type BlankReason = 'culled' | 'empty' | 'hidden' | null;

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
	culling: {
		on: boolean;
		threshold: number;
		measuring: boolean;
		exporting: boolean;
	};
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
	measuring: boolean;
	exporting: boolean;
	trigger: ViewerSample['trigger'];
}

function readPane(): { el: HTMLElement | null; rect: DOMRect | null } {
	const el = document.querySelector('.svelte-flow') as HTMLElement | null;
	return { el, rect: el?.getBoundingClientRect() ?? null };
}

/**
 * One cheap snapshot of everything that decides whether the graph is visible.
 *
 * Reads the DOM rather than SvelteFlow's internals deliberately: the question is what the user is
 * looking at, and the store's opinion of that is exactly what is in doubt.
 */
export function sampleViewerState(inputs: SampleInputs): ViewerSample {
	const { el: pane, rect: paneRect } = readPane();
	const viewport = document.querySelector('.svelte-flow__viewport') as HTMLElement | null;
	const nodeEls = Array.from(document.querySelectorAll('.svelte-flow__node')) as HTMLElement[];

	let bounds: ViewerSample['bounds'] = null;
	let withSize = 0;
	for (const el of nodeEls) {
		const r = el.getBoundingClientRect();
		if (r.width > 0 && r.height > 0) withSize += 1;
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
	} else if (paneHidden && !inputs.measuring) {
		// Hidden *while measuring* is the measure pass doing its job — it mounts every node at
		// full size behind `visibility: hidden` to read their heights, and a viewport sample
		// landing in that window sees an intentionally invisible canvas. Only a pane still hidden
		// with no measure pass running is a fault, and it is the one an operator would describe
		// the same way as the others.
		blank = 'hidden';
	} else if (!paneHidden && (nodeEls.length === 0 || !intersectsPane)) {
		blank = 'culled';
	}

	const round = (n: number) => Math.round(n);
	return {
		at: round(performance.now()),
		trigger: inputs.trigger,
		view: get(activeView),
		store: { nodes: inputs.storeNodes, edges: inputs.storeEdges },
		mounted: nodeEls.length,
		withSize,
		culling: {
			// Recomputed from the same inputs the viewer passes to `shouldCull`, rather than read
			// back from it, so a sample is meaningful even if the two ever disagree.
			on: !inputs.measuring && !inputs.exporting && inputs.storeNodes >= CULLING_THRESHOLD_ELEMENTS,
			threshold: CULLING_THRESHOLD_ELEMENTS,
			measuring: inputs.measuring,
			exporting: inputs.exporting
		},
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
}

function buildReport(current: ViewerSample): DiagnosticsReport {
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
		samples: [...history, current]
	};
}

/**
 * Publish `scanopyTopologyDiagnostics()` on `window`.
 *
 * One command for a customer to run and one file to send back. It takes a fresh sample first, so
 * it works when called *while* the canvas is blank — which is when someone will reach for it.
 */
export function installDiagnostics(read: () => Omit<SampleInputs, 'trigger'>): void {
	if (typeof window === 'undefined') return;
	(
		window as unknown as { scanopyTopologyDiagnostics?: () => DiagnosticsReport }
	).scanopyTopologyDiagnostics = () => {
		const current = sampleViewerState({ ...read(), trigger: 'manual' });
		const report = buildReport(current);

		const blob = new Blob([JSON.stringify(report, null, 2)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = url;
		link.download = `scanopy-topology-diagnostics-${Date.now()}.json`;
		link.click();
		URL.revokeObjectURL(url);

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
}

/** Test seam. */
export function diagnosticsHistory(): ViewerSample[] {
	return [...history];
}
