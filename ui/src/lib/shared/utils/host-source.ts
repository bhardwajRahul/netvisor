/**
 * The badge that says nothing has ever contacted a host.
 *
 * A host Scanopy learned about from something else — an LLDP neighbour publishing an address, a
 * controller listing a device it manages — and never reached itself. Without a badge it is
 * indistinguishable from a device that is simply down: no ports, no services, an address nothing
 * answered on. That distinction is the whole reason the rung exists, so leaving it unrendered would
 * mean carrying it for nothing.
 *
 * Defined once and rendered by the shared `Tag`, for the same reason `getFreshnessTag` and
 * `getCidrSourceTag` are: the host list and the same host drawn in topology must not disagree about
 * how it came to be known, or look different when they say it.
 */

import { Radio } from 'lucide-svelte';

import type { components } from '$lib/api/schema';
import type { CardFieldItem, TagProps } from '$lib/shared/components/data/types';
import { hosts_neverContacted, hosts_neverContactedDetail } from '$lib/paraglide/messages';
import { toColor } from '$lib/shared/utils/styling';

/** Derived from the backend enum rather than restated, so a new variant cannot drift out of sync. */
export type EntitySource = components['schemas']['EntitySource'];

/**
 * Optional, so a payload predating the rung reads as an ordinary discovered host rather than
 * throwing — an absent source is not a claim that nothing contacted it.
 */
type WithSource = { source?: EntitySource };

/** Whether this host is known only at second hand. */
export function isInferredHost(host: WithSource): boolean {
	return host.source?.type === 'Inferred';
}

/**
 * The never-contacted tag, or `null` when there is nothing to say.
 *
 * `null` for every other source: a host the sweep reached and one a person typed in are both
 * first-hand, and the badge exists only to mark the third case.
 */
export function getHostSourceTag(host: WithSource): TagProps | null {
	if (!isInferredHost(host)) return null;
	return {
		label: hosts_neverContacted(),
		color: toColor('violet'),
		icon: Radio,
		title: hosts_neverContactedDetail()
	};
}

/**
 * The badge as a data-table field item, for the name column.
 *
 * Beside the name rather than in a column of its own: it qualifies what the row *is*, and a
 * separate column would read as an attribute to sort by rather than a caveat on the host.
 */
export function hostSourceItems<T extends WithSource>(): (host: T) => CardFieldItem[] | undefined {
	return (host) => {
		const tag = getHostSourceTag(host);
		if (!tag) return undefined;
		return [
			{
				id: 'never-contacted',
				label: tag.label,
				color: tag.color,
				icon: tag.icon,
				title: tag.title
			}
		];
	};
}
