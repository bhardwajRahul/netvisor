/**
 * TanStack Query hooks for VLANs
 *
 * Read-only: VLANs are populated by SNMP discovery, so there are no create,
 * update or delete hooks here.
 */

import { createQuery } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';

/**
 * VLANs list. Called with no arguments this is the shared full-list cache, so
 * any narrowing argument must also change the query key — otherwise a filtered
 * fetch would overwrite the shared cache with a subset.
 */
export function useVlansQuery(atGetter?: () => string | undefined) {
	return createQuery(() => {
		const at = atGetter?.();
		return {
			queryKey: at ? [...queryKeys.vlans.all, 'asOf', at] : queryKeys.vlans.all,
			queryFn: async () => {
				const { data } = await apiClient.GET('/api/v1/vlans', {
					params: { query: { limit: 0, at } }
				});
				if (!data?.success || !data.data) {
					throw new Error(data?.error || 'Failed to fetch VLANs');
				}
				return data.data;
			}
		};
	});
}
