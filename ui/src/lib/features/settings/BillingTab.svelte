<script lang="ts">
	import { CheckCircle, AlertCircle } from 'lucide-svelte';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { useCustomerPortalMutation } from '$lib/features/billing/queries';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import { useUsersQuery } from '$lib/features/users/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import * as m from '$lib/paraglide/messages';

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

	// Customer portal mutation
	const customerPortalMutation = useCustomerPortalMutation();

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
			case 'canceled':
			case 'incomplete':
				return 'text-yellow-400';
			default:
				return 'text-gray-400';
		}
	}

	async function handleManageSubscription() {
		try {
			const url = await customerPortalMutation.mutateAsync();
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
				<!-- Current Plan -->
				<InfoCard>
					<svelte:fragment slot="default">
						<div class="mb-3 flex items-center justify-between">
							<h3 class="text-primary text-sm font-semibold">{m.settings_billing_currentPlan()}</h3>
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
												{m.settings_billing_trialDays({ days: org.plan.trial_days })}
											</p>
										{/if}
									</div>
									<div class="text-right">
										<p class="text-primary text-2xl font-bold">
											${org.plan.base_cents / 100}
										</p>
										<p class="text-secondary text-xs">
											{m.settings_billing_per({ rate: org.plan.rate })}
										</p>
									</div>
								</div>

								<!-- Seats Usage -->
								{#if org.plan.included_seats !== null}
									<div class="border-t border-gray-700 pt-3">
										<div class="flex items-baseline justify-between">
											<div>
												<p class="text-primary font-medium">{m.settings_billing_seats()}</p>
												<p class="text-secondary text-sm">
													{m.settings_billing_seatsUsage({
														count: seatCount,
														included: org.plan.included_seats ?? 0
													})}
													{#if extraSeats > 0}
														{m.settings_billing_extraSeats({
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
														{m.settings_billing_per({ rate: org.plan.rate })}
													</p>
												</div>
											{:else}
												<p class="text-tertiary text-sm">{m.settings_billing_included()}</p>
											{/if}
										</div>
									</div>
								{/if}

								<!-- Networks Usage -->
								{#if org.plan.included_networks !== null}
									<div class="border-t border-gray-700 pt-3">
										<div class="flex items-baseline justify-between">
											<div>
												<p class="text-primary font-medium">{m.settings_billing_networks()}</p>
												<p class="text-secondary text-sm">
													{m.settings_billing_networksUsage({
														count: networkCount,
														included: org.plan.included_networks ?? 0
													})}
													{#if extraNetworks > 0}
														{m.settings_billing_extraNetworks({
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
														{m.settings_billing_per({ rate: org.plan.rate })}
													</p>
												</div>
											{:else}
												<p class="text-tertiary text-sm">{m.settings_billing_included()}</p>
											{/if}
										</div>
									</div>
								{/if}
							{/if}

							{#if org.plan_status === 'trialing'}
								<div
									class="rounded-md border border-blue-800 bg-blue-900/30 p-3 text-sm text-blue-300"
								>
									{m.settings_billing_trialActive()}
								</div>
							{:else if org.plan_status === 'past_due'}
								<div
									class="rounded-md border border-red-800 bg-red-900/30 p-3 text-sm text-red-300"
								>
									{m.settings_billing_pastDue()}
								</div>
							{:else if org.plan_status === 'canceled'}
								<div
									class="rounded-md border border-yellow-800 bg-yellow-900/30 p-3 text-sm text-yellow-300"
								>
									{m.settings_billing_canceled()}
								</div>
							{/if}
						</div>
					</svelte:fragment>
				</InfoCard>

				<!-- Actions -->
				<div class="space-y-3">
					<button onclick={handleManageSubscription} class="btn-primary w-full">
						{m.settings_billing_manageSubscription()}
					</button>
				</div>

				<!-- Additional Info -->
				<InfoCard title={m.settings_billing_needHelp()}>
					<p class="text-secondary text-sm">
						{m.settings_billing_contactUs()}
						<a href="mailto:billing@scanopy.net" class="text-blue-400 hover:underline"
							>billing@scanopy.net</a
						>
						{m.settings_billing_billingQuestions()}
					</p>
				</InfoCard>
			</div>
		{:else}
			<div class="text-secondary py-8 text-center">
				<p>{m.settings_billing_unableToLoad()}</p>
				<p class="text-tertiary mt-2 text-sm">{m.settings_billing_tryAgainLater()}</p>
			</div>
		{/if}
	</div>

	<!-- Footer -->
	<div class="modal-footer">
		<div class="flex justify-end">
			<button type="button" onclick={onClose} class="btn-secondary">{m.common_close()}</button>
		</div>
	</div>
</div>
