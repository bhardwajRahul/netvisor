<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm, validateForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { entities, credentialTypes } from '$lib/shared/stores/metadata';
	import { isDaemonHostOnly } from '$lib/features/credentials/types/base';
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
	import { billingPlans } from '$lib/shared/stores/metadata';
	import {
		Info,
		Crosshair,
		ScanSearch,
		Gauge,
		Calendar,
		ArrowRight,
		KeyRound
	} from 'lucide-svelte';
	import CredentialsStep, {
		type PendingCredential
	} from '$lib/features/credentials/components/CredentialsStep.svelte';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import {
		common_back,
		common_cancel,
		common_close,
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

	function ipIsLoopback(ip: string): boolean {
		const s = ip.trim();
		return s === '127.0.0.1' || s === '::1' || s.startsWith('127.');
	}

	// Credentials targeting the daemon's own host. Daemon-host targeting lives in the
	// host_credentials junction (managed via the host/credential modals), so union:
	// (1) DaemonHost-scope integration targets, (2) loopback Hosts-scope targets, and
	// (3) credentials assigned to the daemon host via the junction (host_assignments).
	function computeDaemonHostCredentials(dHostId: string | null) {
		if (!dHostId) return [];
		const credMap = new Map((allCredentialsQuery.data ?? []).map((c) => [c.id, c]));
		const fromTargets = (discovery?.integration_targets ?? [])
			.filter(
				(t) =>
					t.scope === 'DaemonHost' ||
					(t.scope === 'Hosts' && t.ips.length > 0 && t.ips.every(ipIsLoopback))
			)
			.map((t) => credMap.get(t.credential_id));
		const fromAssignments = (allCredentialsQuery.data ?? []).filter((c) =>
			(c.host_assignments ?? []).some((a) => a.host_id === dHostId)
		);
		const all = [...fromTargets, ...fromAssignments].filter((c): c is NonNullable<typeof c> => !!c);
		return all.filter((c, i) => all.findIndex((x) => x.id === c.id) === i);
	}
	let daemonHostCredentials = $derived(computeDaemonHostCredentials(daemonHostId));

	// Claimed integrations (credential types) on the daemon host — feeds the shared
	// CredentialsStep's bidirectional socket↔proxy blocking. Generic across
	// integrations (Docker, Podman, …); no per-integration capability flag.
	let daemonHostCredentialTypeIds = $derived.by(() => {
		const types = daemonHostCredentials.map((c) => c.credential_type.type);
		return types.filter((t, i) => types.indexOf(t) === i);
	});

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
					// Per-credential targeting comes from the wizard and is delivered as per-daemon
					// integration targets on the Discovery. Parity with the daemon-create modal: a
					// daemon-host-only credential (e.g. a Docker/Podman socket) OR a loopback-only IP
					// target → DaemonHost scope; explicit non-loopback IPs → Hosts; otherwise Network.
					// The backend (apply_integration_targets) merges DaemonHost targets into the
					// host_credentials junction and keeps Network/Hosts on the Discovery — so adding a
					// socket here works exactly like the create modal / host modal.
					const persisted = new Set(ids ?? []);
					// Managed (already-junction-assigned) daemon-host cards are read-only here.
					formData.integration_targets = pendingCredentials
						.filter((p) => !p.isManaged && persisted.has(p.credential.id))
						.map((p) => {
							const ips = p.targetIps.map((s) => s.trim()).filter(Boolean);
							const isDaemonHost =
								isDaemonHostOnly(
									credentialTypes.getMetadata(p.credential.credential_type.type)?.targets
								) ||
								(ips.length > 0 && ips.every(ipIsLoopback));
							if (isDaemonHost) {
								return { scope: 'DaemonHost' as const, credential_id: p.credential.id };
							}
							return ips.length > 0
								? { scope: 'Hosts' as const, credential_id: p.credential.id, ips }
								: { scope: 'Network' as const, credential_id: p.credential.id };
						});
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
		if (allCredentialsQuery.data) {
			const credMap = new Map(allCredentialsQuery.data.map((c) => [c.id, c]));
			// Editable: network/host credentialed targets from integration_targets. Daemon-host
			// targeting (DaemonHost scope or loopback Hosts) is junction-managed — excluded here.
			const editable: PendingCredential[] = (discovery?.integration_targets ?? []).flatMap((t) => {
				if (t.scope === 'DaemonHost') return [];
				if (t.scope === 'Hosts' && t.ips.length > 0 && t.ips.every(ipIsLoopback)) return [];
				const c = credMap.get(t.credential_id);
				if (!c) return [];
				const ips = t.scope === 'Hosts' ? t.ips : [];
				return [
					{ credential: c, targetIps: ips.length ? ips : [''], fieldValues: {}, isExisting: true }
				];
			});
			// Read-only managed cards: credentials assigned to the daemon host (via the
			// host/credential modals). Resolve the daemon host inline (formData not yet settled).
			const dForDiscovery = daemons.find((d) => d.id === discovery?.daemon_id) ?? null;
			const dHostId = dForDiscovery
				? (hosts.find((h) => h.id === dForDiscovery.host_id)?.id ?? null)
				: null;
			const managed: PendingCredential[] = computeDaemonHostCredentials(dHostId)
				.filter((c) => !editable.some((p) => p.credential.id === c.id))
				.map((c) => ({
					credential: c,
					targetIps: [],
					fieldValues: {},
					isExisting: true,
					isManaged: true
				}));
			pendingCredentials = [...editable, ...managed];
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
						daemonVersion={daemon?.version ?? null}
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
