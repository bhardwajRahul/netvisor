<script lang="ts">
	import { ArrowRight } from 'lucide-svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useServicesQuery } from '$lib/features/services/queries';
	import {
		CHECKLIST_STEPS,
		executeStepAction,
		type ChecklistStep
	} from '$lib/shared/onboarding/checklist';
	import { trackEvent, trackOncePerSession } from '$lib/shared/utils/analytics';
	import { getDaysIntoTrial } from '$lib/shared/utils/trial';
	import {
		billing_trialRecapDaemons,
		billing_trialRecapDays,
		billing_trialRecapEmptyBody,
		billing_trialRecapEmptyCta,
		billing_trialRecapEmptyTitle,
		billing_trialRecapHosts,
		billing_trialRecapNetworks,
		billing_trialRecapServices,
		billing_trialRecapTitle,
		common_next
	} from '$lib/paraglide/messages';
	import type { Organization } from '$lib/features/organizations/types';
	import type { components } from '$lib/api/schema';

	type OnboardingOperation = components['schemas']['OnboardingOperation'];

	let { org, onCloseSettings }: { org: Organization; onCloseSettings: () => void } = $props();

	const hostsQuery = useHostsQuery({ limit: 1 });
	const networksQuery = useNetworksQuery();
	const daemonsQuery = useDaemonsQuery();
	const servicesQuery = useServicesQuery({ limit: 1 });

	let hostsCount = $derived(hostsQuery.data?.pagination?.total_count ?? 0);
	let networksCount = $derived(networksQuery.data?.length ?? 0);
	let daemonsCount = $derived(daemonsQuery.data?.length ?? 0);
	let servicesCount = $derived(servicesQuery.data?.pagination?.total_count ?? 0);
	let daysIntoTrial = $derived(getDaysIntoTrial(org) ?? 0);

	let isEmpty = $derived(
		hostsCount === 0 && networksCount === 0 && daemonsCount === 0 && servicesCount === 0
	);

	let onboarding = $derived((org.onboarding ?? []) as OnboardingOperation[]);
	let nextIncompleteStep = $derived<ChecklistStep>(
		CHECKLIST_STEPS.find((step) => !onboarding.includes(step.milestone)) ?? CHECKLIST_STEPS[1]
	);

	$effect(() => {
		if (org.plan_status !== 'trialing') return;
		trackOncePerSession('trial_recap_shown', 'trial_recap_shown', {
			hosts: hostsCount,
			networks: networksCount,
			daemons: daemonsCount,
			services: servicesCount,
			is_empty: isEmpty
		});
		if (isEmpty) {
			trackOncePerSession('trial_recap_empty_state_shown', 'trial_recap_empty_state_shown', {
				next_step_id: nextIncompleteStep.id
			});
		}
	});

	function handleEmptyStateCta() {
		trackEvent('trial_recap_empty_state_cta_clicked', { next_step_id: nextIncompleteStep.id });
		onCloseSettings();
		executeStepAction(nextIncompleteStep, (tab) => {
			if (typeof window !== 'undefined') {
				window.location.hash = tab;
			}
		});
	}
</script>

<InfoCard title={billing_trialRecapTitle()}>
	{#if isEmpty}
		<div class="flex items-start justify-between gap-4">
			<div class="min-w-0 flex-1">
				<p class="text-primary text-sm font-medium">{billing_trialRecapEmptyTitle()}</p>
				<p class="text-secondary mt-1 text-xs">{billing_trialRecapEmptyBody()}</p>
				<p class="text-secondary mt-2 text-xs">
					{common_next()}: <span class="text-primary font-medium">{nextIncompleteStep.label}</span>
				</p>
			</div>
			<button
				type="button"
				onclick={handleEmptyStateCta}
				class="btn-primary flex items-center gap-1.5 text-sm"
			>
				{billing_trialRecapEmptyCta()}
				<ArrowRight size={14} />
			</button>
		</div>
	{:else}
		<div class="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-5">
			<div>
				<p class="text-primary text-2xl font-semibold">{hostsCount}</p>
				<p class="text-secondary text-xs">{billing_trialRecapHosts()}</p>
			</div>
			<div>
				<p class="text-primary text-2xl font-semibold">{networksCount}</p>
				<p class="text-secondary text-xs">{billing_trialRecapNetworks()}</p>
			</div>
			<div>
				<p class="text-primary text-2xl font-semibold">{daemonsCount}</p>
				<p class="text-secondary text-xs">{billing_trialRecapDaemons()}</p>
			</div>
			<div>
				<p class="text-primary text-2xl font-semibold">{servicesCount}</p>
				<p class="text-secondary text-xs">{billing_trialRecapServices()}</p>
			</div>
			<div>
				<p class="text-primary text-2xl font-semibold">{daysIntoTrial}</p>
				<p class="text-secondary text-xs">{billing_trialRecapDays()}</p>
			</div>
		</div>
	{/if}
</InfoCard>
