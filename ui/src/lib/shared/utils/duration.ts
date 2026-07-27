/**
 * Conversions between a duration stored as a total number of hours and the
 * days + hours pair the UI presents.
 *
 * Durations are stored as hours (see `NetworkBase::stale_after_hours`) because
 * that is the granularity the backend compares against, but asking a user to
 * enter "672" for four weeks is a needless bit of mental arithmetic.
 */

export interface DaysAndHours {
	days: number | null;
	hours: number | null;
}

/**
 * Split a total hour count for display. `null` for a component means "leave the
 * box empty" rather than showing a redundant zero, so 48 renders as "2 days"
 * with a blank hours box.
 *
 * Non-positive or missing input yields both boxes empty — the "unset, use the
 * server default" state.
 */
export function splitHours(total: number | null | undefined): DaysAndHours {
	if (typeof total !== 'number' || !Number.isFinite(total) || total <= 0) {
		return { days: null, hours: null };
	}
	const whole = Math.floor(total);
	return {
		days: Math.floor(whole / 24) || null,
		hours: whole % 24 || null
	};
}

/**
 * Recombine into a total, or `null` when the duration is empty. Empty means
 * "unset" (fall back to the server's default), never "expire immediately" —
 * treating a blank field as zero would make everything instantly stale.
 */
export function combineToHours(days: number | null, hours: number | null): number | null {
	const total = Math.max(0, days ?? 0) * 24 + Math.max(0, hours ?? 0);
	return total > 0 ? total : null;
}
