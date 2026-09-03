import { describe, it, expect } from 'vitest';
import { createChangeTracker, sameOrder } from '$lib/shared/components/data/controls/changeTracker';

describe('createChangeTracker', () => {
	it('stays quiet on the first observation', () => {
		// Load-bearing, not an optimisation: the first run of each notifying
		// effect is state being restored from storage. Reporting a change there
		// would tell the parent the user had just acted, resetting the restored
		// page to 1 on every mount.
		const tracker = createChangeTracker<string>();

		expect(tracker.changed('restored')).toBe(false);
	});

	it('reports a real change, once', () => {
		const tracker = createChangeTracker<string>();
		tracker.changed('a');

		expect(tracker.changed('b')).toBe(true);
		expect(tracker.changed('b')).toBe(false);
	});

	it('treats a return to the baseline as a change', () => {
		const tracker = createChangeTracker<string>();
		tracker.changed('a');
		tracker.changed('b');

		expect(tracker.changed('a')).toBe(true);
	});

	it('compares lists by contents, not identity', () => {
		// The tag filter hands over a fresh array every time it is read, so
		// identity would report a change on every effect run and refetch in a
		// loop.
		const tracker = createChangeTracker<string[]>(sameOrder);
		tracker.changed(['a', 'b']);

		expect(tracker.changed(['a', 'b'])).toBe(false);
		expect(tracker.changed(['a', 'c'])).toBe(true);
	});

	it('notices a list growing, shrinking, or reordering', () => {
		const tracker = createChangeTracker<string[]>(sameOrder);
		tracker.changed(['a']);

		expect(tracker.changed(['a', 'b'])).toBe(true);
		expect(tracker.changed(['b', 'a'])).toBe(true);
		expect(tracker.changed([])).toBe(true);
	});
});
