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
	import { SatelliteDish } from 'lucide-svelte';
	import {
		createEmptyApiKeyFormData,
		useCreateApiKeyMutation
	} from '$lib/features/daemon_api_keys/queries';
	import { useProvisionDaemonMutation } from '../queries';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import CreateDaemonForm from './CreateDaemonForm.svelte';
	import {
		common_close,
		common_continue,
		common_failedGenerateApiKey,
		daemons_activateAfterCreation,
		daemons_activateAfterCreationBody,
		daemons_createDaemon,
		daemons_doThisLater,
		daemons_enterApiKey,
		daemons_keyNotAvailableWarning,
		daemons_networkCannotChange,
		daemons_option1,
		daemons_option2,
		daemons_setupScanning,
		daemons_stopDaemonContainer,
		daemons_stopDaemonProcess
	} from '$lib/paraglide/messages';

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
	const provisionDaemonMutation = useProvisionDaemonMutation();

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

	async function handleUseExistingKey() {
		// Validate form first (same pattern as handleCreateNewApiKey)
		const isValid = await daemonFormRef?.validate();
		if (!isValid) {
			return;
		}

		const trimmedKey = daemonFormRef?.getExistingKeyInput()?.trim() ?? '';
		if (!trimmedKey) {
			pushError(daemons_enterApiKey());
			return;
		}

		keyState = trimmedKey;
	}

	/**
	 * Construct full daemon URL from base URL and port.
	 * Combines the base URL (without port) with the specified port number.
	 */
	function constructDaemonUrl(baseUrl: string, port: number): string {
		try {
			// Parse the URL to extract components without mutating
			const parsed = new globalThis.URL(baseUrl);
			const protocol = parsed.protocol;
			const hostname = parsed.hostname;
			const pathname = parsed.pathname === '/' ? '' : parsed.pathname;
			// Construct URL with port
			return `${protocol}//${hostname}:${port}${pathname}`;
		} catch {
			// Fallback for malformed URLs
			return `${baseUrl}:${port}`;
		}
	}

	async function handleCreateNewApiKey() {
		// Validate form first
		const isValid = await daemonFormRef?.validate();
		if (!isValid) {
			return;
		}

		const daemonName = daemonFormRef?.getDaemonName() ?? 'daemon';
		const form = daemonFormRef?.getForm();
		const mode = (form?.state.values['mode'] as string) ?? 'daemon_poll';
		const daemonUrlBase = (form?.state.values['daemonUrl'] as string) ?? '';
		const daemonPort = daemonFormRef?.getDaemonPort() ?? 60073;

		if (mode === 'server_poll') {
			// ServerPoll mode: Call provision endpoint which creates daemon + API key
			// Combine base URL with port for full daemon URL
			const fullDaemonUrl = constructDaemonUrl(daemonUrlBase, daemonPort);

			try {
				const result = await provisionDaemonMutation.mutateAsync({
					name: daemonName,
					network_id: selectedNetworkId,
					url: fullDaemonUrl
				});
				keyState = result.daemon_api_key;
			} catch {
				pushError(common_failedGenerateApiKey());
			}
		} else {
			// DaemonPoll mode: Just create API key, daemon will self-register
			let newApiKey = createEmptyApiKeyFormData(selectedNetworkId);
			newApiKey.name = `${daemonName} Api Key`;

			try {
				const result = await createApiKeyMutation.mutateAsync(newApiKey);
				keyState = result.keyString;
			} catch {
				pushError(common_failedGenerateApiKey());
			}
		}
	}

	// For existing daemon with new key - simple run command
	let existingDaemonRunCommand = $derived(
		daemon && key ? `sudo scanopy-daemon --server-url ${serverUrl} --daemon-api-key ${key}` : ''
	);

	let colorHelper = entities.getColorHelper('Daemon');
	let title = $derived(onboardingMode ? daemons_setupScanning() : daemons_createDaemon());
</script>

<GenericModal
	{isOpen}
	{title}
	size="xl"
	onClose={handleOnClose}
	showCloseButton={!onboardingMode}
	showBackdrop={!onboardingMode}
>
	{#snippet headerIcon()}
		{#if onboardingMode}
			<ModalHeaderIcon Icon={SatelliteDish} color="Green" />
		{:else}
			<ModalHeaderIcon Icon={entities.getIconComponent('Daemon')} color={colorHelper.color} />
		{/if}
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-4">
				{#if onboardingMode}
					<!-- Onboarding mode: show info banner -->
					<InlineInfo
						title={daemons_activateAfterCreation()}
						body={daemons_activateAfterCreationBody()}
					/>
				{:else if !daemon}
					<SelectNetwork
						bind:selectedNetworkId
						disabled={!!key}
						disabledReason={daemons_networkCannotChange()}
					/>
				{/if}

				<!-- Use the extracted CreateDaemonForm component -->
				<CreateDaemonForm
					bind:this={daemonFormRef}
					{daemon}
					networkId={selectedNetworkId}
					apiKey={key}
					showAdvanced={!onboardingMode || !!key}
					allowExistingKey={!onboardingMode && !daemon}
					keySet={!!key}
					onGenerateKey={handleCreateNewApiKey}
					onUseExistingKey={handleUseExistingKey}
				/>

				<!-- Existing daemon with new key warning -->
				{#if daemon && key && selectedNetworkId}
					<InlineWarning title={daemons_keyNotAvailableWarning()} />

					<div class="text-secondary mt-3">
						<b>{daemons_option1()}</b>
						{daemons_stopDaemonProcess()}
					</div>
					<CodeContainer language="bash" expandable={false} code={existingDaemonRunCommand} />
					<div class="text-secondary mt-3">
						<b>{daemons_option2()}</b>
						{daemons_stopDaemonContainer()}
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
							{daemons_doThisLater()}
						</button>
					{/if}
					<button type="button" class="btn-primary ml-auto" onclick={onContinue ?? handleOnClose}>
						{common_continue()}
					</button>
				</div>
			{:else}
				<div class="flex items-center justify-end">
					<button type="button" class="btn-secondary" onclick={handleOnClose}>
						{common_close()}
					</button>
				</div>
			{/if}
		</div>
	</div>
</GenericModal>
