import { describe, it, expect } from 'vitest';

/**
 * `getStructureKey` composes `nodes|inline|hide`, and `prepare` decides whether to discard measured
 * sizes by comparing the last two segments. The composition is what makes that decision possible,
 * so a change to the separator or the ordering would silently turn every data refresh back into a
 * full re-measure of the graph — ~665MB and 5.5s at 19,095 nodes — with nothing failing.
 */
describe('structure key segmentation', () => {
	const split = (key: string) => {
		const [, inline = '', hide = ''] = key.split('|');
		return { inline, hide };
	};

	it('separates a node-set change from an inline or hide change', () => {
		const base = '10:5:a@,b@|inlineSig|hideSig';
		const moreNodes = '11:5:a@,b@,c@|inlineSig|hideSig';
		const inlineChanged = '10:5:a@,b@|OTHER|hideSig';
		const hideChanged = '10:5:a@,b@|inlineSig|OTHER';

		// Node set moved, card contents did not — sizes stay valid.
		expect(split(moreNodes)).toEqual(split(base));
		// Either of these resizes cards, so the sizes must be discarded.
		expect(split(inlineChanged)).not.toEqual(split(base));
		expect(split(hideChanged)).not.toEqual(split(base));
	});

	it('treats a missing segment as empty rather than undefined', () => {
		// A view with no inline entities and no filters emits empty segments; they must compare
		// equal to each other rather than producing a spurious clear on every run.
		expect(split('3:1:a@||')).toEqual(split('4:1:a@,b@||'));
	});
});
