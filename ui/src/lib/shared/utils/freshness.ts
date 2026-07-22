/**
 * How recently discovery observed an entity.
 *
 * This is the frontend half of one shared rule — the backend derives the same
 * verdict in `DiscoveryTracked::freshness` for the discovery digest email, and
 * the same predicate again in `StorableFilter::stale_by_network` for the
 * server-side "Stale only" filter. All three read the same two persisted
 * inputs (`last_seen_at` and the entity's network `stale_after_hours`), so a
 * host reported stale in the digest is the host badged stale in the app.
 * Change one, change all three.
 */

import { Clock } from 'lucide-svelte';
import type { components } from '$lib/api/schema';
import type { Network } from '$lib/features/networks/types';
import type { TagProps } from '$lib/shared/components/data/types';
import { toColor } from '$lib/shared/utils/styling';
import { formatRelativeTime } from '$lib/shared/utils/formatting';
import { common_lastSeenAgo, common_never, common_stale } from '$lib/paraglide/messages';

/** Derived from the backend enum — never hand-maintain this union. */
export type EntityFreshness = components['schemas']['EntityFreshness'];

type EntitySource = components['schemas']['EntitySource'];

/** The subset of an entity freshness depends on. Hosts, services and subnets all satisfy it. */
export interface FreshnessSubject {
	last_seen_at?: string;
	source?: EntitySource;
}

/**
 * Discovery only refreshes `last_seen_at` on entities it created. A manual or
 * system entity's timestamp is frozen at creation, so judging it would mark
 * every hand-curated asset stale once it aged past the window.
 */
function isDiscoveryManaged(source: EntitySource | undefined): boolean {
	return source?.type === 'Discovery' || source?.type === 'DiscoveryWithMatch';
}

/**
 * `Current` unless discovery manages this entity and hasn't observed it within
 * its network's window. Never returns `New` — that bucket exists only for the
 * digest's per-scan framing; the inventory surfaces `created_at` directly.
 *
 * The window comes from `effective_stale_after_hours`, which the server has
 * already resolved against its own default — the frontend deliberately holds no
 * default of its own, so it cannot drift from the digest. With no network in
 * hand we make no claim rather than guessing.
 */
export function entityFreshness(
	entity: FreshnessSubject,
	network: Network | undefined
): EntityFreshness {
	const windowHours = network?.effective_stale_after_hours;
	if (!windowHours || !entity.last_seen_at || !isDiscoveryManaged(entity.source)) return 'current';
	const cutoff = Date.now() - windowHours * 60 * 60 * 1000;
	return new Date(entity.last_seen_at).getTime() < cutoff ? 'stale' : 'current';
}

/**
 * Status tag for an entity card, or `null` when there is nothing to say.
 *
 * Amber rather than red, matching `getDaemonStatusTag`'s split: red means
 * broken (unreachable), amber means behind (outdated). A stale host may be
 * perfectly healthy and simply unobserved — so the badge must not read as an
 * error. The label carries the meaning without relying on colour.
 */
export function getFreshnessTag(
	entity: FreshnessSubject,
	network: Network | undefined
): TagProps | null {
	if (entityFreshness(entity, network) !== 'stale') return null;
	return {
		label: common_stale(),
		color: toColor('amber'),
		icon: Clock,
		title: lastSeenLabel(entity)
	};
}

/** "Last seen 12d ago", or a never-observed fallback. */
export function lastSeenLabel(entity: FreshnessSubject): string {
	if (!entity.last_seen_at) return common_never();
	return common_lastSeenAgo({ time: formatRelativeTime(entity.last_seen_at) });
}
