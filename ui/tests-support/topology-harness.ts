import type { BrowserContext, Page } from '@playwright/test';

/**
 * Shared helpers for the topology Playwright harnesses.
 *
 * Lives outside `tests/` deliberately: `playwright.config.ts` sets
 * `testMatch: '**\/*.ts'`, so anything under the test directory is collected as a spec and a
 * helper module placed there is reported as a file containing no tests.
 */

/** Read the app's own pipeline instrumentation (see `lib/features/topology/perf.ts`). */
export interface PerfSnapshot {
	durations: Record<string, number>;
	counts: Record<string, number>;
	runs: number;
}

export async function readPipelinePerf(page: Page): Promise<PerfSnapshot> {
	return page.evaluate(() => {
		const api = (window as unknown as { __scanopyTopologyPerf?: { snapshot: () => PerfSnapshot } })
			.__scanopyTopologyPerf;
		return api ? api.snapshot() : { durations: {}, counts: {}, runs: 0 };
	});
}

/** The always-on diagnostic's report. Mirrors `lib/features/topology/diagnostics.ts`. */
export interface DiagnosticsReport {
	samples: {
		trigger: string;
		view: string;
		store: { nodes: number; edges: number };
		mounted: number;
		culling: { on: boolean; measuring: boolean; suppressedForTooling: boolean };
		cullable: {
			total: number;
			withMeasured: number;
			withHandleBounds: number;
			forceRendered: number;
		} | null;
		collapse: { level: number | null; collapsedContainers: number };
		/** Mounted nodes at or below a few pixels in either dimension — a stale or missing size. */
		degenerate?: { containers: number; elements: number };
	}[];
	cumulative: {
		pipelineRuns: number;
		nodeStoreWrites: number;
		fullMeasurePasses: number;
		peakStoreNodes: number;
		peakMounted: number;
		usedJSHeapMb?: number;
		/** ELK layouts performed — the allocation driver. See `topology-layout-cost.ts`. */
		elkRuns?: number;
		/** Runs abandoned before ELK because a later press superseded them. */
		runsSuperseded?: number;
		/** Session peak, as distinct from the reading at capture time. Chrome-only. */
		peakUsedJSHeapMb?: number;
		peakTotalJSHeapMb?: number;
	};
}

/**
 * Take a diagnostics sample without triggering a file download.
 *
 * The command exists so a customer gets one file to send back; a headless run wants the object.
 */
export async function readDiagnostics(page: Page): Promise<DiagnosticsReport> {
	return page.evaluate(() => {
		const read = (
			window as unknown as {
				scanopyTopologyDiagnostics?: (o?: { download?: boolean }) => DiagnosticsReport;
			}
		).scanopyTopologyDiagnostics;
		if (!read) throw new Error('scanopyTopologyDiagnostics is not installed');
		return read({ download: false });
	});
}

/** Authenticate the harness as a logged-in user. */
export async function signIn(context: BrowserContext): Promise<void> {
	await context.addCookies([
		{ name: 'session_id', value: process.env.SESSION_ID ?? '', domain: 'localhost', path: '/' }
	]);
}

/**
 * Wait until the render pipeline is genuinely idle.
 *
 * Three conditions, all required — getting this wrong silently invalidates a comparison rather
 * than failing it:
 *
 *  1. **No pipeline run in flight.** The app reports this via `runStartedAt`. Node positions can
 *     be stable *between* stages of a run that is about to re-layout, and a cold load
 *     legitimately runs the pipeline more than once.
 *  2. **At least one run has completed.** Edges are flushed only once `nodesInitialized` fires, so
 *     a node-only fingerprint can settle on a page whose edges have not been built yet — a
 *     strictly cheaper page than the one under test. This used to be checked as "some
 *     `.svelte-flow__edge` exists in the DOM", which viewport culling invalidated: an edge is
 *     drawn only when both its endpoints are, so a correctly culled graph can legitimately show
 *     none, and the wait would time out on a perfectly healthy view. The run counter is what that
 *     check was really proxying for.
 *  3. **Node positions unchanged** across consecutive samples.
 *
 * Deliberately does not key off node *count* alone: with viewport culling the rendered set
 * changes as the graph settles.
 */
export async function waitForStableLayout(page: Page, timeoutMs = 90_000): Promise<void> {
	const started = Date.now();
	let previous = '';
	let stableSamples = 0;

	while (Date.now() - started < timeoutMs) {
		const sample = await page.evaluate(() => {
			const api = (
				window as unknown as {
					__scanopyTopologyPerf?: { snapshot: () => { runStartedAt: number | null; runs: number } };
				}
			).__scanopyTopologyPerf;
			const snapshot = api?.snapshot();
			return {
				running: snapshot ? snapshot.runStartedAt !== null : false,
				runs: snapshot?.runs ?? 0,
				fingerprint: Array.from(document.querySelectorAll('.svelte-flow__node'))
					.map((el) => `${(el as HTMLElement).dataset.id}:${(el as HTMLElement).style.transform}`)
					.sort()
					.join('|')
			};
		});

		const settled =
			!sample.running &&
			sample.runs > 0 &&
			sample.fingerprint !== '' &&
			sample.fingerprint === previous;

		if (settled) {
			stableSamples += 1;
			if (stableSamples >= 2) return;
		} else {
			stableSamples = 0;
		}
		previous = sample.fingerprint;
		await page.waitForTimeout(250);
	}
	throw new Error(`Layout did not settle within ${timeoutMs}ms`);
}

/** Turn the app's pipeline instrumentation on before any app code runs. */
export async function enablePerfInstrumentation(page: Page): Promise<void> {
	await page.addInitScript(() => {
		(window as unknown as { __topoPerf: boolean }).__topoPerf = true;
	});
}
