<script lang="ts">
	import HostCard from './HostCard.svelte';
	import type {
		Host,
		CreateHostWithServicesRequest,
		UpdateHostWithServicesRequest
	} from '../types/base';
	import { toHostPrimitive } from '../queries';
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import HostEditor from './HostEditModal/HostEditor.svelte';
	import HostConsolidationModal from './HostConsolidationModal.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import type { TabProps } from '$lib/shared/types';
	import * as m from '$lib/paraglide/messages';

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
	import { useServicesByIds } from '$lib/features/services/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import type { components } from '$lib/api/schema';

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

	// Queries
	const tagsQuery = useTagsQuery();
	// Paginated hosts with server-side pagination, ordering, and tag filtering
	const hostsQuery = useHostsQuery(
		(): HostQueryOptions => ({
			limit: pageSize,
			offset: (currentPage - 1) * pageSize,
			group_by: groupBy,
			order_by: orderBy,
			order_direction: orderDirection,
			tag_ids: tagIds.length > 0 ? tagIds : undefined
		})
	);
	const networksQuery = useNetworksQuery();
	useDaemonsQuery();

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
	let networksData = $derived(networksQuery.data ?? []);
	// Only show full loading on initial load (no data yet)
	let isInitialLoading = $derived(hostsQuery.isPending && !hostsQuery.data);

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

	let showHostEditor = $state(false);
	let editingHost = $state<Host | null>(null);

	let otherHost = $state<Host | null>(null);
	let showHostConsolidationModal = $state(false);

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all HostOrderField values are covered
	let hostFields = $derived(
		defineFields<Host, HostOrderField>(
			{
				name: { label: m.hosts_fields_name(), type: 'string', searchable: true },
				hostname: { label: m.hosts_fields_hostname(), type: 'string', searchable: true },
				virtualized_by: {
					label: m.hosts_fields_virtualizedBy(),
					type: 'string',
					filterable: true,
					groupable: true,
					getValue: (host) => {
						if (host.virtualization) {
							const virtualizationService = servicesData.find(
								(s) => s.id === host.virtualization?.details.service_id
							);
							if (virtualizationService) {
								return virtualizationService?.name || m.hosts_unknownService();
							}
						}
						return m.hosts_notVirtualized();
					}
				},
				network_id: {
					label: m.hosts_fields_network(),
					type: 'string',
					filterable: true,
					groupable: true,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || m.hosts_unknownNetwork()
				},
				created_at: { label: m.hosts_fields_created(), type: 'date' },
				updated_at: { label: m.hosts_fields_updated(), type: 'date' }
			},
			[
				{
					key: 'description',
					label: m.hosts_fields_description(),
					type: 'string',
					searchable: true
				},
				{ key: 'hidden', label: m.hosts_fields_hidden(), type: 'boolean', filterable: true },
				{
					key: 'tags',
					label: m.hosts_fields_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (entity) =>
						entity.tags
							.map((id) => tagsData.find((t) => t.id === id)?.name)
							.filter((name): name is string => !!name)
				}
			]
		)
	);

	function handleCreateHost() {
		editingHost = null;
		showHostEditor = true;
	}

	function handleEditHost(host: Host) {
		editingHost = host;
		showHostEditor = true;
	}

	function handleStartConsolidate(host: Host) {
		otherHost = host;
		showHostConsolidationModal = true;
	}

	function handleDeleteHost(host: Host) {
		if (confirm(m.hosts_confirmDelete({ name: host.name }))) {
			deleteHostMutation.mutate(host.id);
		}
	}

	async function handleHostCreate(data: CreateHostWithServicesRequest) {
		try {
			await createHostMutation.mutateAsync(data);
			showHostEditor = false;
			editingHost = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleHostCreateAndContinue(data: CreateHostWithServicesRequest) {
		try {
			const result = await createHostMutation.mutateAsync(data);
			// Keep modal open and switch to edit mode with the created host
			// Extract Host primitive from HostResponse
			editingHost = toHostPrimitive(result);
		} catch {
			// Error handled by mutation
		}
	}

	async function handleHostUpdate(data: UpdateHostWithServicesRequest) {
		try {
			await updateHostMutation.mutateAsync(data);
			showHostEditor = false;
			editingHost = null;
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
		if (confirm(m.hosts_confirmBulkDelete({ count: ids.length }))) {
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
			interfaces: null,
			ports: null,
			services: null
		});
	}

	function handleCloseHostEditor() {
		showHostEditor = false;
		editingHost = null;
	}
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={m.hosts_title()}>
		<svelte:fragment slot="actions">
			{#if !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateHost}
					><Plus class="h-5 w-5" />{m.hosts_createHost()}</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	<!-- Loading state (only on initial load) -->
	{#if isInitialLoading}
		<Loading />
	{:else if hostsData.length === 0 && !hostsPagination}
		<!-- Empty state -->
		<EmptyState
			title={m.hosts_noHostsYet()}
			subtitle=""
			onClick={handleCreateHost}
			cta={m.hosts_createFirstHost()}
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
	host={editingHost}
	onCreate={handleHostCreate}
	onCreateAndContinue={handleHostCreateAndContinue}
	onUpdate={handleHostUpdate}
	onClose={handleCloseHostEditor}
/>

<HostConsolidationModal
	isOpen={showHostConsolidationModal}
	{otherHost}
	onConsolidate={handleConsolidateHosts}
	onClose={() => (showHostConsolidationModal = false)}
/>
