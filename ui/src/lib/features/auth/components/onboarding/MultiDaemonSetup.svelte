<script lang="ts">
	import { Check, Network } from 'lucide-svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import CreateDaemonForm from '$lib/features/daemons/components/CreateDaemonForm.svelte';
	import { useDaemonSetupMutation } from '../../queries';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { pushError } from '$lib/shared/stores/feedback';
	import type { NetworkSetup } from '../../types/base';
	import {
		common_continue,
		common_settingUp,
		onboarding_continueToRegistration,
		onboarding_daemonsActivateBody,
		onboarding_daemonsActivateTitle,
		onboarding_exploreDemoInstead,
		onboarding_selectADaemon,
		onboarding_selectDaemon,
		onboarding_selectDaemonHelp,
		onboarding_skipConfirmBody,
		onboarding_skipConfirmTitle,
		onboarding_skipDaemonSetup,
		onboarding_startScanning
	} from '$lib/paraglide/messages';

	// Convert string to kebab-case
	function toKebabCase(str: string): string {
		return str
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/^-+|-+$/g, '');
	}

	interface Props {
		isOpen: boolean;
		networks: NetworkSetup[];
		onComplete: () => void;
		onClose: () => void;
	}

	let { isOpen, networks, onComplete, onClose }: Props = $props();

	// Track the selected network for daemon installation
	let selectedNetworkId = $state<string | null>(null);

	// Track loading state during daemon setup
	let isLoading = $state(false);

	// Track whether daemon has been set up for selected network
	let daemonConfigured = $state(false);

	// API key returned after daemon setup
	let apiKey = $state<string | null>(null);

	// Track skip confirmation modal
	let showSkipConfirm = $state(false);

	// Reference to CreateDaemonForm for getting daemon name
	let daemonFormRef = $state<CreateDaemonForm | null>(null);

	// Daemon setup mutation
	const daemonSetupMutation = useDaemonSetupMutation();

	// Get the selected network object
	let selectedNetwork = $derived(networks.find((n) => n.id === selectedNetworkId));

	// Get daemon name based on selected network
	let defaultDaemonName = $derived(
		selectedNetwork ? toKebabCase(selectedNetwork.name) + '-daemon' : 'daemon'
	);

	function selectNetwork(networkId: string) {
		if (daemonConfigured && selectedNetworkId !== networkId) {
			// Reset if changing selection after configuration
			daemonConfigured = false;
			apiKey = null;
		}
		selectedNetworkId = networkId;
	}

	async function handleContinue() {
		if (!selectedNetworkId) return;

		// If daemon already configured, just proceed
		if (daemonConfigured) {
			onComplete();
			return;
		}

		// Set up the daemon for the selected network
		const daemonName = daemonFormRef?.getDaemonName() ?? defaultDaemonName;

		isLoading = true;

		try {
			const result = await daemonSetupMutation.mutateAsync({
				daemon_name: daemonName,
				network_id: selectedNetworkId,
				install_later: false
			});

			apiKey = result.api_key ?? null;
			daemonConfigured = true;

			// Update onboarding store
			onboardingStore.setDaemonSetup(selectedNetworkId, {
				name: daemonName,
				installNow: true,
				apiKey: result.api_key ?? undefined
			});

			// Set pending daemon setup flag for ScanProgressIndicator
			if (typeof localStorage !== 'undefined') {
				localStorage.setItem('pendingDaemonSetup', 'true');
			}

			// Mark other networks as install later
			for (const network of networks) {
				if (network.id && network.id !== selectedNetworkId) {
					onboardingStore.setDaemonSetup(network.id, {
						name: toKebabCase(network.name) + '-daemon',
						installNow: false
					});
				}
			}

			// Track daemon choice
			trackEvent('onboarding_daemon_choice', {
				choice: 'install_now',
				use_case: onboardingStore.getState().useCase
			});

			isLoading = false;
		} catch {
			isLoading = false;
			pushError('Failed to generate daemon key. Please try again.');
			return;
		}
	}

	function handleSkipClick() {
		showSkipConfirm = true;
	}

	function handleSkipCancel() {
		showSkipConfirm = false;
	}

	function handleExploreDemo() {
		showSkipConfirm = false;
		window.location.href = 'https://demo.scanopy.net';
	}

	// Determine button state
	let canContinue = $derived(selectedNetworkId !== null);
	let buttonText = $derived(() => {
		if (isLoading) return common_settingUp();
		if (daemonConfigured) return onboarding_continueToRegistration();
		return common_continue();
	});
</script>

<GenericModal
	{isOpen}
	title={onboarding_startScanning()}
	{onClose}
	size="xl"
	showCloseButton={false}
	preventCloseOnClickOutside={true}
>
	<div class="space-y-6 overflow-y-auto p-6">
		<div class="space-y-2">
			<p class="text-primary font-medium">{onboarding_selectDaemon()}</p>
			<p class="text-secondary text-sm">
				{onboarding_selectDaemonHelp()}
			</p>
		</div>

		<InlineInfo title={onboarding_daemonsActivateTitle()} body={onboarding_daemonsActivateBody()} />

		<!-- Network selection cards -->
		<div class="space-y-2">
			{#each networks as network (network.id)}
				{#if network.id}
					{@const isSelected = selectedNetworkId === network.id}
					<button
						type="button"
						class="card flex w-full items-center gap-4 p-4 text-left transition-all {isSelected
							? 'ring-2 ring-primary-500'
							: 'hover:bg-gray-800'}"
						onclick={() => network.id && selectNetwork(network.id)}
					>
						<div
							class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg {isSelected
								? 'text-primary-400 bg-primary-500/20'
								: 'bg-gray-700 text-gray-400'}"
						>
							{#if isSelected && daemonConfigured}
								<Check class="h-5 w-5" />
							{:else}
								<Network class="h-5 w-5" />
							{/if}
						</div>
						<div class="flex-1">
							<div class="text-primary font-medium">{network.name}</div>
							{#if isSelected && daemonConfigured}
								<div class="text-xs text-success">Daemon configured</div>
							{/if}
						</div>
						<div
							class="flex h-5 w-5 items-center justify-center rounded-full border-2 {isSelected
								? 'border-primary-500 bg-primary-500'
								: 'border-gray-500'}"
						>
							{#if isSelected}
								<div class="h-2 w-2 rounded-full bg-white"></div>
							{/if}
						</div>
					</button>
				{/if}
			{/each}
		</div>

		<!-- Show daemon form when network is selected and not yet configured -->
		{#if selectedNetworkId && selectedNetwork && !daemonConfigured}
			<div class="card space-y-4">
				<CreateDaemonForm
					bind:this={daemonFormRef}
					daemon={null}
					networkId={selectedNetworkId}
					apiKey={null}
					showAdvanced={false}
					initialName={defaultDaemonName}
					showModeSelect={false}
				/>
			</div>
		{/if}

		<!-- Show installation commands after daemon is configured -->
		{#if selectedNetworkId && daemonConfigured && apiKey}
			<div class="card space-y-4">
				<CreateDaemonForm
					daemon={null}
					networkId={selectedNetworkId}
					{apiKey}
					showAdvanced={true}
					initialName={defaultDaemonName}
					showModeSelect={false}
				/>
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<button
					type="button"
					class="text-secondary hover:text-primary text-sm underline"
					onclick={handleSkipClick}
				>
					{onboarding_skipDaemonSetup()}
				</button>
				<button
					type="button"
					class="btn-primary"
					disabled={!canContinue || isLoading}
					onclick={handleContinue}
				>
					{buttonText()}
				</button>
			</div>
		</div>
	{/snippet}
</GenericModal>

<!-- Skip confirmation modal -->
<ConfirmationDialog
	isOpen={showSkipConfirm}
	title={onboarding_skipConfirmTitle()}
	message={onboarding_skipConfirmBody()}
	confirmLabel={onboarding_selectADaemon()}
	cancelLabel={onboarding_exploreDemoInstead()}
	onConfirm={handleSkipCancel}
	onCancel={handleExploreDemo}
	variant="warning"
/>
