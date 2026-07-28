/**
 * Checks that the element shape key actually predicts card height.
 *
 * Sampling the measure pass rests on one assumption: element nodes sharing a
 * shape key measure to the same height. If that is ever false, nodes are laid
 * out with a wrong height and the graph overlaps — with no error, because
 * nothing threw. This turns that silent failure into a reported one.
 *
 * It runs against a *full* measurement (every node mounted and measured), so it
 * validates the key without depending on sampling being correct. That makes it
 * usable both as a pre-flight check before sampling is trusted, and afterwards
 * as a regression guard when someone changes how cards render.
 *
 * Enabled by `window.__topoVerifyShapes`; off by default, since it needs the
 * expensive full measurement it exists to justify replacing.
 */

import { browser } from '$app/environment';
import type { RenderableTopology, TopologyNode } from '../types/base';
import type { XY } from './types';
import {
	buildElementRender,
	currentElementRenderContext,
	elementShapeKey
} from '../element-render-data';

/**
 * Heights must agree exactly.
 *
 * `offsetHeight` is an integer, so any difference is a real structural
 * difference in the card — not measurement noise. Tolerating a few pixels
 * would mean tolerating a cause we haven't identified, and the whole point of
 * this check is that unexplained height variation is where silent layout
 * corruption comes from.
 */
const HEIGHT_TOLERANCE_PX = 0;

export interface ShapeKeyDisagreement {
	shapeKey: string;
	/** Distinct measured heights seen for this key, with an example node each. */
	heights: { height: number; nodeId: string; count: number }[];
}

export interface ShapeVerifyReport {
	elementsChecked: number;
	distinctKeys: number;
	disagreements: ShapeKeyDisagreement[];
	/** Nodes whose key could not be computed; these must be measured individually. */
	unkeyedNodeIds: string[];
}

export function shapeVerifyEnabled(): boolean {
	if (!browser) return false;
	return (window as unknown as { __topoVerifyShapes?: boolean }).__topoVerifyShapes === true;
}

/**
 * Group measured element nodes by shape key and report keys that map to more
 * than one height.
 */
export function verifyShapeKeys(
	visibleNodes: TopologyNode[],
	topology: RenderableTopology,
	measured: Map<string, XY>
): ShapeVerifyReport {
	const context = currentElementRenderContext();
	// key -> exact height -> { count, exampleNodeId }
	const byKey = new Map<string, Map<number, { count: number; nodeId: string }>>();
	const unkeyedNodeIds: string[] = [];
	let elementsChecked = 0;

	for (const node of visibleNodes) {
		if (node.node_type !== 'Element') continue;
		const size = measured.get(node.id);
		if (!size) continue;

		let key: string;
		try {
			key = elementShapeKey(buildElementRender({ nodeId: node.id, node, topology, ...context }));
		} catch {
			unkeyedNodeIds.push(node.id);
			continue;
		}

		elementsChecked++;
		const heights = byKey.get(key) ?? new Map<number, { count: number; nodeId: string }>();
		const existing = heights.get(size.y);
		if (existing) existing.count++;
		else heights.set(size.y, { count: 1, nodeId: node.id });
		byKey.set(key, heights);
	}

	const disagreements: ShapeKeyDisagreement[] = [];
	for (const [key, heights] of byKey) {
		if (heights.size <= 1) continue;
		// Compare the spread, not adjacent buckets: bucketing would split two
		// heights one pixel apart whenever they straddled a boundary.
		const observed = [...heights.keys()];
		const spread = Math.max(...observed) - Math.min(...observed);
		if (spread <= HEIGHT_TOLERANCE_PX) continue;
		disagreements.push({
			shapeKey: key,
			heights: [...heights.entries()]
				.map(([height, v]) => ({ height, nodeId: v.nodeId, count: v.count }))
				.sort((a, b) => b.count - a.count)
		});
	}

	return { elementsChecked, distinctKeys: byKey.size, disagreements, unkeyedNodeIds };
}

/**
 * Run the check and surface the result. Exposed on `window` so the Playwright
 * harness can assert on it rather than scraping console output.
 */
export function reportShapeVerification(
	visibleNodes: TopologyNode[],
	topology: RenderableTopology,
	measured: Map<string, XY>
): void {
	const report = verifyShapeKeys(visibleNodes, topology, measured);
	(window as unknown as { __topoShapeReport?: ShapeVerifyReport }).__topoShapeReport = report;

	if (report.disagreements.length > 0) {
		console.warn(
			`[topology] ${report.disagreements.length} shape key(s) map to more than one card height. ` +
				`Sampling the measure pass would lay these out wrongly.`,
			report.disagreements
		);
	}
}
