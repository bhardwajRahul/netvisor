import type { components } from '$lib/api/schema';

// Re-export generated types
export type Organization = components['schemas']['Organization'];
export type OrganizationInvite = components['schemas']['Invite'];
export type CreateInviteRequest = components['schemas']['CreateInviteRequest'];

export function isBillingPlanActive(organization: Organization) {
	// Demo and other non-Stripe plans are always considered active
	if (organization.plan?.type === 'Demo' || organization.plan?.type === 'Community' || organization.plan?.type === 'CommercialSelfHosted') {
		return true;
	}
	return (
		organization.plan_status == 'active' ||
		organization.plan_status == 'trialing' ||
		organization.plan_status == 'pending_cancellation' ||
		organization.plan_status == 'past_due'
	);
}
