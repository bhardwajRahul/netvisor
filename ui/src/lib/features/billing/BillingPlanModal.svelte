<script lang="ts">
	import billingPlansJson from '$lib/data/billing-plans.json';
	import featuresJson from '$lib/data/features.json';
	import BillingPlanForm from '$lib/features/billing/BillingPlanForm.svelte';
	import type { BillingPlan } from '$lib/features/billing/types';
	import {
		createStaticHelpers,
		type BillingPlanMetadata,
		type FeatureMetadata
	} from '$lib/shared/stores/metadata';
	import { useCheckoutMutation } from '$lib/features/billing/queries';
	import { onboardingStore } from '$lib/features/auth/stores/onboarding';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import PlanInquiryModal from '$lib/features/billing/PlanInquiryModal.svelte';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';

	let {
		isOpen = false,
		dismissible = true,
		onClose
	}: {
		isOpen?: boolean;
		dismissible?: boolean;
		onClose: () => void;
	} = $props();

	// Create helpers from static fixtures (no API calls needed)
	const billingPlanHelpers = createStaticHelpers<BillingPlanMetadata>(billingPlansJson);
	const featureHelpers = createStaticHelpers<FeatureMetadata>(featuresJson);

	// Transform fixture data to BillingPlan[] format (exclude self-hosted plans, deduplicate)
	const plansData = (() => {
		const seen = new Set<string>(); // eslint-disable-line svelte/prefer-svelte-reactivity
		return billingPlansJson
			.filter((p) => p.metadata.hosting !== 'SelfHosted')
			.filter((p) => !(p.id === 'Free' && p.metadata.rate === 'Year'))
			.map(
				(p) =>
					({
						type: p.id,
						base_cents: p.metadata.base_cents,
						rate: p.metadata.rate,
						trial_days: p.metadata.trial_days,
						seat_cents: p.metadata.seat_cents,
						network_cents: p.metadata.network_cents,
						included_seats: p.metadata.included_seats,
						included_networks: p.metadata.included_networks,
						host_cents: p.metadata.host_cents ?? null,
						included_hosts: p.metadata.included_hosts ?? null
					}) as BillingPlan
			)
			.filter((p) => {
				const key = `${p.type}-${p.rate}`;
				if (seen.has(key)) return false;
				seen.add(key);
				return true;
			});
	})();

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Only show trial offers to orgs that have never trialed a paid plan.
	// trial_end_date is set by Stripe webhook only for subscriptions with trial periods
	// (Free plan has trial_days=0, so it never sets trial_end_date).
	let isReturningCustomer = $derived(!!organization?.trial_end_date);

	// Mutations
	const checkoutMutation = useCheckoutMutation();

	// Determine initial filter based on use case from onboarding
	let useCase = $derived($onboardingStore.useCase);

	let initialPlanFilter = $derived<'commercial' | 'personal'>(
		useCase === 'company' || useCase === 'msp' ? 'commercial' : 'personal'
	);

	// Recommended plan based on use case
	let recommendedPlan = $derived<string | null>(
		useCase === 'company' ? 'Team' : useCase === 'msp' ? 'Business' : null
	);

	async function handlePlanSelect(plan: BillingPlan) {
		try {
			// Track plan selection
			const metadata = billingPlanHelpers.getMetadata(plan.type);
			trackEvent('plan_selected', {
				plan: plan.type,
				is_commercial: metadata?.is_commercial ?? false
			});

			// All plans go through Stripe checkout (including Free)
			const checkoutUrl = await checkoutMutation.mutateAsync(plan);
			if (checkoutUrl) {
				window.location.href = checkoutUrl;
			}
		} catch {
			// Error handled by mutation
		}
	}

	// Plan inquiry modal state
	let inquiryModalOpen = $state(false);
	let selectedPlan = $state<BillingPlan | null>(null);

	function handlePlanInquiry(plan: BillingPlan) {
		selectedPlan = plan;
		inquiryModalOpen = true;
	}
</script>

<GenericModal
	{isOpen}
	title=""
	onClose={dismissible ? onClose : null}
	size="full"
	preventCloseOnClickOutside={!dismissible}
	showCloseButton={dismissible}
>
	<div class="p-2">
		<BillingPlanForm
			plans={plansData}
			{billingPlanHelpers}
			{featureHelpers}
			onPlanSelect={handlePlanSelect}
			onPlanInquiry={handlePlanInquiry}
			{initialPlanFilter}
			{recommendedPlan}
			{isReturningCustomer}
		/>
	</div>

	<PlanInquiryModal
		isOpen={inquiryModalOpen}
		planName={selectedPlan ? billingPlanHelpers.getName(selectedPlan.type) : ''}
		planType={selectedPlan?.type ?? ''}
		userEmail={currentUser?.email ?? ''}
		orgName={organization?.name ?? ''}
		companySize={$onboardingStore.companySize ?? ''}
		onClose={() => (inquiryModalOpen = false)}
	/>
</GenericModal>
