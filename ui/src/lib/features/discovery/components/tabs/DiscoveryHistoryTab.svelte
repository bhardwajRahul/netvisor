<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { Discovery } from '../../types/base';
	import DiscoveryEditModal from '../DiscoveryModal/DiscoveryEditModal.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import DiscoveryHistoryCard from '../cards/DiscoveryHistoryCard.svelte';
	import { formatDuration, formatTimestamp } from '$lib/shared/utils/formatting';
	import { defineFields } from '$lib/shared/components/data/types';
	import {
		useDiscoveryHistoryQuery,
		useCreateDiscoveryMutation,
		useUpdateDiscoveryMutation,
		useBulkDeleteDiscoveriesMutation,
		type DiscoveryHistoryQueryParams
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
	import { Info } from 'lucide-svelte';
	import { daemonItems } from '$lib/features/daemons/columns';
	import { networkItems } from '$lib/features/networks/columns';
	import { toColor, type Color } from '$lib/shared/utils/styling';
	import type { CardAction } from '$lib/shared/components/data/types';
	import {
		common_created,
		common_daemon,
		common_details,
		common_duration,
		common_cancelled,
		common_failed,
		common_name,
		common_status,
		common_warnings,
		common_network,
		common_type,
		common_unknown,
		common_unknownEntity,
		common_unknownNetwork,
		common_updated,
		daemons_installPromptDiscoveries,
		discovery_confirmDeleteHistorical,
		discovery_finishedAt,
		discovery_historyTitle,
		discovery_noHistorySessions,
		discovery_noHistorySessionsSubtitle,
		discovery_startedAt
	} from '$lib/paraglide/messages';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type DiscoveryOrderField = components['schemas']['DiscoveryOrderField'];
	type OrderDirection = components['schemas']['OrderDirection'];

	let { isReadOnly = false, isActive = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Pagination state (managed by DataControls, updated via callback)
	let pageSize = $state(20);
	let currentPage = $state(1);

	// Ordering state (server-side)
	let groupBy = $state<DiscoveryOrderField | undefined>(undefined);
	let orderBy = $state<DiscoveryOrderField | undefined>(undefined);
	let orderDirection = $state<OrderDirection>('asc');

	// Search state (server-side: the run history is paginated, so a client-side
	// search would only ever match the page in hand)
	let search = $state('');

	// Queries
	const discoveriesQuery = useDiscoveryHistoryQuery(
		(): DiscoveryHistoryQueryParams => ({
			limit: pageSize,
			offset: (currentPage - 1) * pageSize,
			group_by: groupBy,
			order_by: orderBy,
			order_direction: orderDirection,
			search: search || undefined
		}),
		() => isActive
	);
	const daemonsQuery = useDaemonsQuery();
	const networksQuery = useNetworksQuery();

	// Mutations
	const createDiscoveryMutation = useCreateDiscoveryMutation();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();
	const bulkDeleteDiscoveriesMutation = useBulkDeleteDiscoveriesMutation();

	// Derived data
	let discoveriesData = $derived(discoveriesQuery.data?.items ?? []);
	let discoveriesPagination = $derived(discoveriesQuery.data?.pagination ?? null);
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
	// means it stays `isPending` forever. The server already returns only
	// historical rows (`historical: true`), so no client-side filter is needed.
	let isLoading = $derived(discoveriesQuery.isPending || daemonsQuery.isPending);

	// Page change handler for server-side pagination
	function handlePageChange(page: number, newPageSize: number) {
		currentPage = page;
		pageSize = newPageSize;
	}

	// Order change handler for server-side ordering
	function handleOrderChange(
		groupField: string | null,
		orderField: string | null,
		direction: 'asc' | 'desc'
	) {
		groupBy = (groupField as DiscoveryOrderField) ?? undefined;
		orderBy = (orderField as DiscoveryOrderField) ?? undefined;
		orderDirection = direction;
	}

	// Search change handler for server-side search (debounced by DataControls)
	function handleSearchChange(query: string) {
		search = query;
	}

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

	/**
	 * How a run ended, as a tag.
	 *
	 * A clean completion has nothing to say, so it returns null and the column
	 * renders empty — the point is that failures, cancellations and warnings
	 * stand out. Same rule the card's header tag uses.
	 */
	function outcomeTag(discovery: Discovery): { label: string; color: Color } | null {
		const results = discovery.run_type.type === 'Historical' ? discovery.run_type.results : null;
		const phase = results?.phase ?? null;
		if (!phase) return null;

		switch (phase) {
			case 'Complete':
				return results?.warnings && results.warnings.length > 0
					? { label: common_warnings(), color: toColor('yellow') }
					: null;
			case 'Failed':
				return { label: common_failed(), color: toColor('red') };
			case 'Cancelled':
				return { label: common_cancelled(), color: toColor('yellow') };
			default:
				// Still running, so worth showing — the phase names its stage.
				return { label: phase, color: toColor('blue') };
		}
	}

	/** Row actions for table mode, matching what the card offers. */
	function discoveryActions(discovery: Discovery): CardAction[] {
		return [{ label: common_details(), icon: Info, onClick: () => handleEditDiscovery(discovery) }];
	}

	let fields = $derived(
		defineFields<Discovery, DiscoveryOrderField>(
			{
				// Identity field: grouping by it would render a header per run.
				name: { label: common_name(), type: 'string', searchable: true, groupable: false },
				daemon_id: {
					label: common_daemon(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					// Displayed as a name, but grouped by id on the server.
					getGroupValue: (item) => item.daemon_id,
					getValue: (item) =>
						daemonsData.find((d) => d.id === item.daemon_id)?.name ??
						common_unknownEntity({ entity: common_daemon() }),
					display: { getItems: (item) => daemonItems(item.daemon_id, daemonsData) }
				},
				network_id: {
					label: common_network(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getGroupValue: (item) => item.network_id,
					getValue: (item) =>
						networksData.find((n) => n.id === item.network_id)?.name ?? common_unknownNetwork(),
					display: { getItems: (item) => networkItems(item.network_id, networksData) }
				},
				discovery_type: {
					label: common_type(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (item) => item.discovery_type.type
				},
				created_at: { label: common_created(), type: 'date' },
				updated_at: { label: common_updated(), type: 'date' }
			},
			[
				{
					// The run's outcome, which the card shows as its header tag. It was
					// card-only, so a failed or cancelled run looked identical to a
					// clean one in the table.
					key: 'outcome',
					label: common_status(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (item) => outcomeTag(item)?.label ?? '',
					display: {
						statusTag: true,
						getItems: (item) => {
							const tag = outcomeTag(item);
							return tag ? [{ id: tag.label, label: tag.label, color: tag.color }] : [];
						}
					}
				},
				// Derived from the run's JSONB results, so these are display-only:
				// there is no column to sort or group on.
				{
					key: 'started_at',
					label: discovery_startedAt(),
					type: 'string',
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
					getValue: (item) => {
						const results = item.run_type.type == 'Historical' ? item.run_type.results : null;
						if (results && results.finished_at && results.started_at) {
							return formatDuration(results.started_at, results.finished_at);
						}
						return common_unknown();
					}
				}
			]
		)
	);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={discovery_historyTitle()} />

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptDiscoveries()} />
	{:else if isLoading}
		<Loading />
	{:else if discoveriesData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={discovery_noHistorySessions()}
			subtitle={discovery_noHistorySessionsSubtitle()}
		/>
	{:else}
		<DataControls
			items={discoveriesData}
			{fields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			storageKey="scanopy-discovery-historical-table-state"
			getItemId={(item) => item.id}
			serverPagination={discoveriesPagination}
			onPageChange={handlePageChange}
			onOrderChange={handleOrderChange}
			onSearchChange={handleSearchChange}
			onCsvExport={handleCsvExport}
			getActions={discoveryActions}
			entityLabel={discovery_historyTitle()}
		>
			{#snippet children(
				item: Discovery,
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<DiscoveryHistoryCard
					discovery={item}
					hosts={hostsData}
					onView={handleEditDiscovery}
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
