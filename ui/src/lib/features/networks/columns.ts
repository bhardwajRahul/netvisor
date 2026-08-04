import { entityRef, type CardFieldItem } from '$lib/shared/components/data/types';
import { entities } from '$lib/shared/stores/metadata';
import type { Network } from './types';

/**
 * A network as a single navigable chip.
 *
 * Entities store a `network_id`, and four tabs surface it as a column. Building
 * the chip here keeps the colour and the entity link identical across all of
 * them, and matches what the cards already render.
 */
export function networkItems(networkId: string, networks: Network[]): CardFieldItem[] {
	const network = networks.find((n) => n.id === networkId);
	if (!network) return [];

	return [
		{
			id: network.id,
			label: network.name,
			color: entities.getColorHelper('Network').color,
			entityRef: entityRef('Network', network.id, network)
		}
	];
}
