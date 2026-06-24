<script lang="ts">
	import { untrack, tick } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { entities } from '$lib/shared/stores/metadata';
	import {
		Settings,
		Terminal,
		Loader2,
		ArrowRight,
		ArrowLeft,
		Mail,
		Copy,
		Check,
		KeyRound
	} from 'lucide-svelte';
	import confetti from 'canvas-confetti';
	import {
		createEmptyApiKeyFormData,
		useCreateApiKeyMutation
	} from '$lib/features/daemon_api_keys/queries';
	import {
		useProvisionDaemonMutation,
		useDaemonQuery,
		useEmailInstallCommandMutation
	} from '../../queries';
	import { apiClient } from '$lib/api/client';
	import { useConfigQuery, isCloud } from '$lib/shared/stores/config-query';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useTestReachabilityMutation } from '../../queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { getVisibleFieldIds } from '../../config';
	import {
		buildDefaultValues,
		buildRunCommand,
		buildDockerCompose,
		constructDaemonUrl,
		detectOS,
		slugifyNetworkName,
		type DaemonOS
	} from '../../utils';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import {
		useBulkCreateCredentialsMutation,
		useDeleteCredentialMutation,
		useUpdateCredentialMutation,
		useCredentialsQuery
	} from '$lib/features/credentials/queries';
	import { daemonSetupState, type DaemonConnectionStatus } from '../../stores/daemon-setup';
	import ConfigureStep from './steps/ConfigureStep.svelte';
	import InstallStep from './steps/InstallStep.svelte';
	import AdvancedStep from './steps/AdvancedStep.svelte';
	import CredentialTypeSelectStep from './steps/CredentialTypeSelectStep.svelte';
	import CredentialWizardStep, {
		type PendingCredential
	} from './steps/CredentialWizardStep.svelte';
	import {
		common_close,
		common_configure,
		common_continue,
		common_credentials,
		common_failedGenerateApiKey,
		common_install,
		common_next,
		common_skip,
		daemons_createDaemon,
		daemons_credentialWizardReturn,
		daemons_credentialWizardReturnToInstall,
		daemons_enterApiKey,
		daemons_emailInstallCommand,
		daemons_installCommandEmailed,
		daemons_installIveRunCommand,
		daemons_installIveStartedDocker,
		daemons_installCopyCommand,
		daemons_installCopiedToastLinux,
		daemons_installCopiedToastDocker,
		daemons_installCopiedToastMac,
		daemons_installCopiedToastWindows,
		daemons_installBackToInstall,
		daemons_installReturnToCommands
	} from '$lib/paraglide/messages';
	import { createDefaultCredential } from '$lib/features/credentials/types/base';
	import { credentialTypes } from '$lib/shared/stores/metadata';
	import { v4 as uuidv4 } from 'uuid';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onNavigate?: (tab: string) => void;
		name?: string;
	}

	let { isOpen = false, onClose, onNavigate, name = undefined }: Props = $props();

	// Queries & mutations
	const configQuery = useConfigQuery();
	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();
	const createApiKeyMutation = useCreateApiKeyMutation();
	const provisionDaemonMutation = useProvisionDaemonMutation();
	const bulkCreateCredentialsMutation = useBulkCreateCredentialsMutation();
	const deleteCredentialMutation = useDeleteCredentialMutation();
	const updateCredentialMutation = useUpdateCredentialMutation();
	const credentialsQuery = useCredentialsQuery();

	// Derived data
	let serverUrl = $derived(configQuery.data?.public_url ?? '');
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);
	let currentUserId = $derived(currentUserQuery.data?.id ?? null);
	let org = $derived(organizationQuery.data);
	let isFirstDaemon = $derived(!org?.onboarding?.includes('FirstDaemonRegistered'));
	// Snapshot: tracks whether wizard was opened as first-daemon flow.
	// Prevents reactivity from flipping showWaitingUI when FirstDaemonRegistered appears.
	let startedAsFirstDaemon = $state(false);
	let hasEmailSupport = $derived.by(() => {
		if (!org?.plan?.type) return false;
		return billingPlans.getMetadata(org.plan.type).features.email_support;
	});

	// Email install command
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);
	const emailInstallMutation = useEmailInstallCommandMutation();
	const installScript = `bash -c "$(curl -fsSL https://raw.githubusercontent.com/scanopy/scanopy/refs/heads/main/install.sh)"`;
	const windowsDownloadUrl =
		'https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.exe';
	let currentInstallCommand = $derived.by(() => {
		if (selectedOS === 'windows')
			return `Invoke-WebRequest -Uri "${windowsDownloadUrl}" -OutFile "scanopy-daemon-windows-amd64.exe"; ${runCommand}`;
		if (selectedOS === 'linux' && linuxMethod === 'docker' && dockerCompose) return dockerCompose;
		return `${installScript} && ${runCommand}`;
	});
	let currentOsLabel = $derived.by(() => {
		if (selectedOS === 'linux') return linuxMethod === 'docker' ? 'Linux (Docker)' : 'Linux';
		if (selectedOS === 'macos') return 'macOS';
		if (selectedOS === 'windows') return 'Windows';
		return selectedOS;
	});

	// Networks
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// Network selection
	let selectedNetworkId = $state('');
	let nameManuallyEdited = $state(false);

	// API key state
	let keyState = $state<string | null>(null);
	let key = $derived(keyState);
	let keySet = $derived(!!key);

	// Auto-generation state (for first daemon flow)
	let isAutoGenerating = $state(false);

	// Docker config state
	let dockerMode = $state<string>('local_socket');

	// Credentials is its own stepper step (activeTab === 'credentials'); this
	// tracks the sub-view within it: the type grid, then the wizard.
	let credentialWizardRef: ReturnType<typeof CredentialWizardStep> | undefined = $state();
	let credentialSubStep = $state<'typeSelect' | 'wizard'>('typeSelect');
	let selectedCredentialTypeIds = $state<string[]>([]);
	// Auto-local capability toggles (e.g. DockerSocket) — map to daemon install flags.
	let localCapabilityEnabled = $state<Record<string, boolean>>({});
	let pendingCredentials = $state<PendingCredential[]>([]);
	let credentialIds = $state<string[]>([]);
	let hasDockerProxyCredential = $derived(
		pendingCredentials.some(
			(p) =>
				p.credential.credential_type.type === 'DockerProxy' &&
				p.targetIps.some((ip) => ip === '127.0.0.1' || ip === '::1')
		)
	);
	let unsavedCredentialCount = $derived(
		pendingCredentials.filter((p) => !p.isExisting && !credentialIds.includes(p.credential.id))
			.length
	);

	function initDefaultFieldValues(typeId: string): Record<string, string> {
		const meta = credentialTypes.getMetadata(typeId);
		const fields = meta?.fields ?? [];
		const values: Record<string, string> = {};
		for (const field of fields) {
			if (field.field_type === 'pathorinline') {
				values[field.id] = JSON.stringify({ mode: 'Inline', value: '' });
			} else {
				values[field.id] = field.default_value ?? '';
			}
		}
		return values;
	}

	function handleNavigateToCredentialWizard() {
		// Add a DockerProxy pending credential if one doesn't already exist
		if (!hasDockerProxyCredential && org) {
			const cred = {
				...createDefaultCredential(org.id),
				id: uuidv4(),
				name: (formValues.name as string) || 'scanopy-daemon',
				credential_type: {
					type: 'DockerProxy'
				} as import('$lib/features/credentials/types/base').Credential['credential_type']
			};
			// Set defaults from fixture metadata
			const meta = credentialTypes.getMetadata('DockerProxy');
			if (meta?.fields) {
				const ct = cred.credential_type as unknown as Record<string, unknown>;
				for (const field of meta.fields) {
					if (field.default_value != null && ct[field.id] === undefined) {
						if (field.field_type === 'secretpathorinline' || field.field_type === 'pathorinline') {
							ct[field.id] = { mode: 'Inline', value: field.default_value };
						} else {
							const num = Number(field.default_value);
							ct[field.id] = !isNaN(num) ? num : field.default_value;
						}
					}
				}
			}
			const fieldValues = initDefaultFieldValues('DockerProxy');
			pendingCredentials = [
				...pendingCredentials,
				{ credential: cred, targetIps: ['127.0.0.1'], fieldValues }
			];
		}
		showAdvanced = false;
		if (furthestReached < 1) furthestReached = 1;
		activeTab = 'credentials';
		credentialSubStep = 'wizard';
	}

	// Open the Credentials step at the type-selection grid (from the Install CTA).
	function openCredentialStep() {
		if (furthestReached < 1) furthestReached = 1;
		showAdvanced = false;
		activeTab = 'credentials';
		credentialSubStep = 'typeSelect';
	}

	// Move from the type-selection grid into the wizard, seeding the chosen types.
	async function handleContinueToWizard() {
		credentialSubStep = 'wizard';
		await tick();
		credentialWizardRef?.addTypes(selectedCredentialTypeIds);
	}

	function handleSkipCredentials() {
		trackEvent('daemon_wizard_step_completed', {
			step: 'credentials',
			skipped: true,
			types_selected: selectedCredentialTypeIds.length,
			credentials_attached: 0
		});
		activeTab = 'install';
	}

	// OS selection
	let selectedOS: DaemonOS = $state(detectOS());
	let linuxMethod = $state<'binary' | 'docker'>('binary');
	let isDockerInstall = $derived(selectedOS === 'linux' && linuxMethod === 'docker');
	let installCtaLabel = $derived(
		isDockerInstall ? daemons_installIveStartedDocker() : daemons_installIveRunCommand()
	);
	let hasCopied = $state(false);

	// Reset hasCopied when the install command changes (Advanced settings, credentials, etc.)
	let prevInstallCommand = $state('');
	$effect(() => {
		const cmd = currentInstallCommand;
		if (hasCopied && prevInstallCommand && cmd !== prevInstallCommand) {
			hasCopied = false;
		}
		prevInstallCommand = cmd;
	});

	// ServerPoll reachability state
	let serverPollReachable = $state<boolean | null>(null);
	let isTestingReachability = $state(false);
	let serverPollReachabilityResult = $state<{ reachable: boolean; error?: string } | null>(null);
	const testReachabilityMutation = useTestReachabilityMutation();

	// Connection waiting state
	let provisionedDaemonId = $state('');
	let connectionStatus = $state<DaemonConnectionStatus>('idle');
	let troubleTimeoutId = $state<ReturnType<typeof setTimeout> | null>(null);
	let daemonIdsAtWaitStart = $state<Set<string>>(new Set());

	// Daemon-specific queries for connection detection
	const provisionedDaemonQuery = useDaemonQuery(() => provisionedDaemonId || null, {
		enabled: () =>
			(connectionStatus === 'waiting' || connectionStatus === 'trouble') && !!provisionedDaemonId
	});
	function getDefaultDaemonName(networkId: string): string {
		const network = networksData.find((n) => n.id === networkId);
		if (network) {
			const slug = slugifyNetworkName(network.name);
			if (slug) return `scanopy-daemon-${slug}`;
		}
		return 'scanopy-daemon';
	}

	// Auto-select first network when SelectNetwork is hidden (first daemon)
	$effect(() => {
		if (isFirstDaemon && !selectedNetworkId && networksData.length > 0) {
			selectedNetworkId = networksData[0].id;
		}
	});

	// Per-step funnel analytics: fire once each time a step becomes the active step
	let lastViewedStep = $state<string | null>(null);
	$effect(() => {
		if (!isOpen) {
			lastViewedStep = null;
			return;
		}
		if (activeTab !== lastViewedStep) {
			lastViewedStep = activeTab;
			trackEvent('daemon_wizard_step_viewed', { step: activeTab });
		}
	});

	$effect(() => {
		if (selectedNetworkId && !nameManuallyEdited) {
			const defaultName = getDefaultDaemonName(selectedNetworkId);
			untrack(() => form.setFieldValue('name', defaultName));
		}
	});

	// TanStack Form
	const form = createForm(() => ({
		defaultValues: buildDefaultValues(),
		onSubmit: async () => {
			// No-op; submission is handled by step navigation
		}
	}));

	// Reactive form values (form.state.values is NOT tracked by $derived)
	let formValues = $state<Record<string, string | number | boolean>>(buildDefaultValues());

	$effect(() => {
		return form.store.subscribe(() => {
			formValues = { ...form.state.values } as Record<string, string | number | boolean>;
		});
	});

	// Derived commands
	let dockerConfig = $derived({
		mode: dockerMode,
		credentialId: null as string | null,
		// Local Docker socket is disabled when a localhost proxy is configured, or
		// when the user turns off the "Scan local Docker" integration toggle.
		disableLocalSocket: hasDockerProxyCredential || localCapabilityEnabled['DockerSocket'] === false
	});
	let allCredentialIds = $derived([...credentialIds]);
	let runCommand = $derived(
		buildRunCommand(
			serverUrl,
			selectedNetworkId,
			key,
			formValues,
			null,
			currentUserId,
			selectedOS,
			dockerConfig,
			allCredentialIds
		)
	);
	let dockerCompose = $derived(
		key
			? buildDockerCompose(
					serverUrl,
					selectedNetworkId,
					key,
					formValues,
					currentUserId,
					dockerConfig,
					allCredentialIds
				)
			: ''
	);

	// Check for form validation errors (only visible fields)
	let visibleFields = $derived(getVisibleFieldIds(formValues));
	let hasErrors = $derived.by(() => {
		const fieldMeta = form.state.fieldMeta;
		for (const fieldKey of Object.keys(fieldMeta)) {
			if (!visibleFields.has(fieldKey)) continue;
			const meta = fieldMeta[fieldKey];
			if (meta?.errors && meta.errors.length > 0) {
				return true;
			}
		}
		return false;
	});

	// --- Tab / step state ---
	// Steps: Configure -> Credentials (optional) -> Install
	let activeTab = $state('configure');
	let furthestReached = $state(0);
	let showAdvanced = $state(false);

	let tabs: ModalTab[] = $derived([
		{ id: 'configure', label: common_configure(), icon: Settings },
		{
			id: 'credentials',
			label: common_credentials(),
			icon: KeyRound,
			disabled: furthestReached < 1
		},
		{
			id: 'install',
			label: common_install(),
			icon: Terminal,
			disabled: furthestReached < 1
		}
	]);

	function handleTabChange(tabId: string) {
		showAdvanced = false;
		activeTab = tabId;
	}

	// --- Key generation ---
	async function handleCreateNewApiKey() {
		const fields = getVisibleFieldIds(formValues);
		const isValid = await validateForm(form, fields);
		if (!isValid) return;

		const daemonName = (form.state.values['name'] as string) ?? 'daemon';
		const mode = (form.state.values['mode'] as string) ?? 'daemon_poll';
		const daemonUrlBase = (form.state.values['daemonUrl'] as string) ?? '';
		const daemonPort = (() => {
			const port = form.state.values['daemonPort'];
			return typeof port === 'number' ? port : 60073;
		})();

		if (mode === 'server_poll') {
			const fullDaemonUrl = constructDaemonUrl(daemonUrlBase, daemonPort);
			try {
				const result = await provisionDaemonMutation.mutateAsync({
					name: daemonName,
					network_id: selectedNetworkId,
					url: fullDaemonUrl
				});
				keyState = result.daemon_api_key;
				provisionedDaemonId = result.daemon.id;
			} catch {
				pushError(common_failedGenerateApiKey());
			}
		} else {
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

	async function handleUseExistingKey() {
		const fields = getVisibleFieldIds(formValues);
		const isValid = await validateForm(form, fields);
		if (!isValid) return;

		const trimmedKey = ((form.state.values['existingKeyInput'] as string) ?? '').trim();
		if (!trimmedKey) {
			pushError(daemons_enterApiKey());
			return;
		}
		keyState = trimmedKey;
	}

	// --- Navigation handlers ---
	async function handleNext() {
		if (activeTab === 'configure') {
			const fields = getVisibleFieldIds(formValues);
			const isValid = await validateForm(form, fields);

			if (!isValid) return;

			// ServerPoll: run reachability test from Next button (cloud only — self-hosted doesn't need port forwarding)
			if (formValues.mode === 'server_poll' && isCloudDeployment && serverPollReachable !== true) {
				const daemonUrlBase = String(formValues.daemonUrl ?? '');
				if (!daemonUrlBase) return;
				const port = Number(formValues.daemonPort) || 60073;
				const fullUrl = constructDaemonUrl(daemonUrlBase, port);
				isTestingReachability = true;
				try {
					const result = await testReachabilityMutation.mutateAsync({
						url: fullUrl,
						check_health: false
					});
					serverPollReachable = result.reachable;
					serverPollReachabilityResult = {
						reachable: result.reachable,
						error: result.error ?? undefined
					};
					if (!result.reachable) return; // stay on step, result shown inline
				} catch {
					serverPollReachable = false;
					serverPollReachabilityResult = {
						reachable: false,
						error: 'Failed to test reachability'
					};
					return;
				} finally {
					isTestingReachability = false;
				}
			}

			trackEvent('daemon_wizard_step_completed', { step: 'configure' });

			// Auto-generate key for: first daemon (any mode), or server_poll, or daemon_poll with generate source
			const mode = formValues.mode as string;
			const keySource = formValues.keySource as string;
			const needsAutoGenerate =
				!key && (isFirstDaemon || mode === 'server_poll' || keySource === 'generate');

			if (needsAutoGenerate) {
				isAutoGenerating = true;
				try {
					await handleCreateNewApiKey();
				} finally {
					isAutoGenerating = false;
				}
				if (!keyState) return; // generation failed
			}

			// For daemon_poll with existing key source, key must be set already
			if (!isFirstDaemon && mode === 'daemon_poll' && keySource === 'existing' && !key) {
				pushError(daemons_enterApiKey());
				return;
			}

			// Snapshot daemon IDs NOW, before showing install commands.
			// Must happen before user can install, so fast-connecting daemons are detected.
			if (formValues.mode !== 'server_poll') {
				try {
					const { data } = await apiClient.GET('/api/v1/daemons', {
						params: { query: { limit: 0 } }
					});
					const daemons = data?.data ?? [];
					daemonIdsAtWaitStart = new Set(daemons.map((d) => d.id));
				} catch {
					daemonIdsAtWaitStart = new Set();
				}
			}

			if (furthestReached < 1) furthestReached = 1;
			// Advance to the Credentials step (step 2). It's optional — the user can
			// Skip to Install (step 3), which is also unlocked.
			activeTab = 'credentials';
			credentialSubStep = 'typeSelect';
		}
	}

	// --- Connection waiting ---
	function startWaitingTimeout() {
		if (troubleTimeoutId) return; // already started
		troubleTimeoutId = setTimeout(() => {
			if (connectionStatus === 'waiting') {
				connectionStatus = 'trouble';
				daemonSetupState.set({ connectionStatus: 'trouble' });
				trackEvent('daemon_connection_timeout');
			}
		}, 45_000);
	}

	function getCopiedToastMessage(): string {
		if (selectedOS === 'linux' && linuxMethod === 'docker')
			return daemons_installCopiedToastDocker();
		if (selectedOS === 'macos') return daemons_installCopiedToastMac();
		if (selectedOS === 'windows') return daemons_installCopiedToastWindows();
		return daemons_installCopiedToastLinux();
	}

	async function handleCopyCommand() {
		if (!currentInstallCommand) return;
		try {
			await navigator.clipboard.writeText(currentInstallCommand);
			hasCopied = true;
			trackEvent('daemon_install_command_copied', { os: selectedOS, context: 'footer-cta' });
			pushSuccess(getCopiedToastMessage());
		} catch {
			// Clipboard API failed — user can still use inline copy icon
		}
	}

	function handleInstalled() {
		connectionStatus = 'waiting';
		daemonSetupState.set({ connectionStatus: 'waiting' });
		trackEvent('daemon_install_confirmed');

		// DaemonPoll: start timeout immediately. ServerPoll: wait for health check to pass.
		if (formValues.mode !== 'server_poll') {
			startWaitingTimeout();
		}
	}

	function handleReviewCommands() {
		connectionStatus = 'idle';
		// Don't update store — polling continues via org query
	}

	function handleViewDiscovery() {
		onNavigate?.('topology');
		handleOnClose();
	}

	function handleProgressComplete() {
		if (connectionStatus === 'waiting') {
			connectionStatus = 'trouble';
			daemonSetupState.set({ connectionStatus: 'trouble' });
			trackEvent('daemon_connection_timeout');
		}
	}

	function handleReviewCommandsFromTrouble() {
		connectionStatus = 'idle';
		trackEvent('daemon_trouble_review_commands');
	}

	function handleEnableSelfSigned() {
		form.setFieldValue('allowSelfSignedCerts', true);
		connectionStatus = 'idle';
		trackEvent('daemon_trouble_enable_self_signed');
	}

	function markConnected() {
		connectionStatus = 'connected';
		daemonSetupState.set({ connectionStatus: 'connected' });
		if (troubleTimeoutId) {
			clearTimeout(troubleTimeoutId);
			troubleTimeoutId = null;
		}
		confetti({ particleCount: 100, spread: 70, origin: { y: 0.6 } });
		trackEvent('daemon_connected');
	}

	// ServerPoll: poll provisionedDaemonQuery every 5s when waiting/trouble
	$effect(() => {
		if ((connectionStatus === 'waiting' || connectionStatus === 'trouble') && provisionedDaemonId) {
			const interval = setInterval(() => {
				provisionedDaemonQuery.refetch();
			}, 5000);
			return () => clearInterval(interval);
		}
	});

	// ServerPoll: detect connection when last_seen becomes non-null
	$effect(() => {
		if (
			(connectionStatus === 'waiting' || connectionStatus === 'trouble') &&
			provisionedDaemonId &&
			provisionedDaemonQuery.data?.last_seen
		) {
			markConnected();
		}
	});

	// DaemonPoll: poll API directly every 5s when waiting/trouble, detect new daemon
	$effect(() => {
		if (
			(connectionStatus === 'waiting' || connectionStatus === 'trouble') &&
			!provisionedDaemonId
		) {
			let active = true;

			async function checkForNewDaemon() {
				if (!active) return;
				try {
					const { data } = await apiClient.GET('/api/v1/daemons', {
						params: { query: { limit: 0 } }
					});
					if (!active) return;
					const currentIds = (data?.data ?? []).map((d) => d.id);
					const hasNewDaemon = currentIds.some((id) => !daemonIdsAtWaitStart.has(id));
					if (hasNewDaemon) {
						markConnected();
					}
				} catch {
					// Ignore fetch errors, will retry on next interval
				}
			}

			// Check immediately, then every 5s
			checkForNewDaemon();
			const interval = setInterval(checkForNewDaemon, 5000);

			return () => {
				active = false;
				clearInterval(interval);
			};
		}
	});

	// --- Close / Open ---
	function handleOnClose() {
		trackEvent('daemon_wizard_closed');

		daemonSetupState.set({ connectionStatus: 'idle' });

		if (troubleTimeoutId) {
			clearTimeout(troubleTimeoutId);
			troubleTimeoutId = null;
		}

		keyState = null;
		isAutoGenerating = false;
		nameManuallyEdited = false;
		activeTab = 'configure';
		furthestReached = 0;
		showAdvanced = false;
		credentialSubStep = 'typeSelect';
		selectedCredentialTypeIds = [];
		localCapabilityEnabled = {};
		pendingCredentials = [];
		credentialIds = [];
		connectionStatus = 'idle';
		serverPollReachable = null;
		isTestingReachability = false;
		serverPollReachabilityResult = null;
		dockerMode = 'local_socket';
		daemonIdsAtWaitStart = new Set();

		// Reset form fields to defaults so advanced overrides don't persist
		const defaults = buildDefaultValues();
		for (const [key, value] of Object.entries(defaults)) {
			form.setFieldValue(key, value);
		}

		onClose();
	}

	function handleOpen() {
		trackEvent('daemon_wizard_opened');
		nameManuallyEdited = false;
		activeTab = 'configure';
		furthestReached = 0;
		showAdvanced = false;
		credentialSubStep = 'typeSelect';
		selectedCredentialTypeIds = [];
		localCapabilityEnabled = {};
		connectionStatus = 'idle';
		startedAsFirstDaemon = isFirstDaemon;
		serverPollReachable = null;
		serverPollReachabilityResult = null;
		daemonIdsAtWaitStart = new Set();
		hasCopied = false;
	}

	let colorHelper = entities.getColorHelper('Daemon');
	let title = daemons_createDaemon();
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	size="full"
	fixedHeight={true}
	onClose={handleOnClose}
	onOpen={handleOpen}
	{tabs}
	{activeTab}
	tabStyle="stepper"
	onTabChange={handleTabChange}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Daemon')} color={colorHelper.color} />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col overflow-hidden">
		{#if showAdvanced}
			<div class="flex-1 overflow-auto p-4 sm:p-6">
				<AdvancedStep
					{form}
					{formValues}
					{selectedOS}
					{linuxMethod}
					bind:dockerMode
					{hasDockerProxyCredential}
					onNavigateToCredentialWizard={handleNavigateToCredentialWizard}
				/>
			</div>
		{:else if activeTab === 'credentials'}
			{#if credentialSubStep === 'typeSelect'}
				<div class="flex min-h-0 flex-1 flex-col">
					<CredentialTypeSelectStep
						bind:selectedTypeIds={selectedCredentialTypeIds}
						bind:localCapabilityEnabled
					/>
				</div>
			{:else}
				<div class="flex min-h-0 flex-1 flex-col">
					<CredentialWizardStep
						bind:this={credentialWizardRef}
						daemonName={formValues.name as string}
						networkId={selectedNetworkId}
						daemonHasDockerSocket={null}
						bind:pendingCredentials
						onRemoveCredential={(credential) => {
							// If credential was already created on server, delete it
							if (credentialIds.includes(credential.id)) {
								deleteCredentialMutation.mutate(credential.id);
								credentialIds = credentialIds.filter((id) => id !== credential.id);
							}
						}}
					/>
				</div>
			{/if}
		{:else}
			<div class="flex-1 overflow-auto p-4 sm:p-6">
				{#key activeTab}
					{#if activeTab === 'configure'}
						<ConfigureStep
							{form}
							{formValues}
							{selectedNetworkId}
							onNetworkChange={(id) => (selectedNetworkId = id)}
							onNameInput={() => (nameManuallyEdited = true)}
							{keySet}
							{isFirstDaemon}
							onUseExistingKey={handleUseExistingKey}
							onReachabilityChange={(r) => {
								serverPollReachable = r;
								if (r === null) serverPollReachabilityResult = null;
							}}
							bind:reachabilityResult={serverPollReachabilityResult}
						/>
					{:else if activeTab === 'install'}
						<InstallStep
							{selectedOS}
							onOsSelect={(os) => (selectedOS = os)}
							{linuxMethod}
							onLinuxMethodChange={(method) => (linuxMethod = method)}
							{runCommand}
							{dockerCompose}
							{hasErrors}
							isFirstDaemon={startedAsFirstDaemon}
							{connectionStatus}
							onViewDiscovery={handleViewDiscovery}
							{hasEmailSupport}
							onAdvanced={() => (showAdvanced = true)}
							onCredentialWizard={openCredentialStep}
							daemonMode={String(formValues.mode ?? 'daemon_poll')}
							daemonName={String(formValues.name ?? 'scanopy-daemon')}
							logFilePath={String(formValues.logFile ?? '')}
							daemonUrl={constructDaemonUrl(
								String(formValues.daemonUrl ?? ''),
								Number(formValues.daemonPort) || 60073
							)}
							{provisionedDaemonId}
							onStartWaitingTimeout={startWaitingTimeout}
							onProgressComplete={handleProgressComplete}
							onReviewCommands={handleReviewCommandsFromTrouble}
							onEnableSelfSigned={handleEnableSelfSigned}
							onCopied={() => {
								hasCopied = true;
								pushSuccess(getCopiedToastMessage());
							}}
						/>
					{/if}
				{/key}
			</div>
		{/if}

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex flex-wrap items-center justify-end gap-3">
				{#if activeTab === 'credentials' && credentialSubStep === 'typeSelect'}
					<button type="button" class="btn-secondary" onclick={handleSkipCredentials}>
						{common_skip()}
					</button>
					<button
						type="button"
						class="btn-primary"
						disabled={selectedCredentialTypeIds.length === 0}
						onclick={handleContinueToWizard}
					>
						{common_continue()}
						<ArrowRight class="h-4 w-4" />
					</button>
				{:else if activeTab === 'credentials' && credentialSubStep === 'wizard'}
					<button
						type="button"
						class="btn-primary"
						disabled={bulkCreateCredentialsMutation.isPending}
						onclick={async () => {
							// Collect existing credential IDs and set target_ips on them
							const existingCreds = credentialWizardRef?.getExistingCredentials() ?? [];
							for (const ec of existingCreds) {
								const ips = ec.targetIps.map((s) => s.trim()).filter(Boolean);
								if (ips.length > 0) {
									const cred = credentialsQuery.data?.find((c) => c.id === ec.credentialId);
									if (cred) {
										await updateCredentialMutation.mutateAsync({
											...cred,
											target_ips: ips
										});
									}
								}
							}
							const existingIds = existingCreds.map((c) => c.credentialId);

							const unsaved = pendingCredentials.filter(
								(p) => !p.isExisting && !credentialIds.includes(p.credential.id)
							);
							if (unsaved.length > 0) {
								// Validate all fields before creating
								if (credentialWizardRef) {
									const isValid = await credentialWizardRef.validate();
									if (!isValid) return;
								}
								try {
									const prepared = credentialWizardRef?.getCredentialsForCreate() ?? [];
									const unsavedPrepared = prepared.filter(
										(p) => !credentialIds.includes(p.credential.id)
									);
									const toCreate = unsavedPrepared.map((p) => {
										const ips = p.targetIps.map((s) => s.trim()).filter(Boolean);
										return {
											...p.credential,
											target_ips: ips.length > 0 ? ips : undefined
										};
									});
									const created = await bulkCreateCredentialsMutation.mutateAsync(toCreate);
									credentialIds = [
										...new Set([...credentialIds, ...created.map((c) => c.id), ...existingIds])
									];
								} catch {
									return;
								}
							} else if (existingIds.length > 0) {
								// No new credentials but some existing ones to add
								credentialIds = [...new Set([...credentialIds, ...existingIds])];
							}
							trackEvent('daemon_wizard_step_completed', {
								step: 'credentials',
								skipped: false,
								types_selected: selectedCredentialTypeIds.length,
								credentials_attached: credentialIds.length
							});
							activeTab = 'install';
						}}
					>
						{#if unsavedCredentialCount > 0}
							{daemons_credentialWizardReturn({ count: unsavedCredentialCount })}
						{:else}
							{daemons_credentialWizardReturnToInstall()}
						{/if}
						<ArrowRight class="h-4 w-4" />
					</button>
				{:else if showAdvanced}
					<button type="button" class="btn-primary" onclick={() => (showAdvanced = false)}>
						<ArrowLeft class="h-4 w-4" />
						{daemons_installBackToInstall()}
					</button>
				{:else if activeTab === 'configure'}
					<button
						type="button"
						class="btn-primary btn-primary-lg"
						onclick={handleNext}
						disabled={isAutoGenerating || isTestingReachability}
					>
						{#if isTestingReachability}
							<Loader2 class="h-4 w-4 animate-spin" />
							Testing connection to {formValues.daemonUrl}:{formValues.daemonPort || 60073}...
						{:else if isAutoGenerating}
							<Loader2 class="h-4 w-4 animate-spin" />
						{:else}
							{common_next()}
							<ArrowRight class="h-4 w-4" />
						{/if}
					</button>
				{:else if activeTab === 'install'}
					{#if connectionStatus === 'connected'}
						<button type="button" class="btn-primary" onclick={handleOnClose}>
							{common_close()}
						</button>
					{:else if connectionStatus === 'waiting' || connectionStatus === 'trouble'}
						<button type="button" class="btn-secondary" onclick={handleReviewCommands}>
							{daemons_installReturnToCommands()}
						</button>
						<button type="button" class="btn-secondary" onclick={handleOnClose}>
							{common_close()}
						</button>
					{:else}
						{#if hasEmail && currentInstallCommand}
							<button
								type="button"
								class="btn-secondary w-full text-sm sm:w-auto"
								disabled={emailInstallMutation.isPending}
								onclick={() => {
									emailInstallMutation.mutate(
										{ installCommand: currentInstallCommand, os: currentOsLabel },
										{
											onSuccess: () => pushSuccess(daemons_installCommandEmailed())
										}
									);
								}}
							>
								<Mail class="h-4 w-4" />
								{daemons_emailInstallCommand()}
							</button>
						{/if}
						{#if hasCopied}
							<button type="button" class="btn-primary w-full sm:w-auto" onclick={handleInstalled}>
								<Check class="h-4 w-4" />
								{installCtaLabel}
							</button>
						{:else}
							<button
								type="button"
								class="btn-primary w-full sm:w-auto"
								onclick={handleCopyCommand}
							>
								<Copy class="h-4 w-4" />
								{daemons_installCopyCommand()}
							</button>
						{/if}
					{/if}
				{/if}
			</div>
		</div>
	</div>
</GenericModal>
