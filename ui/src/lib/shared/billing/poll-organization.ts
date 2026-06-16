import { queryClient, queryKeys } from '$lib/api/query-client';
import { fetchOrganization } from '$lib/features/organizations/queries';
import type { Organization } from '$lib/features/organizations/types';

/**
 * Poll the current-organization query until `predicate(org)` is true.
 *
 * Stripe actions (checkout, portal, add-payment) are applied server-side by
 * webhooks, so the org payload changes asynchronously after the user finishes
 * in the (new-tab) Stripe flow. This repeatedly invalidates + refetches the org
 * so the originating tab converges on the new state on its own — no window-focus
 * or same-tab redirect required.
 *
 * Returns the matching org, or `null` if the predicate never held within
 * `maxAttempts`.
 */
export async function pollOrganizationUntil(
	predicate: (org: Organization) => boolean,
	{ maxAttempts = 10, intervalMs = 2000 }: { maxAttempts?: number; intervalMs?: number } = {}
): Promise<Organization | null> {
	for (let attempt = 0; attempt < maxAttempts; attempt++) {
		await queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
		const org = await fetchOrganization();
		if (org && predicate(org)) {
			return org;
		}
		await new Promise((resolve) => setTimeout(resolve, intervalMs));
	}
	return null;
}
