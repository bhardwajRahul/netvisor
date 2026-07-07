<script lang="ts">
	import type { components } from '$lib/api/schema';
	import { Check, ChevronDown, Circle, Info, Loader2 } from 'lucide-svelte';
	import { onMount } from 'svelte';
	import {
		CHECKLIST_STEPS,
		isStepComplete,
		isStepEnabled,
		isAllComplete,
		getCompletedCount,
		executeStepAction,
		trackChecklistStepClicked
	} from '$lib/shared/onboarding/checklist';
	import {
		gettingStarted_complete,
		gettingStarted_havingTrouble,
		gettingStarted_title
	} from '$lib/paraglide/messages';
	import DaemonTroubleshootingModal from './DaemonTroubleshootingModal.svelte';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type DaemonStatus = 'idle' | 'waiting' | 'connected' | 'trouble';

	let {
		onboarding,
		collapsed,
		onNavigate,
		isDiscoveryActive,
		daemonStatus,
		onExpandSidebar
	}: {
		onboarding: OnboardingOperation[];
		collapsed: boolean;
		onNavigate: (tab: string) => void;
		isDiscoveryActive: boolean;
		daemonStatus: DaemonStatus;
		onExpandSidebar: () => void;
	} = $props();

	const STORAGE_KEY = 'scanopy-sidebar-checklist-expanded';

	let expanded = $state(true);

	onMount(() => {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (stored !== null) {
			expanded = JSON.parse(stored);
		}
	});

	function toggleExpanded() {
		expanded = !expanded;
		localStorage.setItem(STORAGE_KEY, JSON.stringify(expanded));
	}

	let completedCount = $derived(getCompletedCount(onboarding));
	let allComplete = $derived(isAllComplete(onboarding));
	let showTroubleshootingModal = $state(false);

	function isDaemonTrouble(stepId: string, complete: boolean): boolean {
		return (
			stepId === 'daemon' && !complete && (daemonStatus === 'waiting' || daemonStatus === 'trouble')
		);
	}

	function isDiscoverySpinner(stepId: string, complete: boolean): boolean {
		return stepId === 'discovery' && !complete && isDiscoveryActive;
	}
</script>

{#if !allComplete}
	{#if collapsed}
		<!-- Collapsed sidebar: small progress badge -->
		<div class="flex justify-center px-2 py-2">
			<button
				onclick={onExpandSidebar}
				class="text-tertiary hover:text-secondary flex h-8 w-8 items-center justify-center rounded-full border border-gray-200 text-[10px] font-bold transition-colors hover:bg-gray-100 dark:border-gray-700 dark:hover:bg-gray-800"
				title="{gettingStarted_title()}: {gettingStarted_complete({
					completed: completedCount,
					total: CHECKLIST_STEPS.length
				})}"
			>
				{completedCount}/{CHECKLIST_STEPS.length}
			</button>
		</div>
	{:else}
		<!-- Expanded sidebar: pill header + accordion steps -->
		<div class="border-b border-t px-2 py-2" style="border-color: var(--color-border)">
			<button
				onclick={toggleExpanded}
				class="text-secondary hover:text-primary flex w-full items-center justify-between rounded-lg px-2 py-1.5 text-xs font-semibold transition-colors hover:bg-gray-100 dark:hover:bg-gray-800"
			>
				<span class="whitespace-nowrap">{gettingStarted_title()}</span>
				<div class="flex items-center gap-2">
					<div class="flex gap-0.5">
						{#each CHECKLIST_STEPS as step (step.id)}
							<span
								class="inline-block h-1.5 w-1.5 rounded-full {isStepComplete(step, onboarding)
									? 'bg-green-400'
									: 'bg-gray-300 dark:bg-gray-600'}"
							></span>
						{/each}
					</div>
					<ChevronDown class="h-3.5 w-3.5 transition-transform {expanded ? '' : '-rotate-90'}" />
				</div>
			</button>

			{#if expanded}
				<div class="mt-1 space-y-0.5">
					{#each CHECKLIST_STEPS as step (step.id)}
						{@const complete = isStepComplete(step, onboarding)}
						{@const enabled = isStepEnabled(step, onboarding)}
						{@const isAccountStep = step.id === 'account'}
						{@const daemonTrouble = isDaemonTrouble(step.id, complete)}
						{@const discoverySpinner = isDiscoverySpinner(step.id, complete)}
						{#if daemonTrouble}
							<div
								class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-xs {!enabled
									? 'opacity-50'
									: ''}"
							>
								<Info class="h-3.5 w-3.5 flex-shrink-0 text-yellow-500" />
								<button
									class="text-primary truncate rounded hover:underline"
									onclick={() => {
										trackChecklistStepClicked(step.id, 'sidebar');
										executeStepAction(step, onNavigate);
									}}
								>
									{step.label()}
								</button>
								<button
									class="ml-auto flex-shrink-0 rounded text-[10px] font-medium text-yellow-500 hover:underline"
									onclick={() => {
										showTroubleshootingModal = true;
									}}
								>
									{gettingStarted_havingTrouble()}
								</button>
							</div>
						{:else}
							<button
								class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-xs transition-colors {!complete &&
								!isAccountStep &&
								enabled
									? 'hover:bg-gray-100 dark:hover:bg-gray-800'
									: ''} {!enabled ? 'opacity-50' : ''}"
								disabled={complete || !enabled || isAccountStep}
								onclick={() => {
									trackChecklistStepClicked(step.id, 'sidebar');
									executeStepAction(step, onNavigate);
								}}
							>
								{#if complete}
									<Check class="h-3.5 w-3.5 flex-shrink-0 text-green-400" />
								{:else if discoverySpinner}
									<Loader2 class="h-3.5 w-3.5 flex-shrink-0 animate-spin text-blue-500" />
								{:else if enabled}
									<Circle class="text-tertiary h-3.5 w-3.5 flex-shrink-0" />
								{:else}
									<Circle class="text-disabled h-3.5 w-3.5 flex-shrink-0" />
								{/if}
								<span
									class="truncate"
									class:text-tertiary={complete}
									class:line-through={complete}
									class:text-primary={!complete && enabled}
									class:text-disabled={!complete && !enabled}
								>
									{step.label()}
								</span>
							</button>
						{/if}
					{/each}
				</div>
			{/if}
		</div>
	{/if}
{/if}

<DaemonTroubleshootingModal
	isOpen={showTroubleshootingModal}
	onClose={() => (showTroubleshootingModal = false)}
/>
