<script lang="ts">
	import { CreditCard } from 'lucide-svelte';
	import AppBanner from './AppBanner.svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useSetupPaymentMethodMutation } from '$lib/features/billing/queries';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';
	import { getTrialDaysLeft, isTrialingWithoutPayment } from '$lib/shared/utils/trial';
	import { trackOncePerSession } from '$lib/shared/utils/analytics';
	import {
		billing_addPaymentMethod,
		billing_noPaymentMethodBannerBody
	} from '$lib/paraglide/messages';

	const organizationQuery = useOrganizationQuery();
	const setupPaymentMutation = useSetupPaymentMethodMutation();

	let org = $derived(organizationQuery.data);
	let trialDaysLeft = $derived(getTrialDaysLeft(org));

	// `plan_status` is non-null only for live Stripe self-serve subscriptions;
	// non-Stripe plans (Free/Community/Demo/SelfHosted/Enterprise) are null and
	// excluded automatically. The final clause defers to TrialEndingBanner in its
	// window (trialing + no card + <= 3 days) so the two never render together.
	let shouldShow = $derived(
		(org?.plan_status === 'trialing' ||
			org?.plan_status === 'active' ||
			org?.plan_status === 'past_due') &&
			!(org?.has_payment_method ?? false) &&
			!(isTrialingWithoutPayment(org) && trialDaysLeft !== null && trialDaysLeft <= 3)
	);

	$effect(() => {
		if (shouldShow) {
			trackOncePerSession('no_payment_banner_shown', 'no_payment_banner_shown', {
				plan_status: org?.plan_status
			});
		}
	});

	async function handleCta() {
		await startSetupPayment({
			mutation: setupPaymentMutation,
			org,
			source: 'trial_banner',
			trialDaysLeft
		});
	}
</script>

{#if shouldShow}
	<AppBanner variant="warning" icon={CreditCard} body={billing_noPaymentMethodBannerBody()}>
		{#snippet actions()}
			<button
				onclick={handleCta}
				disabled={setupPaymentMutation.isPending}
				class="ml-2 rounded px-2 py-0.5 text-xs font-medium underline hover:no-underline disabled:opacity-50"
			>
				{billing_addPaymentMethod()}
			</button>
		{/snippet}
	</AppBanner>
{/if}
