<script lang="ts">
	import { CheckCircle } from 'lucide-svelte';
	import AppBanner from './AppBanner.svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import { isPlanActivationRecent } from '$lib/shared/billing/plan-activation-marker';
	import { trackOncePerSession } from '$lib/shared/utils/analytics';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { billing_welcomeBannerBody } from '$lib/paraglide/messages';

	const organizationQuery = useOrganizationQuery();

	let org = $derived(organizationQuery.data);

	let shouldShow = $derived(
		isPlanActivationRecent() &&
			org != null &&
			isBillingPlanActive(org) &&
			org.plan_status === 'active'
	);

	let planName = $derived(org?.plan?.type ? billingPlans.getName(org.plan.type) : '');

	let body = $derived(billing_welcomeBannerBody({ planName: planName || '' }));

	$effect(() => {
		if (shouldShow) {
			trackOncePerSession('welcome_banner_shown', 'welcome_banner_shown', {
				plan: org?.plan?.type
			});
		}
	});

	function handleDismiss() {
		trackOncePerSession('welcome_banner_dismissed', 'welcome_banner_dismissed', {
			plan: org?.plan?.type
		});
	}
</script>

{#if shouldShow}
	<AppBanner
		variant="info"
		icon={CheckCircle}
		{body}
		dismissableKey="welcome_banner"
		onDismiss={handleDismiss}
	/>
{/if}
