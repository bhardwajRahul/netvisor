import { storeEventForAfterRedirect, trackEvent } from '$lib/shared/utils/analytics';
import type { useSetupPaymentMethodMutation } from '$lib/features/billing/queries';
import type { Organization } from '$lib/features/organizations/types';

type SetupPaymentMutation = ReturnType<typeof useSetupPaymentMethodMutation>;

interface StartSetupPaymentArgs {
	mutation: SetupPaymentMutation;
	org: Organization | null | undefined;
	source: 'trial_card' | 'trial_banner' | 'trial_modal';
	trialDaysLeft: number | null;
}

export async function startSetupPayment({
	mutation,
	org,
	source,
	trialDaysLeft
}: StartSetupPaymentArgs): Promise<void> {
	trackEvent(`${source}_cta_clicked`, {
		trial_days_left: trialDaysLeft,
		has_payment_method: org?.has_payment_method ?? false
	});
	storeEventForAfterRedirect('payment_method_setup_initiated', {
		plan_status: org?.plan_status,
		trial_days_left: trialDaysLeft,
		source
	});
	try {
		const url = await mutation.mutateAsync();
		if (url) {
			window.location.href = url;
		}
	} catch {
		// Error handling is done by the mutation's onError
	}
}
