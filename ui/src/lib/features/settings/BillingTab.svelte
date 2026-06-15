<script lang="ts">
	import { CheckCircle, AlertCircle, CreditCard, AlertTriangle } from 'lucide-svelte';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import {
		trackEvent,
		storeEventForAfterRedirect,
		trackOncePerSession
	} from '$lib/shared/utils/analytics';
	import {
		useCustomerPortalMutation,
		useSetupPaymentMethodMutation,
		useResumeSubscriptionMutation,
		useReactivateSubscriptionMutation,
		useExtendTrialMutation
	} from '$lib/features/billing/queries';
	import CancelSubscriptionModal from '$lib/features/billing/CancelSubscriptionModal.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import { useDashboardQuery } from '$lib/features/home/queries';
	import {
		common_billingExtra,
		common_billingUsage,
		common_close,
		common_included,
		common_networks,
		common_seats,
		common_tryAgainLater,
		settings_billing_billingQuestions,
		settings_billing_canceled,
		settings_billing_contactUs,
		settings_billing_currentPlan,
		settings_billing_downgrade_pending,
		settings_billing_manageSubscription,
		settings_billing_needHelp,
		settings_billing_pastDue,
		settings_billing_paused_status,
		settings_billing_cancelSubscription,
		settings_billing_per,
		settings_billing_trialActive,
		settings_billing_unableToLoad,
		settings_billing_upgradePlan,
		settings_billing_changePlan,
		settings_billing_resume_button,
		settings_billing_resume_confirmBody,
		settings_billing_reactivateSubscription,
		settings_billing_extendTrial_link,
		settings_billing_extendTrial_confirmBody
	} from '$lib/paraglide/messages';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import { startSetupPayment } from '$lib/shared/billing/setup-payment';

	let {
		isOpen = false,
		onClose,
		dismissible = true
	}: {
		isOpen?: boolean;
		onClose: () => void;
		dismissible?: boolean;
	} = $props();

	// Dashboard summary aggregates host/network/seat counts into one query —
	// reuse it here instead of re-counting users/networks/hosts independently.
	const dashboardQuery = useDashboardQuery();
	let planUsage = $derived(dashboardQuery.data?.plan_usage);

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);

	// Customer portal mutation
	const customerPortalMutation = useCustomerPortalMutation();
	const setupPaymentMutation = useSetupPaymentMethodMutation();
	const resumeMutation = useResumeSubscriptionMutation();
	const reactivateMutation = useReactivateSubscriptionMutation();
	const extendTrialMutation = useExtendTrialMutation();

	// Cancel modal state. Replaces the legacy Stripe-Portal handoff.
	let showCancelModal = $state(false);

	let seatCount = $derived(planUsage?.seat_count ?? 0);
	let networkCount = $derived(planUsage?.network_count ?? 0);
	let hostCount = $derived(planUsage?.host_count ?? 0);

	let extraSeats = $derived.by(() => {
		if (!org?.plan?.included_seats) return 0;
		return Math.max(seatCount - org.plan.included_seats, 0);
	});

	let extraNetworks = $derived.by(() => {
		if (!org?.plan?.included_networks) return 0;
		return Math.max(networkCount - org.plan.included_networks, 0);
	});

	let extraSeatsCents = $derived(extraSeats * (org?.plan?.seat_cents || 0));
	let extraNetworksCents = $derived(extraNetworks * (org?.plan?.network_cents || 0));

	let planActive = $derived(org ? isBillingPlanActive(org) : false);

	function formatPlanStatus(status: string): string {
		if (status === 'pending_cancellation') return 'Downgrading';
		return status.charAt(0).toUpperCase() + status.slice(1);
	}

	function getPlanStatusColor(status: string): string {
		switch (status.toLowerCase()) {
			case 'active':
				return 'text-green-600 dark:text-green-400';
			case 'trialing':
				return 'text-blue-600 dark:text-blue-400';
			case 'past_due':
			case 'unpaid':
				return 'text-red-600 dark:text-red-400';
			case 'pending_cancellation':
				return 'text-amber-600 dark:text-amber-400';
			case 'paused':
				return 'text-orange-600 dark:text-orange-400';
			case 'cancelled':
				return 'text-yellow-600 dark:text-yellow-400';
			default:
				return 'text-gray-600 dark:text-gray-400';
		}
	}

	let isFree = $derived(org?.plan?.type === 'Free');
	let hasPaymentMethod = $derived(org?.has_payment_method ?? false);
	let trialEndDate = $derived(org?.trial_end_date ? new Date(org.trial_end_date) : null);
	let trialDaysLeft = $derived.by(() => {
		if (!trialEndDate) return null;
		const now = new Date();
		const diff = trialEndDate.getTime() - now.getTime();
		return Math.max(0, Math.ceil(diff / (1000 * 60 * 60 * 24)));
	});

	// Track billing tab view
	$effect(() => {
		if (isOpen && org) {
			trackEvent('billing_tab_viewed', {
				plan_type: org.plan?.type,
				plan_status: org.plan_status
			});
		}
	});

	// Trial-card impression: once per browser tab session
	$effect(() => {
		if (org?.plan_status === 'trialing' && trialDaysLeft !== null && !hasPaymentMethod) {
			trackOncePerSession('trial_card_impression', 'trial_card_impression', {
				trial_days_left: trialDaysLeft,
				has_payment_method: hasPaymentMethod
			});
		}
	});

	async function handleManageSubscription() {
		storeEventForAfterRedirect('billing_portal_opened', { plan_type: org?.plan?.type });
		try {
			const url = await customerPortalMutation.mutateAsync();
			if (url) {
				window.location.href = url;
			}
		} catch {
			// Error handling is done by the mutation's onError
		}
	}

	function openCancelModal() {
		trackEvent('cancel_modal_opened', { plan_type: org?.plan?.type });
		showCancelModal = true;
	}

	async function handleResume() {
		if (!confirm(settings_billing_resume_confirmBody())) return;
		try {
			await resumeMutation.mutateAsync();
			organizationQuery.refetch();
		} catch {
			// Mutation onError handles toast.
		}
	}

	async function handleReactivate() {
		try {
			await reactivateMutation.mutateAsync();
			organizationQuery.refetch();
		} catch {
			// Mutation onError handles toast.
		}
	}

	async function handleExtendTrial() {
		if (!confirm(settings_billing_extendTrial_confirmBody())) return;
		try {
			await extendTrialMutation.mutateAsync();
			organizationQuery.refetch();
		} catch {
			// Mutation onError handles toast.
		}
	}

	function handleSetupPayment() {
		return startSetupPayment({
			mutation: setupPaymentMutation,
			org,
			source: 'trial_card',
			trialDaysLeft
		});
	}
</script>

<div class="flex min-h-0 flex-1 flex-col">
	<div class="flex-1 overflow-auto p-6">
		{#if org}
			<div class="space-y-6">
				<!-- Trial Countdown (shown above current plan when trialing without payment) -->
				{#if org.plan_status === 'trialing' && trialDaysLeft !== null && !hasPaymentMethod}
					<InfoCard>
						<div class="flex items-center justify-between">
							<div class="flex items-center gap-3">
								<AlertTriangle class="h-5 w-5 text-amber-500" />
								<div>
									<p class="text-primary text-sm font-medium">
										Trial ends in {trialDaysLeft} days ({trialEndDate?.toLocaleDateString(
											undefined,
											{ month: 'long', day: 'numeric', year: 'numeric' }
										)})
									</p>
									<p class="text-secondary mt-1 text-xs">
										Add a payment method to continue after the trial
									</p>
								</div>
							</div>
							<button
								onclick={handleSetupPayment}
								class="btn-primary flex items-center gap-1.5 text-sm"
							>
								<CreditCard size={14} />
								Add Payment Method
							</button>
						</div>
						{#if !org.trial_extended_used && trialDaysLeft !== null && trialDaysLeft <= 3}
							<div class="mt-3 border-t pt-3" style="border-color: var(--color-border)">
								<button
									type="button"
									onclick={handleExtendTrial}
									class="text-link text-sm hover:underline disabled:opacity-50"
									disabled={extendTrialMutation.isPending}
								>
									{settings_billing_extendTrial_link()}
								</button>
							</div>
						{/if}
					</InfoCard>
				{/if}

				<!-- Current Plan -->
				<InfoCard>
					<svelte:fragment slot="default">
						<div class="mb-3 flex items-center justify-between">
							<h3 class="text-primary text-sm font-semibold">{settings_billing_currentPlan()}</h3>
							<div class="flex items-center gap-2">
								{#if planActive}
									<CheckCircle class="h-4 w-4 text-green-600 dark:text-green-400" />
								{:else}
									<AlertCircle class="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
								{/if}
								<span class={`text-sm font-medium ${getPlanStatusColor(org.plan_status || '')}`}>
									{formatPlanStatus(org.plan_status || '')}
								</span>
							</div>
						</div>

						<div class="space-y-4">
							{#if org.plan}
								<!-- Base Plan -->
								<div class="flex items-baseline justify-between">
									<div>
										<p class="text-primary text-lg font-semibold">
											{billingPlans.getName(org.plan.type || null)}
										</p>
										{#if org.plan_status === 'trialing' && trialEndDate}
											<p class="text-secondary mt-1 text-xs">
												Trial ends on {trialEndDate.toLocaleDateString(undefined, {
													month: 'long',
													day: 'numeric',
													year: 'numeric'
												})}
											</p>
										{/if}
									</div>
									<div class="text-right">
										<p class="text-primary text-2xl font-bold">
											${org.plan.base_cents / 100}
										</p>
										<p class="text-secondary text-xs">
											{settings_billing_per({ rate: org.plan.rate })}
										</p>
									</div>
								</div>

								<!-- Seats Usage -->
								{#if org.plan.included_seats !== null}
									<div class="border-t pt-3" style="border-color: var(--color-border)">
										<div class="flex items-baseline justify-between">
											<div>
												<p class="text-primary font-medium">{common_seats()}</p>
												<p class="text-secondary text-sm">
													{common_billingUsage({
														count: seatCount,
														included: org.plan.included_seats ?? 0
													})}
													{#if extraSeats > 0}
														{common_billingExtra({
															extra: extraSeats,
															price: org.plan.seat_cents ? org.plan.seat_cents / 100 : 0
														})}
													{:else})
													{/if}
												</p>
											</div>
											{#if extraSeatsCents > 0}
												<div class="text-right">
													<p class="text-primary text-xl font-bold">
														+${extraSeatsCents / 100}
													</p>
													<p class="text-secondary text-xs">
														{settings_billing_per({ rate: org.plan.rate })}
													</p>
												</div>
											{:else}
												<p class="text-tertiary text-sm">{common_included()}</p>
											{/if}
										</div>
									</div>
								{/if}

								<!-- Networks Usage -->
								{#if org.plan.included_networks !== null}
									<div class="border-t pt-3" style="border-color: var(--color-border)">
										<div class="flex items-baseline justify-between">
											<div>
												<p class="text-primary font-medium">{common_networks()}</p>
												<p class="text-secondary text-sm">
													{common_billingUsage({
														count: networkCount,
														included: org.plan.included_networks ?? 0
													})}
													{#if extraNetworks > 0}
														{common_billingExtra({
															extra: extraNetworks,
															price: org.plan.network_cents ? org.plan.network_cents / 100 : 0
														})}
													{:else})
													{/if}
												</p>
											</div>
											{#if extraNetworksCents > 0}
												<div class="text-right">
													<p class="text-primary text-xl font-bold">
														+${extraNetworksCents / 100}
													</p>
													<p class="text-secondary text-xs">
														{settings_billing_per({ rate: org.plan.rate })}
													</p>
												</div>
											{:else}
												<p class="text-tertiary text-sm">{common_included()}</p>
											{/if}
										</div>
									</div>
								{/if}

								<!-- Hosts Usage -->
								{#if org.plan.included_hosts !== null}
									<div class="border-t pt-3" style="border-color: var(--color-border)">
										<div class="flex items-baseline justify-between">
											<div>
												<p class="text-primary font-medium">Hosts</p>
												<p class="text-secondary text-sm">
													{hostCount} / {org.plan.included_hosts} used
												</p>
											</div>
											{#if hostCount >= (org.plan.included_hosts ?? 0)}
												<p class="text-sm text-amber-600 dark:text-amber-400">At limit</p>
											{:else}
												<p class="text-tertiary text-sm">{common_included()}</p>
											{/if}
										</div>
										{#if hostCount > 0}
											<ProgressTrack
												class="mt-2 w-full"
												progress={Math.min(100, (hostCount / (org.plan.included_hosts || 1)) * 100)}
												color={hostCount >= (org.plan.included_hosts ?? 0)
													? 'bg-amber-500'
													: 'bg-blue-500'}
											/>
										{/if}
									</div>
								{/if}
							{/if}

							{#if org.plan_status === 'trialing'}
								<InlineInfo title={settings_billing_trialActive()} />
							{:else if org.plan_status === 'past_due'}
								<InlineDanger title={settings_billing_pastDue()} />
							{:else if org.plan_status === 'cancelled'}
								<InlineWarning title={settings_billing_canceled()} />
							{:else if org.plan_status === 'pending_cancellation'}
								<InlineWarning title={settings_billing_downgrade_pending()} />
							{:else if org.plan_status === 'paused'}
								<InlineInfo title={settings_billing_paused_status()} />
							{/if}

							<button
								onclick={() =>
									triggerUpgrade({
										source: 'settings_billing',
										surface: 'billing_tab',
										reopenSettings: true,
										beforeModal: () => onClose()
									})}
								class="btn-primary w-full"
							>
								{isFree ? settings_billing_upgradePlan() : settings_billing_changePlan()}
							</button>

							{#if !isFree}
								{#if org.plan_status === 'paused'}
									<button
										type="button"
										onclick={handleResume}
										class="btn-primary w-full"
										disabled={resumeMutation.isPending}
									>
										{settings_billing_resume_button()}
									</button>
								{:else if org.plan_status === 'past_due'}
									<button onclick={handleManageSubscription} class="btn-primary w-full">
										{settings_billing_manageSubscription()}
									</button>
								{:else if org.plan_status === 'pending_cancellation'}
									<div class="flex flex-col gap-2">
										<button
											type="button"
											onclick={handleReactivate}
											class="btn-primary w-full"
											disabled={reactivateMutation.isPending}
										>
											{settings_billing_reactivateSubscription()}
										</button>
										<button onclick={handleManageSubscription} class="btn-secondary w-full">
											{settings_billing_manageSubscription()}
										</button>
									</div>
								{:else}
									<div class="flex flex-col gap-2">
										<button onclick={handleManageSubscription} class="btn-secondary w-full">
											{settings_billing_manageSubscription()}
										</button>
										<button onclick={openCancelModal} class="btn-secondary w-full">
											{settings_billing_cancelSubscription()}
										</button>
									</div>
								{/if}
							{/if}
						</div>
					</svelte:fragment>
				</InfoCard>

				<!-- Additional Info -->
				<InfoCard title={settings_billing_needHelp()}>
					<p class="text-secondary text-sm">
						{settings_billing_contactUs()}
						<a href="mailto:billing@scanopy.net" class="text-link hover:underline"
							>billing@scanopy.net</a
						>
						{settings_billing_billingQuestions()}
					</p>
				</InfoCard>
			</div>
		{:else}
			<div class="text-secondary py-8 text-center">
				<p>{settings_billing_unableToLoad()}</p>
				<p class="text-tertiary mt-2 text-sm">{common_tryAgainLater()}</p>
			</div>
		{/if}
	</div>

	<!-- Footer -->
	{#if dismissible}
		<div class="modal-footer">
			<div class="flex justify-end">
				<button type="button" onclick={onClose} class="btn-secondary">{common_close()}</button>
			</div>
		</div>
	{/if}
</div>

<CancelSubscriptionModal
	isOpen={showCancelModal}
	onClose={() => (showCancelModal = false)}
	lastPausedAt={org?.last_paused_at ?? null}
	onSubscriptionChanged={() => organizationQuery.refetch()}
/>
