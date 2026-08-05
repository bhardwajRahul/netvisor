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
 * Ceiling on the fraction of the graph that may be mounted while zoomed in.
 *
 * Asserted only after zooming, because mounting most of the graph is *correct* when most of the
 * graph is on screen — a fit-view at level 4 legitimately puts ~92% of the nodes in the viewport,
 * and an assertion that ignored the viewport would fail on healthy behaviour. The customer's
 * working view culled 222 of 1,248 (82%) at a zoom where most of the graph was off screen; half
 * is well clear of that and still fails outright on the behaviour this guards.
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
	//
	// Driven off the reported level rather than a fixed number of presses, and given a plain wait
	// between presses rather than a full settle: expanding runs a 350ms fade whose opacity changes
	// keep the node fingerprint moving, so insisting on two identical consecutive samples at every
	// rung is flaky for reasons that have nothing to do with what is being tested.
	await page.locator('.svelte-flow').click({ position: { x: 5, y: 5 } });
	let expanded = loaded;
	for (let press = 0; press < 6 && expanded.collapse.level !== 4; press++) {
		await page.keyboard.press(']');
		await page.waitForTimeout(4000);
		expanded = (await readDiagnostics(page)).samples.at(-1) ?? expanded;
	}

	expect(expanded.collapse.level, 'never reached the fully expanded level').toBe(4);
	await waitForStableLayout(page, 120_000);

	// Zoom in so the viewport genuinely covers a small part of the graph. Without this the graph is
	// fitted to the pane and nearly every node is legitimately on screen, so the mounted count says
	// nothing about whether culling works.
	//
	// Anchored on a node rather than the middle of the pane: the layout is sparse columns, so the
	// centre is often empty space, and zooming there mounts nothing — which satisfies a "few nodes
	// are mounted" assertion for entirely the wrong reason.
	const anchor = await page.locator('.svelte-flow__node').first().boundingBox();
	if (!anchor) throw new Error('no mounted node to zoom towards');
	await page.mouse.move(anchor.x + anchor.width / 2, anchor.y + anchor.height / 2);
	for (let i = 0; i < 5; i++) await page.mouse.wheel(0, -120);
	await page.waitForTimeout(1500);

	const afterZoom = await readDiagnostics(page);
	const zoomed = afterZoom.samples.at(-1);
	if (!zoomed) throw new Error('diagnostics returned no samples after zooming');

	console.log('\n=== Topology culling ===');
	console.log(`  Store nodes (loaded / expanded): ${loaded.store.nodes} / ${expanded.store.nodes}`);
	console.log(`  Mounted     (loaded / expanded): ${loaded.mounted} / ${expanded.mounted}`);
	console.log(`  Zoomed in   (mounted / store):   ${zoomed.mounted} / ${zoomed.store.nodes}`);
	console.log(`  Collapse level after expanding:  ${expanded.collapse.level}`);
	console.log(`  Cullable: ${JSON.stringify(zoomed.cullable)}`);
	console.log(`  Cumulative: ${JSON.stringify(afterZoom.cumulative)}`);

	expect(
		zoomed.culling.suppressedForTooling,
		'__topoNoCull is set — culling is not under test'
	).toBe(false);

	// Culling has to be *enabled* at this size. It regressed once by staying suspended after a
	// measurement pass, which no count-based check would have caught — every node was cullable and
	// nothing was culling them.
	expect(zoomed.culling.on, 'culling is off at a size well past its threshold').toBe(true);
	expect(
		zoomed.culling.measuring,
		'a measure pass is still marked active after the graph settled'
	).toBe(false);

	// Culling can only work on nodes it can test, so this says which of the two mechanisms
	// regressed: a force-rendered node skips the viewport check entirely.
	expect(
		zoomed.cullable?.forceRendered ?? 0,
		'nodes are force-rendered — they are being built without measured sizes or handles'
	).toBeLessThan(zoomed.store.nodes * 0.05);

	// Looking at something, so the ratio below means what it says. Zero mounted would satisfy any
	// upper bound while proving nothing.
	expect(
		zoomed.mounted,
		'zoomed onto empty canvas — the ratio below would be meaningless'
	).toBeGreaterThan(0);

	// The assertion the original bug would have failed: `mounted` tracking `store.nodes`.
	expect(
		zoomed.mounted / zoomed.store.nodes,
		`zoomed in, ${zoomed.mounted} of ${zoomed.store.nodes} nodes are mounted`
	).toBeLessThan(MAX_MOUNTED_FRACTION);
});
