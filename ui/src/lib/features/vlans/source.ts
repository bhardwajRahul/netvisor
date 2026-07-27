/**
 * Human-readable labels for `EntitySource`.
 *
 * The variant union itself comes from the generated schema — only the localized
 * label lives here, since the backend cannot produce translated strings.
 */

import type { EntitySource } from '$lib/shared/types';
import {
	common_discovered,
	common_manual,
	common_system,
	common_unknown
} from '$lib/paraglide/messages';

export function sourceLabel(source: EntitySource | undefined): string {
	switch (source?.type) {
		case 'Manual':
			return common_manual();
		case 'System':
			return common_system();
		case 'Discovery':
		case 'DiscoveryWithMatch':
			return common_discovered();
		default:
			return common_unknown();
	}
}
