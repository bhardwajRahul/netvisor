import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { queryClient, queryKeys } from '$lib/api/query-client';
import type { Organization } from '$lib/features/organizations/types';
import type { PublicServerConfig } from '$lib/shared/stores/config-query';
import { isBillingPlanActive } from '$lib/features/organizations/types';

/**
 * Determines the correct route for an authenticated user based on their state
 */
export function getRoute(): string {
	const organization = queryClient.getQueryData<Organization | null>(
		queryKeys.organizations.current()
	);
	const configData = queryClient.getQueryData<PublicServerConfig>(queryKeys.config.all);

	if (!organization) {
		return resolve('/onboarding');
	}

	// Check onboarding first
	const onboardingModalCompleted = organization.onboarding.includes('OnboardingModalCompleted');
	if (!onboardingModalCompleted) {
		return resolve('/onboarding');
	}

	// Check billing if enabled
	const billingEnabled = configData?.billing_enabled ?? false;
	if (billingEnabled && !isBillingPlanActive(organization)) {
		return resolve('/billing');
	}

	// All checks passed - go to main app
	return resolve('/');
}

/**
 * Navigate to the appropriate route after authentication
 */
export async function navigate(): Promise<void> {
	const route = getRoute();
	// eslint-disable-next-line svelte/no-navigation-without-resolve
	await goto(route);
}
