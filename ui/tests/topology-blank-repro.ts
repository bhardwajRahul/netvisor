import { test, expect, type Page, type BrowserContext } from '@playwright/test';
import { waitForStableLayout, enablePerfInstrumentation } from '../tests-support/topology-harness';

/**
 * The two faults behind a customer's blank L2 canvas, driven against a real browser.
 *
 * Both were established by reading the renderer against the customer's diagnostics export; this is
 * the part that watches them happen. Runs with culling **on**, which is not optional — culling is
 * the mechanism in both faults, and every DOM-derived measurement here is meaningless without it.
 *
 *  1. **A fit that never runs.** `fitView()` only queues; the fit is applied from the
 *     `nodesInitialized` derived, false while any node lacks `measured`, and the sole fallback is
 *     `updateNodeInternals`, which needs a node to mount. A transform left over from a previous
 *     view can select nothing, so nothing mounts, nothing measures, and the fit never fires.
 *  2. **A fit clamped at the zoom floor.** `minZoom` was a fixed 0.1 and `getViewportForBounds`
 *     clamps to it, so a graph needing 0.03 was centred and shown at a third of its width.
 *
 * Needs a live dev stack, SESSION_ID from a logged-in browser, and the large synthetic estate:
 *
 *   psql ... -v network_id=<uuid> -v hosts_per_switch=185 < backend/scripts/seed-l2-perf.sql
 *   SESSION_ID=<session> NETWORK_ID=<uuid> npx playwright test tests/topology-blank-repro.ts
 */

const SELECTED_NETWORK_KEY = 'scanopy_topology_selected_network_id';

/** Below this the culling gate is off and neither fault can occur. */
const CULLING_THRESHOLD = 150;

interface Reading {
	label: string;
	view: string;
	level: number | null;
	storeNodes: number;
	mounted: number;
	transform: string | null;
	zoom: number | null;
	/** Whether any mounted node's box meets the pane. False with nodes in the store is the fault. */
	intersectsPane: boolean;
	fitZoom: { required: number; applied: number; clampedAtFloor: boolean } | null;
	blank: string | null;
}

async function setup(page: Page, context: BrowserContext) {
	await context.addCookies([
		{ name: 'session_id', value: process.env.SESSION_ID ?? '', domain: 'localhost', path: '/' }
	]);
	await enablePerfInstrumentation(page);
	await page.addInitScript(
		([key, networkId]) => {
			// Pick the synthetic estate before any app code runs, so the first load is already the
			// large graph rather than a small one that never crosses the culling threshold.
			if (networkId) localStorage.setItem(key, networkId);
		},
		[SELECTED_NETWORK_KEY, process.env.NETWORK_ID ?? '']
	);
	await page.goto('/?view=L2Physical#topology');
	await page.waitForSelector('.svelte-flow__node', { timeout: 180_000 });
	await waitForStableLayout(page, 180_000);
	await page.locator('.svelte-flow').click({ position: { x: 5, y: 5 } });
}

/**
 * Read the viewport and what is drawn in it.
 *
 * `intersectsPane` and `mounted` are read from the DOM rather than from the diagnostics sample so
 * the measurement does not depend on the code under test. `fitZoom` does come from the diagnostic,
 * because the zoom a fit *wanted* is not observable any other way.
 */
async function read(page: Page, label: string): Promise<Reading> {
	return page.evaluate((readingLabel) => {
		const viewport = document.querySelector('.svelte-flow__viewport') as HTMLElement | null;
		const pane = document.querySelector('.svelte-flow');
		const paneRect = pane?.getBoundingClientRect();
		const els = Array.from(document.querySelectorAll('.svelte-flow__node')) as HTMLElement[];

		let intersects = false;
		for (const el of els) {
			const r = el.getBoundingClientRect();
			if (
				paneRect &&
				r.right > paneRect.left &&
				r.left < paneRect.right &&
				r.bottom > paneRect.top &&
				r.top < paneRect.bottom
			) {
				intersects = true;
				break;
			}
		}

		const transform = viewport?.style.transform ?? null;
		const scale = transform?.match(/scale\(([\d.]+)\)/);

		const diag = (
			window as unknown as {
				scanopyTopologyDiagnostics?: (o?: { download?: boolean }) => {
					samples: {
						view: string;
						store: { nodes: number };
						collapse: { level: number | null };
						fitZoom: { required: number; applied: number; clampedAtFloor: boolean } | null;
						blank: string | null;
					}[];
				};
			}
		).scanopyTopologyDiagnostics;
		const latest = diag?.({ download: false }).samples.at(-1);

		return {
			label: readingLabel,
			view: latest?.view ?? 'unknown',
			level: latest?.collapse.level ?? null,
			storeNodes: latest?.store.nodes ?? 0,
			mounted: els.length,
			transform,
			zoom: scale ? Number(scale[1]) : null,
			intersectsPane: intersects,
			fitZoom: latest?.fitZoom ?? null,
			blank: latest?.blank ?? null
		};
	}, label);
}

function report(readings: Reading[]) {
	console.log(
		'\nlabel                     view         lvl  store  mounted  zoom      required  clamped  onPane  blank'
	);
	for (const r of readings) {
		console.log(
			[
				r.label.padEnd(25),
				(r.view ?? '').padEnd(12),
				String(r.level ?? '-').padEnd(4),
				String(r.storeNodes).padStart(5),
				String(r.mounted).padStart(8),
				(r.zoom?.toFixed(4) ?? '-').padStart(9),
				(r.fitZoom?.required.toFixed(4) ?? '-').padStart(9),
				String(r.fitZoom?.clampedAtFloor ?? '-').padStart(8),
				String(r.intersectsPane).padStart(7),
				` ${r.blank ?? '-'}`
			].join(' ')
		);
	}
	console.log();
}

/**
 * Switch perspective through the app's own picker.
 *
 * Not a navigation: reloading would reset the viewport, and a viewport carried *across* the switch
 * is the precondition for fault 1. The control is a custom `RichSelect`, so this clicks the
 * trigger and then the option by its label.
 */
async function switchView(page: Page, label: string) {
	await page
		.locator('button.select-trigger')
		.filter({ hasText: /Physical|Logical|Workloads|Applications/ })
		.first()
		.click();
	await page.getByRole('button', { name: label, exact: false }).last().click();
	await page.waitForTimeout(1500);
	await waitForStableLayout(page, 180_000).catch(() => {});
}

test.setTimeout(600_000);

test('the collapse ladder always leaves the graph on screen', async ({ page, context }) => {
	await setup(page, context);
	const readings: Reading[] = [];
	readings.push(await read(page, 'initial'));

	expect(
		readings[0].storeNodes,
		'estate too small for culling to engage — reseed with more hosts'
	).toBeGreaterThan(CULLING_THRESHOLD);

	// Down the ladder and back up. Each rung rewrites the whole node set with coordinates unrelated
	// to the ones the current transform was computed for, which is the precondition for fault 1.
	for (const [key, direction] of [
		['[', 'collapse'],
		['[', 'collapse'],
		['[', 'collapse'],
		[']', 'expand'],
		[']', 'expand'],
		[']', 'expand']
	] as const) {
		await page.keyboard.press(key);
		await waitForStableLayout(page, 180_000);
		readings.push(await read(page, `${direction} -> level`));
	}

	report(readings);

	for (const r of readings) {
		expect(r.mounted, `${r.label}: nothing mounted`).toBeGreaterThan(0);
		expect(r.intersectsPane, `${r.label}: graph is off the pane`).toBe(true);
		expect(r.blank, `${r.label}: canvas classified blank`).toBeNull();
	}
});

test('a view switch does not strand the viewport', async ({ page, context }) => {
	await setup(page, context);
	const readings: Reading[] = [];
	readings.push(await read(page, 'L2 initial'));

	// L3 Logical then back. The two views' coordinate spaces are unrelated — `prepare.ts` drops the
	// previous view's sizes and positions — so on each switch the standing transform points
	// somewhere the incoming graph is not. Before the fix, the queued fit could never resolve from
	// there, because resolving it required a node to mount and no node was selected.
	for (const [view, label] of [
		['L3Logical', 'L3 Logical'],
		['L2Physical', 'L2 Physical']
	] as const) {
		await switchView(page, label);
		readings.push(await read(page, `switch -> ${view}`));
	}

	report(readings);

	const back = readings.at(-1)!;
	expect(back.view).toBe('L2Physical');
	expect(back.mounted, 'nothing mounted after switching back').toBeGreaterThan(0);
	expect(back.intersectsPane, 'graph is off the pane after switching back').toBe(true);
});

test('F fits a graph too large for the old floor', async ({ page, context }) => {
	await setup(page, context);

	// Expand fully: the largest the graph gets, and the level the fault was reported at.
	for (let i = 0; i < 4; i++) {
		await page.keyboard.press(']');
		await waitForStableLayout(page, 180_000);
	}
	const expanded = await read(page, 'level 4');

	// Pan somewhere the graph is not, so F has real work to do.
	await page.mouse.move(600, 400);
	await page.mouse.down();
	await page.mouse.move(1400, 900, { steps: 10 });
	await page.mouse.up();
	await page.waitForTimeout(600);
	const panned = await read(page, 'after pan');

	await page.keyboard.press('f');
	await page.waitForTimeout(1500);
	const fitted = await read(page, 'after F');

	report([expanded, panned, fitted]);

	expect(fitted.transform, 'F did not move the viewport').not.toBe(panned.transform);
	expect(fitted.mounted, 'nothing mounted after F').toBeGreaterThan(0);
	expect(fitted.intersectsPane, 'graph off the pane after F').toBe(true);

	// The heart of fault 2: this graph needs a zoom the old fixed floor forbade, and the fit must
	// now actually reach it rather than stopping at 0.1.
	if (fitted.fitZoom && fitted.fitZoom.required < 0.1) {
		expect(
			fitted.zoom,
			`fit clamped: needed ${fitted.fitZoom.required}, applied ${fitted.zoom}`
		).toBeLessThan(0.1);
	}
});

/**
 * What fitting the whole graph costs, measured rather than assumed.
 *
 * Letting a fit reach the zoom the graph needs also means the viewport now *contains* the graph,
 * and culling mounts what the viewport contains. The old fixed floor was, incidentally, culling
 * most of a large estate by keeping the view too zoomed-in to hold it. That is not a defence of it
 * — the operator could not see their network — but the fix moves DOM cost, and on the estate class
 * that was reported running out of memory, moved cost is worth a number rather than a shrug.
 *
 * Measured on the 5,936-node synthetic estate, level 4:
 *
 *   floor      zoom    mounted   domInNodes   domTotal
 *   fixed 0.1  0.1000     1,510        7,675     27,401
 *   derived    0.0108     5,936       31,168     63,820
 *
 * Reported, not asserted: the right ceiling is a product decision, and a threshold invented here
 * would fail on the next estate rather than tell anyone anything.
 */
test('cost of a fitted graph at every collapse level', async ({ page, context }) => {
	await setup(page, context);

	// Expand to the top, then read every rung on the way down, so each reading follows a fit
	// rather than a pan.
	for (let i = 0; i < 4; i++) {
		await page.keyboard.press(']');
		await waitForStableLayout(page, 180_000);
	}

	const rows: string[] = [];
	for (let i = 0; i < 4; i++) {
		const r = await page.evaluate(() => {
			const viewport = document.querySelector('.svelte-flow__viewport') as HTMLElement | null;
			const scale = viewport?.style.transform?.match(/scale\(([\d.]+)\)/);
			const diag = (
				window as unknown as {
					scanopyTopologyDiagnostics?: (o?: { download?: boolean }) => {
						samples: {
							store: { nodes: number };
							mounted: number;
							dom: { total: number; inNodes: number };
							collapse: { level: number | null };
							fitZoom: { required: number; clampedAtFloor: boolean } | null;
						}[];
						cumulative: { usedJSHeapMb?: number; peakDomInNodes: number };
					};
				}
			).scanopyTopologyDiagnostics;
			const reportData = diag?.({ download: false });
			const s = reportData?.samples.at(-1);
			return {
				level: s?.collapse.level ?? null,
				store: s?.store.nodes ?? 0,
				mounted: s?.mounted ?? 0,
				domInNodes: s?.dom.inNodes ?? 0,
				domTotal: s?.dom.total ?? 0,
				zoom: scale ? Number(scale[1]) : null,
				required: s?.fitZoom?.required ?? null,
				heapMb: reportData?.cumulative.usedJSHeapMb ?? null
			};
		});
		rows.push(
			[
				`lvl ${r.level}`.padEnd(6),
				`store ${String(r.store).padStart(5)}`,
				`mounted ${String(r.mounted).padStart(5)}`,
				`domInNodes ${String(r.domInNodes).padStart(6)}`,
				`domTotal ${String(r.domTotal).padStart(6)}`,
				`zoom ${(r.zoom?.toFixed(4) ?? '-').padStart(7)}`,
				`required ${(r.required?.toFixed(4) ?? '-').padStart(7)}`,
				`heapMb ${String(r.heapMb ?? '-').padStart(6)}`
			].join('  ')
		);
		if (i < 3) {
			await page.keyboard.press('[');
			await waitForStableLayout(page, 180_000);
		}
	}

	console.log('\n' + rows.join('\n') + '\n');
});
