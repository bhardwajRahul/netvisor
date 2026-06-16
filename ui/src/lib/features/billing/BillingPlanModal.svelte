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
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { upgradeContext } from '$lib/features/billing/stores';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';

	let {
		isOpen = false,
		dismissible = true,
		onClose,
		name = undefined
	}: {
		isOpen?: boolean;
		dismissible?: boolean;
		onClose: () => void;
		name?: string;
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

	let isCurrentlyTrialing = $derived(organization?.plan_status === 'trialing');

	// Only show trial offers to orgs that have never had a non-Free paid plan and never trialed.
	// trial_end_date is set by Stripe webhook only for subscriptions with trial periods
	// (Free plan has trial_days=0, so it never sets trial_end_date).
	// Trialing users are NOT returning — they should see trial-aware UI instead.
	let isReturningCustomer = $derived(
		!isCurrentlyTrialing &&
			((organization?.plan != null && organization.plan.type !== 'Free') ||
				!!organization?.trial_end_date)
	);

	// Mutations
	const queryClient = useQueryClient();
	const checkoutMutation = useCheckoutMutation();

	// Determine initial filter based on use case from onboarding
	let useCase = $derived($onboardingStore.useCase);

	// Recommended plan based on use case
	let baseRecommendedPlan = $derived<string | null>(
		useCase === 'internal_it' ? 'Team' : useCase === 'msp' ? 'Business' : null
	);

	// Feature-contextual plan highlighting from upgrade CTAs
	let upgradeCtx = $derived($upgradeContext);

	let contextHighlightPlan = $derived.by(() => {
		if (!upgradeCtx) return null;
		const feat = upgradeCtx.feature;
		// Feature-based: look up minimum_plan from feature metadata
		const featureMeta = featureHelpers.getMetadata(feat);
		if (featureMeta?.minimum_plan) return featureMeta.minimum_plan;
		// Resource-based: find first plan with addon pricing
		if (feat === 'seats') return plansData.find((p) => p.seat_cents)?.type ?? null;
		if (feat === 'networks') return plansData.find((p) => p.network_cents)?.type ?? null;
		if (feat === 'hosts') return plansData.find((p) => p.host_cents)?.type ?? null;
		return null;
	});

	let recommendedPlan = $derived(contextHighlightPlan ?? baseRecommendedPlan);

	async function handlePlanSelect(plan: BillingPlan) {
		// Open the tab synchronously (inside the click) so popup blockers allow it;
		// only the http (Stripe Checkout) branch uses it. (No 'noopener' — that makes
		// window.open return null, losing the handle.)
		const stripeTab = window.open('', '_blank');
		try {
			// New tab — this tab stays put, so track immediately rather than stashing
			// the event for a post-redirect flush.
			const metadata = billingPlanHelpers.getMetadata(plan.type);
			trackEvent('plan_selected', {
				plan: plan.type,
				is_commercial: metadata?.is_commercial ?? false
			});

			// Backend decides: new subscriber → checkout URL, existing → plan change message
			const result = await checkoutMutation.mutateAsync(plan);
			if (result?.startsWith('http')) {
				// First-time checkout: open Stripe in a new tab and close the modal. This
				// tab converges once the checkout webhook activates the plan.
				if (stripeTab) {
					stripeTab.location.href = result;
					upgradeContext.set(null);
					onClose();
					void waitForOrgUpdate(isBillingPlanActive);
				} else {
					// Popup blocked — fall back to a same-tab redirect.
					window.location.href = result;
				}
			} else {
				// Direct activation needs no Stripe tab.
				stripeTab?.close();
				// Plan activated directly (Free or trial) — refetch org so needsPlanSelection
				// becomes false before we close the modal, preventing reactive reopening.
				await queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
				upgradeContext.set(null);
				onClose();
			}
		} catch {
			// Error handled by mutation
			stripeTab?.close();
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
	{name}
	onClose={dismissible
		? () => {
				upgradeContext.set(null);
				onClose();
			}
		: null}
	size="max"
	preventCloseOnClickOutside={!dismissible}
	showCloseButton={false}
	floatingCloseButton={dismissible}
	borderless={true}
	compactPadding={true}
>
	<div class="flex min-h-0 flex-1 flex-col">
		<BillingPlanForm
			plans={organization?.plan?.type === 'Free'
				? plansData
				: plansData.filter((p) => p.type !== 'Free')}
			{billingPlanHelpers}
			{featureHelpers}
			onPlanSelect={handlePlanSelect}
			onPlanInquiry={handlePlanInquiry}
			{recommendedPlan}
			{isReturningCustomer}
			{isCurrentlyTrialing}
		/>
	</div>

	<PlanInquiryModal
		isOpen={inquiryModalOpen}
		planName={selectedPlan ? billingPlanHelpers.getName(selectedPlan.type) : ''}
		planType={selectedPlan?.type ?? ''}
		userEmail={currentUser?.email ?? ''}
		orgName={organization?.name ?? ''}
		companySize=""
		onClose={() => (inquiryModalOpen = false)}
	/>
</GenericModal>
