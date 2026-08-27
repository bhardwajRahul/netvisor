import { describe, it, expect } from 'vitest';
import { collapsedContainerLines } from '$lib/features/topology/labels';

describe('collapsedContainerLines', () => {
	it('gives the title to the container name and demotes the summary', () => {
		const lines = collapsedContainerLines('sw-core-01', '4 interfaces');
		expect(lines.title).toBe('sw-core-01');
		expect(lines.subtitle).toBe('4 interfaces');
	});

	it('keeps the summary as the title when the container has no name', () => {
		const lines = collapsedContainerLines('', '4 interfaces');
		expect(lines.title).toBe('4 interfaces');
		expect(lines.subtitle).toBeNull();
	});

	it('treats a whitespace-only header as no name rather than a blank title line', () => {
		expect(collapsedContainerLines('   ', '1 interface')).toEqual({
			title: '1 interface',
			subtitle: null
		});
	});
});
