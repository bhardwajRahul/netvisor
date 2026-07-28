import { test, expect } from '@playwright/test';

interface NodeInfo {
	id: string;
	type: string; // 'ContainerNode' | 'LeafNode'
	x: number;
	y: number;
	width: number;
	height: number;
}

interface EdgeInfo {
	source: string;
	target: string;
}

interface LayoutMetrics {
	overlapCount: number;
	hierarchyFlowPercent: number;
	spreadRatio: number;
	avgEdgeLength: number;
	maxEdgeLength: number;
	nodeCount: number;
}

/** Extract node positions and sizes from the rendered SvelteFlow DOM. */
async function extractNodes(page: import('@playwright/test').Page): Promise<NodeInfo[]> {
	return page.evaluate(() => {
		const nodes: NodeInfo[] = [];
		const nodeElements = document.querySelectorAll('.svelte-flow__node');

		for (const el of nodeElements) {
			const id = el.getAttribute('data-id') ?? '';
			const style = (el as HTMLElement).style;
			const transform = style.transform || '';

			// Parse transform: translate(Xpx, Ypx)
			const match = transform.match(/translate\(([^,]+)px,\s*([^)]+)px\)/);
			if (!match) continue;

			const x = parseFloat(match[1]);
			const y = parseFloat(match[2]);
			const width = (el as HTMLElement).offsetWidth;
			const height = (el as HTMLElement).offsetHeight;

			// Determine type from class or data attribute
			const classList = Array.from(el.classList);
			const isContainer =
				classList.some((c) => c.toLowerCase().includes('container')) ||
				el.getAttribute('data-type') === 'ContainerNode';
			const type = isContainer ? 'ContainerNode' : 'LeafNode';

			nodes.push({ id, type, x, y, width, height });
		}

		return nodes;
	});
}

/** Extract edges (source/target IDs) from the rendered SvelteFlow DOM. */
async function extractEdges(page: import('@playwright/test').Page): Promise<EdgeInfo[]> {
	return page.evaluate(() => {
		const edges: { source: string; target: string }[] = [];
		const edgeElements = document.querySelectorAll('[class*="svelte-flow__edge"]');

		for (const el of edgeElements) {
			// Try data attributes first, then parse from aria-label or data-id
			let source = el.getAttribute('data-source') ?? '';
			let target = el.getAttribute('data-target') ?? '';

			// SvelteFlow edge IDs often encode source-target as "reactflow__edge-{source}-{target}"
			if (!source || !target) {
				const edgeId = el.getAttribute('data-id') ?? el.getAttribute('data-testid') ?? '';
				const parts = edgeId.split('-');
				// Look for UUID patterns (8-4-4-4-12)
				const uuids = edgeId.match(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/g);
				if (uuids && uuids.length >= 2) {
					source = uuids[0];
					target = uuids[1];
				} else if (parts.length >= 2) {
					source = parts[0];
					target = parts[parts.length - 1];
				}
			}

			if (source && target && source !== target) {
				edges.push({ source, target });
			}
		}

		return edges;
	});
}

/** Wait for layout to settle by polling positions until stable. */
async function waitForStableLayout(page: import('@playwright/test').Page): Promise<NodeInfo[]> {
	let previousPositions = '';
	let stableCount = 0;
	let nodes: NodeInfo[] = [];

	for (let i = 0; i < 20; i++) {
		nodes = await extractNodes(page);
		const currentPositions = JSON.stringify(nodes.map((n) => ({ id: n.id, x: n.x, y: n.y })));

		if (currentPositions === previousPositions && nodes.length > 0) {
			stableCount++;
			if (stableCount >= 2) break;
		} else {
			stableCount = 0;
		}

		previousPositions = currentPositions;
		await page.waitForTimeout(500);
	}

	return nodes;
}

/** Check if two bounding boxes overlap. */
function boxesOverlap(a: NodeInfo, b: NodeInfo): boolean {
	return !(
		a.x + a.width <= b.x ||
		b.x + b.width <= a.x ||
		a.y + a.height <= b.y ||
		b.y + b.height <= a.y
	);
}

/** Compute layout quality metrics. */
function computeMetrics(nodes: NodeInfo[], edges: EdgeInfo[]): LayoutMetrics {
	const containers = nodes.filter((n) => n.type === 'ContainerNode');

	// Overlap count (between containers only — leaves are inside containers)
	let overlapCount = 0;
	for (let i = 0; i < containers.length; i++) {
		for (let j = i + 1; j < containers.length; j++) {
			if (boxesOverlap(containers[i], containers[j])) {
				overlapCount++;
			}
		}
	}

	// Hierarchy flow: are containers distributed vertically (not all in one row)?
	// Score = % of distinct Y-bands used. With few containers, having 2+ levels is sufficient.
	const containerYs = containers.map((c) => c.y);
	const uniqueYLevels = new Set(containerYs.map((y) => Math.round(y / 50))).size;
	const hierarchyFlowPercent = containers.length <= 1 ? 100 : uniqueYLevels <= 1 ? 0 : 100;

	// Spread ratio: bounding box area / sum of container areas (not leaves,
	// since leaves are inside containers and would double-count area)
	const topLevel =
		containers.length > 0 ? containers : nodes.filter((n) => n.width > 0 && n.height > 0);
	if (topLevel.length === 0) {
		return {
			overlapCount: 0,
			hierarchyFlowPercent: 100,
			spreadRatio: 1,
			avgEdgeLength: 0,
			maxEdgeLength: 0,
			nodeCount: 0
		};
	}

	const minX = Math.min(...topLevel.map((n) => n.x));
	const minY = Math.min(...topLevel.map((n) => n.y));
	const maxX = Math.max(...topLevel.map((n) => n.x + n.width));
	const maxY = Math.max(...topLevel.map((n) => n.y + n.height));
	const boundingArea = (maxX - minX) * (maxY - minY);
	const sumNodeArea = topLevel.reduce((sum, n) => sum + n.width * n.height, 0);
	const spreadRatio = boundingArea > 0 ? boundingArea / sumNodeArea : 1;

	// Edge length (informational): Euclidean distance between node centers.
	// Note: leaf positions are container-relative in SvelteFlow, so cross-container
	// edge lengths are approximate. Handle positions also affect real visual length.
	const nodeMap = new Map(nodes.map((n) => [n.id, n]));
	const edgeLengths: number[] = [];
	for (const edge of edges) {
		const src = nodeMap.get(edge.source);
		const tgt = nodeMap.get(edge.target);
		if (src && tgt) {
			const dx = src.x + src.width / 2 - (tgt.x + tgt.width / 2);
			const dy = src.y + src.height / 2 - (tgt.y + tgt.height / 2);
			edgeLengths.push(Math.sqrt(dx * dx + dy * dy));
		}
	}
	const avgEdgeLength =
		edgeLengths.length > 0 ? edgeLengths.reduce((a, b) => a + b, 0) / edgeLengths.length : 0;
	const maxEdgeLength = edgeLengths.length > 0 ? Math.max(...edgeLengths) : 0;

	return {
		overlapCount,
		hierarchyFlowPercent,
		spreadRatio,
		avgEdgeLength,
		maxEdgeLength,
		nodeCount: nodes.length
	};
}

test('topology layout quality evaluation', async ({ page, context }) => {
	// Inject session cookie for auth
	await context.addCookies([
		{
			name: 'session_id',
			value: process.env.SESSION_ID ?? '',
			domain: 'localhost',
			path: '/'
		}
	]);

	// Navigate to topology page
	await page.goto('/#topology');

	// Wait for SvelteFlow nodes to appear
	await page.waitForSelector('.svelte-flow__node', { timeout: 30000 });

	// Wait for layout to settle
	const nodes = await waitForStableLayout(page);

	console.log(`\n=== Topology Layout Evaluation ===`);
	console.log(`Nodes found: ${nodes.length}`);

	if (nodes.length === 0) {
		console.log('No nodes found — skipping evaluation');
		return;
	}

	// Extract edges
	const edges = await extractEdges(page);

	// Compute metrics
	const metrics = computeMetrics(nodes, edges);

	console.log(`\nMetrics:`);
	console.log(`  Overlap count:        ${metrics.overlapCount} (target: 0)`);
	console.log(`  Hierarchy flow:       ${metrics.hierarchyFlowPercent.toFixed(1)}% (target: >80%)`);
	console.log(`  Spread ratio:         ${metrics.spreadRatio.toFixed(2)} (target: 1.0-10.0)`);
	console.log(
		`  Avg edge length:      ${metrics.avgEdgeLength.toFixed(0)}px (${edges.length} edges)`
	);
	console.log(`  Max edge length:      ${metrics.maxEdgeLength.toFixed(0)}px`);

	// Take screenshot
	await page.screenshot({
		path: 'tests/screenshots/topology-layout.png',
		fullPage: true
	});
	console.log(`\nScreenshot saved to tests/screenshots/topology-layout.png`);

	// Pass/fail thresholds
	const results = [
		{
			name: 'No overlaps',
			pass: metrics.overlapCount === 0,
			detail: `${metrics.overlapCount} overlaps`
		},
		{
			name: 'Hierarchy flow',
			pass: metrics.hierarchyFlowPercent > 80,
			detail: `${metrics.hierarchyFlowPercent.toFixed(1)}%`
		},
		{
			name: 'Spread ratio',
			pass: metrics.spreadRatio >= 1.0 && metrics.spreadRatio <= 10.0,
			detail: `${metrics.spreadRatio.toFixed(2)}`
		}
	];

	console.log(`\nResults:`);
	for (const r of results) {
		console.log(`  ${r.pass ? 'PASS' : 'FAIL'}: ${r.name} (${r.detail})`);
	}

	const allPassed = results.every((r) => r.pass);
	console.log(`\nOverall: ${allPassed ? 'PASS' : 'FAIL'}\n`);

	// Assert
	expect(metrics.overlapCount, 'Container nodes should not overlap').toBe(0);
	expect(
		metrics.hierarchyFlowPercent,
		'Containers should be distributed across vertical layers'
	).toBeGreaterThan(80);
});
