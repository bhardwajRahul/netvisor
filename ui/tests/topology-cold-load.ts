import { test, expect, type BrowserContext, type Page } from '@playwright/test';
import { waitForStableLayout, enablePerfInstrumentation } from '../tests-support/topology-harness';

/**
 * The topology cold-loads, once, on graphs either side of the level-of-detail threshold.
 *
 * Cheap and blunt on purpose. `loadInProgress` is read and written by `triggerLoad`, which an
 * `$effect` calls — so making that flag a rune makes the write re-invalidate the effect and the
 * page hangs before it ever lays out, on a graph of any size. That shipped once. The signal is
 * `pipelineRuns`: a healthy cold load is one, a loop climbs without bound.
 *
 * Also covers the size gate from the outside — the small graph must come up with no badge at all,
 * because simplifying a graph that costs nothing to draw is a loss with no compensating gain.
 *
 *   SESSION_ID=<session> NETWORK_ID=<large> SMALL_NETWORK_ID=<small> \
 *     npx playwright test tests/topology-cold-load.ts
 */
const KEY = 'scanopy_topology_selected_network_id';

async function load(page: Page, context: BrowserContext, networkId: string, label: string) {
	await context.addCookies([
		{ name: 'session_id', value: process.env.SESSION_ID ?? '', domain: 'localhost', path: '/' }
	]);
	await enablePerfInstrumentation(page);
	await page.addInitScript(
		([k, n]) => {
			if (n) localStorage.setItem(k, n);
		},
		[KEY, networkId]
	);
	const started = Date.now();
	await page.goto('/?view=L2Physical#topology');
	await page.waitForSelector('.svelte-flow__node', { timeout: 120_000 });
	await waitForStableLayout(page, 120_000);
	const s = await page.evaluate(() => ({
		nodes: document.querySelectorAll('.svelte-flow__node').length,
		badge: document.querySelectorAll('.detail-hidden-badge').length,
		badgeText:
			document.querySelector('.detail-hidden-badge')?.textContent?.replace(/\s+/g, ' ').trim() ??
			'',
		runs:
			(
				window as unknown as { __scanopyTopologyPerf?: { snapshot: () => { runs: number } } }
			).__scanopyTopologyPerf?.snapshot()?.runs ?? 0
	}));
	console.log(
		`${label.padEnd(8)} settled in ${((Date.now() - started) / 1000).toFixed(1)}s  nodes=${s.nodes}  pipelineRuns=${s.runs}  badge=${s.badge} "${s.badgeText}"`
	);
	return s;
}

test('small graph loads and is never simplified', async ({ page, context }) => {
	test.skip(!process.env.SMALL_NETWORK_ID, 'needs SMALL_NETWORK_ID');
	test.setTimeout(300_000);
	const s = await load(page, context, process.env.SMALL_NETWORK_ID ?? '', 'small');
	expect(s.nodes, 'nothing rendered').toBeGreaterThan(0);
	// A loop shows up as runs climbing without bound; a healthy cold load is a handful.
	expect(s.runs, `pipeline ran ${s.runs} times — runaway`).toBeLessThan(20);
	expect(s.badge, 'small graph should never say it is simplified').toBe(0);
});

test('large graph loads', async ({ page, context }) => {
	test.skip(!process.env.NETWORK_ID, 'needs NETWORK_ID');
	test.setTimeout(300_000);
	const s = await load(page, context, process.env.NETWORK_ID ?? '', 'large');
	expect(s.nodes).toBeGreaterThan(0);
	expect(s.runs, `pipeline ran ${s.runs} times — runaway`).toBeLessThan(20);
});
