import { chromium } from 'playwright';

const SESSION_ID = '7RTvkKYWyLY2tI5FNUwg-w';
const TOPO_OPTIONS =
	'{"local":{"hide_edge_types":["Hypervisor","RequestPath","HubAndSpoke","PhysicalLink"],"no_fade_edges":false,"hide_resize_handles":false,"bundle_edges":true,"tag_filter":{"hidden_host_tag_ids":[],"hidden_service_tag_ids":[],"hidden_subnet_tag_ids":[]},"show_minimap":true},"request":{"hide_ports":false,"hide_vm_title_on_docker_container":false,"hide_service_categories":["NetworkCore","OpenPorts"],"container_rules":["BySubnet","ByVirtualizingService"],"leaf_rules":[{"ByServiceCategory":{"categories":[],"title":"Infrastructure"}},{"ByTag":{"tag_ids":["6cce763c-0d6f-44f7-a8e7-ca1004c73b08"],"title":"aaa"}}]}}';

async function main() {
	const browser = await chromium.launch({ headless: true });
	const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
	await ctx.addCookies([
		{
			name: 'session_id',
			value: SESSION_ID,
			domain: 'localhost',
			path: '/'
		}
	]);

	const page = await ctx.newPage();

	// Set localStorage before navigation
	await page.goto('http://localhost:5173/', { waitUntil: 'domcontentloaded', timeout: 10000 });
	await page.evaluate((opts) => {
		localStorage.setItem('scanopy_topology_options', opts);
	}, TOPO_OPTIONS);

	// Now navigate to topology
	await page.goto('http://localhost:5173/#topology', {
		waitUntil: 'domcontentloaded',
		timeout: 15000
	});
	await page.waitForSelector('.svelte-flow__node', { timeout: 15000 });
	await page.waitForTimeout(8000);

	const nodeCount = await page.locator('.svelte-flow__node').count();
	console.log(`Nodes: ${nodeCount}`);

	// Capture all console logs with [edge-swap]
	// But they already fired during layout, so let's inject a re-layout trigger
	// Instead, let's look at the topology data directly

	// Get topology data from the app's internal state
	const debugInfo = await page.evaluate(() => {
		// Try to access the topology store or window.__debug
		// Since we can't easily access Svelte stores, let's examine the DOM

		const nodes = document.querySelectorAll('.svelte-flow__node');
		const edges = document.querySelectorAll('.svelte-flow__edge');

		const nodeList = Array.from(nodes).map((node) => {
			const el = node as HTMLElement;
			const id = el.getAttribute('data-id') || '';
			const transform = el.style.transform || '';
			const match = transform.match(/translate\((-?[\d.]+)px,\s*(-?[\d.]+)px\)/);
			return {
				id,
				x: match ? parseFloat(match[1]) : 0,
				y: match ? parseFloat(match[2]) : 0,
				width: Math.round(el.offsetWidth),
				height: Math.round(el.offsetHeight),
				parentId: el.getAttribute('data-parent') || null,
				text: el.textContent?.substring(0, 100)?.trim() || ''
			};
		});

		// Get edge data from the SVG elements
		const edgeList: Array<{ source: string; target: string; type: string }> = [];
		const edgeSvgs = document.querySelectorAll('.svelte-flow__edge');
		for (const edge of edgeSvgs) {
			const el = edge as HTMLElement;
			edgeList.push({
				source:
					el.getAttribute('data-source') ||
					el.querySelector('[data-source]')?.getAttribute('data-source') ||
					'',
				target:
					el.getAttribute('data-target') ||
					el.querySelector('[data-target]')?.getAttribute('data-target') ||
					'',
				type: el.getAttribute('data-testid') || el.className || ''
			});
		}

		return { nodes: nodeList, edges: edgeList, edgeCount: edges.length };
	});

	console.log(`Edges: ${debugInfo.edgeCount}`);

	// Find the bridge node candidate - the one with "en0: 192.168" in its text
	const bridgeCandidate = debugInfo.nodes.find((n) => n.text.includes('en0'));
	if (bridgeCandidate) {
		console.log(`\nBridge candidate: ${bridgeCandidate.id}`);
		console.log(`  Text: ${bridgeCandidate.text.substring(0, 60)}`);
		console.log(`  Position: x=${bridgeCandidate.x} y=${bridgeCandidate.y}`);
		console.log(`  Parent: ${bridgeCandidate.parentId}`);
	} else {
		console.log('\nNo node with "en0" found. Looking for Scanopy Daemon:');
		const candidates = debugInfo.nodes.filter((n) => n.text.includes('Scanopy Daemon'));
		for (const c of candidates) {
			console.log(`  ${c.id}: "${c.text.substring(0, 60)}" parent=${c.parentId} x=${c.x}`);
		}
	}

	// Show containers and their child counts
	console.log('\n=== Containers ===');
	const containers = new Map<string, typeof debugInfo.nodes>();
	for (const n of debugInfo.nodes) {
		if (n.parentId) {
			if (!containers.has(n.parentId)) containers.set(n.parentId, []);
			containers.get(n.parentId)!.push(n);
		}
	}
	for (const [cid, children] of containers) {
		const c = debugInfo.nodes.find((n) => n.id === cid);
		console.log(`${cid}: ${children.length} children "${c?.text?.substring(0, 50)}"`);

		if (children.length >= 5) {
			// Group by x
			const byX = new Map<number, typeof debugInfo.nodes>();
			for (const ch of children) {
				const x = Math.round(ch.x);
				if (!byX.has(x)) byX.set(x, []);
				byX.get(x)!.push(ch);
			}
			const xs = Array.from(byX.keys()).sort((a, b) => a - b);
			console.log(`  Columns: ${xs.length} at x=[${xs.join(',')}]`);

			// Show bridge candidate position
			if (bridgeCandidate && children.some((c) => c.id === bridgeCandidate.id)) {
				console.log(
					`  Bridge node at x=${bridgeCandidate.x} (leftX=${xs[0]}, rightX=${xs[xs.length - 1]})`
				);
			}
		}
	}

	await page.screenshot({ path: '/tmp/topology-debug.png', fullPage: false });
	console.log('\nScreenshot: /tmp/topology-debug.png');
	await browser.close();
}

main().catch(console.error);
