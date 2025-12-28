<script lang="ts">
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import { entities } from '$lib/shared/stores/metadata';
	import type { Daemon } from '../types/base';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import { RotateCcwKey, SatelliteDish } from 'lucide-svelte';
	import {
		createEmptyApiKeyFormData,
		useCreateApiKeyMutation
	} from '$lib/features/api_keys/queries';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import CreateDaemonForm from './CreateDaemonForm.svelte';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		daemon?: Daemon | null;
		onboardingMode?: boolean;
		onSkip?: (() => void) | null;
		onContinue?: (() => void) | null;
		provisionalApiKey?: string | null;
		provisionalNetworkId?: string | null;
	}

	let {
		isOpen = false,
		onClose,
		daemon = null,
		onboardingMode = false,
		onSkip = null,
		onContinue = null,
		provisionalApiKey = null,
		provisionalNetworkId = null
	}: Props = $props();

	const networksQuery = useNetworksQuery();
	const configQuery = useConfigQuery();
	const createApiKeyMutation = useCreateApiKeyMutation();

	let networksData = $derived(networksQuery.data ?? []);
	let configData = $derived(configQuery.data);

	let keyState = $state<string | null>(null);
	// In onboarding mode, use the provisionalApiKey; otherwise use keyState
	let key = $derived(onboardingMode ? provisionalApiKey : keyState);
	// In onboarding mode, use the provisionalNetworkId; otherwise use first network or daemon's network
	let selectedNetworkId = $state('');

	$effect(() => {
		if (daemon) {
			selectedNetworkId = daemon.network_id;
		} else if (onboardingMode && provisionalNetworkId) {
			selectedNetworkId = provisionalNetworkId;
		} else if (!selectedNetworkId && networksData[0]?.id) {
			selectedNetworkId = networksData[0].id;
		}
	});

	let serverUrl = $derived(configData?.public_url ?? '');

	// Reference to CreateDaemonForm for getting daemon name
	let daemonFormRef: CreateDaemonForm;

	function handleOnClose() {
		keyState = null;
		onClose();
	}

	async function handleCreateNewApiKey() {
		// Validate form first
		const isValid = await daemonFormRef?.validate();
		if (!isValid) {
			return;
		}

		const daemonName = daemonFormRef?.getDaemonName() ?? 'daemon';
		let newApiKey = createEmptyApiKeyFormData(selectedNetworkId);
		newApiKey.name = `${daemonName} Api Key`;

		try {
			const result = await createApiKeyMutation.mutateAsync(newApiKey);
			keyState = result.keyString;
		} catch {
			pushError('Failed to generate API key');
		}
	}

	// For existing daemon with new key - simple run command
	let existingDaemonRunCommand = $derived(
		daemon && key ? `sudo scanopy-daemon --server-url ${serverUrl} --daemon-api-key ${key}` : ''
	);

	let colorHelper = entities.getColorHelper('Daemon');
	let title = $derived(onboardingMode ? 'Set up network scanning' : 'Create Daemon');
</script>

<GenericModal
	{isOpen}
	{title}
	size="xl"
	onClose={handleOnClose}
	showCloseButton={!onboardingMode}
	showBackdrop={!onboardingMode}
>
	<svelte:fragment slot="header-icon">
		{#if onboardingMode}
			<ModalHeaderIcon Icon={SatelliteDish} color="Green" />
		{:else}
			<ModalHeaderIcon Icon={entities.getIconComponent('Daemon')} color={colorHelper.color} />
		{/if}
	</svelte:fragment>

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-4">
				{#if onboardingMode}
					<!-- Onboarding mode: show info banner -->
					<InlineInfo
						title="Your daemon will activate after account creation"
						body="To visualize your network, Scanopy needs to discover what's on it. Install the daemon below—after registration, it'll connect and start mapping automatically."
					/>
				{:else if !daemon}
					<SelectNetwork bind:selectedNetworkId />
				{/if}

				<!-- Use the extracted CreateDaemonForm component -->
				<CreateDaemonForm
					bind:this={daemonFormRef}
					{daemon}
					networkId={selectedNetworkId}
					apiKey={key}
					showAdvanced={!onboardingMode || !!key}
				/>

				<!-- API Key Section (hidden in onboarding mode) -->
				{#if !onboardingMode && !daemon}
					<div class="pb-2">
						<div class="flex items-start gap-2">
							<button
								class="btn-primary m-1 flex-shrink-0 self-stretch"
								disabled={!!key}
								type="button"
								onclick={handleCreateNewApiKey}
							>
								<RotateCcwKey />
								<span>Generate Key</span>
							</button>

							<div class="flex-1">
								<CodeContainer
									language="bash"
									expandable={false}
									code={key ? key : 'Press Generate Key...'}
								/>
							</div>
						</div>
						{#if !key}
							<div class="text-tertiary mt-1 text-xs">
								This will create a new API key, which you can manage later in the API Keys tab.
							</div>
						{/if}
					</div>
				{/if}

				<!-- Existing daemon with new key warning -->
				{#if daemon && key && selectedNetworkId}
					<InlineWarning
						title="This API key will not be available once you close this modal. Please use the provided run command or update your docker compose with the API key as depicted below."
					/>

					<div class="text-secondary mt-3">
						<b>Option 1.</b> Stop the daemon process, and use this command to start it
					</div>
					<CodeContainer language="bash" expandable={false} code={existingDaemonRunCommand} />
					<div class="text-secondary mt-3">
						<b>Option 2.</b> Stop the daemon container, and add this environment variable
					</div>
					<CodeContainer
						language="bash"
						expandable={false}
						code={`- SCANOPY_DAEMON_API_KEY=${key}\n`}
					/>
				{/if}
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			{#if onboardingMode}
				<div class="flex w-full items-center justify-between gap-4">
					{#if onSkip}
						<button type="button" class="btn-secondary" onclick={onSkip}>
							I'll do this later
						</button>
					{/if}
					<button type="button" class="btn-primary ml-auto" onclick={onContinue ?? handleOnClose}>
						Continue
					</button>
				</div>
			{:else}
				<div class="flex items-center justify-end">
					<button type="button" class="btn-secondary" onclick={handleOnClose}> Close </button>
				</div>
			{/if}
		</div>
	</div>
</GenericModal>
