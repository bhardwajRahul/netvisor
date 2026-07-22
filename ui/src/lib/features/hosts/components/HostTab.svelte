<script lang="ts">
	import HostCard from './HostCard.svelte';
	import type {
		Host,
		CreateHostWithServicesRequest,
		UpdateHostWithServicesRequest
	} from '../types/base';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import HostEditor from './HostEditModal/HostEditor.svelte';
	import HostConsolidationModal from './HostConsolidationModal.svelte';
	import HostExportModal from './HostExportModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import type { TabProps } from '$lib/shared/types';
	import {
		common_confirmDeleteName,
		common_create,
		common_created,
		common_description,
		common_hidden,
		common_hostname,
		common_hosts,
		common_lastSeen,
		common_confirmBulkDelete,
		common_name,
		common_network,
		common_noEntityYet,
		common_service,
		common_services,
		common_tags,
		common_unknownEntity,
		common_unknownNetwork,
		common_updated,
		daemons_installPromptHosts,
		hosts_fields_interfaceIp,
		hosts_fields_virtualizedBy,
		hosts_notVirtualized
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();
	import {
		useHostsQuery,
		useCreateHostMutation,
		useUpdateHostMutation,
		useDeleteHostMutation,
		useBulkDeleteHostsMutation,
		useConsolidateHostsMutation,
		type HostQueryOptions
	} from '../queries';
	import { useServicesByIds, useServicesCacheQuery } from '$lib/features/services/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import type { components } from '$lib/api/schema';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type HostOrderField = components['schemas']['HostOrderField'];
	type OrderDirection = components['schemas']['OrderDirection'];

	// Pagination state
	let pageSize = $state(20);
	let currentPage = $state(1);

	// Ordering state (for server-side ordering)
	let groupBy = $state<HostOrderField | undefined>(undefined);
	let orderBy = $state<HostOrderField | undefined>(undefined);
	let orderDirection = $state<OrderDirection>('asc');

	// Tag filter state (for server-side filtering)
	let tagIds = $state<string[]>([]);
	// Staleness filter state (server-side: the list is server-paginated)
	let stale = $state<boolean | null>(null);

	// Queries
	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let hostLimit = $derived(org?.plan?.included_hosts ?? null);
	let canBuyMoreHosts = $derived(
		org?.plan?.host_cents !== undefined && org?.plan?.host_cents !== null
	);
	let onboarding = $derived((org?.onboarding ?? []) as OnboardingOperation[]);

	const tagsQuery = useTagsQuery();
	// Paginated hosts with server-side pagination, ordering, and tag filtering
	const hostsQuery = useHostsQuery(
		(): HostQueryOptions => ({
			limit: pageSize,
			offset: (currentPage - 1) * pageSize,
			group_by: groupBy,
			order_by: orderBy,
			order_direction: orderDirection,
			tag_ids: tagIds.length > 0 ? tagIds : undefined,
			stale: stale ?? undefined
		})
	);
	const networksQuery = useNetworksQuery();
	useDaemonsQuery();
	const ipAddressesQuery = useIPAddressesQuery();

	// Selective service lookup - only fetches services needed for virtualization display
	// Extract service IDs from visible hosts for "Virtualized By" field
	const servicesQuery = useServicesByIds(() => {
		return (hostsQuery.data?.items ?? [])
			.filter((h) => h.virtualization?.details.service_id)
			.map((h) => h.virtualization!.details.service_id)
			.filter((id, idx, arr) => arr.indexOf(id) === idx);
	});

	// Mutations
	const createHostMutation = useCreateHostMutation();
	const updateHostMutation = useUpdateHostMutation();
	const deleteHostMutation = useDeleteHostMutation();
	const bulkDeleteHostsMutation = useBulkDeleteHostsMutation();
	const consolidateHostsMutation = useConsolidateHostsMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let hostsPagination = $derived(hostsQuery.data?.pagination ?? null);
	let servicesData = $derived(servicesQuery.data ?? []);
	const servicesCacheQuery = useServicesCacheQuery();
	let allServicesData = $derived(servicesCacheQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let ipAddressesData = $derived(ipAddressesQuery.data ?? []);
	// Only show full loading on initial load (no data yet)
	let isInitialLoading = $derived(hostsQuery.isPending && !hostsQuery.data);

	// Host limit tracking
	let totalHostCount = $derived(hostsPagination?.total_count ?? hostsData.length);
	let isAtHostLimit = $derived(
		hostLimit !== null && totalHostCount >= hostLimit && !canBuyMoreHosts
	);
	let isNearHostLimit = $derived(
		hostLimit !== null &&
			totalHostCount >= hostLimit - 5 &&
			totalHostCount < hostLimit &&
			!canBuyMoreHosts
	);

	// Page change handler for server-side pagination
	function handlePageChange(page: number, newPageSize: number) {
		currentPage = page;
		pageSize = newPageSize;
	}

	// Order change handler for server-side ordering
	// Values are now directly HostOrderField values from the orderField property
	function handleOrderChange(
		groupField: string | null,
		orderField: string | null,
		direction: 'asc' | 'desc'
	) {
		groupBy = (groupField as HostOrderField) ?? undefined;
		orderBy = (orderField as HostOrderField) ?? undefined;
		orderDirection = direction;
	}

	// Tag filter change handler for server-side filtering
	function handleTagFilterChange(selectedTagIds: string[]) {
		tagIds = selectedTagIds;
		// Reset to page 1 is handled by DataControls
	}

	function handleStaleFilterChange(next: boolean | null) {
		stale = next;
	}

	// Export modal state
	let showExportModal = $state(false);
	let exportParams = $derived({
		tag_ids: tagIds.length > 0 ? tagIds : undefined,
		order_by: orderBy,
		order_direction: orderDirection
	});

	let showHostEditor = $state(false);
	// Track the host being edited by id + a snapshot, and resolve `editingHost` from the
	// live query cache. So when an external change (e.g. a credential assignment removed
	// elsewhere) invalidates and refetches hosts, the open editor reflects it without a
	// page reload. The snapshot is the fallback for hosts not in the current page.
	let editingHostId = $state<string | null>(null);
	let editingHostSnapshot = $state<Host | null>(null);
	let editingHost = $derived(
		editingHostId ? (hostsData.find((h) => h.id === editingHostId) ?? editingHostSnapshot) : null
	);
	function setEditingHost(host: Host | null) {
		editingHostId = host?.id ?? null;
		editingHostSnapshot = host;
	}

	let otherHost = $state<Host | null>(null);
	let showHostConsolidationModal = $state(false);

	// Deep-link: open host editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'host-editor',
			hostsData,
			showHostEditor,
			editingHost?.id
		);
		if (result !== undefined) {
			setEditingHost(result);
			showHostEditor = true;
		}
	});

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all HostOrderField values are covered
	let hostFields = $derived(
		defineFields<Host, HostOrderField>(
			{
				name: { label: common_name(), type: 'string', searchable: true },
				hostname: { label: common_hostname(), type: 'string', searchable: true },
				virtualized_by: {
					label: hosts_fields_virtualizedBy(),
					type: 'string',
					filterable: true,
					groupable: true,
					getValue: (host) => {
						if (host.virtualization) {
							const virtualizationService = servicesData.find(
								(s) => s.id === host.virtualization?.details.service_id
							);
							if (virtualizationService) {
								return (
									virtualizationService?.name || common_unknownEntity({ entity: common_service() })
								);
							}
						}
						return hosts_notVirtualized();
					}
				},
				interface_ip: {
					label: hosts_fields_interfaceIp(),
					type: 'string',
					getValue: (host) => {
						const iface = ipAddressesData
							.filter((i) => i.host_id === host.id)
							.sort((a, b) => (a.position ?? 0) - (b.position ?? 0))[0];
						return iface?.ip_address ?? '';
					}
				},
				network_id: {
					label: common_network(),
					type: 'string',
					filterable: true,
					groupable: true,
					filterOptions: networksData.map((n) => n.name),
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || common_unknownNetwork()
				},
				created_at: { label: common_created(), type: 'date' },
				updated_at: { label: common_updated(), type: 'date' },
				last_seen_at: { label: common_lastSeen(), type: 'date' }
			},
			[
				{
					key: 'description',
					label: common_description(),
					type: 'string',
					searchable: true
				},
				{ key: 'hidden', label: common_hidden(), type: 'boolean', filterable: true },
				{
					key: 'tags',
					label: common_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (entity) =>
						entity.tags
							.map((id) => tagsData.find((t) => t.id === id)?.name)
							.filter((name): name is string => !!name)
				},
				{
					key: 'services',
					label: common_services(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (host) =>
						allServicesData.filter((s) => s.host_id === host.id).map((s) => s.name)
				}
			]
		)
	);

	function handleCreateHost() {
		setEditingHost(null);
		showHostEditor = true;
	}

	function handleEditHost(host: Host) {
		setEditingHost(host);
		showHostEditor = true;
	}

	function handleStartConsolidate(host: Host) {
		otherHost = host;
		showHostConsolidationModal = true;
	}

	function handleDeleteHost(host: Host) {
		if (confirm(common_confirmDeleteName({ name: host.name }))) {
			deleteHostMutation.mutate(host.id);
		}
	}

	async function handleHostCreate(data: CreateHostWithServicesRequest) {
		try {
			await createHostMutation.mutateAsync(data);
			showHostEditor = false;
			setEditingHost(null);
		} catch {
			// Error handled by mutation
		}
	}

	async function handleHostUpdate(data: UpdateHostWithServicesRequest) {
		try {
			await updateHostMutation.mutateAsync(data);
			showHostEditor = false;
			setEditingHost(null);
		} catch {
			// Error handled by mutation
		}
	}

	async function handleConsolidateHosts(destinationHostId: string, otherHostId: string) {
		try {
			await consolidateHostsMutation.mutateAsync({
				destinationHostId,
				otherHostId,
				otherHostName: otherHost?.name
			});
			showHostConsolidationModal = false;
			otherHost = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_hosts() }))) {
			await bulkDeleteHostsMutation.mutateAsync(ids);
		}
	}

	function getHostTags(host: Host): string[] {
		return host.tags;
	}

	async function handleHostHide(host: Host) {
		const updatedHost = { ...host, hidden: !host.hidden };
		await updateHostMutation.mutateAsync({
			host: updatedHost,
			ip_addresses: null,
			ports: null,
			services: null
		});
	}

	function handleCloseHostEditor() {
		showHostEditor = false;
		setEditingHost(null);
	}
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_hosts()}>
		<svelte:fragment slot="actions">
			{#if hasDaemon(onboarding)}
				<div class="flex items-center gap-3">
					{#if hostLimit !== null && !canBuyMoreHosts}
						<span
							class="text-sm {isAtHostLimit
								? 'text-amber-400'
								: isNearHostLimit
									? 'text-yellow-400'
									: 'text-tertiary'}"
						>
							{totalHostCount} / {hostLimit}
						</span>
					{/if}
					{#if !isReadOnly}
						{#if isAtHostLimit}
							<UpgradeButton feature="hosts" surface="hosts_tab" gate_type="limit_hit" />
						{:else}
							{#if isNearHostLimit}
								<UpgradeButton feature="hosts" surface="hosts_tab" gate_type="limit_hit" />
							{/if}
							<button class="btn-primary flex items-center" onclick={handleCreateHost}
								><Plus class="h-5 w-5" />{common_create()}</button
							>
						{/if}
					{/if}
				</div>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptHosts()} />
	{:else if isInitialLoading}
		<!-- Loading state (only on initial load) -->
		<Loading />
	{:else if hostsData.length === 0 && !hostsPagination}
		<!-- Empty state -->
		<EmptyState
			title={common_noEntityYet({ entity: common_hosts() })}
			subtitle=""
			onClick={handleCreateHost}
			cta={common_create()}
		/>
	{:else}
		<DataControls
			items={hostsData}
			fields={hostFields}
			storageKey="scanopy-hosts-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Host'}
			getItemTags={getHostTags}
			getItemId={(item) => item.id}
			serverPagination={hostsPagination}
			onPageChange={handlePageChange}
			onOrderChange={handleOrderChange}
			onTagFilterChange={handleTagFilterChange}
			onStaleFilterChange={handleStaleFilterChange}
			onExportClick={() => {
				showExportModal = true;
			}}
		>
			{#snippet children(
				item: Host,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<HostCard
					host={item}
					{viewMode}
					selected={isSelected}
					{onSelectionChange}
					onEdit={isReadOnly ? undefined : handleEditHost}
					onDelete={isReadOnly ? undefined : handleDeleteHost}
					onConsolidate={isReadOnly ? undefined : handleStartConsolidate}
					onHide={isReadOnly ? undefined : handleHostHide}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<HostEditor
	isOpen={showHostEditor}
	name="host-editor"
	host={editingHost}
	onCreate={handleHostCreate}
	onDelete={handleDeleteHost}
	onUpdate={handleHostUpdate}
	onClose={handleCloseHostEditor}
/>

<HostConsolidationModal
	isOpen={showHostConsolidationModal}
	{otherHost}
	onConsolidate={handleConsolidateHosts}
	onClose={() => (showHostConsolidationModal = false)}
/>

<HostExportModal
	isOpen={showExportModal}
	onClose={() => (showExportModal = false)}
	{exportParams}
/>
