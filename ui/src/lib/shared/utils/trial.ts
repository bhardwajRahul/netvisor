import type { Organization } from '$lib/features/organizations/types';

export function getTrialEndDate(org: Organization | null | undefined): Date | null {
	return org?.trial_end_date ? new Date(org.trial_end_date) : null;
}

export function getTrialDaysLeft(org: Organization | null | undefined): number | null {
	const end = getTrialEndDate(org);
	if (!end) return null;
	const diff = end.getTime() - Date.now();
	return Math.max(0, Math.ceil(diff / (1000 * 60 * 60 * 24)));
}

export function isTrialingWithoutPayment(org: Organization | null | undefined): boolean {
	return org?.plan_status === 'trialing' && !(org?.has_payment_method ?? false);
}

export function getDaysIntoTrial(org: Organization | null | undefined): number | null {
	if (!org?.created_at) return null;
	const created = new Date(org.created_at).getTime();
	if (Number.isNaN(created)) return null;
	const diff = Date.now() - created;
	return Math.max(0, Math.floor(diff / (1000 * 60 * 60 * 24)));
}
