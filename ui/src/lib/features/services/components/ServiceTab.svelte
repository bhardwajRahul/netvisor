<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { defineFields, entityRef, type CardAction } from '$lib/shared/components/data/types';
	import { tagNames } from '$lib/features/tags/columns';
	import { networkItems } from '$lib/features/networks/columns';
	import { entities } from '$lib/shared/stores/metadata';
	import { Trash2, Edit } from 'lucide-svelte';
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
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { SvelteMap } from 'svelte/reactivity';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { closeModal, modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import {
		common_confirmBulkDelete,
		common_confirmDeleteName,
		common_containerized,
		common_created,
		common_delete,
		common_edit,
		common_host,
		common_lastSeen,
		common_category,
		common_name,
		common_network,
		common_noEntityYet,
		common_port,
		common_position,
		common_services,
		common_tags,
		common_type,
		common_unknown,
		common_unknownEntity,
		common_unknownNetwork,
		common_updated,
		daemons_installPromptServices,
		services_matchConfidence,
		services_notContainerized,
		services_notDiscovered,
		services_subtitle
	} from '$lib/paraglide/messages';
	import {
		serviceDefinitions,
		serviceCategories as serviceCategoryMeta,
		concepts,
		ports as ports_metadata
	} from '$lib/shared/stores/metadata';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';

	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type ServiceOrderField = components['schemas']['ServiceOrderField'];
	type OrderDirection = components['schemas']['OrderDirection'];

	// Well-known port numbers for the filter panel. People look for 443, not
	// "HTTPS", so the options are the numbers themselves. Deduplicated because
	// some ports have both a TCP and a UDP definition (53, 515), and stripped of
	// the Custom entry, whose number is a placeholder 0. Free-form port entry
	// would need a control the value-list filter panel doesn't have.
	let wellKnownPortNumbers = $derived(
		[
			...new Set(
				(ports_metadata.getItems() ?? [])
					.filter((port) => !port.metadata.is_custom)
					.map((port) => port.metadata.number)
			)
		]
			.sort((a, b) => a - b)
			.map(String)
	);

	let { isReadOnly = false }: TabProps = $props();

	// Organization query for onboarding state
	const organizationQuery = useOrganizationQuery();
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);

	// Pagination state (managed by DataControls, updated via callback)
	let pageSize = $state(20);
	let currentPage = $state(1);

	// Ordering state (for server-side ordering)
	let groupBy = $state<ServiceOrderField | undefined>(undefined);
	let orderBy = $state<ServiceOrderField | undefined>(undefined);
	let orderDirection = $state<OrderDirection>('asc');

	// Tag filter state (for server-side filtering)
	let tagIds = $state<string[]>([]);

	// Exclude categories state (for server-side filtering)
	let excludeCategories = $state<string[]>(['OpenPorts']);

	// Port filter (server-side: a client-side pass would only narrow the loaded
	// page)
	let ports = $state<number[]>([]);

	// Staleness filter state (server-side: the list is server-paginated)
	let stale = $state<boolean | null>(null);

	// Search state (server-side, for the same reason)
	let search = $state('');

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
			tag_ids: tagIds.length > 0 ? tagIds : undefined,
			stale: stale ?? undefined,
			search: search || undefined,
			ports: ports.length > 0 ? ports : undefined,
			exclude_categories:
				excludeCategories.length > 0
					? (excludeCategories as components['schemas']['ServiceCategory'][])
					: undefined
		})
	);
	const networksQuery = useNetworksQuery();

	// Selective host lookup - only fetches hosts needed for service display
	// Extract host IDs from visible services for host name display
	const hostsQuery = useHostsByIds(() => {
		const ids = (servicesQuery.data?.items ?? [])
			.filter((s) => s.host_id)
			.map((s) => s.host_id)
			.filter((id, idx, arr) => arr.indexOf(id) === idx);
		// Include host for service navigated via EntityTag
		const ms = $modalState;
		if (ms.name === 'service-editor' && ms.entityData?.host_id) {
			const hostId = ms.entityData.host_id as string;
			if (!ids.includes(hostId)) ids.push(hostId);
		}
		return ids;
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

	// Server-side field filter handler. Each key here must match a field marked
	// `serverFiltered`, otherwise DataControls would skip the client-side filter
	// for a value nothing acts on.
	function handleFilterChange(fieldKey: string, values: string[]) {
		if (fieldKey === 'category') {
			excludeCategories = values;
		} else if (fieldKey === 'port') {
			ports = values.map(Number).filter((port) => Number.isFinite(port));
		}
	}

	function handleStaleFilterChange(next: boolean | null) {
		stale = next;
	}

	// Search change handler for server-side search (debounced by DataControls)
	function handleSearchChange(query: string) {
		search = query;
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('Service', {
			tag_ids: tagIds.length > 0 ? tagIds : undefined,
			order_by: orderBy,
			order_direction: orderDirection
		});
	}

	let showServiceEditor = $state(false);
	let editingService = $state<Service | null>(null);

	// Deep-link: open service editor from URL (edit-only, validates host exists)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'service-editor',
			servicesData,
			showServiceEditor,
			editingService?.id,
			(s) => !!serviceHosts.get(s.id)
		);
		if (result) {
			editingService = result;
			showServiceEditor = true;
		}
	});

	/** Row actions for table mode, matching what the card offers. */
	function serviceActions(service: Service): CardAction[] {
		if (isReadOnly) return [];

		return [
			{ label: common_edit(), icon: Edit, onClick: () => handleEditService(service) },
			{
				label: common_delete(),
				icon: Trash2,
				class: 'btn-icon-danger',
				onClick: () => handleDeleteService(service)
			}
		];
	}

	function handleEditService(service: Service) {
		editingService = service;
		showServiceEditor = true;
	}
	function handleCloseServiceEditor() {
		// Clear URL modal state before setting local state to prevent the deep-link
		// effect from re-opening the modal (parent effects run before child effects,
		// so the deep-link effect would see showServiceEditor=false but $modalState
		// still pointing to this service)
		closeModal();
		showServiceEditor = false;
		editingService = null;
	}

	let serviceHosts = $derived(
		(() => {
			const map = new SvelteMap(
				servicesData.map((service) => {
					const foundHost = hostsData.find((h) => h.id == service.host_id);
					return [service.id, foundHost] as [string, (typeof hostsData)[0] | undefined];
				})
			);
			// Include entityData service (EntityTag navigation)
			const ms = $modalState;
			if (ms.name === 'service-editor' && ms.entityData?.id && ms.entityData?.host_id) {
				if (!map.has(ms.entityData.id as string)) {
					const host = hostsData.find((h) => h.id === ms.entityData!.host_id);
					if (host) map.set(ms.entityData.id as string, host);
				}
			}
			return map;
		})()
	);

	function handleDeleteService(service: Service) {
		if (confirm(common_confirmDeleteName({ name: service.name }))) {
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
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_services() }))) {
			await bulkDeleteServicesMutation.mutateAsync(ids);
		}
	}

	function getServiceTags(service: Service): string[] {
		return service.tags;
	}

	// Derive available service categories from metadata
	let serviceCategories = $derived.by(() => {
		const items = serviceDefinitions.getItems() || [];
		const categoriesSet = new Set(items.map((i) => serviceDefinitions.getCategory(i.id)));
		return Array.from(categoriesSet)
			.filter((c) => c)
			.sort();
	});

	// Define field configuration for the DataTableControls
	// Uses defineFields to ensure all ServiceOrderField values are covered
	let serviceFields = $derived(
		defineFields<Service, ServiceOrderField>(
			{
				// Identity field: grouping by it would render a header per service.
				name: {
					label: common_name(),
					type: 'string',
					searchable: true,
					groupable: false,
					display: { primary: true, width: 220 }
				},
				host: {
					label: common_host(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					// The server groups on the host's name, coalescing services
					// with no host to an empty string.
					getGroupValue: (service) => serviceHosts.get(service.id)?.name ?? '',
					getValue: (service) =>
						serviceHosts.get(service.id)?.name || common_unknownEntity({ entity: common_host() }),
					display: {
						getItems: (service) => {
							const host = serviceHosts.get(service.id);
							if (!host) return [];
							return [
								{
									id: host.id,
									label: host.name,
									color: entities.getColorHelper('Host').color,
									entityRef: entityRef('Host', host.id, host)
								}
							];
						}
					}
				},
				network_id: {
					label: common_network(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					// Displayed as a name, but grouped by id on the server.
					getGroupValue: (item) => item.network_id,
					getValue: (item) =>
						networksData.find((n) => n.id == item.network_id)?.name || common_unknownNetwork(),
					display: { getItems: (item) => networkItems(item.network_id, networksData) }
				},
				// Per-service ordinal, so grouping by it is one header per service.
				position: {
					label: common_position(),
					type: 'string',
					groupable: false,
					display: { hiddenByDefault: true, align: 'right' }
				},
				service_definition: {
					label: common_type(),
					type: 'string',
					searchable: true,
					filterable: true,
					groupable: true,
					// The server groups on the raw definition id; the UI renders its
					// friendly name, so the group key has to be supplied separately.
					getGroupValue: (service) => service.service_definition,
					getValue: (service) => serviceDefinitions.getName(service.service_definition),
					display: {
						getItems: (service) => [
							{
								id: service.service_definition,
								label: serviceDefinitions.getName(service.service_definition),
								color: serviceDefinitions.getColorHelper(service.service_definition).color,
								icon: serviceDefinitions.getIconComponent(service.service_definition)
							}
						]
					}
				},
				created_at: { label: common_created(), type: 'date', display: { hiddenByDefault: true } },
				updated_at: { label: common_updated(), type: 'date', display: { hiddenByDefault: true } },
				last_seen_at: { label: common_lastSeen(), type: 'date' }
			},
			[
				{
					key: 'category',
					label: common_category(),
					type: 'string',
					searchable: true,
					filterable: true,
					serverFiltered: true,
					filterMode: 'exclude',
					filterOptions: serviceCategories,
					filterDefaults: ['OpenPorts'],
					getValue: (item) =>
						serviceDefinitions.getCategory(item.service_definition) || common_unknown(),
					display: {
						getItems: (item) => {
							const category = serviceDefinitions.getCategory(item.service_definition);
							if (!category) return [];
							// Categories carry their own colour in the metadata fixture, so
							// use it rather than rendering every category identically grey.
							return [
								{
									id: category,
									label: serviceCategoryMeta.getName(category) || category,
									color: serviceCategoryMeta.getColorHelper(category).color,
									icon: serviceCategoryMeta.getIconComponent(category)
								}
							];
						}
					}
				},
				{
					key: 'containerized_by',
					type: 'string',
					label: common_containerized(),
					searchable: true,
					filterable: true,
					getValue: (item) =>
						servicesData.find((s) => s.id == item.virtualization_service_id)?.name ||
						services_notContainerized(),
					display: {
						// No chip when a service isn't containerized, so the cell shows an
						// em dash rather than repeating the phrase down the column. The
						// phrase stays in `getValue`, so the filter still offers it.
						getItems: (item) => {
							const runtime = servicesData.find((s) => s.id == item.virtualization_service_id);
							if (!runtime) return [];
							return [
								{
									id: runtime.id,
									label: runtime.name,
									color: concepts.getColorHelper('Containerization').color,
									entityRef: entityRef('Service', runtime.id, runtime)
								}
							];
						}
					}
				},
				{
					key: 'confidence',
					label: services_matchConfidence(),
					type: 'string',
					searchable: true,
					filterable: true,
					display: { hiddenByDefault: true },
					getValue: (item) =>
						item.source.type == 'DiscoveryWithMatch'
							? matchConfidenceLabel(item.source.details.confidence)
							: services_notDiscovered()
				},
				{
					key: 'port',
					label: common_port(),
					type: 'string',
					filterable: true,
					serverFiltered: true,
					filterOptions: wellKnownPortNumbers,
					// Drives the port filter only: it has no `getValue`, so as a column
					// it would render an empty cell on every row.
					display: { hidden: true }
				},
				{
					key: 'tags',
					label: common_tags(),
					type: 'array',
					searchable: true,
					filterable: true,
					getValue: (entity) => tagNames(entity.tags, tagsData)
				}
			]
		)
	);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={common_services()} subtitle={services_subtitle()} />

	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title={daemons_installPromptServices()} />
	{:else if isInitialLoading}
		<!-- Loading state (only on initial load) -->
		<Loading />
	{:else if servicesData.length === 0 && !servicesPagination}
		<!-- Empty state -->
		<EmptyState title={common_noEntityYet({ entity: common_services() })} subtitle="" />
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
			onFilterChange={handleFilterChange}
			onStaleFilterChange={handleStaleFilterChange}
			onSearchChange={handleSearchChange}
			onCsvExport={handleCsvExport}
			getActions={serviceActions}
			entityLabel={common_services()}
		>
			{#snippet children(
				item: Service,
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
			name="service-editor"
			service={editingService}
			host={editingServiceHost}
			isOpen={showServiceEditor}
			onUpdate={handleServiceUpdate}
			onClose={handleCloseServiceEditor}
			onDelete={() => {
				handleDeleteService(editingService!);
				handleCloseServiceEditor();
			}}
		/>
	{/if}
{/if}
