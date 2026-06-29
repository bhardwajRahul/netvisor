<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm, validateForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import EntityMetadataSection from '$lib/shared/components/forms/EntityMetadataSection.svelte';
	import DiscoveryDetailsForm from './DiscoveryDetailsForm.svelte';
	import DiscoveryTargetsForm from './DiscoveryTargetsForm.svelte';
	import DiscoveryDetectionForm from './DiscoveryDetectionForm.svelte';
	import DiscoveryScanSettingsForm from './DiscoveryScanSettingsForm.svelte';
	import DiscoveryScheduleForm from './DiscoveryScheduleForm.svelte';
	import type { Discovery } from '../../types/base';
	import DiscoveryHistoricalSummary from './DiscoveryHistoricalSummary.svelte';
	import { uuidv4Sentinel } from '$lib/shared/utils/formatting';
	import { createEmptyDiscoveryFormData, parseDayTimeCronSchedule } from '../../queries';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import type { Host } from '$lib/features/hosts/types/base';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans, credentialTypes } from '$lib/shared/stores/metadata';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import {
		Info,
		Crosshair,
		ScanSearch,
		Gauge,
		Calendar,
		ArrowRight,
		KeyRound,
		X
	} from 'lucide-svelte';
	import CredentialsStep, {
		type PendingCredential
	} from '$lib/features/credentials/components/CredentialsStep.svelte';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import {
		common_assigned,
		common_back,
		common_cancel,
		common_close,
		common_remove,
		common_credentials,
		common_delete,
		common_deleting,
		common_details,
		common_next,
		common_saving,
		common_schedule,
		common_detection,
		common_performance,
		common_targets,
		discovery_couldNotGetNetworkId,
		discovery_createDiscovery,
		discovery_createScheduled,
		discovery_credentialsDescription,
		discovery_daemonHostIntegrationsTitle,
		discovery_daemonHostNewThisScan,
		discovery_edit,
		discovery_failedToDelete,
		discovery_failedToSave,
		discovery_noDaemonSelected,
		discovery_editActiveInfo,
		discovery_updateDiscovery,
		discovery_viewRun
	} from '$lib/paraglide/messages';

	interface Props {
		discovery?: Discovery | null;
		hasActiveSession?: boolean;
		isOpen?: boolean;
		daemons?: Daemon[];
		hosts?: Host[];
		onCreate: (data: Discovery) => Promise<void> | void;
		onUpdate: (id: string, data: Discovery) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	}

	let {
		discovery = null,
		hasActiveSession = false,
		isOpen = false,
		daemons = [],
		hosts = [],
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
	}: Props = $props();

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	const subnetsQuery = useSubnetsQuery();
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let hasScheduledDiscovery = $derived.by(() => {
		if (!org?.plan?.type) return true;
		return billingPlans.getMetadata(org.plan.type).features.scheduled_discovery;
	});

	let loading = $state(false);
	let deleting = $state(false);
	let rawCronMode = $state(false);
	let activeTab = $state('details');
	let furthestReached = $state(0);
	let pendingCredentials = $state<PendingCredential[]>([]);
	let credentialsStep: ReturnType<typeof CredentialsStep> | undefined = $state();
	let credentialSubStep = $state<'typeSelect' | 'wizard'>('typeSelect');
	let credentialIds = $state<string[]>([]);
	const allCredentialsQuery = useCredentialsQuery();

	// Mutable form data that sub-components can update
	let formData = $state<Discovery>(createEmptyDiscoveryFormData(null));

	let isEditing = $derived(discovery !== null);
	let isHistoricalRun = $derived(discovery?.run_type.type === 'Historical');
	let readOnly = $derived(formData.run_type.type == 'Historical');

	let title = $derived(
		isEditing
			? isHistoricalRun
				? discovery_viewRun({ name: discovery?.name ?? '' })
				: discovery_edit({ name: discovery?.name ?? '' })
			: discovery_createScheduled()
	);

	let daemon = $derived(daemons.find((d) => d.id === formData.daemon_id) || null);
	let daemonHostId = $derived(
		(daemon ? hosts.find((h) => h.id === daemon.host_id)?.id : null) || null
	);

	// Credential types already targeting the daemon's own host, derived from this
	// discovery's DaemonHost-scope integration targets — generic across integrations
	// (Docker, Podman, …) with no per-integration capability flag. Feeds the shared
	// CredentialsStep's claimed-daemon-host blocking, so a socket and a proxy of the
	// same integration can't both target the daemon host (bidirectional).
	let daemonHostCredentialTypeIds = $derived.by(() => {
		const credMap = new Map((allCredentialsQuery.data ?? []).map((c) => [c.id, c]));
		const types = (discovery?.integration_targets ?? [])
			.filter(
				(t) => t.scope === 'DaemonHost' && !removedDaemonHostCredIds.includes(t.credential_id)
			)
			.map((t) => credMap.get(t.credential_id)?.credential_type.type)
			.filter((t): t is NonNullable<typeof t> => t != null);
		return types.filter((t, i) => types.indexOf(t) === i);
	});

	function ipIsLoopback(ip: string): boolean {
		const s = ip.trim();
		return s === '127.0.0.1' || s === '::1' || s.startsWith('127.');
	}

	// Read-only summary of credentials targeting the daemon's own host, so users see
	// what's already in play. Distinguishes already-assigned (persisted) from
	// newly-staged-this-session (a proxy pointed at the daemon host that isn't saved yet).
	let daemonHostIntegrationList = $derived.by(() => {
		const credMap = new Map((allCredentialsQuery.data ?? []).map((c) => [c.id, c]));
		const persistedDaemonHostIds = (discovery?.integration_targets ?? [])
			.filter(
				(t) =>
					t.scope === 'DaemonHost' ||
					(t.scope === 'Hosts' && t.ips.length > 0 && t.ips.every(ipIsLoopback))
			)
			.map((t) => t.credential_id);
		const items: {
			id: string;
			name: string;
			integration: string;
			isNew: boolean;
			removable: boolean;
		}[] = [];
		const push = (
			cred: { id: string; name: string; credential_type: { type: string } } | undefined,
			isNew: boolean,
			removable: boolean
		) => {
			if (!cred || items.some((i) => i.id === cred.id)) return;
			items.push({
				id: cred.id,
				name: cred.name,
				integration:
					credentialTypes.getMetadata(cred.credential_type.type)?.associated_service ??
					cred.credential_type.type,
				isNew,
				removable
			});
		};
		// Persisted daemon-host (socket) targets — already-assigned, removable here.
		for (const t of discovery?.integration_targets ?? []) {
			if (t.scope === 'DaemonHost' && !removedDaemonHostCredIds.includes(t.credential_id))
				push(credMap.get(t.credential_id), false, true);
		}
		// Pending credentials pointed at the daemon host (loopback) — new unless persisted;
		// removed via the wizard, not here.
		for (const p of pendingCredentials) {
			if ((p.targetIps ?? []).some(ipIsLoopback))
				push(p.credential, !persistedDaemonHostIds.includes(p.credential.id), false);
		}
		return items;
	});

	// Persisted daemon-host targets the user has staged for removal this session.
	// Applied on save (dropped from the submitted integration_targets) and excluded
	// from the claimed set so a freed integration can be re-targeted.
	let removedDaemonHostCredIds = $state<string[]>([]);
	function removeDaemonHostTarget(credId: string) {
		if (!removedDaemonHostCredIds.includes(credId))
			removedDaemonHostCredIds = [...removedDaemonHostCredIds, credId];
	}
	// User-chosen configurable integrations (the fixed socket card is shown checked
	// via the step's read-only handling, not via this selection).
	let selectedCredentialTypeIds = $state<string[]>([]);

	let hasTargetsTab = $derived(
		formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'
	);
	let hasDetectionTab = $derived(
		formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'
	);
	let hasPerformanceTab = $derived(
		formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'
	);
	let daemonSupportsUnified = $derived(
		!daemon || daemon.version_status?.supports_unified_discovery !== false
	);
	let hasCredentialsTab = $derived(formData.discovery_type.type === 'Unified');
	let hasScheduleTab = $derived(formData.run_type.type === 'Scheduled');

	let tabs: ModalTab[] = $derived(
		isHistoricalRun
			? []
			: [
					{ id: 'details', label: common_details(), icon: Info },
					...(hasTargetsTab
						? [
								{
									id: 'targets',
									label: common_targets(),
									icon: Crosshair,
									disabled: !isEditing && furthestReached < 1
								}
							]
						: []),
					...(hasCredentialsTab
						? [
								{
									id: 'credentials',
									label: common_credentials(),
									icon: KeyRound,
									disabled: !isEditing && furthestReached < 2
								}
							]
						: []),
					...(hasDetectionTab
						? [
								{
									id: 'detection',
									label: common_detection(),
									icon: ScanSearch,
									disabled:
										!isEditing && furthestReached < (hasCredentialsTab ? 3 : hasTargetsTab ? 2 : 1)
								}
							]
						: []),
					...(hasPerformanceTab
						? [
								{
									id: 'performance',
									label: common_performance(),
									icon: Gauge,
									disabled:
										!isEditing &&
										furthestReached <
											(hasCredentialsTab ? 4 : hasDetectionTab ? 3 : hasTargetsTab ? 2 : 1)
								}
							]
						: []),
					...(hasScheduleTab
						? [
								{
									id: 'schedule',
									label: common_schedule(),
									icon: Calendar,
									disabled:
										!isEditing &&
										furthestReached <
											(hasCredentialsTab
												? hasPerformanceTab
													? 5
													: hasDetectionTab
														? 4
														: 3
												: hasPerformanceTab
													? 4
													: hasDetectionTab
														? 3
														: hasTargetsTab
															? 2
															: 1)
								}
							]
						: [])
				]
	);

	// Auto-navigate away from tabs that no longer exist
	$effect(() => {
		if (activeTab === 'schedule' && !hasScheduleTab) {
			activeTab = 'details';
		}
		if (activeTab === 'targets' && !hasTargetsTab) {
			activeTab = 'details';
		}
		if (activeTab === 'detection' && !hasDetectionTab) {
			activeTab = hasTargetsTab ? 'targets' : 'details';
		}
		if (activeTab === 'performance' && !hasPerformanceTab) {
			activeTab = hasDetectionTab ? 'detection' : hasTargetsTab ? 'targets' : 'details';
		}
		if (activeTab === 'credentials' && !hasCredentialsTab) {
			activeTab = 'details';
		}
	});

	function getFlow() {
		return [
			'details',
			...(hasTargetsTab ? ['targets'] : []),
			...(hasCredentialsTab ? ['credentials'] : []),
			...(hasDetectionTab ? ['detection'] : []),
			...(hasPerformanceTab ? ['performance'] : []),
			...(hasScheduleTab ? ['schedule'] : [])
		];
	}

	function nextTab() {
		const flow = getFlow();
		const idx = flow.indexOf(activeTab);
		if (idx >= 0 && idx < flow.length - 1) {
			activeTab = flow[idx + 1];
		}
	}

	function previousTab() {
		// Step back within the credentials sub-flow (wizard → Integrations grid)
		// before leaving the tab.
		if (activeTab === 'credentials' && credentialSubStep === 'wizard') {
			credentialsStep?.backToTypeSelect();
			return;
		}
		const flow = getFlow();
		const idx = flow.indexOf(activeTab);
		if (idx > 0) {
			activeTab = flow[idx - 1];
		}
	}

	async function handleNext() {
		if (activeTab === 'details') {
			const isValid = await validateForm(form);
			if (isValid) {
				if (furthestReached < 1) furthestReached = 1;
				nextTab();
			}
		} else if (activeTab === 'targets') {
			if (furthestReached < 2) furthestReached = 2;
			nextTab();
		} else if (activeTab === 'credentials') {
			// Credentials has a sub-flow: the Integrations grid → the wizard. Advance
			// within it before moving on to the next tab.
			if (credentialSubStep === 'typeSelect') {
				await credentialsStep?.continueToWizard();
				return;
			}
			if (furthestReached < 3) furthestReached = 3;
			nextTab();
		} else if (activeTab === 'detection') {
			const level = hasCredentialsTab ? 4 : 3;
			if (furthestReached < level) furthestReached = level;
			nextTab();
		} else if (activeTab === 'performance') {
			const level = hasCredentialsTab ? 5 : 4;
			if (furthestReached < level) furthestReached = level;
			nextTab();
		}
	}

	let isLastTab = $derived.by(() => {
		const flow = getFlow();
		return activeTab === flow[flow.length - 1];
	});

	let isFirstTab = $derived(activeTab === 'details');

	function getDefaultFormData(): Discovery {
		const defaultDaemon = daemons.length > 0 ? daemons[0] : null;
		if (discovery) {
			return { ...discovery };
		}
		const empty = createEmptyDiscoveryFormData(defaultDaemon);
		if (defaultDaemon) {
			empty.daemon_id = defaultDaemon.id;
			empty.network_id = defaultDaemon.network_id;
		}
		// Default to AdHoc for plans without scheduled discovery (e.g. Free)
		if (!hasScheduledDiscovery) {
			empty.run_type = { type: 'AdHoc', last_run: null };
		}
		return empty;
	}

	// TanStack Form for validation (only fields that need validation)
	// NOTE: defaultValues must NOT read from $state to avoid reactivity loops
	const form = createForm(() => ({
		defaultValues: {
			name: '',
			run_type_type: (hasScheduledDiscovery ? 'Scheduled' : 'AdHoc') as 'AdHoc' | 'Scheduled',
			discovery_type_type: 'Unified' as 'Network' | 'Docker' | 'SelfReport' | 'Unified',
			host_naming_fallback: 'BestService' as 'BestService' | 'Ip',
			schedule_days_of_week: '0',
			schedule_time: '00:00',
			schedule_timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
			schedule_cron: '0 0 0 * * 0'
		},
		onSubmit: async ({ value }) => {
			// Update formData with form values
			formData.name = value.name.trim();

			if (daemon) {
				loading = true;
				// Persist credentials via the shared step (validate + create/update).
				const ids = await credentialsStep?.collectCredentialIds();
				if (ids === null) {
					loading = false;
					return; // validation failed — stay on the form
				}
				try {
					// Per-credential target IPs come from the wizard and are delivered as
					// per-daemon integration targets (Credentialed) on the Discovery — replacing
					// the old one-shot pending_credential_ids + credential.target_ips. The discovery
					// modal only handles persisted credentials (sockets are daemon-level, fixed).
					// Manages only network/host credentialed targets: empty IPs → Network scope,
					// explicit IPs → Hosts scope. Daemon-host (socket) targets are daemon-level —
					// pass them through untouched so editing a discovery doesn't drop them.
					const persisted = new Set(ids ?? []);
					// Daemon-host (socket) targets pass through untouched, EXCEPT any the
					// user removed this session (the discovery modal is authoritative now).
					const daemonHostTargets = (discovery?.integration_targets ?? []).filter(
						(t) => t.scope === 'DaemonHost' && !removedDaemonHostCredIds.includes(t.credential_id)
					);
					const rebuilt = pendingCredentials
						.filter((p) => persisted.has(p.credential.id))
						.map((p) => {
							const ips = p.targetIps.map((s) => s.trim()).filter(Boolean);
							return ips.length > 0
								? { scope: 'Hosts' as const, credential_id: p.credential.id, ips }
								: { scope: 'Network' as const, credential_id: p.credential.id };
						});
					formData.integration_targets = [...daemonHostTargets, ...rebuilt];
					if (isEditing && discovery) {
						await onUpdate(discovery.id, formData);
					} else {
						await onCreate(formData);
					}
					onClose();
				} catch (error) {
					pushError(error instanceof Error ? error.message : discovery_failedToSave());
				} finally {
					loading = false;
				}
			} else {
				pushError(discovery_couldNotGetNetworkId());
			}
		}
	}));

	function handleOpen() {
		activeTab = 'details';
		furthestReached = discovery ? Infinity : 0;
		formData = getDefaultFormData();
		pendingCredentials = [];
		credentialIds = [];
		removedDaemonHostCredIds = [];
		if (discovery?.integration_targets?.length && allCredentialsQuery.data) {
			const credMap = new Map(allCredentialsQuery.data.map((c) => [c.id, c]));
			// Reconstruct network/host credentialed targets as wizard entries. DaemonHost
			// (socket) targets are daemon-level and shown via fixed capabilities, not edited here.
			pendingCredentials = discovery.integration_targets.flatMap((t) => {
				if (t.scope === 'DaemonHost') return [];
				const c = credMap.get(t.credential_id);
				if (!c) return [];
				const ips = t.scope === 'Hosts' ? t.ips : [];
				return [
					{
						credential: c,
						targetIps: ips.length ? ips : [''],
						fieldValues: {},
						isExisting: true
					}
				];
			});
		}
		// Show the Integrations grid only as a first-run aid (no credentials exist
		// yet); otherwise — existing assignments on this session, or the org already
		// has credentials — open straight on the wizard. Mirrors the daemon modal.
		credentialSubStep =
			pendingCredentials.length > 0 || (allCredentialsQuery.data?.length ?? 0) > 0
				? 'wizard'
				: 'typeSelect';

		// Parse schedule fields from cron
		let scheduleDaysOfWeek = '0';
		let scheduleTime = '00:00';
		let scheduleCron = '0 0 0 * * 0';
		let scheduleTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;

		if (formData.run_type.type === 'Scheduled') {
			scheduleCron = formData.run_type.cron_schedule;
			scheduleTimezone = formData.run_type.timezone || scheduleTimezone;

			// Sync computed timezone back to formData so submit sends the correct value
			// even if the user never touches the timezone dropdown
			formData.run_type = { ...formData.run_type, timezone: scheduleTimezone };

			const parsed = parseDayTimeCronSchedule(formData.run_type.cron_schedule);
			if (parsed) {
				scheduleDaysOfWeek = parsed.daysOfWeek.join(',');
				scheduleTime = `${String(parsed.hour).padStart(2, '0')}:${String(parsed.minute).padStart(2, '0')}`;
				rawCronMode = false;
			} else {
				// Unmappable cron — open in raw cron mode
				rawCronMode = true;
			}
		}

		// Compute host naming fallback
		const hostNamingFallback =
			formData.discovery_type.type === 'Network' ||
			formData.discovery_type.type === 'Docker' ||
			formData.discovery_type.type === 'Unified'
				? formData.discovery_type.host_naming_fallback
				: 'BestService';

		form.reset({
			name: formData.name,
			run_type_type: formData.run_type.type === 'Historical' ? 'AdHoc' : formData.run_type.type,
			discovery_type_type: formData.discovery_type.type,
			host_naming_fallback: hostNamingFallback,
			schedule_days_of_week: scheduleDaysOfWeek,
			schedule_time: scheduleTime,
			schedule_timezone: scheduleTimezone,
			schedule_cron: scheduleCron
		});
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && discovery) {
			deleting = true;
			try {
				await onDelete(discovery.id);
				onClose();
			} catch (error) {
				pushError(error instanceof Error ? error.message : discovery_failedToDelete());
			} finally {
				deleting = false;
			}
		}
	}

	// Set default daemon when available and formData has sentinel
	$effect(() => {
		if (formData.daemon_id === uuidv4Sentinel && daemons.length > 0) {
			formData.daemon_id = daemons[0].id;
			formData.network_id = daemons[0].network_id;
		}
	});

	let saveLabel = $derived(isEditing ? discovery_updateDiscovery() : discovery_createDiscovery());
	let showSave = $derived(!isHistoricalRun);

	let colorHelper = entities.getColorHelper('Discovery');
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={discovery?.id}
	{onClose}
	onOpen={handleOpen}
	size="full"
	fixedHeight={true}
	showCloseButton={true}
	{tabs}
	bind:activeTab
	tabStyle={isEditing ? 'tabs' : 'stepper'}
	onTabChange={(id) => (activeTab = id)}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Discovery')} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			if (showSave) handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div
			class="min-h-0 flex-1"
			class:overflow-y-auto={activeTab !== 'credentials'}
			class:flex={activeTab === 'credentials'}
			class:flex-col={activeTab === 'credentials'}
		>
			{#if isHistoricalRun && discovery?.run_type.type === 'Historical'}
				<div class="space-y-8 p-6">
					<DiscoveryHistoricalSummary payload={discovery.run_type.results} />
				</div>
			{:else if activeTab === 'details'}
				<div class="space-y-8 p-6">
					{#if hasActiveSession && isEditing}
						<InlineInfo
							title=""
							body={discovery_editActiveInfo()}
							dismissableKey="discovery-edit-active-session"
						/>
					{/if}
					<DiscoveryDetailsForm
						{form}
						{daemons}
						{hosts}
						subnets={subnetsData}
						bind:formData
						{readOnly}
						{hasScheduledDiscovery}
						{daemon}
					/>
				</div>
			{:else if activeTab === 'targets'}
				<div class="space-y-8 p-6">
					{#if daemon}
						<DiscoveryTargetsForm bind:formData {daemonHostId} {daemon} />
					{:else}
						<InlineWarning body={discovery_noDaemonSelected()} />
					{/if}
				</div>
			{:else if activeTab === 'detection'}
				<div class="space-y-8 p-6">
					<DiscoveryDetectionForm bind:formData {readOnly} {isEditing} />
				</div>
			{:else if activeTab === 'performance'}
				<div class="space-y-8 p-6">
					<DiscoveryScanSettingsForm bind:formData {daemon} {readOnly} />
				</div>
			{:else if activeTab === 'schedule'}
				<div class="space-y-8 p-6">
					<DiscoveryScheduleForm
						{form}
						bind:formData
						{readOnly}
						bind:rawCronMode
						schedulePaused={!hasScheduledDiscovery}
					/>
				</div>
			{/if}
			{#if hasCredentialsTab}
				<div class="flex min-h-0 flex-1 flex-col" class:hidden={activeTab !== 'credentials'}>
					{#if daemonHostIntegrationList.length > 0}
						<div class="card-static mx-4 mt-4 space-y-2 rounded-lg border p-3 sm:mx-6">
							<p class="text-secondary text-xs font-medium uppercase tracking-wide">
								{discovery_daemonHostIntegrationsTitle()}
							</p>
							<ul class="space-y-1">
								{#each daemonHostIntegrationList as item (item.id)}
									<li class="flex items-center gap-2 text-sm">
										<span class="font-medium">{item.name}</span>
										<span class="text-secondary">{item.integration}</span>
										<Tag
											color={item.isNew ? 'Blue' : 'Gray'}
											label={item.isNew ? discovery_daemonHostNewThisScan() : common_assigned()}
										/>
										{#if item.removable && !readOnly}
											<button
												type="button"
												class="text-secondary hover:text-primary ml-auto"
												title={common_remove()}
												aria-label={common_remove()}
												onclick={() => removeDaemonHostTarget(item.id)}
											>
												<X class="h-4 w-4" />
											</button>
										{/if}
									</li>
								{/each}
							</ul>
						</div>
					{/if}
					<CredentialsStep
						bind:this={credentialsStep}
						networkId={formData.network_id}
						description={discovery_credentialsDescription()}
						bind:pendingCredentials
						bind:credentialIds
						bind:subStep={credentialSubStep}
						bind:selectedTypeIds={selectedCredentialTypeIds}
						localAutoMode="fixed"
						fixedCapabilityTypeIds={daemonHostCredentialTypeIds}
					/>
				</div>
			{/if}
		</div>

		{#if isEditing}
			<EntityMetadataSection entities={[discovery]} />
		{/if}

		<div class="modal-footer">
			<div class="flex items-center justify-between">
				<div>
					{#if isEditing && !isHistoricalRun && onDelete}
						<button
							type="button"
							disabled={deleting || loading}
							onclick={handleDelete}
							class="btn-danger"
						>
							{deleting ? common_deleting() : common_delete()}
						</button>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					{#if isEditing || isHistoricalRun}
						<button
							type="button"
							disabled={loading || deleting}
							onclick={onClose}
							class="btn-secondary"
						>
							{isHistoricalRun ? common_close() : common_cancel()}
						</button>
						{#if showSave}
							<button type="submit" disabled={loading || deleting} class="btn-primary">
								{loading ? common_saving() : saveLabel}
							</button>
						{/if}
					{:else}
						{#if !isFirstTab}
							<button type="button" class="btn-secondary" onclick={previousTab}>
								{common_back()}
							</button>
						{:else}
							<button type="button" onclick={onClose} class="btn-secondary">
								{common_cancel()}
							</button>
						{/if}
						{#if isLastTab}
							<button
								type="submit"
								disabled={loading || (!isEditing && !daemonSupportsUnified)}
								class="btn-primary"
							>
								{loading ? common_saving() : saveLabel}
							</button>
						{:else}
							<button
								type="button"
								class="btn-primary btn-primary-lg"
								onclick={handleNext}
								disabled={!isEditing && !daemonSupportsUnified}
							>
								{common_next()}
								<ArrowRight class="h-4 w-4" />
							</button>
						{/if}
					{/if}
				</div>
			</div>
		</div>
	</form>
</GenericModal>
