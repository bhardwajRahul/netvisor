<script lang="ts">
	import billingPlansJson from '$lib/data/billing-plans.json';
	import featuresJson from '$lib/data/features.json';
	import Toast from '$lib/shared/components/feedback/Toast.svelte';
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

	// Create helpers from static fixtures (no API calls needed)
	const billingPlanHelpers = createStaticHelpers<BillingPlanMetadata>(billingPlansJson);
	const featureHelpers = createStaticHelpers<FeatureMetadata>(featuresJson);

	// Transform fixture data to BillingPlan[] format (exclude self-hosted plans)
	const plansData = billingPlansJson
		.filter((p) => p.metadata.hosting !== 'SelfHosted')
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
					included_networks: p.metadata.included_networks
				}) as BillingPlan
		);

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Returning customers (have had a subscription) shouldn't see trial offers
	let isReturningCustomer = $derived(!!organization?.plan_status);

	// Checkout mutation
	const checkoutMutation = useCheckoutMutation();

	// Determine initial filter based on use case from onboarding
	// homelab = personal, company/msp = commercial
	let useCase = $derived($onboardingStore.useCase);
	let networkCount = $derived($onboardingStore.networks.length);

	// If user has > 3 networks, they must use commercial plans
	let forceCommercial = $derived(networkCount > 3);

	let initialPlanFilter = $derived<'commercial' | 'personal'>(
		forceCommercial || useCase === 'company' || useCase === 'msp' ? 'commercial' : 'personal'
	);

	// Recommended plan based on use case
	// company → Team, msp → Business
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

<div class="relative min-h-dvh bg-gray-900">
	<!-- Background image with overlay -->
	<div class="absolute inset-0 z-0">
		<div
			class="h-full w-full bg-cover bg-center bg-no-repeat"
			style="background-image: url('/images/diagram.png')"
		></div>
		<div class="absolute inset-0 bg-black/70"></div>
	</div>

	<!-- Content (sits above background) -->
	<section class="py-10 pb-24 lg:pb-10">
		<div class="container mx-auto px-2">
			<BillingPlanForm
				plans={plansData}
				{billingPlanHelpers}
				{featureHelpers}
				onPlanSelect={handlePlanSelect}
				onPlanInquiry={handlePlanInquiry}
				{initialPlanFilter}
				{recommendedPlan}
				{forceCommercial}
				{isReturningCustomer}
			/>
		</div>
	</section>

	<Toast />

	<PlanInquiryModal
		isOpen={inquiryModalOpen}
		planName={selectedPlan ? billingPlanHelpers.getName(selectedPlan.type) : ''}
		planType={selectedPlan?.type ?? ''}
		userEmail={currentUser?.email ?? ''}
		onClose={() => (inquiryModalOpen = false)}
	/>
</div>
