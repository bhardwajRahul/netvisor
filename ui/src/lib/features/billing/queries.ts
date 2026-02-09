/**
 * TanStack Query hooks for Billing
 */

import { createQuery, createMutation, useQueryClient } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { BillingPlan, BillingRate } from './types';
import { pushError, pushSuccess } from '$lib/shared/stores/feedback';

/**
 * Query hook for fetching current billing plans
 */
export function useBillingPlansQuery() {
	return createQuery(() => ({
		queryKey: queryKeys.billing.plans(),
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/billing/plans');
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to fetch billing plans');
			}
			return data.data;
		}
	}));
}

/**
 * Mutation hook for checkout
 */
export function useCheckoutMutation() {
	const queryClient = useQueryClient();
	return createMutation(() => ({
		mutationFn: async (plan: BillingPlan) => {
			const { data } = await apiClient.POST('/api/billing/checkout', {
				body: { plan, url: window.location.origin }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to get checkout URL');
			}
			return data.data;
		},
		onSuccess: (data: string) => {
			// Non-URL response means plan was changed directly (existing subscriber)
			if (!data.startsWith('http')) {
				pushSuccess(data);
				// Invalidate org query so UI reflects the new plan
				queryClient.invalidateQueries({ queryKey: queryKeys.organizations.all });
			}
		},
		onError: (error: Error) => {
			pushError(`Error changing plan: ${error.message}. Please try again.`);
		}
	}));
}

/**
 * Mutation hook for opening customer portal
 */
export function useCustomerPortalMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/portal', {
				body: window.location.origin
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to get billing portal URL');
			}
			return data.data;
		},
		onError: (error: Error) => {
			pushError(`Error getting billing portal URL: ${error.message}. Please try again.`);
		}
	}));
}

/**
 * Mutation hook for setting up payment method
 */
export function useSetupPaymentMethodMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/setup-payment-method', {
				body: { url: window.location.origin }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to get setup URL');
			}
			return data.data;
		},
		onError: (error: Error) => {
			pushError(`Error setting up payment method: ${error.message}. Please try again.`);
		}
	}));
}

/**
 * Mutation hook for changing plan
 */
export function useChangePlanMutation() {
	return createMutation(() => ({
		mutationFn: async ({ plan, rate }: { plan: BillingPlan; rate: BillingRate }) => {
			const { data } = await apiClient.POST('/api/billing/change-plan', {
				body: { plan, rate }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to change plan');
			}
			return data.data;
		},
		onSuccess: (data: string) => {
			pushSuccess(data);
		},
		onError: (error: Error) => {
			pushError(`Error changing plan: ${error.message}. Please try again.`);
		}
	}));
}

/**
 * Query hook for previewing plan change overage
 */
export function useChangePlanPreviewQuery(plan: () => BillingPlan | null) {
	return createQuery(() => ({
		queryKey: [...queryKeys.billing.plans(), 'preview', plan()],
		queryFn: async () => {
			const planValue = plan();
			if (!planValue) return null;
			const { data } = await apiClient.GET('/api/billing/change-plan/preview', {
				params: { query: { plan: JSON.stringify(planValue) } }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to get plan preview');
			}
			return data.data;
		},
		enabled: !!plan()
	}));
}
