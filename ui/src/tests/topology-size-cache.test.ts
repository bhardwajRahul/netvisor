import { describe, it, expect } from 'vitest';

/**
 * `getStructureKey` composes `nodes|inline|resizingHide|structuralHide`, and `prepare` decides
 * whether to discard every measured size by comparing only the middle two. Getting that wrong is
 * silent in both directions: compare too much and a filter toggle throws away 19,095 sizes and
 * re-measures the graph (~665MB, 5.5s); compare too little and ELK lays out against sizes the cards
 * no longer have, and they overlap.
 */
describe('structure key segmentation', () => {
	/** What `prepare` reads to decide whether cards may have resized. */
	const resizeSignal = (key: string) => {
		const [, inline = '', resizingHide = ''] = key.split('|');
		return { inline, resizingHide };
	};

	const base = '10:5:a@,b@|inlineSig|e:svc-1|n:node-9';

	it('ignores a node-set change', () => {
		// More nodes, same card contents — the survivors' sizes are still correct.
		expect(resizeSignal('11:5:a@,b@,c@|inlineSig|e:svc-1|n:node-9')).toEqual(resizeSignal(base));
	});

	it('ignores a hide that only removes nodes', () => {
		// Hiding unlinked ports in L2 removes Interface element nodes. Nothing that survives is
		// drawn differently, so the sizes must be kept — this is the case that produced seven full
		// measurement passes across nine runs.
		expect(resizeSignal('10:5:a@,b@|inlineSig|e:svc-1|m:Interface.LinkState=Unlinked')).toEqual(
			resizeSignal(base)
		);
	});

	it('reacts to a hide that resizes cards', () => {
		// Hiding an entity drawn inside another node's card changes that card's height.
		expect(resizeSignal('10:5:a@,b@|inlineSig|m:Service.Category=OpenPorts|n:node-9')).not.toEqual(
			resizeSignal(base)
		);
	});

	it('reacts to an inline content change', () => {
		expect(resizeSignal('10:5:a@,b@|OTHER|e:svc-1|n:node-9')).not.toEqual(resizeSignal(base));
	});

	it('treats a missing segment as empty rather than undefined', () => {
		// A view with no inline entities and no filters emits empty segments; they must compare equal
		// rather than producing a spurious clear on every run.
		expect(resizeSignal('3:1:a@|||')).toEqual(resizeSignal('4:1:a@,b@|||'));
	});
});

/**
 * The hide-state segments are built from a *typed* view of what is hidden. A flat set of entity ids
 * cannot say whether an entity is drawn inside another node's card or is a node of its own, and
 * reading the flat one classified a link-state toggle as card-resizing — re-measuring all 19,095
 * nodes on every press. Four full passes in one capture, matching its four filter toggles exactly.
 */
describe('hidden entities are classified by type', () => {
	const resizeSignal = (key: string) => {
		const [, inline = '', resizingHide = ''] = key.split('|');
		return { inline, resizingHide };
	};

	it('puts an element-entity hide on the structural side', () => {
		// Interface is an element node in L2, so hiding some changes only the node set.
		const before = '10:5:a@,b@|inline||';
		const after = '8:4:a@|inline||e:Interface:if-1,if-2';
		expect(resizeSignal(after)).toEqual(resizeSignal(before));
	});

	it('puts an inline-entity hide on the resizing side', () => {
		// Service is inlined on host cards in L3, so hiding some shrinks those cards.
		const before = '10:5:a@,b@|inline||';
		const after = '10:5:a@,b@|inline|e:Service:svc-1|';
		expect(resizeSignal(after)).not.toEqual(resizeSignal(before));
	});
});
