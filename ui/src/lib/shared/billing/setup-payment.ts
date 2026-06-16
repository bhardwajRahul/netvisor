import { trackEvent } from '$lib/shared/utils/analytics';
import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
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
	// The originating tab no longer navigates away (Stripe opens in a new tab),
	// so fire the event immediately rather than stashing it for after a redirect.
	trackEvent('add_payment_cta_clicked', {
		source,
		plan_status: org?.plan_status,
		trial_days_left: trialDaysLeft,
		has_payment_method: org?.has_payment_method ?? false
	});
	// Open the tab synchronously (inside the click) so popup blockers allow it,
	// then point it at the Stripe URL once the session resolves. (No 'noopener' —
	// that makes window.open return null, losing the handle.)
	const stripeTab = window.open('', '_blank');
	try {
		const url = await mutation.mutateAsync();
		if (!url) {
			stripeTab?.close();
			return;
		}
		if (stripeTab) {
			stripeTab.location.href = url;
			// Converge this tab once the webhook records the new payment method.
			void waitForOrgUpdate((o) => o.has_payment_method ?? false);
		} else {
			// Popup blocked — fall back to a same-tab redirect (the AppShell return
			// handler confirms on the way back).
			window.location.href = url;
		}
	} catch {
		stripeTab?.close();
		// Error handling is done by the mutation's onError
	}
}
