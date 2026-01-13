<script lang="ts">
	import { AlertTriangle, Home, Building2, Users } from 'lucide-svelte';
	import { type UseCase, USE_CASES } from '../../types/base';
	import { useConfigQuery, isCloud, isCommunity } from '$lib/shared/stores/config-query';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import * as m from '$lib/paraglide/messages';

	const configQuery = useConfigQuery();
	// eslint-disable-next-line svelte/no-immutable-reactive-statements -- configQuery.data changes when query resolves
	$: configData = configQuery.data;

	export let isOpen: boolean;
	export let onNext: () => void;
	export let onBlockerFlow: () => void;
	export let onClose: () => void;
	export let onSwitchToLogin: (() => void) | null = null;

	let selectedUseCase: UseCase | null = null;
	let showLicenseWarning = false;

	// Icons for each use case (kept separate from types for flexibility)
	const useCaseIcons: Record<UseCase, typeof Home> = {
		homelab: Home,
		company: Building2,
		msp: Users
	};

	// Use case IDs for iteration
	const useCaseIds: UseCase[] = ['homelab', 'company', 'msp'];

	function selectUseCase(useCase: UseCase) {
		selectedUseCase = useCase;

		// For Community self-hosted + Company/MSP: show license warning
		if (configData && isCommunity(configData) && (useCase === 'company' || useCase === 'msp')) {
			showLicenseWarning = true;
		} else {
			showLicenseWarning = false;
		}
	}

	function handleLicenseAcknowledge() {
		showLicenseWarning = false;
	}

	// Self hosted handlers
	function handleContinue() {
		if (!selectedUseCase) return;
		onboardingStore.setUseCase(selectedUseCase);
		onNext();
	}

	// Cloud handlers
	function handleReadyYes() {
		if (!selectedUseCase) return;
		trackEvent('onboarding_use_case_selected', { use_case: selectedUseCase });
		trackEvent('onboarding_ready_to_scan', { ready: true, use_case: selectedUseCase });
		onboardingStore.setUseCase(selectedUseCase);
		onboardingStore.setReadyToScan(true);
		onNext();
	}

	function handleReadyNo() {
		if (!selectedUseCase) return;
		trackEvent('onboarding_use_case_selected', { use_case: selectedUseCase });
		trackEvent('onboarding_ready_to_scan', { ready: false, use_case: selectedUseCase });
		onboardingStore.setUseCase(selectedUseCase);
		onboardingStore.setReadyToScan(false);
		onBlockerFlow();
	}

	$: isCloudDeployment = configData && isCloud(configData);
	$: canProceed = selectedUseCase !== null && !showLicenseWarning;
</script>

<GenericModal
	{isOpen}
	title={m.onboarding_howWillYouUse()}
	{onClose}
	size="lg"
	centerTitle={true}
	showBackdrop={false}
	showCloseButton={false}
	preventCloseOnClickOutside={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={m.auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<div class="space-y-6 p-6">
		<p class="text-secondary text-center text-sm">{m.onboarding_tailorSetup()}</p>

		<!-- Use Case Cards -->
		<div class="grid gap-3">
			{#each useCaseIds as useCaseId (useCaseId)}
				{@const useCaseConfig = USE_CASES[useCaseId]}
				{@const isSelected = selectedUseCase === useCaseId}
				{@const Icon = useCaseIcons[useCaseId]}
				<button
					type="button"
					class="card flex items-center gap-4 p-4 text-left transition-all {isSelected
						? `ring-2 ${useCaseConfig.colors.ring}`
						: 'hover:bg-gray-800'}"
					on:click={() => selectUseCase(useCaseId)}
				>
					<div
						class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg {isSelected
							? `${useCaseConfig.colors.bg} ${useCaseConfig.colors.text}`
							: 'bg-gray-700 text-gray-400'}"
					>
						<svelte:component this={Icon} class="h-5 w-5" />
					</div>
					<div>
						<div class="text-primary font-medium">{useCaseConfig.label}</div>
						<div class="text-secondary text-sm">{useCaseConfig.description}</div>
					</div>
				</button>
			{/each}
		</div>

		<!-- License Warning (Community + Company/MSP) -->
		{#if showLicenseWarning}
			<div class="rounded-lg border border-yellow-600/30 bg-yellow-900/20 p-4">
				<div class="flex items-start gap-2">
					<AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-warning" />
					<div class="flex-1">
						<p class="text-sm font-medium text-warning">{m.onboarding_commercialNoticeTitle()}</p>
						<p class="mt-1 text-sm text-warning">
							{@html m.onboarding_commercialNoticeBody()}
						</p>
						<button type="button" class="btn-primary mt-4" on:click={handleLicenseAcknowledge}>
							{m.onboarding_understandContinue()}
						</button>
					</div>
				</div>
			</div>
		{/if}

		{#if isCloudDeployment}
			<!-- Cloud: Show ready to scan buttons (disabled until use case selected) -->
			<div class="space-y-3">
				<p class="text-secondary text-center text-sm">
					{m.onboarding_readyToScan()}
				</p>
				<div class="flex gap-3">
					<button
						type="button"
						class="btn-secondary flex-1"
						disabled={!canProceed}
						on:click={handleReadyNo}
					>
						{m.onboarding_haveQuestionsFirst()}
					</button>
					<button
						type="button"
						class="btn-primary flex-1"
						disabled={!canProceed}
						on:click={handleReadyYes}
					>
						{m.onboarding_yesLetsGo()}
					</button>
				</div>
			</div>
		{:else}
			<!-- Self-hosted: Show continue button -->
			<div class="space-y-3">
				<div class="flex justify-end">
					<button
						type="button"
						class="btn-primary"
						disabled={!canProceed}
						on:click={handleContinue}
					>
						{m.common_continue()}
					</button>
				</div>
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<div class="modal-footer">
			{#if onSwitchToLogin}
				<p class="text-secondary text-center text-sm">
					{m.onboarding_alreadyHaveAccount()}
					<button
						type="button"
						on:click={onSwitchToLogin}
						class="font-medium text-blue-400 hover:text-blue-300"
					>
						{m.onboarding_logInHere()}
					</button>
				</p>
			{/if}
		</div>
	{/snippet}
</GenericModal>
