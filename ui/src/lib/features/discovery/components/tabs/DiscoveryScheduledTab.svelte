<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
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
		useInitiateDiscoveryMutation
	} from '../../queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useHostsQuery } from '$lib/features/hosts/queries';
	import type { TabProps } from '$lib/shared/types';

	let { isReadOnly = false }: TabProps = $props();

	// Queries
	const tagsQuery = useTagsQuery();
	const discoveriesQuery = useDiscoveriesQuery();
	const daemonsQuery = useDaemonsQuery();
	// Use limit: 0 to get all hosts for modal dropdown
	const hostsQuery = useHostsQuery({ limit: 0 });

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
	let hostsData = $derived(hostsQuery.data?.items ?? []);
	let isLoading = $derived(
		discoveriesQuery.isPending || daemonsQuery.isPending || hostsQuery.isPending
	);

	let showDiscoveryModal = $state(false);
	let editingDiscovery: Discovery | null = $state(null);

	function handleCreateDiscovery() {
		editingDiscovery = null;
		showDiscoveryModal = true;
	}

	function handleEditDiscovery(discovery: Discovery) {
		editingDiscovery = discovery;
		showDiscoveryModal = true;
	}

	function handleDeleteDiscovery(discovery: Discovery) {
		if (confirm(`Are you sure you want to delete "${discovery.name}"?`)) {
			deleteDiscoveryMutation.mutate(discovery.id);
		}
	}

	function handleDiscoveryRun(discovery: Discovery) {
		initiateDiscoveryMutation.mutate(discovery.id);
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
		if (confirm(`Are you sure you want to delete ${ids.length} Scheduled Discoveries?`)) {
			await bulkDeleteDiscoveriesMutation.mutateAsync(ids);
		}
	}

	let fields: FieldConfig<Discovery>[] = $derived([
		...discoveryFields(daemonsData),
		{
			key: 'run_type',
			label: 'Run Type',
			type: 'string',
			searchable: true,
			filterable: true,
			sortable: true,
			getValue: (item) => item.run_type.type
		},
		{
			key: 'tags',
			label: 'Tags',
			type: 'array',
			searchable: true,
			filterable: true,
			sortable: false,
			getValue: (entity) => {
				// Return tag names for search/filter display
				return entity.tags
					.map((id) => tagsData.find((t) => t.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		}
	]);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title="Scheduled Discovery Sessions">
		<svelte:fragment slot="actions">
			{#if !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateDiscovery}
					><Plus class="h-5 w-5" />Schedule Discovery</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if isLoading}
		<Loading />
	{:else if discoveriesData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title="No discovery sessions are scheduled"
			subtitle=""
			onClick={isReadOnly ? undefined : handleCreateDiscovery}
			cta={isReadOnly ? undefined : 'Schedule a discovery session'}
		/>
	{:else}
		<DataControls
			items={discoveriesData.filter(
				(d) => d.run_type.type == 'AdHoc' || d.run_type.type == 'Scheduled'
			)}
			{fields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			storageKey="scanopy-discovery-scheduled-table-state"
			getItemId={(item) => item.id}
			entityType={isReadOnly ? undefined : 'Discovery'}
			getItemTags={(item) => item.tags}
		>
			{#snippet children(
				item: Discovery,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<DiscoveryRunCard
					discovery={item}
					selected={isSelected}
					{onSelectionChange}
					onDelete={isReadOnly ? undefined : handleDeleteDiscovery}
					onEdit={isReadOnly ? undefined : handleEditDiscovery}
					onRun={isReadOnly ? undefined : handleDiscoveryRun}
					{viewMode}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<DiscoveryEditModal
	isOpen={showDiscoveryModal}
	daemons={daemonsData}
	hosts={hostsData}
	discovery={editingDiscovery}
	onCreate={handleDiscoveryCreate}
	onUpdate={handleDiscoveryUpdate}
	onClose={handleCloseEditor}
/>
