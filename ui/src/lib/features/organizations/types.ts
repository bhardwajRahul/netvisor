import type { components } from '$lib/api/schema';

// Re-export generated types
export type Organization = components['schemas']['Organization'];
export type OrganizationInvite = components['schemas']['Invite'];
export type CreateInviteRequest = components['schemas']['CreateInviteRequest'];

export function isStripeManagedPlan(organization: Organization): boolean {
	const type = organization.plan?.type;
	return type != null && type !== 'Demo' && type !== 'Community' && type !== 'CommercialSelfHosted';
}

export function isBillingPlanActive(organization: Organization) {
	if (!isStripeManagedPlan(organization)) {
		return true;
	}
	return (
		organization.plan_status == 'active' ||
		organization.plan_status == 'trialing' ||
		organization.plan_status == 'pending_cancellation' ||
		organization.plan_status == 'past_due' ||
		organization.plan_status == 'paused'
	);
}
