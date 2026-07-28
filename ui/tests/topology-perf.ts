import { test, expect, type Page } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

/**
 * Topology render-performance measurement.
 *
 * Companion to `topology-layout-eval.ts`: that one scores layout *quality*,
 * this one scores render *cost*. Both run against a live dev stack.
 *
 * Prerequisites:
 *   1. `npm run dev` (Vite on :5173) plus a running backend.
 *   2. A seeded large L2 dataset — see `backend/scripts/seed-l2-perf.sql`.
 *   3. SESSION_ID from a logged-in browser session.
 *
 * Run:
 *   SESSION_ID=<session> npx playwright test tests/topology-perf.ts
 *
 * Results are written to `tests/results/topology-perf.json` so runs are
 * comparable across commits — the point is before/after numbers, not a
 * one-time console dump. Set PERF_LABEL to tag a run (e.g. PERF_LABEL=baseline).
 */

const OUTPUT_PATH = resolve('tests/results/topology-perf.json');

interface PerfSnapshot {
	durations: Record<string, number>;
	counts: Record<string, number>;
	runs: number;
}

interface FrameStats {
	frames: number;
	meanMs: number;
	p95Ms: number;
	worstMs: number;
	longFrames: number;
}

interface PerfReport {
	label: string;
	nodeCount: number;
	edgeCount: number;
	domNodeCount: number;
	domEdgePathCount: number;
	timeToInteractiveMs: number;
	pipeline: PerfSnapshot;
	pan: FrameStats;
	zoom: FrameStats;
}

/** Read the app's own pipeline instrumentation (see `lib/features/topology/perf.ts`). */
async function readPipelinePerf(page: Page): Promise<PerfSnapshot> {
	return page.evaluate(() => {
		const api = (
			window as unknown as {
				__scanopyTopologyPerf?: { snapshot: () => PerfSnapshot };
			}
		).__scanopyTopologyPerf;
		return api ? api.snapshot() : { durations: {}, counts: {}, runs: 0 };
	});
}

/**
 * Wait until node positions stop changing.
 *
 * Deliberately does not key off `.svelte-flow__node` count alone: with viewport
 * culling the rendered set changes as the graph settles, so we compare a
 * position fingerprint and require two identical consecutive samples.
 */
async function waitForStableLayout(page: Page, timeoutMs = 60_000): Promise<void> {
	const started = Date.now();
	let previous = '';
	let stableSamples = 0;

	while (Date.now() - started < timeoutMs) {
		const fingerprint = await page.evaluate(() =>
			Array.from(document.querySelectorAll('.svelte-flow__node'))
				.map((el) => `${(el as HTMLElement).dataset.id}:${(el as HTMLElement).style.transform}`)
				.sort()
				.join('|')
		);

		if (fingerprint !== '' && fingerprint === previous) {
			stableSamples += 1;
			if (stableSamples >= 2) return;
		} else {
			stableSamples = 0;
		}
		previous = fingerprint;
		await page.waitForTimeout(250);
	}
	throw new Error(`Layout did not settle within ${timeoutMs}ms`);
}

/**
 * Sample frame durations via rAF while `action` drives the viewport.
 *
 * requestAnimationFrame deltas are what the user actually perceives as jank,
 * and unlike CDP tracing they need no protocol plumbing to interpret.
 */
async function measureFrames(page: Page, action: () => Promise<void>): Promise<FrameStats> {
	await page.evaluate(() => {
		const w = window as unknown as { __frameDeltas?: number[]; __framesStop?: () => void };
		w.__frameDeltas = [];
		let last = performance.now();
		let running = true;
		const tick = () => {
			if (!running) return;
			const now = performance.now();
			w.__frameDeltas!.push(now - last);
			last = now;
			requestAnimationFrame(tick);
		};
		requestAnimationFrame(tick);
		w.__framesStop = () => {
			running = false;
		};
	});

	await action();

	return page.evaluate(() => {
		const w = window as unknown as { __frameDeltas?: number[]; __framesStop?: () => void };
		w.__framesStop?.();
		// Drop the first delta: it spans the gap before the interaction started.
		const deltas = (w.__frameDeltas ?? []).slice(1).sort((a, b) => a - b);
		if (deltas.length === 0) {
			return { frames: 0, meanMs: 0, p95Ms: 0, worstMs: 0, longFrames: 0 };
		}
		const sum = deltas.reduce((acc, d) => acc + d, 0);
		return {
			frames: deltas.length,
			meanMs: sum / deltas.length,
			p95Ms: deltas[Math.min(deltas.length - 1, Math.floor(deltas.length * 0.95))],
			worstMs: deltas[deltas.length - 1],
			// 50ms is the "long task" threshold — a frame this slow is visible jank.
			longFrames: deltas.filter((d) => d > 50).length
		};
	});
}

async function panViewport(page: Page): Promise<void> {
	const box = await page.locator('.svelte-flow').boundingBox();
	if (!box) throw new Error('No .svelte-flow pane to pan');

	const midX = box.x + box.width / 2;
	const midY = box.y + box.height / 2;

	await page.mouse.move(midX, midY);
	await page.mouse.down();
	// A slow, many-step drag: each step is a separate mousemove, so the renderer
	// gets a realistic stream of viewport updates rather than one jump.
	for (let step = 1; step <= 40; step++) {
		await page.mouse.move(midX - step * 8, midY - step * 4);
	}
	await page.mouse.up();
	await page.waitForTimeout(250);
}

async function zoomViewport(page: Page): Promise<void> {
	const box = await page.locator('.svelte-flow').boundingBox();
	if (!box) throw new Error('No .svelte-flow pane to zoom');

	await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
	for (let step = 0; step < 20; step++) {
		await page.mouse.wheel(0, step % 2 === 0 ? -120 : 120);
	}
	await page.waitForTimeout(250);
}

test('topology render performance', async ({ page, context }) => {
	test.setTimeout(180_000);

	await context.addCookies([
		{ name: 'session_id', value: process.env.SESSION_ID ?? '', domain: 'localhost', path: '/' }
	]);

	// Turn instrumentation on before any app code runs, so the very first
	// pipeline run is recorded.
	await page.addInitScript(() => {
		(window as unknown as { __topoPerf: boolean }).__topoPerf = true;
	});

	const navigationStart = Date.now();
	await page.goto('/#topology');
	await page.waitForSelector('.svelte-flow__node', { timeout: 60_000 });
	await waitForStableLayout(page);
	const timeToInteractiveMs = Date.now() - navigationStart;

	const pipeline = await readPipelinePerf(page);

	const { nodeCount, edgeCount, domNodeCount, domEdgePathCount } = await page.evaluate(() => ({
		// Graph size as the app knows it, independent of what is rendered.
		nodeCount: document.querySelectorAll('.svelte-flow__node').length,
		edgeCount: document.querySelectorAll('.svelte-flow__edge').length,
		domNodeCount: document.querySelectorAll('.svelte-flow__node *').length,
		domEdgePathCount: document.querySelectorAll('.svelte-flow__edge path').length
	}));

	const pan = await measureFrames(page, () => panViewport(page));
	const zoom = await measureFrames(page, () => zoomViewport(page));

	const report: PerfReport = {
		label: process.env.PERF_LABEL ?? 'unlabelled',
		nodeCount,
		edgeCount,
		domNodeCount,
		domEdgePathCount,
		timeToInteractiveMs,
		pipeline,
		pan,
		zoom
	};

	mkdirSync(dirname(OUTPUT_PATH), { recursive: true });
	writeFileSync(OUTPUT_PATH, JSON.stringify(report, null, 2));

	const round = (n: number) => Math.round(n * 10) / 10;
	console.log(`\n=== Topology Render Performance (${report.label}) ===`);
	console.log(`  Rendered nodes / edges:   ${nodeCount} / ${edgeCount}`);
	console.log(`  DOM elements in nodes:    ${domNodeCount}`);
	console.log(`  Edge <path> elements:     ${domEdgePathCount}`);
	console.log(`  Time to interactive:      ${timeToInteractiveMs}ms`);
	console.log(`  Pipeline runs:            ${pipeline.runs}`);
	console.log(`  elk.layout() calls:       ${pipeline.counts['elk.layout'] ?? 0}`);
	console.log(`  Full measure passes:      ${pipeline.counts['full-measure-pass'] ?? 0}`);
	console.log(`  Post-render re-layouts:   ${pipeline.counts['post-render-relayout'] ?? 0}`);
	for (const [name, ms] of Object.entries(pipeline.durations)) {
		console.log(`    ${name.padEnd(22)} ${round(ms)}ms total`);
	}
	console.log(
		`  Pan:  mean ${round(pan.meanMs)}ms  p95 ${round(pan.p95Ms)}ms  worst ${round(pan.worstMs)}ms  >50ms: ${pan.longFrames}/${pan.frames}`
	);
	console.log(
		`  Zoom: mean ${round(zoom.meanMs)}ms  p95 ${round(zoom.p95Ms)}ms  worst ${round(zoom.worstMs)}ms  >50ms: ${zoom.longFrames}/${zoom.frames}`
	);
	console.log(`\n  Written to ${OUTPUT_PATH}`);

	// This is a measurement harness, not a gate — the only hard assertion is
	// that we actually measured something, so a silently-empty graph can't be
	// reported as a great result.
	expect(nodeCount, 'no nodes rendered — is the dataset seeded?').toBeGreaterThan(0);
	expect(pan.frames, 'no frames sampled during pan').toBeGreaterThan(0);
});
