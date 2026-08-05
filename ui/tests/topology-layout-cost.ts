import { test, expect, type Page } from '@playwright/test';
import {
	enablePerfInstrumentation,
	readDiagnostics,
	signIn,
	waitForStableLayout
} from '../tests-support/topology-harness';

/**
 * Collapse presses must cost one layout each, and must not hold two ELK graphs at once.
 *
 * A customer's L2 view exhausted browser memory, and the resting footprint turned out not to be
 * the cause: the tab was measured swinging 881MB to 2050MB and back inside 90 seconds of
 * interaction, while diagnostics captured moments later read 267MB live. The cost is transient and
 * concentrated in the layout pipeline, which is why it survived several rounds of DOM and culling
 * work that all reduced the resting footprint.
 *
 * Two faults produced it, both asserted here:
 *
 *  1. **Runs were not cancelled.** ELK is ~96% of pipeline time, so a run takes 1.8-3.7s while
 *     presses arrive every ~2s. Every press landed mid-run, and the run in flight completed a full
 *     layout that the queued run discarded on arrival — a capture showed 8 runs for ~4 presses,
 *     all of them running ELK.
 *  2. **Both ELK passes were live at once.** `computeElkLayout` lays out twice, and the pass-1
 *     graph stayed reachable from the async function's heap-allocated scope while pass 2 built its
 *     own, doubling the peak.
 *
 * Absolute megabytes are not asserted: they depend on the machine, the dataset, and where GC
 * happens to fall — the same repro was seen peaking anywhere between 1.2GB and 2.6GB. What is
 * stable, and what actually regressed, is the *count* of layouts performed per press. Heap figures
 * are reported for context.
 *
 * Prerequisites:
 *   1. `npm run dev` (Vite on :5173) plus a running backend.
 *   2. A seeded large L2 dataset — see `backend/scripts/seed-l2-perf.sql`.
 *   3. SESSION_ID from a logged-in browser session.
 *
 * Run:
 *   SESSION_ID=<session> npx playwright test tests/topology-layout-cost.ts --project=chromium
 */

/** Presses to fire back-to-back, without waiting for the layout each one triggers. */
const RAPID_PRESSES = 4;

/**
 * Layouts allowed on top of one per press.
 *
 * A cold load legitimately lays out more than once — the initial render, and a corrective pass once
 * collapsed containers report their real sizes. The fault this guards produced *double* per press,
 * so a fixed small allowance separates the two without pinning incidental behaviour.
 */
const COLD_LOAD_ALLOWANCE = 3;

async function elkRuns(page: Page): Promise<number> {
	return (await readDiagnostics(page)).cumulative.elkRuns ?? 0;
}

test('rapid collapse presses cost one ELK layout each', async ({ page, context }) => {
	test.setTimeout(240_000);

	await signIn(context);
	await enablePerfInstrumentation(page);

	await page.goto('/?view=L2Physical#topology');
	await page.waitForSelector('.svelte-flow__node', { timeout: 60_000 });
	await waitForStableLayout(page);
	await page.locator('.svelte-flow').click({ position: { x: 5, y: 5 } });

	const settled = await readDiagnostics(page);
	const baselineElk = settled.cumulative.elkRuns ?? 0;

	// Fire without waiting: the point is that presses overtake the run in flight. Waiting for each
	// layout would test the case that never failed.
	for (let i = 0; i < RAPID_PRESSES; i++) {
		await page.keyboard.press('[');
		await page.waitForTimeout(250);
	}
	await waitForStableLayout(page);

	const after = await readDiagnostics(page);
	const layouts = (after.cumulative.elkRuns ?? 0) - baselineElk;
	const superseded = after.cumulative.runsSuperseded ?? 0;

	console.log(
		`presses=${RAPID_PRESSES} elkRuns=${layouts} superseded=${superseded} ` +
			`peakUsedHeap=${after.cumulative.peakUsedJSHeapMb}MB ` +
			`peakTotalHeap=${after.cumulative.peakTotalJSHeapMb}MB ` +
			`retainedHeap=${after.cumulative.usedJSHeapMb}MB`
	);

	expect(
		layouts,
		`${RAPID_PRESSES} presses caused ${layouts} ELK layouts — presses are overtaking runs that ` +
			`are not being cancelled, so each one pays for a layout it then discards`
	).toBeLessThanOrEqual(RAPID_PRESSES + COLD_LOAD_ALLOWANCE);

	// Cancellation only fires when a press genuinely overtakes a run. On a machine fast enough that
	// every layout finishes inside 250ms there is nothing to cancel and nothing to assert, so this
	// reports rather than fails — the count above is the real guard either way.
	if (superseded === 0) {
		console.log('no run was overtaken; layouts completed faster than presses arrived');
	}
});

test('a single collapse press does not hold both ELK graphs at once', async ({ page, context }) => {
	test.setTimeout(240_000);

	await signIn(context);
	await enablePerfInstrumentation(page);

	await page.goto('/?view=L2Physical#topology');
	await page.waitForSelector('.svelte-flow__node', { timeout: 60_000 });
	await waitForStableLayout(page);
	await page.locator('.svelte-flow').click({ position: { x: 5, y: 5 } });

	const before = await readDiagnostics(page);
	const baselineElk = before.cumulative.elkRuns ?? 0;

	// One press, allowed to finish: isolates a single layout's cost from any queueing effect.
	await page.keyboard.press('[');
	await waitForStableLayout(page);

	const after = await readDiagnostics(page);
	const layouts = (after.cumulative.elkRuns ?? 0) - baselineElk;
	const peak = after.cumulative.peakUsedJSHeapMb;
	const retained = after.cumulative.usedJSHeapMb;

	console.log(
		`single press: elkRuns=${layouts} peakUsedHeap=${peak}MB retained=${retained}MB ` +
			`peakTotalHeap=${after.cumulative.peakTotalJSHeapMb}MB`
	);

	expect(layouts, 'one press should cause one ELK layout').toBeLessThanOrEqual(2);

	// The ratio rather than the absolute: a pass-1 graph left reachable through pass 2 shows up as
	// a peak far above what the view actually retains, and that relationship holds across machines
	// where the megabyte figures do not. Chrome-only — `performance.memory` is absent elsewhere, in
	// which case there is nothing to compare and the check is skipped.
	if (peak !== undefined && retained !== undefined && retained > 0) {
		expect(
			peak / retained,
			`peak heap ${peak}MB is ${(peak / retained).toFixed(1)}x what the view retains ` +
				`(${retained}MB) — the layout is holding more than one graph at a time`
		).toBeLessThan(6);
	}
});
