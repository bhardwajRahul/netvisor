import type { components } from '$lib/api/schema';

// Re-export generated types
export type BillingPlan = components['schemas']['BillingPlan'];
export type BillingRate = components['schemas']['BillingRate'];

export function formatPrice(cents: number, rate: string): string {
	return `${cents / 100} per ${rate}`;
}
