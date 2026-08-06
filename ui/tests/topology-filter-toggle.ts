import { test, expect, type Page } from '@playwright/test';
import {
	enablePerfInstrumentation,
	readDiagnostics,
	signIn,
	waitForStableLayout
} from '../tests-support/topology-harness';

/**
 * Toggling a filter must not re-measure the graph.
 *
 * The full measurement pass mounts every node — at 19,095 that is ~665MB and 5.5s. Four separate
 * attempts to stop it running on every filter toggle were wrong, each one a plausible inference
 * from adjacent counters: superseded runs discarding measurements, wholesale cache clearing, the
 * hide-state key, and finally the untyped hidden-entity set. The fault was always the same shape —
 * a cache key bundling things with different invalidation semantics, so the strictest member forced
 * the most expensive path for everything.
 *
 * That kind of regression is silent: the view still renders correctly, just at several times the
 * memory. Nothing but a count of full passes catches it, which is what this asserts.
 *
 * Prerequisites:
 *   1. `npm run dev` (Vite on :5173) plus a running backend.
 *   2. A seeded large L2 dataset, with unlinked ports *visible* — the filter has to have something
 *      substantial to hide or this measures nothing.
 *   3. SESSION_ID from a logged-in browser session.
 *
 * Run:
 *   SESSION_ID=<session> npx playwright test tests/topology-filter-toggle.ts --project=chromium
 */

/** Below this the graph is too small for a re-measure to cost anything worth guarding. */
const MIN_NODES_TO_ASSERT = 5_000;

const TOGGLES = 4;

/**
 * Quiet period required after a toggle before the next one.
 *
 * `waitForStableLayout` returns as soon as no run is in flight, which at 19,095 nodes is well before
 * the view has finished with itself: a corrective re-layout, a post-render size reconciliation or an
 * SSE-driven refresh all arrive afterwards, and those are runs too. Pressing again at that point
 * measures a half-settled view rather than the steady state.
 */
const SETTLE_MS = 30_000;

async function fullPassReasons(page: Page): Promise<Record<string, number>> {
	return page.evaluate(() => {
		const api = (
			window as unknown as {
				__scanopyTopologyPerf?: { snapshot: () => { counts: Record<string, number> } };
			}
		).__scanopyTopologyPerf;
		return Object.fromEntries(
			Object.entries(api?.snapshot().counts ?? {}).filter(([k]) => k.includes('full-reason'))
		);
	});
}

test('toggling a filter does not re-measure the graph', async ({ page, context }) => {
	test.setTimeout(1_800_000);

	await signIn(context);
	await enablePerfInstrumentation(page);
	await page.goto('/?view=L2Physical#topology');
	await page.waitForSelector('.svelte-flow__node', { timeout: 180_000 });
	await waitForStableLayout(page, 300_000);

	// Fully expand: the interesting state is the one holding every node.
	for (let i = 0; i < 5; i++) {
		await page.keyboard.press(']');
		await waitForStableLayout(page, 300_000);
	}

	const expand = page.getByRole('button', { name: /expand.*panel/i }).first();
	if (await expand.isVisible().catch(() => false)) await expand.click();
	await page.waitForTimeout(500);
	const filtersTab = page.getByRole('button', { name: /^Filters$/i }).first();
	if (await filtersTab.isVisible().catch(() => false)) await filtersTab.click();
	await page.waitForTimeout(800);

	const unlinked = page.getByText('Unlinked', { exact: true }).first();
	expect(
		await unlinked.count(),
		'no Unlinked filter chip — open the Filters tab on a seeded L2 dataset'
	).toBeGreaterThan(0);

	const before = await readDiagnostics(page);
	const nodes = before.samples.at(-1)?.store.nodes ?? 0;
	expect(
		nodes,
		`graph holds only ${nodes} nodes — re-enable unlinked ports, or a re-measure costs too little to guard`
	).toBeGreaterThanOrEqual(MIN_NODES_TO_ASSERT);
	const passesBefore = before.cumulative.fullMeasurePasses;

	for (let i = 0; i < TOGGLES; i++) {
		await unlinked.click({ timeout: 20_000 });
		await waitForStableLayout(page, 300_000);
		await page.waitForTimeout(SETTLE_MS);
		await waitForStableLayout(page, 300_000);
	}

	const after = await readDiagnostics(page);
	const added = after.cumulative.fullMeasurePasses - passesBefore;
	console.log(
		`toggles=${TOGGLES} nodes=${nodes} fullPasses ${passesBefore} -> ${after.cumulative.fullMeasurePasses} ` +
			`reasons=${JSON.stringify(await fullPassReasons(page))}`
	);

	// The measurement is already cached by this point; a filter that only removes nodes invalidates
	// none of it. Measured before the fix that prompted this: 12 passes for these same toggles.
	expect(
		added,
		`${TOGGLES} filter toggles caused ${added} full measurement passes — each mounts the whole ` +
			`graph. Check measure.full-reason:* for which cache was discarded.`
	).toBe(0);

	// A stale size shows up as a container drawn at the wrong size, so guard the fix as well as the cost.
	expect(after.samples.at(-1)?.degenerate?.containers ?? 0).toBe(0);
});
