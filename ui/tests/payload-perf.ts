import { test, expect, type Page } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

/**
 * Network-payload measurement.
 *
 * Companion to `topology-perf.ts`: that one measures render cost, this one
 * measures what the app downloads. It exists because the expensive thing in this
 * app was never a slow render — it was `useHostsQuery({ limit: 0 })`, an
 * unpaginated org-wide host list with every nested ip-address, port, service and
 * interface (~1.9MB on a 440-host estate), shared by query key across a dozen
 * consumers so that no single one could remove it.
 *
 * Two numbers matter and only the first was ever guessed at:
 *   - cold boot bytes, and
 *   - refetch bytes during a discovery scan. The discovery SSE stream invalidates
 *     entity keys on a 1s throttle for the whole scan, so any large *active*
 *     query is re-downloaded up to 60x/minute. Nobody had measured this.
 *
 * Prerequisites:
 *   1. `npm run dev` (Vite on :5173) plus a running backend.
 *   2. A seeded dataset with enough hosts to be meaningful (hundreds).
 *   3. SESSION_ID from a logged-in browser session.
 *
 * Run:
 *   SESSION_ID=<session> npx playwright test tests/payload-perf.ts
 *   SESSION_ID=<session> PERF_LABEL=baseline npx playwright test tests/payload-perf.ts
 *
 * To include the scan scenario, start a discovery in the UI, then run with
 * SCAN_SECONDS=30. It is opt-in because it needs a scan in flight.
 *
 * Results go to `tests/results/payload-perf.json` so runs are comparable across
 * commits. Tag runs with PERF_LABEL.
 */

const OUTPUT_PATH = resolve('tests/results/payload-perf.json');

interface RequestRecord {
	path: string;
	query: string;
	status: number;
	/** Decoded body size — what the browser parses. */
	bodyBytes: number;
	/** On-the-wire size, i.e. after gzip. */
	wireBytes: number;
}

interface PathTotals {
	requests: number;
	bodyBytes: number;
	wireBytes: number;
}

interface ScenarioReport {
	name: string;
	requests: number;
	totalBodyBytes: number;
	totalWireBytes: number;
	/** Aggregated by path with query strings collapsed. */
	byPath: Record<string, PathTotals>;
	/** Every `/api/v1/hosts` call, with its query, so regressions are legible. */
	hostRequests: { query: string; bodyBytes: number }[];
	/** Host list calls that are unpaginated AND carry nested children. */
	unboundedNestedHostRequests: string[];
}

/**
 * Collects response sizes until stopped. Playwright's `response` event is the only
 * place both the decoded and the on-wire size are available.
 */
function startRecording(page: Page) {
	const records: RequestRecord[] = [];
	const pending: Promise<void>[] = [];

	const onResponse = (response: import('@playwright/test').Response) => {
		pending.push(
			(async () => {
				const url = new URL(response.url());

				let bodyBytes = 0;
				let wireBytes = 0;
				try {
					bodyBytes = (await response.body()).byteLength;
				} catch {
					// Redirects and 204s have no retrievable body; count them as 0 rather
					// than dropping the request, so request counts stay honest.
				}
				try {
					wireBytes = (await response.request().sizes()).responseBodySize;
				} catch {
					// `sizes()` is unavailable for served-from-cache and aborted requests;
					// leave it at 0 rather than dropping the record.
				}

				records.push({
					path: url.pathname,
					query: url.search,
					status: response.status(),
					bodyBytes,
					wireBytes
				});
			})()
		);
	};

	page.on('response', onResponse);

	return {
		async stop(): Promise<RequestRecord[]> {
			page.off('response', onResponse);
			await Promise.all(pending);
			return records;
		}
	};
}

/**
 * A host list request is the thing this work exists to eliminate when it is both
 * unpaginated (`limit=0`) and carrying nested children (no
 * `include_children=false`). Paginated list calls and summary calls are fine.
 */
function isUnboundedNestedHostRequest(record: RequestRecord): boolean {
	if (record.path !== '/api/v1/hosts') return false;
	const params = new URLSearchParams(record.query);
	const unbounded = params.get('limit') === '0';
	const childrenIncluded = params.get('include_children') !== 'false';
	return unbounded && childrenIncluded;
}

function summarize(name: string, records: RequestRecord[]): ScenarioReport {
	const byPath: Record<string, PathTotals> = {};
	for (const r of records) {
		const totals = (byPath[r.path] ??= { requests: 0, bodyBytes: 0, wireBytes: 0 });
		totals.requests += 1;
		totals.bodyBytes += r.bodyBytes;
		totals.wireBytes += r.wireBytes;
	}

	return {
		name,
		requests: records.length,
		totalBodyBytes: records.reduce((sum, r) => sum + r.bodyBytes, 0),
		totalWireBytes: records.reduce((sum, r) => sum + r.wireBytes, 0),
		byPath,
		hostRequests: records
			.filter((r) => r.path === '/api/v1/hosts')
			.map((r) => ({ query: r.query, bodyBytes: r.bodyBytes })),
		unboundedNestedHostRequests: records
			.filter(isUnboundedNestedHostRequest)
			.map((r) => r.query || '(no query)')
	};
}

const kb = (bytes: number) => `${Math.round(bytes / 1024)} KB`;

function logScenario(report: ScenarioReport) {
	console.log(`\n--- ${report.name} ---`);
	console.log(`  Requests:        ${report.requests}`);
	console.log(`  Decoded bytes:   ${kb(report.totalBodyBytes)}`);
	console.log(`  On-wire bytes:   ${kb(report.totalWireBytes)}`);
	const heaviest = Object.entries(report.byPath)
		.sort((a, b) => b[1].bodyBytes - a[1].bodyBytes)
		.slice(0, 8);
	console.log('  Heaviest paths:');
	for (const [path, totals] of heaviest) {
		console.log(`    ${kb(totals.bodyBytes).padStart(9)}  ${totals.requests}x  ${path}`);
	}
	if (report.unboundedNestedHostRequests.length > 0) {
		console.log(`  UNBOUNDED NESTED HOST REQUESTS: ${report.unboundedNestedHostRequests.length}`);
		for (const q of report.unboundedNestedHostRequests) console.log(`    ${q}`);
	}
}

test('payload cost on boot, hosts tab, and during a scan', async ({ page, context }) => {
	test.setTimeout(240_000);

	await context.addCookies([
		{ name: 'session_id', value: process.env.SESSION_ID ?? '', domain: 'localhost', path: '/' }
	]);

	const scenarios: ScenarioReport[] = [];

	// --- Scenario 1: cold boot to interactive -------------------------------
	// Every tab mounts on first paint (inactive ones are hidden with CSS), so this
	// captures what the whole app asks for, not just the landing tab.
	const boot = startRecording(page);
	await page.goto('/');
	await page.waitForLoadState('networkidle');
	const bootReport = summarize('cold boot to interactive', await boot.stop());
	scenarios.push(bootReport);

	// --- Scenario 2: hosts tab, one page ------------------------------------
	// The hosts tab is the one surface that legitimately needs nested children, so
	// this should show a paginated call, not an unbounded one.
	const hostsTab = startRecording(page);
	await page.goto('/#hosts');
	await page.waitForLoadState('networkidle');
	scenarios.push(summarize('hosts tab, first page', await hostsTab.stop()));

	// --- Scenario 3: refetch traffic during a live scan ---------------------
	// Opt-in: needs a discovery actually running, since the cost comes from the
	// SSE stream's throttled invalidations.
	const scanSeconds = Number(process.env.SCAN_SECONDS ?? 0);
	if (scanSeconds > 0) {
		const scan = startRecording(page);
		await page.waitForTimeout(scanSeconds * 1000);
		scenarios.push(summarize(`${scanSeconds}s during a live discovery scan`, await scan.stop()));
	}

	const report = {
		label: process.env.PERF_LABEL ?? 'unlabelled',
		scanSeconds,
		scenarios
	};

	mkdirSync(dirname(OUTPUT_PATH), { recursive: true });
	writeFileSync(OUTPUT_PATH, JSON.stringify(report, null, 2));

	console.log(`\n=== Payload Cost (${report.label}) ===`);
	for (const scenario of scenarios) logScenario(scenario);
	console.log(`\nWrote ${OUTPUT_PATH}`);

	// The regression gate. Everything else in this file is measurement; this is the
	// assertion that keeps the org-wide nested host fetch from creeping back onto
	// the boot path.
	expect(
		bootReport.unboundedNestedHostRequests,
		'no unpaginated nested /api/v1/hosts request should fire on cold boot'
	).toEqual([]);
});
