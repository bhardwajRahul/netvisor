import { trackEvent } from '$lib/shared/utils/analytics';
import { openModal } from '$lib/shared/stores/modal-registry';
import type { Organization } from '$lib/features/organizations/types';

interface StartSetupPaymentArgs {
	org: Organization | null | undefined;
	source: 'trial_card' | 'trial_banner' | 'trial_modal' | 'sidebar_trial_pill' | 'billing_tab';
	trialDaysLeft: number | null;
}

/**
 * Open the in-app payment-method dialog (Stripe Elements). Every "Add/Update
 * payment method" nudge funnels through here, so card collection happens in a
 * modal instead of redirecting out to a Stripe-hosted page.
 */
export function startSetupPayment({ org, source, trialDaysLeft }: StartSetupPaymentArgs): void {
	trackEvent('add_payment_cta_clicked', {
		source,
		plan_status: org?.plan_status,
		trial_days_left: trialDaysLeft,
		has_payment_method: org?.has_payment_method ?? false
	});
	openModal('payment-method');
}
