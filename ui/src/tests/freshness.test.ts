import { describe, it, expect, vi, afterEach } from 'vitest';
import { entityFreshness } from '$lib/shared/utils/freshness';
import type { Network } from '$lib/features/networks/types';
import type { components } from '$lib/api/schema';

type EntitySource = components['schemas']['EntitySource'];

const HOUR_MS = 60 * 60 * 1000;
const NOW = new Date('2026-07-22T12:00:00Z').getTime();

/**
 * `effective_stale_after_hours` is what the server publishes after applying its
 * own default — the frontend holds no default of its own, precisely so it
 * cannot drift from the digest.
 */
function network(effective_stale_after_hours: number): Network {
	return { id: 'n1', effective_stale_after_hours } as Network;
}

function entity(hoursAgo: number, source: EntitySource = { type: 'Discovery' }) {
	return { last_seen_at: new Date(NOW - hoursAgo * HOUR_MS).toISOString(), source };
}

afterEach(() => vi.useRealTimers());

function freezeClock() {
	vi.useFakeTimers();
	vi.setSystemTime(NOW);
}

describe('entityFreshness', () => {
	it('uses each network’s own window rather than one global cutoff', () => {
		freezeClock();
		const strict = network(1);
		const lenient = network(24 * 30);
		const seenTwoHoursAgo = entity(2);

		expect(entityFreshness(seenTwoHoursAgo, strict)).toBe('stale');
		expect(entityFreshness(seenTwoHoursAgo, lenient)).toBe('current');
	});

	// Without a network we cannot know the window, so make no claim rather than
	// guessing — a wrong guess would badge assets stale that the digest calls
	// current.
	it('makes no staleness claim when the network is not loaded', () => {
		freezeClock();
		expect(entityFreshness(entity(24 * 365), undefined)).toBe('current');
	});

	// Discovery never refreshes last_seen_at on entities it didn't create, so
	// judging them would mark every hand-curated asset stale once it aged out.
	// Mirrors the backend's `is_discovery_managed` guard.
	it('never marks entities discovery does not manage as stale', () => {
		freezeClock();
		const net = network(1);
		const longAgo = 24 * 365;

		expect(entityFreshness(entity(longAgo, { type: 'Manual' }), net)).toBe('current');
		expect(entityFreshness(entity(longAgo, { type: 'System' }), net)).toBe('current');
		expect(entityFreshness(entity(longAgo, { type: 'Discovery' }), net)).toBe('stale');
		expect(
			entityFreshness(
				entity(longAgo, {
					type: 'DiscoveryWithMatch'
				} as EntitySource),
				net
			)
		).toBe('stale');
	});

	it('treats a never-observed entity as current rather than guessing', () => {
		freezeClock();
		expect(entityFreshness({ source: { type: 'Discovery' } }, network(1))).toBe('current');
	});

	// The frontend cutoff must agree with the backend's
	// `Network::stale_cutoff` (reference - stale_after_hours), or a host badged
	// stale in the inventory would not be the host reported stale in the digest.
	it('places the boundary exactly at the window edge, matching the backend rule', () => {
		freezeClock();
		const net = network(24);

		expect(entityFreshness(entity(23.9), net)).toBe('current');
		expect(entityFreshness(entity(24.1), net)).toBe('stale');
	});
});
