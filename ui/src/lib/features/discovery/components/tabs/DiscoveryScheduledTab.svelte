<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { Discovery } from '../../types/base';
	import { discoveryFields } from '../../queries';
	import DiscoveryEditModal from '../DiscoveryModal/DiscoveryEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import DiscoveryRunCard from '../cards/DiscoveryScheduledCard.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useDiscoveriesQuery,
		useCreateDiscoveryMutation,
		useUpdateDiscoveryMutation,
		useDeleteDiscoveryMutation,
		useBulkDeleteDiscoveriesMutation,
		useInitiateDiscoveryMutation,
		useActiveSessionsQuery,
		useCancelDiscoveryMutation
	} from '../../queries';
	import type { DiscoveryUpdatePayload } from '../../types/api';
	import { SvelteMap } from 'svelte/reactivity';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import type { components } from '$lib/api/schema';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import {
		common_confirmDeleteName,
		common_create,
		common_created,
		common_tags,
		daemons_installPromptDiscoveries,
		discovery_confirmDeleteScheduled,
		common_scans,
		discovery_legacyDaemonsWarning,
		discovery_noScheduledSessions,
		discovery_runType
	} from '$lib/paraglide/messages';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	let { isReadOnly = false, isActive = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	const tagsQuery = useTagsQuery();
	// Gated on `isActive` — see the matching comment in DiscoveryHistoryTab. Both
	// subscribers must be gated for the query to actually go inactive.
	const discoveriesQuery = useDiscoveriesQuery(() => isActive);
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();

	// Active sessions
	const sessionsQuery = useActiveSessionsQuery();
	const cancelDiscoveryMutation = useCancelDiscoveryMutation();

	// Mutations
	const createDiscoveryMutation = useCreateDiscoveryMutation();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();
	const deleteDiscoveryMutation = useDeleteDiscoveryMutation();
	const bulkDeleteDiscoveriesMutation = useBulkDeleteDiscoveriesMutation();
	const initiateDiscoveryMutation = useInitiateDiscoveryMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let discoveriesData = $derived(discoveriesQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let sessionsList = $derived(sessionsQuery.data ?? []);

	// Only the hosts the daemons run on. This was an unpaginated org-wide hosts
	// query (~1.9MB on a few hundred hosts), issued so the edit modal's daemon
	// picker could label each daemon with its host name — and because TanStack
	// dedupes by key, it was shared with every other consumer, so it loaded on
	// pages that never opened the modal. Scoped to the ids in hand.
	let daemonHostIds = $derived([
		...new Set(daemonsData.map((d) => d.host_id).filter((id): id is string => !!id))
	]);
	const hostsQuery = useHostsByIds(() => daemonHostIds);
	let hostsData = $derived(hostsQuery.data ?? []);

	// Host names are decoration inside the modal, so the list must not block on
	// them — and with no daemons the by-ids query is disabled, which in TanStack
	// means it stays `isPending` forever.
	let isLoading = $derived(
		discoveriesQuery.isPending || daemonsQuery.isPending || sessionsQuery.isPending
	);

	// Build lookup: discovery_id -> session (server always enriches discovery_id)
	let sessionByDiscoveryId = $derived.by(() => {
		const map = new SvelteMap<string, DiscoveryUpdatePayload>();
		for (const session of sessionsList) {
			if (session.discovery_id) {
				map.set(session.discovery_id, session);
			}
		}
		return map;
	});

	function getActiveSession(discovery: Discovery): DiscoveryUpdatePayload | null {
		return sessionByDiscoveryId.get(discovery.id) ?? null;
	}
	let hasLegacyDaemons = $derived(
		daemonsData.some((d) => d.version_status?.supports_unified_discovery === false)
	);

	let showDiscoveryModal = $state(false);
	let editingDiscovery: Discovery | null = $state(null);

	// Deep-link: open discovery editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'discovery-editor',
			discoveriesData,
			showDiscoveryModal,
			editingDiscovery?.id
		);
		if (result !== undefined) {
			editingDiscovery = result;
			showDiscoveryModal = true;
		}
	});

	function handleCreateDiscovery() {
		editingDiscovery = null;
		showDiscoveryModal = true;
	}

	function handleEditDiscovery(discovery: Discovery) {
		editingDiscovery = discovery;
		showDiscoveryModal = true;
	}

	function handleDeleteDiscovery(discovery: Discovery) {
		if (confirm(common_confirmDeleteName({ name: discovery.name }))) {
			deleteDiscoveryMutation.mutate(discovery.id);
		}
	}

	function handleDiscoveryRun(discovery: Discovery) {
		initiateDiscoveryMutation.mutate(discovery.id);
	}

	function handleCancelDiscovery(sessionId: string) {
		cancelDiscoveryMutation.mutate(sessionId);
	}

	function handleToggleEnabled(discovery: Discovery) {
		if (discovery.run_type.type !== 'Scheduled') return;
		updateDiscoveryMutation.mutate({
			...discovery,
			run_type: {
				...discovery.run_type,
				enabled: !discovery.run_type.enabled
			}
		});
	}

	async function handleDiscoveryCreate(data: Discovery) {
		await createDiscoveryMutation.mutateAsync(data);
		showDiscoveryModal = false;
		editingDiscovery = null;
	}

	async function handleDiscoveryUpdate(id: string, data: Discovery) {
		await updateDiscoveryMutation.mutateAsync(data);
		showDiscoveryModal = false;
		editingDiscovery = null;
	}

	function handleCloseEditor() {
		showDiscoveryModal = false;
		editingDiscovery = null;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(discovery_confirmDeleteScheduled({ count: ids.length }))) {
			await bulkDeleteDiscoveriesMutation.mutateAsync(ids);
		}
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Discovery', {});
	}

	let fields: FieldConfig<Discovery>[] = $derived([
		...discoveryFields(daemonsData, networksData),
		{
			key: 'run_type',
			label: discovery_runType(),
			type: 'string',
			searchable: true,
			filterable: true,
			groupable: true,
			getValue: (item) => item.run_type.type
		},
		{
			key: 'tags',
			label: common_tags(),
			type: 'array',
			searchable: true,
			filterable: true,
			getValue: (entity) => {
				// Return tag names for search/filter display
				return entity.tags
					.map((id) => tagsData.find((t) => t.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		},
		{
			key: 'created_at',
			label: common_created(),
			type: 'date',
			sortable: true
		}
	]);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_scans()}>
		<svelte:fragment slot="actions">
			{#if hasDaemon(onboarding) && !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateDiscovery}
					><Plus class="h-5 w-5" />{common_create()}</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if hasLegacyDaemons}
		<InlineWarning
			title=""
			body={discovery_legacyDaemonsWarning()}
			dismissableKey="unified-discovery-migration"
		/>
	{/if}

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptDiscoveries()} />
	{:else if isLoading}
		<Loading />
	{:else if discoveriesData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={discovery_noScheduledSessions()}
			subtitle=""
			onClick={isReadOnly ? undefined : handleCreateDiscovery}
			cta={isReadOnly ? undefined : common_create()}
		/>
	{:else}
		<DataControls
			items={discoveriesData.filter(
				(d) => d.run_type.type == 'AdHoc' || d.run_type.type == 'Scheduled'
			)}
			{fields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			storageKey="scanopy-discovery-scans-table-state"
			getItemId={(item) => item.id}
			entityType={isReadOnly ? undefined : 'Discovery'}
			getItemTags={(item) => item.tags}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Discovery,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<DiscoveryRunCard
					discovery={item}
					hosts={hostsData}
					activeSession={getActiveSession(item)}
					selected={isSelected}
					{onSelectionChange}
					onDelete={isReadOnly ? undefined : handleDeleteDiscovery}
					onEdit={isReadOnly ? undefined : handleEditDiscovery}
					onRun={isReadOnly ? undefined : handleDiscoveryRun}
					onCancel={isReadOnly ? undefined : handleCancelDiscovery}
					onToggleEnabled={isReadOnly ? undefined : handleToggleEnabled}
					{viewMode}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<DiscoveryEditModal
	name="discovery-editor"
	isOpen={showDiscoveryModal}
	daemons={daemonsData}
	hosts={hostsData}
	discovery={editingDiscovery}
	hasActiveSession={editingDiscovery ? !!getActiveSession(editingDiscovery) : false}
	onCreate={handleDiscoveryCreate}
	onUpdate={handleDiscoveryUpdate}
	onClose={handleCloseEditor}
	onDelete={editingDiscovery
		? () => {
				handleDeleteDiscovery(editingDiscovery!);
				handleCloseEditor();
			}
		: null}
/>
