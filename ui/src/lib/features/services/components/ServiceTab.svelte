<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields } from '$lib/shared/components/data/types';
	import type { Service } from '../types/base';
	import ServiceCard from './ServiceCard.svelte';
	import { matchConfidenceLabel } from '$lib/shared/types';
	import ServiceEditModal from './ServiceEditModal.svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useServicesQuery,
		useUpdateServiceMutation,
		useDeleteServiceMutation,
		useBulkDeleteServicesMutation,
		type ServicesQueryParams
	} from '../queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import * as m from '$lib/paraglide/messages';

	type ServiceOrderField = components['schemas']['ServiceOrderField'];
	type OrderDirection = components['schemas']['OrderDirection'];

	let { isReadOnly = false }: TabProps = $props();

	// Pagination state (managed by DataControls, updated via callback)
	let pageSize = $state(20);
	let currentPage = $state(1);

	// Ordering state (for server-side ordering)
	let groupBy = $state<ServiceOrderField | undefined>(undefined);
	let orderBy = $state<ServiceOrderField | undefined>(undefined);
	let orderDirection = $state<OrderDirection>('asc');

	// Tag filter state (for server-side filtering)
	let tagIds = $state<string[]>([]);

	// Queries
	const tagsQuery = useTagsQuery();
	// Paginated services with server-side pagination, ordering, and tag filtering
	const servicesQuery = useServicesQuery(
		(): ServicesQueryParams => ({
			limit: pageSize,
			offset: (currentPage - 1) * pageSize,
			group_by: groupBy,
			order_by: orderBy,
			order_direction: orderDirection,
			tag_ids: tagIds.length > 0 ? tagIds : undefined
		})
	);
	const networksQuery = useNetworksQuery();

	// Selective host lookup - only fetches hosts needed for service display
	// Extract host IDs from visible services for host name display
	const hostsQuery = useHostsByIds(() => {
		return (servicesQuery.data?.items ?? [])
			.filter((s) => s.host_id)
			.map((s) => s.host_id)
			.filter((id, idx, arr) => arr.indexOf(id) === idx);
	});

	// Mutations
	const updateServiceMutation = useUpdateServiceMutation();
	const deleteServiceMutation = useDeleteServiceMutation();
	const bulkDeleteServicesMutation = useBulkDeleteServicesMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let servicesData = $derived(servicesQuery.data?.items ?? []);
	let servicesPagination = $derived(servicesQuery.data?.pagination ?? null);
	let hostsData = $derived(hostsQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	// Only show full loading on initial load (no data yet)
	let isInitialLoading = $derived(servicesQuery.isPending && !servicesQuery.data);

	// Page change handler for server-side pagination
	function handlePageChange(page: number, newPageSize: number) {
		currentPage = page;
		pageSize = newPageSize;
	}

	// Order change handler for server-side ordering
	// Values are now directly ServiceOrderField values from the orderField property
	function handleOrderChange(
		groupField: string | null,
		orderField: string | null,
		direction: 'asc' | 'desc'
	) {
		groupBy = (groupField as ServiceOrderField) ?? undefined;
		orderBy = (orderField as ServiceOrderField) ?? undefined;
		orderDirection = direction;
	}

	// Tag filter change handler for server-side filtering
	function handleTagFilterChange(selectedTagIds: string[]) {
		tagIds = selectedTagIds;
		// Reset to page 1 is handled by DataControls
	}

	let showServiceEditor = $state(false);
	let editingService = $state<Service | null>(null);

	function handleEditService(service: Service) {
		editingService = service;
		showServiceEditor = true;
	}
	function handleCloseServiceEditor() {
		showServiceEditor = false;
		editingService = null;
	}

	let serviceHosts = $derived(
		new Map(
			servicesData.map((service) => {
				const foundHost = hostsData.find((h) => {
					return h.id == service.host_id;
				});

				return [service.id, foundHost];
			})
		)
	);

	function handleDeleteService(service: Service) {
		if (confirm(m.services_confirmDelete({ serviceName: service.name }))) {
			deleteServiceMutation.mutate(service.id);
		}
	}

	async function handleServiceUpdate(id: string, data: Service) {
		try {
			await updateServiceMutation.mutateAsync(data);
			showServiceEditor = false;
			editingService = null;
		} catch {
			// Error handled by mutation
		}
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(m.services_confirmBulkDelete({ count: ids.length }))) {
			await bulkDeleteServicesMutation.mutateAsync(ids);
		}
	}

	function getServiceTags(service: Service): string[] {
		return service.tags;
	}

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all ServiceOrderField values are covered
	let serviceFields = $derived(
		defineFields<Service, ServiceOrderField>(
			{
				name: { label: 'Name', type: 'string', searchable: true },
				host: {
					label: 'Host',
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					getValue: (service) => serviceHosts.get(service.id)?.name || 'Unknown Host'
				},
				network_id: {
					label: 'Network',
					type: 'string',
					filterable: true,
					groupable: true,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || 'Unknown Network'
				},
				position: { label: 'Position', type: 'string' },
				created_at: { label: 'Created', type: 'date' },
				updated_at: { label: 'Updated', type: 'date' }
			},
			[
				{
					key: 'containerized_by',
					type: 'string',
					label: 'Containerized',
					searchable: true,
					filterable: true,
					getValue: (item) =>
						servicesData.find((s) => s.id == item.virtualization?.details.service_id)?.name ||
						'Not Containerized'
				},
				{
					key: 'confidence',
					label: 'Match Confidence',
					type: 'string',
					filterable: true,
					getValue: (item) =>
						item.source.type == 'DiscoveryWithMatch'
							? matchConfidenceLabel(item.source.details.confidence)
							: 'N/A (Not a discovered service)'
				},
				{
					key: 'tags',
					label: 'Tags',
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
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={m.services_title()} subtitle={m.services_subtitle()} />

	<!-- Loading state (only on initial load) -->
	{#if isInitialLoading}
		<Loading />
	{:else if servicesData.length === 0 && !servicesPagination}
		<!-- Empty state -->
		<EmptyState title={m.services_noServicesYet()} subtitle="" />
	{:else}
		<DataControls
			items={servicesData}
			fields={serviceFields}
			storageKey="scanopy-services-table-state"
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'Service'}
			getItemTags={getServiceTags}
			getItemId={(item) => item.id}
			serverPagination={servicesPagination}
			onPageChange={handlePageChange}
			onOrderChange={handleOrderChange}
			onTagFilterChange={handleTagFilterChange}
		>
			{#snippet children(
				item: Service,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				{@const host = serviceHosts.get(item.id)}
				{#if host}
					<ServiceCard
						service={item}
						selected={isSelected}
						{host}
						{onSelectionChange}
						{viewMode}
						onDelete={isReadOnly ? undefined : handleDeleteService}
						onEdit={isReadOnly ? undefined : handleEditService}
					/>
				{/if}
			{/snippet}
		</DataControls>
	{/if}
</div>

{#if editingService}
	{@const editingServiceHost = serviceHosts.get(editingService.id)}
	{#if editingServiceHost}
		<ServiceEditModal
			service={editingService}
			host={editingServiceHost}
			isOpen={showServiceEditor}
			onUpdate={handleServiceUpdate}
			onClose={handleCloseServiceEditor}
		/>
	{/if}
{/if}
