<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { AlertTriangle, Home, Building2, Users } from 'lucide-svelte';
	import { type UseCase, USE_CASES } from '../../types/base';
	import { useConfigQuery, isCloud, isCommunity } from '$lib/shared/stores/config-query';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import {
		auth_scanopyLogo,
		common_continue,
		onboarding_alreadyHaveAccount,
		onboarding_commercialNoticeBody,
		onboarding_commercialNoticeTitle,
		onboarding_haveQuestionsFirst,
		onboarding_howWillYouUse,
		onboarding_logInHere,
		onboarding_readyToScan,
		onboarding_tailorSetup,
		onboarding_understandContinue,
		onboarding_yesLetsGo
	} from '$lib/paraglide/messages';

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

	// Role options
	const roleOptions = [
		{ value: '', label: 'Select your role', disabled: true },
		{ value: 'it_admin', label: 'IT Admin' },
		{ value: 'network_engineer', label: 'Network Engineer' },
		{ value: 'devops', label: 'DevOps' },
		{ value: 'manager', label: 'Manager / Director' },
		{ value: 'executive', label: 'Owner / Executive' },
		{ value: 'other', label: 'Other' }
	];

	// Company size options
	const companySizeOptions = [
		{ value: '', label: 'Select company size', disabled: true },
		{ value: '1-10', label: '1-10 employees' },
		{ value: '11-25', label: '11-25 employees' },
		{ value: '26-50', label: '26-50 employees' },
		{ value: '51-100', label: '51-100 employees' },
		{ value: '101-250', label: '101-250 employees' },
		{ value: '251-500', label: '251-500 employees' },
		{ value: '501-1000', label: '501-1000 employees' },
		{ value: '1001+', label: '1001+ employees' }
	];

	// Icons for each use case (kept separate from types for flexibility)
	const useCaseIcons: Record<UseCase, typeof Home> = {
		homelab: Home,
		company: Building2,
		msp: Users
	};

	// Use case IDs for iteration
	const useCaseIds: UseCase[] = ['homelab', 'company', 'msp'];

	// Show business fields for company/msp
	$: showBusinessFields = selectedUseCase === 'company' || selectedUseCase === 'msp';

	// Form for business qualification fields
	const form = createForm(() => ({
		defaultValues: {
			role: '',
			companySize: ''
		}
	}));

	function selectUseCase(useCase: UseCase) {
		selectedUseCase = useCase;

		// Reset business fields when switching to homelab
		if (useCase === 'homelab') {
			form.reset({ role: '', companySize: '' });
		}

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

	function saveBusinessFields() {
		if (showBusinessFields) {
			const values = form.state.values;
			onboardingStore.setJobTitle(values.role || null);
			onboardingStore.setCompanySize(values.companySize || null);
		}
	}

	// Self hosted handlers
	function handleContinue() {
		if (!selectedUseCase) return;
		saveBusinessFields();
		onboardingStore.setUseCase(selectedUseCase);
		onNext();
	}

	// Cloud handlers
	function handleReadyYes() {
		if (!selectedUseCase) return;
		saveBusinessFields();
		const values = form.state.values;
		trackEvent('onboarding_use_case_selected', {
			use_case: selectedUseCase,
			role: values.role || undefined,
			company_size: values.companySize || undefined
		});
		trackEvent('onboarding_ready_to_scan', { ready: true, use_case: selectedUseCase });
		onboardingStore.setUseCase(selectedUseCase);
		onboardingStore.setReadyToScan(true);
		onNext();
	}

	function handleReadyNo() {
		if (!selectedUseCase) return;
		saveBusinessFields();
		const values = form.state.values;
		trackEvent('onboarding_use_case_selected', {
			use_case: selectedUseCase,
			role: values.role || undefined,
			company_size: values.companySize || undefined
		});
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
	title={onboarding_howWillYouUse()}
	{onClose}
	size="lg"
	centerTitle={true}
	showBackdrop={false}
	showCloseButton={false}
	preventCloseOnClickOutside={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-y-auto p-6">
			<div class="space-y-6">
				<p class="text-secondary text-center text-sm">{onboarding_tailorSetup()}</p>

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

				<!-- Business Qualification Fields (Company/MSP only) -->
				{#if showBusinessFields}
					<div class="card card-static">
						<p class="text-secondary text-sm">Help us tailor your experience:</p>

						<div class="grid gap-4 sm:grid-cols-2">
							<form.Field name="role">
								{#snippet children(field)}
									<SelectInput label="Your role" id="role" {field} options={roleOptions} />
								{/snippet}
							</form.Field>

							<form.Field name="companySize">
								{#snippet children(field)}
									<SelectInput
										label="Company size"
										id="company-size"
										{field}
										options={companySizeOptions}
									/>
								{/snippet}
							</form.Field>
						</div>
					</div>
				{/if}

				<!-- License Warning (Community + Company/MSP) -->
				{#if showLicenseWarning}
					<div class="rounded-lg border border-yellow-600/30 bg-yellow-900/20 p-4">
						<div class="flex items-start gap-2">
							<AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-warning" />
							<div class="flex-1">
								<p class="text-sm font-medium text-warning">{onboarding_commercialNoticeTitle()}</p>
								<p class="mt-1 text-sm text-warning">
									{@html onboarding_commercialNoticeBody()}
								</p>
								<button type="button" class="btn-primary mt-4" on:click={handleLicenseAcknowledge}>
									{onboarding_understandContinue()}
								</button>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				{#if isCloudDeployment}
					<!-- Cloud: Show ready to scan buttons (disabled until use case selected) -->
					<p class="text-secondary text-center text-sm">
						{onboarding_readyToScan()}
					</p>
					<div class="flex gap-3">
						<button
							type="button"
							class="btn-secondary flex-1"
							disabled={!canProceed}
							on:click={handleReadyNo}
						>
							{onboarding_haveQuestionsFirst()}
						</button>
						<button
							type="button"
							class="btn-primary flex-1"
							disabled={!canProceed}
							on:click={handleReadyYes}
						>
							{onboarding_yesLetsGo()}
						</button>
					</div>
				{:else}
					<!-- Self-hosted: Show continue button -->
					<div class="flex justify-end">
						<button
							type="button"
							class="btn-primary"
							disabled={!canProceed}
							on:click={handleContinue}
						>
							{common_continue()}
						</button>
					</div>
				{/if}

				{#if onSwitchToLogin}
					<p class="text-secondary text-center text-sm">
						{onboarding_alreadyHaveAccount()}
						<button
							type="button"
							on:click={onSwitchToLogin}
							class="font-medium text-blue-400 hover:text-blue-300"
						>
							{onboarding_logInHere()}
						</button>
					</p>
				{/if}
			</div>
		</div>
	</div>
</GenericModal>
