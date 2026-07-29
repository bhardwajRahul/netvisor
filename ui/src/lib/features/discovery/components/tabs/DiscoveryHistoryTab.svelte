<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { Discovery } from '../../types/base';
	import { discoveryFields } from '../../queries';
	import DiscoveryEditModal from '../DiscoveryModal/DiscoveryEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import DiscoveryHistoryCard from '../cards/DiscoveryHistoryCard.svelte';
	import { formatDuration, formatTimestamp } from '$lib/shared/utils/formatting';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import {
		useDiscoveriesQuery,
		useCreateDiscoveryMutation,
		useUpdateDiscoveryMutation,
		useBulkDeleteDiscoveriesMutation
	} from '../../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import type { components } from '$lib/api/schema';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, openModal, closeModal } from '$lib/shared/stores/modal-registry';
	import {
		common_created,
		common_duration,
		common_unknown,
		daemons_installPromptDiscoveries,
		discovery_confirmDeleteHistorical,
		discovery_finishedAt,
		discovery_historyTitle,
		discovery_noHistorySessions,
		discovery_noHistorySessionsSubtitle,
		discovery_startedAt
	} from '$lib/paraglide/messages';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];

	let { isReadOnly = false, isActive = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Queries
	// Gated on `isActive`. Every tab is mounted at once (inactive ones hidden with
	// CSS), so an ungated list query fetches on app boot and refetches whenever the
	// discovery SSE stream invalidates it — for a tab nobody is looking at. This
	// only works because *every* subscriber to `queryKeys.discovery.all` among the
	// always-mounted components is gated the same way (the scheduled tab); one
	// ungated subscriber keeps the query active for all of them.
	const discoveriesQuery = useDiscoveriesQuery(() => isActive);
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();

	// Mutations
	const createDiscoveryMutation = useCreateDiscoveryMutation();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();
	const bulkDeleteDiscoveriesMutation = useBulkDeleteDiscoveriesMutation();

	// Derived data
	let discoveriesData = $derived(discoveriesQuery.data ?? []);
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);

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
	let isLoading = $derived(discoveriesQuery.isPending || daemonsQuery.isPending);
	let historicalDiscoveries = $derived(
		discoveriesData.filter((d) => d.run_type.type === 'Historical')
	);

	let showDiscoveryModal = $state(false);
	let editingDiscovery: Discovery | null = $state(null);

	// Deep-link: open detail modal from URL
	$effect(() => {
		if ($modalState.name === 'discovery-history-detail' && !showDiscoveryModal) {
			if ($modalState.id) {
				const disc = discoveriesData.find((d) => d.id === $modalState.id);
				if (disc) {
					editingDiscovery = disc;
					showDiscoveryModal = true;
				}
			}
		}
	});

	function handleEditDiscovery(discovery: Discovery) {
		editingDiscovery = discovery;
		showDiscoveryModal = true;
		openModal('discovery-history-detail', { id: discovery.id });
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
		closeModal();
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(discovery_confirmDeleteHistorical({ count: ids.length }))) {
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
			key: 'started_at',
			label: discovery_startedAt(),
			type: 'string',
			searchable: true,
			getValue: (item) => {
				const results = item.run_type.type == 'Historical' ? item.run_type.results : null;
				return results && results.started_at
					? formatTimestamp(results.started_at)
					: common_unknown();
			}
		},
		{
			key: 'finished_at',
			label: discovery_finishedAt(),
			type: 'string',
			searchable: true,
			getValue: (item) => {
				const results = item.run_type.type == 'Historical' ? item.run_type.results : null;
				return results && results.finished_at
					? formatTimestamp(results.finished_at)
					: common_unknown();
			}
		},
		{
			key: 'duration',
			label: common_duration(),
			type: 'string',
			searchable: true,
			getValue: (item) => {
				const results = item.run_type.type == 'Historical' ? item.run_type.results : null;
				if (results && results.finished_at && results.started_at) {
					return formatDuration(results.started_at, results.finished_at);
				}
				return common_unknown();
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
	<TabHeader title={discovery_historyTitle()} />

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptDiscoveries()} />
	{:else if isLoading}
		<Loading />
	{:else if historicalDiscoveries.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={discovery_noHistorySessions()}
			subtitle={discovery_noHistorySessionsSubtitle()}
		/>
	{:else}
		<DataControls
			items={historicalDiscoveries}
			{fields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			storageKey="scanopy-discovery-historical-table-state"
			getItemId={(item) => item.id}
			onCsvExport={handleCsvExport}
		>
			{#snippet children(
				item: Discovery,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<DiscoveryHistoryCard
					discovery={item}
					hosts={hostsData}
					onView={handleEditDiscovery}
					{viewMode}
					selected={isSelected}
					{onSelectionChange}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<DiscoveryEditModal
	name="discovery-history-detail"
	isOpen={showDiscoveryModal}
	hosts={hostsData}
	daemons={daemonsData}
	discovery={editingDiscovery}
	onCreate={handleDiscoveryCreate}
	onUpdate={handleDiscoveryUpdate}
	onClose={handleCloseEditor}
/>
