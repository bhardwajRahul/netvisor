<script lang="ts">
	import { ChevronDown, ChevronRight, Check, CalendarClock } from 'lucide-svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import CreateDaemonForm from '$lib/features/daemons/components/CreateDaemonForm.svelte';
	import { useDaemonSetupMutation } from '../../queries';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import type { NetworkSetup } from '../../types/base';
	import * as m from '$lib/paraglide/messages';

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

	interface NetworkCardState {
		choice: 'pending' | 'install_now' | 'install_later';
		apiKey: string | null;
		isExpanded: boolean;
		isLoading: boolean;
	}

	// Initialize card state for each network
	let cardStates = $state<Record<string, NetworkCardState>>({});

	// References to CreateDaemonForm components for getting daemon names
	let daemonFormRefs = $state<Record<string, CreateDaemonForm>>({});

	// Daemon setup mutation
	const daemonSetupMutation = useDaemonSetupMutation();

	// Initialize states for new networks
	$effect(() => {
		networks.forEach((network) => {
			if (network.id && !cardStates[network.id]) {
				cardStates[network.id] = {
					choice: 'pending',
					apiKey: null,
					isExpanded: false,
					isLoading: false
				};
			}
		});
	});

	async function handleInstallNow(networkId: string) {
		const state = cardStates[networkId];
		if (!state) return;

		const daemonName = daemonFormRefs[networkId]?.getDaemonName() ?? 'daemon';

		cardStates[networkId] = { ...state, isLoading: true };

		try {
			const result = await daemonSetupMutation.mutateAsync({
				daemon_name: daemonName,
				network_id: networkId,
				install_later: false
			});

			cardStates[networkId] = {
				...state,
				choice: 'install_now',
				apiKey: result.api_key ?? null,
				isExpanded: true,
				isLoading: false
			};

			// Update onboarding store
			onboardingStore.setDaemonSetup(networkId, {
				name: daemonName,
				installNow: true,
				apiKey: result.api_key ?? undefined
			});

			// Track daemon choice
			trackEvent('onboarding_daemon_choice', {
				choice: 'install_now',
				use_case: onboardingStore.getState().useCase
			});
		} catch {
			cardStates[networkId] = { ...state, isLoading: false };
		}
	}

	async function handleInstallLater(networkId: string) {
		const state = cardStates[networkId];
		if (!state) return;

		const daemonName = daemonFormRefs[networkId]?.getDaemonName() ?? 'daemon';

		cardStates[networkId] = { ...state, isLoading: true };

		try {
			await daemonSetupMutation.mutateAsync({
				daemon_name: daemonName,
				network_id: networkId,
				install_later: true
			});

			cardStates[networkId] = {
				...state,
				choice: 'install_later',
				isExpanded: false,
				isLoading: false
			};

			// Update onboarding store
			onboardingStore.setDaemonSetup(networkId, {
				name: daemonName,
				installNow: false
			});

			// Track daemon choice
			trackEvent('onboarding_daemon_choice', {
				choice: 'install_later',
				use_case: onboardingStore.getState().useCase
			});
		} catch {
			cardStates[networkId] = { ...state, isLoading: false };
		}
	}

	function toggleExpanded(networkId: string) {
		const state = cardStates[networkId];
		if (state && state.choice === 'install_now') {
			cardStates[networkId] = { ...state, isExpanded: !state.isExpanded };
		}
	}

	// Check if all networks have been configured
	let allConfigured = $derived(
		networks.every((n) => n.id && cardStates[n.id]?.choice !== 'pending')
	);
</script>

<GenericModal
	{isOpen}
	title={m.onboarding_startScanning()}
	{onClose}
	size="xl"
	showCloseButton={false}
	preventCloseOnClickOutside={true}
>
	<div class="space-y-6 overflow-y-auto p-6">
		<p class="text-secondary text-sm">
			{m.onboarding_daemonInstallInfo()}
		</p>

		<InlineInfo
			title={m.onboarding_daemonsActivateTitle()}
			body={m.onboarding_daemonsActivateBody()}
		/>

		<!-- Network cards -->
		<div class="space-y-4">
			{#each networks as network (network.id)}
				{#if network.id}
					{@const state = cardStates[network.id]}
					{#if state}
						<div class="card overflow-hidden">
							<!-- Header -->
							<div class="mb-2 flex items-center justify-between">
								<div class="flex items-center gap-3">
									{#if state.choice == 'install_now'}
										<div
											class="flex h-6 w-6 items-center justify-center rounded-full bg-success/20"
										>
											<Check class="h-5 w-5 text-success" />
										</div>
									{:else if state.choice == 'install_later'}
										<div class="bg-gray/20 flex h-6 w-6 items-center justify-center rounded-full">
											<CalendarClock class="text-tertiary h-5 w-5" />
										</div>
									{/if}
									<div>
										<span class="text-secondary"
											>{m.onboarding_daemonForNetwork({ networkName: network.name })}</span
										>
										{#if state.choice === 'install_later'}
											<div class="text-tertiary text-xs">
												{m.onboarding_installLaterHelp()}
											</div>
										{:else if state.choice === 'install_now'}
											<div class="text-xs text-success">
												{m.onboarding_installNowHelp()}
											</div>
										{/if}
									</div>
								</div>

								<div class="flex items-center gap-2">
									{#if state.choice === 'install_now' && network.id}
										<button
											type="button"
											class="btn-secondary"
											onclick={() => network.id && handleInstallLater(network.id)}
										>
											{m.onboarding_installLater()}
										</button>
										<button
											type="button"
											class="text-secondary hover:text-primary p-1"
											onclick={() => network.id && toggleExpanded(network.id)}
										>
											{#if state.isExpanded}
												<ChevronDown class="h-5 w-5" />
											{:else}
												<ChevronRight class="h-5 w-5" />
											{/if}
										</button>
									{:else if state.choice === 'install_later' && network.id}
										<button
											type="button"
											class="btn-secondary"
											onclick={() => network.id && handleInstallNow(network.id)}
										>
											{m.onboarding_installNow()}
										</button>
									{/if}
								</div>
							</div>

							<!-- Daemon configuration form -->
							<div class={`space-y-4 ${state.choice == 'install_now' ? 'mt-4' : ''}`}>
								{#if state.choice == 'pending' || (state.choice == 'install_now' && state.isExpanded && state.apiKey && network.id)}
									<CreateDaemonForm
										bind:this={daemonFormRefs[network.id]}
										daemon={null}
										networkId={network.id}
										apiKey={state.apiKey}
										showAdvanced={state.choice == 'install_now'}
										initialName={toKebabCase(network.name) + '-daemon'}
										showModeSelect={state.choice == 'install_now'}
									/>
								{/if}

								{#if state.choice == 'pending'}
									<div class="flex gap-2">
										<button
											type="button"
											class="btn-secondary flex-1"
											disabled={state.isLoading}
											onclick={() => network.id && handleInstallLater(network.id)}
										>
											{m.onboarding_installLater()}
										</button>
										<button
											type="button"
											class="btn-primary flex-1"
											disabled={state.isLoading}
											onclick={() => network.id && handleInstallNow(network.id)}
										>
											{state.isLoading ? m.common_settingUp() : m.onboarding_installNow()}
										</button>
									</div>
								{/if}
							</div>
						</div>
					{/if}
				{/if}
			{/each}
		</div>
	</div>

	{#snippet footer()}
		<div class="modal-footer">
			<div class="flex justify-end">
				<button type="button" class="btn-primary" disabled={!allConfigured} onclick={onComplete}>
					{allConfigured
						? m.onboarding_continueToRegistration()
						: m.onboarding_configureRemaining()}
				</button>
			</div>
		</div>
	{/snippet}
</GenericModal>
