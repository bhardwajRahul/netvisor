import { entityRef, type LabelledCardFieldItem } from '$lib/shared/components/data/types';
import { entities } from '$lib/shared/stores/metadata';
import type { Network } from './types';

/**
 * Networks as navigable chips.
 *
 * Takes one id or many, because entities reference a network either way
 * (`network_id` on most, `network_ids` on user API keys). Building the chip
 * here keeps the colour and the entity link identical wherever a network
 * appears, and matches what the cards already render.
 */
export function networkItems(
	networkIds: string | string[] | null | undefined,
	networks: Network[]
): LabelledCardFieldItem[] {
	if (!networkIds) return [];
	const ids = Array.isArray(networkIds) ? networkIds : [networkIds];

	return ids
		.map((id) => networks.find((n) => n.id === id))
		.filter((network): network is Network => Boolean(network))
		.map((network) => ({
			id: network.id,
			label: network.name,
			color: entities.getColorHelper('Network').color,
			entityRef: entityRef('Network', network.id, network)
		}));
}
