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
	import {
		useCheckoutMutation,
		useCreateSetupIntentMutation,
		useFinalizePaymentMethodMutation
	} from '$lib/features/billing/queries';
	import StripeCardForm from '$lib/features/billing/StripeCardForm.svelte';
	import { billing_cardStepTitle, billing_cardStepSubtitle } from '$lib/paraglide/messages';
	import { onboardingStore } from '$lib/features/auth/stores/onboarding';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import PlanInquiryModal from '$lib/features/billing/PlanInquiryModal.svelte';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { waitForOrgUpdate } from '$lib/shared/billing/wait-for-org-update';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { upgradeContext } from '$lib/features/billing/stores';

	let {
		isOpen = false,
		dismissible = true,
		requireCard = false,
		onClose,
		name = undefined
	}: {
		isOpen?: boolean;
		dismissible?: boolean;
		/**
		 * Initial-signup gate: hide the Free plan so the user must pick a
		 * Stripe-managed plan (which then requires a card on file).
		 */
		requireCard?: boolean;
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
			.filter((p) => !(p.metadata.is_free && p.metadata.rate === 'Year'))
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
			((organization?.plan != null &&
				billingPlanHelpers.getMetadata(organization.plan.type)?.is_free !== true) ||
				!!organization?.trial_end_date)
	);

	// Mutations
	const checkoutMutation = useCheckoutMutation();
	const createSetupIntentMutation = useCreateSetupIntentMutation();
	const finalizeMutation = useFinalizePaymentMethodMutation();

	// In-app card-collection step: set after a Stripe-managed plan is selected
	// with no card on file. Holds the chosen plan and the SetupIntent secret.
	let cardStep = $state<{ plan: BillingPlan; clientSecret: string } | null>(null);

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
		const metadata = billingPlanHelpers.getMetadata(plan.type);
		trackEvent('plan_selected', {
			plan: plan.type,
			is_commercial: metadata?.is_commercial ?? false
		});

		const isFree = metadata?.is_free === true;
		const hasCard = organization?.has_payment_method ?? false;

		// Stripe-managed (paid or trial) plan with no card on file → collect a
		// card in-app via the Payment Element before creating the subscription.
		if (!isFree && !hasCard) {
			try {
				const clientSecret = await createSetupIntentMutation.mutateAsync();
				cardStep = { plan, clientSecret };
			} catch {
				// setup-intent error already surfaced by the mutation's toast
			}
			return;
		}

		// Free, or a card is already on file — activate directly.
		try {
			await proceedCheckout(plan);
		} catch {
			// checkout error already surfaced by the mutation's toast
		}
	}

	// Called by StripeCardForm once the card is confirmed: persist it as the
	// default payment method, then create the subscription for the chosen plan.
	async function handleCardConfirmed(setupIntentId: string) {
		const plan = cardStep?.plan;
		if (!plan) return;
		// Both throw on failure (and toast); StripeCardForm catches and re-enables
		// the form so the user can retry.
		await finalizeMutation.mutateAsync(setupIntentId);
		await proceedCheckout(plan);
	}

	async function proceedCheckout(plan: BillingPlan) {
		// Backend decides: trial / paid-with-card-on-file / Free all activate
		// in-app and return a message; only a no-card edge path returns a Stripe
		// Checkout URL, handled here as a defensive same-tab fallback.
		const result = await checkoutMutation.mutateAsync(plan);
		if (result?.startsWith('http')) {
			window.location.href = result;
			return;
		}
		upgradeContext.set(null);
		cardStep = null;
		onClose();
		// Activation is webhook-driven, so a single refetch races the webhook and
		// reads stale state. Poll until the org reflects the active plan. Closing
		// first is safe: onClose sets planJustActivated, suppressing reopen.
		void waitForOrgUpdate(isBillingPlanActive);
	}

	// Plan inquiry modal state
	let inquiryModalOpen = $state(false);
	let selectedPlan = $state<BillingPlan | null>(null);

	function handlePlanInquiry(plan: BillingPlan) {
		selectedPlan = plan;
		inquiryModalOpen = true;
	}

	// Free is hidden for paid orgs (can't "select" Free as an upgrade) and during
	// the initial-signup gate (`requireCard`), where a card-backed plan is required.
	let availablePlans = $derived(
		billingPlanHelpers.getMetadata(organization?.plan?.type ?? null)?.is_free && !requireCard
			? plansData
			: plansData.filter((p) => billingPlanHelpers.getMetadata(p.type)?.is_free !== true)
	);
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
			plans={availablePlans}
			{billingPlanHelpers}
			{featureHelpers}
			onPlanSelect={handlePlanSelect}
			onPlanInquiry={handlePlanInquiry}
			{recommendedPlan}
			{isReturningCustomer}
			{isCurrentlyTrialing}
		/>
	</div>

	<!-- Card collection: a normal-chromed dialog stacked over the (borderless,
	     full-screen) plan grid — mirrors the nested PlanInquiryModal pattern. -->
	{#if cardStep}
		<GenericModal
			isOpen={true}
			title={billing_cardStepTitle()}
			size="md"
			showCloseButton={true}
			onClose={() => (cardStep = null)}
		>
			<StripeCardForm
				clientSecret={cardStep.clientSecret}
				description={billing_cardStepSubtitle({
					planName: billingPlanHelpers.getName(cardStep.plan.type)
				})}
				onSuccess={handleCardConfirmed}
				onCancel={() => (cardStep = null)}
			/>
		</GenericModal>
	{/if}

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
