import { test, expect, type Page, type BrowserContext } from '@playwright/test';

/**
 * Collapse-ladder behaviour for the topology views.
 *
 * Asserts the invariants that must hold at every level, so the behaviour is pinned
 * rather than merely observed.
 *
 * Two faults this covers, both found on the 440-host `seed-l2-perf.sql` dataset:
 *
 *  - Collapsing used to *grow* the graph (415 -> 1234 nodes on the first press).
 *    `computeCollapsedForLevel` derived its set purely from container-type metadata,
 *    with no knowledge of the scale-collapse set applied at >=300 Element nodes, so
 *    the first press discarded scale-collapse and expanded every host container.
 *  - The viewport never refit after a collapse (`fit-view` stuck at 1 for a whole
 *    session), because the refit was a fixed 100ms timer that fired before the
 *    relayout landed and the post-layout gate excludes collapse changes.
 *
 * Most tests here run with culling disabled. With culling on, off-screen nodes are
 * absent from the DOM, so any DOM-derived count or on-screen fraction only ever sees
 * survivors and is structurally blind to a graph that has left the viewport — the
 * measurement mistake that made this bug look like a viewport problem at first.
 *
 * The exception is the last test, which turns culling on deliberately. Everything
 * above measures *where the graph is*; that one measures *whether it is drawn*, and
 * culling is precisely the mechanism that can stop it being drawn. Excluding it left
 * the ladder's most-reported symptom — a blank canvas at levels 3 and 4, with the
 * minimap still showing content — outside the reach of every assertion here.
 *
 * Requires a live dev stack, a SESSION_ID from a logged-in browser, and a seeded
 * dataset. Which dataset decides which tests are meaningful, so each test asserts
 * only what holds for the size it finds and skips otherwise:
 *
 *   SESSION_ID=<session> npx playwright test tests/l2-collapse.spec.ts
 */

/** Above this many Element nodes the loader starts containers collapsed. */
const SCALE_COLLAPSE_ELEMENTS = 300;

interface Sample {
	nodes: number;
	edges: number;
	level: number | null;
	fitViewCount: number;
	fractionOnScreen: number;
	/** Rendered node rects that overlap another by more than a trivial margin. */
	overlapping: number;
	elementCards: number;
}

async function setup(
	page: Page,
	context: BrowserContext,
	view = 'L2Physical',
	{ cull = false }: { cull?: boolean } = {}
) {
	await context.addCookies([
		{ name: 'session_id', value: process.env.SESSION_ID ?? '', domain: 'localhost', path: '/' }
	]);
	await page.addInitScript((noCull) => {
		(window as unknown as { __topoPerf: boolean }).__topoPerf = true;
		(window as unknown as { __topoNoCull: boolean }).__topoNoCull = noCull;
	}, !cull);
	await page.goto(`/?view=${view}#topology`);
	await page.waitForSelector('.svelte-flow__node', { timeout: 120_000 });
	await waitForStableLayout(page);
	// Focus the pane so the '[' / ']' shortcuts reach the viewer's key handler.
	await page.locator('.svelte-flow').click({ position: { x: 5, y: 5 } });
}

async function sample(page: Page): Promise<Sample> {
	return page.evaluate(() => {
		const pane = document.querySelector('.svelte-flow');
		const paneRect = pane?.getBoundingClientRect();
		const els = Array.from(document.querySelectorAll('.svelte-flow__node')) as HTMLElement[];

		let onScreen = 0;
		const rects = els.map((el) => el.getBoundingClientRect());
		for (const r of rects) {
			if (
				paneRect &&
				r.right > paneRect.left &&
				r.left < paneRect.right &&
				r.bottom > paneRect.top &&
				r.top < paneRect.bottom
			) {
				onScreen += 1;
			}
		}

		// Overlap between *sibling* cards is the visible symptom of ELK being handed
		// wrong sizes. Containers legitimately contain their children, so only compare
		// leaf element cards, and allow a small margin for borders and shadows.
		// SvelteFlow tags each node with `svelte-flow__node-<NodeType>`.
		const leafEls = Array.from(
			document.querySelectorAll('.svelte-flow__node-Element')
		) as HTMLElement[];
		const leafRects = leafEls.map((el) => el.getBoundingClientRect());
		// Measured as a *fraction of card area*, not a pixel margin. A fixed margin is in screen
		// pixels, so at the zoom a fully expanded graph forces (0.1, the floor) a 4px allowance
		// is ~40 graph px and counts merely adjacent cards as overlapping. Area is scale-free.
		const MIN_OVERLAP_FRACTION = 0.25;
		let overlapping = 0;
		for (let i = 0; i < leafRects.length; i++) {
			for (let j = i + 1; j < leafRects.length; j++) {
				const a = leafRects[i];
				const b = leafRects[j];
				const w = Math.min(a.right, b.right) - Math.max(a.left, b.left);
				const h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
				if (w <= 0 || h <= 0) continue;
				const smaller = Math.min(a.width * a.height, b.width * b.height);
				if (smaller > 0 && (w * h) / smaller > MIN_OVERLAP_FRACTION) {
					overlapping += 1;
					break;
				}
			}
		}

		const perf = (
			window as unknown as {
				__scanopyTopologyPerf?: { snapshot: () => { counts: Record<string, number> } };
			}
		).__scanopyTopologyPerf;

		const level =
			Array.from(document.querySelectorAll('div'))
				.filter((d) => (d as HTMLElement).style.width === '58px')
				.map((d) => (d as HTMLElement).innerText.trim())
				.find((t) => /^[1-4]$/.test(t)) ?? null;

		return {
			nodes: els.length,
			edges: document.querySelectorAll('.svelte-flow__edge').length,
			level: level ? Number(level) : null,
			fitViewCount: perf?.snapshot().counts['fit-view'] ?? 0,
			fractionOnScreen: els.length ? onScreen / els.length : 0,
			overlapping,
			elementCards: leafEls.length
		};
	});
}

async function waitForStableLayout(page: Page, timeoutMs = 90_000): Promise<void> {
	const started = Date.now();
	let previous = '';
	let stable = 0;
	while (Date.now() - started < timeoutMs) {
		const s = await page.evaluate(() => {
			const api = (
				window as unknown as {
					__scanopyTopologyPerf?: { snapshot: () => { runStartedAt: number | null } };
				}
			).__scanopyTopologyPerf;
			return {
				running: api ? api.snapshot().runStartedAt !== null : false,
				fingerprint: Array.from(document.querySelectorAll('.svelte-flow__node'))
					.map((el) => `${(el as HTMLElement).dataset.id}:${(el as HTMLElement).style.transform}`)
					.sort()
					.join('|')
			};
		});
		if (!s.running && s.fingerprint !== '' && s.fingerprint === previous) {
			if (++stable >= 2) return;
		} else {
			stable = 0;
		}
		previous = s.fingerprint;
		await page.waitForTimeout(250);
	}
	throw new Error(`Layout did not settle within ${timeoutMs}ms`);
}

async function press(page: Page, key: '[' | ']'): Promise<Sample> {
	await page.keyboard.press(key);
	await waitForStableLayout(page);
	await page.waitForTimeout(800); // let any late refit land
	return sample(page);
}

/**
 * Press until nothing further changes, returning a sample after each press.
 *
 * Stops on "no change" rather than on a target level number: a view can have fewer distinct
 * states than the ladder has rungs, so the button runs out before the number does.
 */
async function walkTo(page: Page, key: '[' | ']', maxPresses = 6): Promise<Sample[]> {
	const seen: Sample[] = [];
	let previous = await sample(page);
	for (let i = 0; i < maxPresses; i++) {
		const s = await press(page, key);
		if (s.nodes === previous.nodes && s.level === previous.level) break;
		seen.push(s);
		previous = s;
	}
	return seen;
}

test.describe('collapse ladder', () => {
	test.describe.configure({ mode: 'serial' });

	test('collapsing never grows the graph', async ({ page, context }) => {
		test.setTimeout(300_000);
		await setup(page, context);

		// A large graph opens already collapsed, so expand out first — otherwise there is
		// nothing to collapse and the assertion passes without exercising anything.
		await walkTo(page, ']');
		const start = await sample(page);
		const steps = await walkTo(page, '[');
		expect(
			steps.length,
			'collapsing should do something from a fully expanded graph'
		).toBeGreaterThan(0);

		const counts = [start.nodes, ...steps.map((s) => s.nodes)];
		for (let i = 1; i < counts.length; i++) {
			expect(
				counts[i],
				`press ${i} grew the graph from ${counts[i - 1]} to ${counts[i]} nodes`
			).toBeLessThanOrEqual(counts[i - 1]);
		}

		// Once the walk stops, the button must agree there is nowhere left to go.
		const further = await press(page, '[');
		expect(further.nodes, 'a press past the end changed the graph').toBe(steps.at(-1)!.nodes);
	});

	test('expanding never shrinks the graph', async ({ page, context }) => {
		test.setTimeout(300_000);
		await setup(page, context);

		await walkTo(page, '[');
		const bottom = await sample(page);
		const steps = await walkTo(page, ']');

		const counts = [bottom.nodes, ...steps.map((s) => s.nodes)];
		for (let i = 1; i < counts.length; i++) {
			expect(
				counts[i],
				`expand ${i} shrank the graph from ${counts[i - 1]} to ${counts[i]} nodes`
			).toBeGreaterThanOrEqual(counts[i - 1]);
		}
		// Expanding can always reach the fully expanded end.
		expect(steps.at(-1)!.level, 'expanding should reach level 4').toBe(4);
	});

	test('every level change refits the viewport and keeps the graph on screen', async ({
		page,
		context
	}) => {
		test.setTimeout(300_000);
		await setup(page, context);

		let previous = await sample(page);
		const all = [...(await walkTo(page, '[')), ...(await walkTo(page, ']'))];

		for (const [i, s] of all.entries()) {
			if (s.level !== previous.level) {
				expect(
					s.fitViewCount,
					`press ${i + 1} changed level ${previous.level} -> ${s.level} without refitting`
				).toBeGreaterThan(previous.fitViewCount);
			}
			// Reachable, not necessarily wholly visible. A fully expanded graph of this size
			// needs a zoom below the flow's `minZoom={0.1}` floor to fit, so fitView clamps and
			// the graph legitimately overflows — asserting "most of it is on screen" would be
			// asserting something the zoom floor forbids. What must hold is that the canvas is
			// never blank and the viewport sits over the graph rather than off beside it.
			expect(s.nodes, `press ${i + 1} emptied the canvas`).toBeGreaterThan(0);
			expect(
				s.fractionOnScreen,
				`press ${i + 1} left the viewport off the graph entirely`
			).toBeGreaterThan(0);
			previous = s;
		}
	});

	// Regression guard for zero-sized ELK children. At scale, containers start collapsed, so
	// their element cards are never mounted and never measured. The collapse-path cache fill in
	// `resolveNodeSizes` only counted *containers* as misses, so a partial size map survived; ELK
	// then fell back to the server's `node.size` (`Uxy::default()` = 0x0) and packed zero-sized
	// children a spacing apart, while the DOM rendered them 250x54 — ~200 of ~1631 cards
	// overlapping a sibling by >80%, on exactly the 8 containers with 50 children each.
	test('level 4 lays element cards out without overlapping them', async ({ page, context }) => {
		test.setTimeout(300_000);
		await setup(page, context);

		// Fully expanded is where ELK has the most to place and where handing it
		// placeholder sizes showed up as overlapping cards.
		const steps = await walkTo(page, ']');
		const top = steps.at(-1) ?? (await sample(page));
		expect(top.level).toBe(4);
		test.skip(top.elementCards === 0, 'no element cards rendered in this view');

		expect(
			top.overlapping,
			`${top.overlapping} of ${top.elementCards} element cards overlap a sibling`
		).toBe(0);
	});

	test('a small topology opens expanded and is untouched by scale collapse', async ({
		page,
		context
	}) => {
		test.setTimeout(300_000);
		await setup(page, context);

		const start = await sample(page);

		// The dataset's size has to be measured at level 4: at scale the view opens
		// with every container collapsed, so the count of *rendered* element cards is
		// 0 there and would read as a small topology.
		const expanded = (await walkTo(page, ']')).at(-1) ?? start;
		test.skip(
			expanded.elementCards >= SCALE_COLLAPSE_ELEMENTS,
			`dataset renders ${expanded.elementCards} element cards at level 4, at or above the ${SCALE_COLLAPSE_ELEMENTS} scale-collapse threshold — reseed smaller to exercise this`
		);

		// Below the threshold scale collapse is inert, so reloading opens the view at
		// the same place the ladder's top puts it, and stepping down visibly changes it.
		await setup(page, context);
		const reopened = await sample(page);
		expect(reopened.level, 'a small topology should not open scale-collapsed').toBeGreaterThan(2);
		const first = await press(page, '[');
		expect(first.nodes, 'collapsing a small topology should change the graph').toBeLessThan(
			reopened.nodes
		);
	});

	/**
	 * The customer-reported symptom, and the one every other test here is blind to.
	 *
	 * Reported on a ~300-host network in Chromium: level 1 draws, level 2 draws, level 3 shows
	 * content in the minimap and nothing on the canvas, and any zoom, minimap click, or step to
	 * level 4 locks that in. Pressing F does nothing. Stepping back down to level 1 restores it.
	 *
	 * Culling is on above 150 rendered elements (`pipeline/render-mode.ts`), which is why level 1
	 * — below the threshold — is the only rung that recovers. So this walks the ladder with
	 * culling *on*, where `sample().nodes` counts what SvelteFlow actually mounted rather than
	 * what the store holds. A blank canvas is `nodes === 0` while the level indicator still
	 * reports a level, which is exactly the state the minimap keeps drawing through.
	 */
	test('the canvas is never blank at any level with culling on', async ({ page, context }) => {
		test.setTimeout(300_000);
		await setup(page, context, 'L2Physical', { cull: true });

		const start = await sample(page);
		test.skip(
			start.nodes < 150,
			`only ${start.nodes} nodes rendered — below the culling threshold, so this dataset cannot exercise it`
		);

		const all = [start, ...(await walkTo(page, ']')), ...(await walkTo(page, '['))];
		for (const [i, s] of all.entries()) {
			expect(s.nodes, `step ${i} left the canvas blank at level ${s.level}`).toBeGreaterThan(0);
		}

		// Interacting with the viewport re-evaluates which nodes are inside it, and is what
		// locked the blank state in for the customer.
		await page.mouse.move(400, 400);
		await page.mouse.wheel(0, -300);
		await waitForStableLayout(page);
		const zoomed = await sample(page);
		expect(zoomed.nodes, 'zooming emptied the canvas').toBeGreaterThan(0);

		// And F must always be able to bring the graph back — it is the only escape hatch a user
		// has, and "pressing F does nothing" was half the report.
		await page.keyboard.press('f');
		await waitForStableLayout(page);
		const refit = await sample(page);
		expect(refit.nodes, 'fit view did not restore the graph').toBeGreaterThan(0);
		expect(refit.fractionOnScreen, 'fit view left the viewport off the graph').toBeGreaterThan(0);
	});
});
