import { storeEventForAfterRedirect } from '$lib/shared/utils/analytics';
import type { useSetupPaymentMethodMutation } from '$lib/features/billing/queries';
import type { Organization } from '$lib/features/organizations/types';

type SetupPaymentMutation = ReturnType<typeof useSetupPaymentMethodMutation>;

interface StartSetupPaymentArgs {
	mutation: SetupPaymentMutation;
	org: Organization | null | undefined;
	source: 'trial_card' | 'trial_banner' | 'trial_modal' | 'sidebar_trial_pill';
	trialDaysLeft: number | null;
}

export async function startSetupPayment({
	mutation,
	org,
	source,
	trialDaysLeft
}: StartSetupPaymentArgs): Promise<void> {
	storeEventForAfterRedirect('add_payment_cta_clicked', {
		source,
		plan_status: org?.plan_status,
		trial_days_left: trialDaysLeft,
		has_payment_method: org?.has_payment_method ?? false
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
