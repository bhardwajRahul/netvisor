/**
 * TanStack Query hooks for Billing
 */

import { createQuery, createMutation } from '@tanstack/svelte-query';
import { queryKeys } from '$lib/api/query-client';
import { apiClient } from '$lib/api/client';
import type { BillingPlan, BillingRate } from './types';
import type { components } from '$lib/api/schema';
import { pushError, pushSuccess } from '$lib/shared/stores/feedback';

type PauseDuration = components['schemas']['PauseDuration'];
type CancelSubscriptionRequest = components['schemas']['CancelSubscriptionRequest'];
type CancelSubscriptionResponse = components['schemas']['CancelSubscriptionResponse'];

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
 * Mutation hook for pausing the subscription
 */
export function usePauseSubscriptionMutation() {
	return createMutation(() => ({
		mutationFn: async (duration_days: PauseDuration) => {
			const { data } = await apiClient.POST('/api/billing/pause', {
				body: { duration_days }
			});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to pause subscription');
			}
			return data.data;
		},
		// No onSuccess toast — the call site fires it AFTER waitForOrgUpdate
		// confirms the org actually flipped to paused. The API 200 only means
		// Stripe accepted the request, not that downstream state is consistent.
		onError: (error: Error) => {
			pushError(`Error pausing subscription: ${error.message}. Please try again.`);
		}
	}));
}

/**
 * Mutation hook for resuming a paused subscription
 */
export function useResumeSubscriptionMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/resume', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to resume subscription');
			}
			return data.data;
		},
		// No onSuccess toast — call site fires it after waitForOrgUpdate.
		onError: (error: Error) => {
			pushError(`Error resuming subscription: ${error.message}. Please try again.`);
		}
	}));
}

/**
 * Mutation hook for reactivating a subscription pending cancellation
 */
export function useReactivateSubscriptionMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/reactivate', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to reactivate subscription');
			}
			return data.data;
		},
		// No onSuccess toast — call site fires it after waitForOrgUpdate.
		onError: (error: Error) => {
			pushError(`Error reactivating subscription: ${error.message}. Please try again.`);
		}
	}));
}

/**
 * Mutation hook for self-serve trial extend (+7 days, once per lifetime)
 */
export function useExtendTrialMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/extend-trial', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to extend trial');
			}
			return data.data;
		},
		// No onSuccess toast — call site fires it after waitForOrgUpdate.
		onError: (error: Error) => {
			pushError(`Error extending trial: ${error.message}. Please try again.`);
		}
	}));
}

/**
 * Mutation hook for in-app subscription cancel.
 * Returns the period_end so the modal can render the retention disclosure.
 */
export function useCancelSubscriptionMutation() {
	return createMutation(() => ({
		mutationFn: async (request: CancelSubscriptionRequest): Promise<CancelSubscriptionResponse> => {
			const { data } = await apiClient.POST('/api/billing/cancel', { body: request });
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to cancel subscription');
			}
			return data.data;
		},
		onError: (error: Error) => {
			pushError(`Error cancelling subscription: ${error.message}. Please try again.`);
		}
	}));
}

/**
 * Query hook for the live save-offer coupon terms.
 * Returns null when STRIPE_SAVE_OFFER_COUPON_ID is unset — the cancel
 * modal hides the discount panel in that case.
 */
export function useSaveOfferCouponQuery(enabled: () => boolean = () => true) {
	return createQuery(() => ({
		queryKey: queryKeys.billing.saveOfferCoupon(),
		enabled: enabled(),
		queryFn: async () => {
			const { data } = await apiClient.GET('/api/billing/save-offer-coupon', {});
			if (!data?.success) {
				throw new Error(data?.error || 'Failed to read save-offer coupon');
			}
			return data.data ?? null;
		}
	}));
}

/**
 * Mutation hook for the discount save offer.
 * Server returns 400 with a clear message when STRIPE_SAVE_OFFER_COUPON_ID
 * is unset; the auto-toast pipeline surfaces the error.
 */
export function useApplyDiscountSaveOfferMutation() {
	return createMutation(() => ({
		mutationFn: async () => {
			const { data } = await apiClient.POST('/api/billing/cancel/apply-discount', {});
			if (!data?.success || !data.data) {
				throw new Error(data?.error || 'Failed to apply discount');
			}
			return data.data;
		},
		// No onSuccess toast — call site fires it after waitForOrgUpdate confirms
		// `org.last_discount_at` is populated, so success is tied to the actual
		// downstream write rather than the Stripe acknowledgement.
		onError: (error: Error) => {
			pushError(`Error applying discount: ${error.message}. Please try again.`);
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
