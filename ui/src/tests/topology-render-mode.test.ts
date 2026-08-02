import { describe, it, expect } from 'vitest';
import { shouldCull } from '$lib/features/topology/pipeline/render-mode';

/**
 * Culling has two hard suspensions. Both exist because something else needs
 * every node present in the DOM, and in both cases the failure is silent
 * rather than loud — a truncated measurement or a cropped export, not an error.
 */

const LARGE = 500;
const SMALL = 20;

describe('shouldCull', () => {
	it('culls a large graph when nothing needs the full DOM', () => {
		expect(shouldCull({ renderedCount: LARGE, measuring: false, exporting: false })).toBe(true);
	});

	it('leaves small graphs alone', () => {
		// The guardrail: normal-scale topologies keep today's behaviour exactly.
		expect(shouldCull({ renderedCount: SMALL, measuring: false, exporting: false })).toBe(false);
	});

	it('suspends while measuring, even at scale', () => {
		// The measure pass mounts every node to read its height; culled nodes
		// never mount, so ELK would receive fallback sizes.
		expect(shouldCull({ renderedCount: LARGE, measuring: true, exporting: false })).toBe(false);
	});

	it('suspends while exporting, even at scale', () => {
		// Export rasterises the whole flow element; culling crops the image.
		expect(shouldCull({ renderedCount: LARGE, measuring: false, exporting: true })).toBe(false);
	});

	it('stays suspended when measuring and exporting overlap', () => {
		expect(shouldCull({ renderedCount: LARGE, measuring: true, exporting: true })).toBe(false);
	});
});
