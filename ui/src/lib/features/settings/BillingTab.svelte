<script lang="ts">
	import { CheckCircle, AlertCircle, CreditCard, AlertTriangle } from 'lucide-svelte';
	import { showBillingPlanModal, reopenSettingsAfterBilling } from '$lib/features/billing/stores';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import {
		useCustomerPortalMutation,
		useSetupPaymentMethodMutation
	} from '$lib/features/billing/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import { useUsersQuery } from '$lib/features/users/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
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
		settings_billing_manageSubscription,
		settings_billing_needHelp,
		settings_billing_pastDue,
		settings_billing_per,
		settings_billing_trialActive,
		settings_billing_trialDays,
		settings_billing_unableToLoad
	} from '$lib/paraglide/messages';

	let {
		isOpen = false,
		onClose
	}: {
		isOpen?: boolean;
		onClose: () => void;
	} = $props();

	// TanStack Query for users - only fetch when modal is open (Owner only)
	const usersQuery = useUsersQuery({ enabled: () => isOpen });
	let usersData = $derived(usersQuery.data ?? []);

	// TanStack Query for networks
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);

	// Host count query (limit 1 to get total count from pagination)
	const hostsQuery = useHostsQuery({ limit: 1 });
	let hostCount = $derived(hostsQuery.data?.pagination?.total_count ?? 0);

	// Customer portal mutation
	const customerPortalMutation = useCustomerPortalMutation();
	const setupPaymentMutation = useSetupPaymentMethodMutation();

	let seatCount = $derived(usersData.length);
	let networkCount = $derived(networksData.length);

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
				return 'text-green-400';
			case 'trialing':
				return 'text-blue-400';
			case 'past_due':
			case 'unpaid':
				return 'text-red-400';
			case 'pending_cancellation':
				return 'text-amber-400';
			case 'canceled':
			case 'incomplete':
				return 'text-yellow-400';
			default:
				return 'text-gray-400';
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

	async function handleManageSubscription() {
		trackEvent('billing_portal_opened', { plan_type: org?.plan?.type });
		try {
			const url = await customerPortalMutation.mutateAsync();
			if (url) {
				window.location.href = url;
			}
		} catch {
			// Error handling is done by the mutation's onError
		}
	}

	async function handleSetupPayment() {
		trackEvent('payment_method_setup_initiated', {
			plan_status: org?.plan_status,
			trial_days_left: trialDaysLeft
		});
		try {
			const url = await setupPaymentMutation.mutateAsync();
			if (url) {
				window.location.href = url;
			}
		} catch {
			// Error handling is done by the mutation's onError
		}
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
										Trial ends in {trialDaysLeft} days
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
					</InfoCard>
				{/if}

				<!-- Current Plan -->
				<InfoCard>
					<svelte:fragment slot="default">
						<div class="mb-3 flex items-center justify-between">
							<h3 class="text-primary text-sm font-semibold">{settings_billing_currentPlan()}</h3>
							<div class="flex items-center gap-2">
								{#if planActive}
									<CheckCircle class="h-4 w-4 text-green-400" />
								{:else}
									<AlertCircle class="h-4 w-4 text-yellow-400" />
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
										{#if org.plan.trial_days > 0 && org.plan_status === 'trialing'}
											<p class="text-secondary mt-1 text-xs">
												{settings_billing_trialDays({ days: org.plan.trial_days })}
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
									<div class="border-t border-gray-700 pt-3">
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
									<div class="border-t border-gray-700 pt-3">
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
									<div class="border-t border-gray-700 pt-3">
										<div class="flex items-baseline justify-between">
											<div>
												<p class="text-primary font-medium">Hosts</p>
												<p class="text-secondary text-sm">
													{hostCount} / {org.plan.included_hosts} used
												</p>
											</div>
											{#if hostCount >= (org.plan.included_hosts ?? 0)}
												<p class="text-sm text-amber-400">At limit</p>
											{:else}
												<p class="text-tertiary text-sm">{common_included()}</p>
											{/if}
										</div>
										{#if hostCount > 0}
											<div class="mt-2 h-2 w-full overflow-hidden rounded-full bg-gray-700">
												<div
													class="h-full rounded-full transition-all {hostCount >=
													(org.plan.included_hosts ?? 0)
														? 'bg-amber-500'
														: 'bg-blue-500'}"
													style="width: {Math.min(
														100,
														(hostCount / (org.plan.included_hosts || 1)) * 100
													)}%"
												></div>
											</div>
										{/if}
									</div>
								{/if}
							{/if}

							{#if org.plan_status === 'trialing'}
								<div
									class="rounded-md border border-blue-800 bg-blue-900/30 p-3 text-sm text-blue-300"
								>
									{settings_billing_trialActive()}
								</div>
							{:else if org.plan_status === 'past_due'}
								<div
									class="rounded-md border border-red-800 bg-red-900/30 p-3 text-sm text-red-300"
								>
									{settings_billing_pastDue()}
								</div>
							{:else if org.plan_status === 'canceled'}
								<div
									class="rounded-md border border-yellow-800 bg-yellow-900/30 p-3 text-sm text-yellow-300"
								>
									{settings_billing_canceled()}
								</div>
							{:else if org.plan_status === 'pending_cancellation'}
								<div
									class="rounded-md border border-amber-800 bg-amber-900/30 p-3 text-sm text-amber-300"
								>
									Your plan will change to Free at the end of your current billing cycle. To cancel
									this change, upgrade to a paid plan or manage your subscription in the billing
									portal.
								</div>
							{/if}
						</div>
					</svelte:fragment>
				</InfoCard>

				<!-- View Plans -->
				<InfoCard>
					<div class="flex items-center justify-between">
						<div>
							<p class="text-primary text-sm font-medium">
								{isFree ? 'Upgrade your plan' : 'Change your plan'}
							</p>
							<p class="text-secondary mt-1 text-xs">
								{isFree
									? 'Get scheduled discovery, DaemonPoll mode, and more hosts'
									: 'View available plans and upgrade or downgrade'}
							</p>
						</div>
						<button
							onclick={() => {
								showBillingPlanModal.set(true);
								reopenSettingsAfterBilling.set(true);
								onClose();
							}}
							class="btn-primary whitespace-nowrap text-sm"
						>
							View Plans
						</button>
					</div>
				</InfoCard>

				<!-- Actions -->
				<div class="space-y-3">
					{#if !isFree}
						<button onclick={handleManageSubscription} class="btn-secondary w-full">
							{settings_billing_manageSubscription()}
						</button>
					{/if}
				</div>

				<!-- Additional Info -->
				<InfoCard title={settings_billing_needHelp()}>
					<p class="text-secondary text-sm">
						{settings_billing_contactUs()}
						<a href="mailto:billing@scanopy.net" class="text-blue-400 hover:underline"
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
	<div class="modal-footer">
		<div class="flex justify-end">
			<button type="button" onclick={onClose} class="btn-secondary">{common_close()}</button>
		</div>
	</div>
</div>
