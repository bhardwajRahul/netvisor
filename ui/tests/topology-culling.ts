import { test, expect } from '@playwright/test';
import {
	enablePerfInstrumentation,
	readDiagnostics,
	signIn,
	waitForStableLayout
} from '../tests-support/topology-harness';

/**
 * Viewport culling must keep the mounted node count off the graph's node count.
 *
 * A customer's L2 view held 17,236 nodes and mounted all of them, which exhausted browser memory
 * in Firefox. The gate reported itself on throughout: what failed was per-node, in SvelteFlow's
 * `getNodesInside`, and no test could see it. `topology-culling.test.ts` covers the node-building
 * side without a browser; this covers the part that only a real render shows — that expanding a
 * container does not mount the graph it reveals.
 *
 * **Deliberately does not set `window.__topoNoCull`.** `topology-layout-eval.ts` sets it on
 * purpose, because scoring layout quality against a culled graph would score only what is on
 * screen — which also means that harness structurally cannot measure culling, and this one has to
 * exist separately.
 *
 * Prerequisites:
 *   1. `npm run dev` (Vite on :5173) plus a running backend.
 *   2. A seeded large L2 dataset — see `backend/scripts/seed-l2-perf.sql`.
 *   3. SESSION_ID from a logged-in browser session.
 *
 * Run (Firefox is the browser the fault was reported on; Chromium tolerates far more):
 *   SESSION_ID=<session> npx playwright test tests/topology-culling.ts --project=firefox
 */

/**
 * Ceiling on the fraction of the graph that may be mounted at once.
 *
 * The customer's working view culled 222 of 1,248 nodes, an 82% reduction, at a zoom where most
 * of the graph was off screen. Half is well clear of that and still fails outright on the
 * behaviour this guards: `mounted` tracking `store.nodes`.
 */
const MAX_MOUNTED_FRACTION = 0.5;

/** Below this many nodes culling is off by design (`CULLING_THRESHOLD_ELEMENTS`), so skip. */
const MIN_NODES_TO_ASSERT = 400;

test('L2 culling keeps the mounted set off the graph size', async ({ page, context }) => {
	test.setTimeout(180_000);

	await signIn(context);
	await enablePerfInstrumentation(page);

	await page.goto('/?view=L2Physical#topology');
	await page.waitForSelector('.svelte-flow__node', { timeout: 60_000 });
	await waitForStableLayout(page);

	const afterLoad = await readDiagnostics(page);
	const loaded = afterLoad.samples.at(-1);
	if (!loaded) throw new Error('diagnostics returned no samples');

	test.skip(
		loaded.store.nodes < MIN_NODES_TO_ASSERT,
		`only ${loaded.store.nodes} nodes — seed a larger dataset (backend/scripts/seed-l2-perf.sql)`
	);

	// Walk the collapse ladder to fully expanded. This is the customer's action, and the one that
	// introduces thousands of nodes that have never mounted — the case that defeats culling by way
	// of `forceInitialRender` rather than by way of the viewport test. `]` is step-expand; a large
	// graph opens scale-collapsed at level 1, so it takes several presses to reach level 4.
	await page.locator('.svelte-flow').click({ position: { x: 5, y: 5 } });
	for (let i = 0; i < 4; i++) {
		await page.keyboard.press(']');
		await page.waitForTimeout(500);
		await waitForStableLayout(page);
	}

	const afterExpand = await readDiagnostics(page);
	const expanded = afterExpand.samples.at(-1);
	if (!expanded) throw new Error('diagnostics returned no samples after expanding');

	console.log('\n=== Topology culling ===');
	console.log(`  Store nodes (loaded / expanded): ${loaded.store.nodes} / ${expanded.store.nodes}`);
	console.log(`  Mounted     (loaded / expanded): ${loaded.mounted} / ${expanded.mounted}`);
	console.log(`  Collapse level after expanding:  ${expanded.collapse.level}`);
	console.log(`  Cullable: ${JSON.stringify(expanded.cullable)}`);
	console.log(`  Cumulative: ${JSON.stringify(afterExpand.cumulative)}`);

	expect(
		expanded.culling.suppressedForTooling,
		'__topoNoCull is set — culling is not under test'
	).toBe(false);

	// The assertion the bug would have failed. Peak across the whole run, not the final count: a
	// transient full mount is what exhausts memory, and it is torn down before a point-in-time
	// sample would see it.
	const peakFraction = afterExpand.cumulative.peakMounted / afterExpand.cumulative.peakStoreNodes;
	expect(
		peakFraction,
		`peak mounted ${afterExpand.cumulative.peakMounted} of ${afterExpand.cumulative.peakStoreNodes} store nodes`
	).toBeLessThan(MAX_MOUNTED_FRACTION);

	// Culling can only work on nodes it can test. A graph that is fully mounted *and* fully
	// force-rendered is the exact failure; reporting it separately says which of the two
	// mechanisms regressed.
	expect(
		expanded.cullable?.forceRendered ?? 0,
		'every node is force-rendered — nodes are being built without measured sizes or handles'
	).toBeLessThan(expanded.store.nodes * MAX_MOUNTED_FRACTION);
});
