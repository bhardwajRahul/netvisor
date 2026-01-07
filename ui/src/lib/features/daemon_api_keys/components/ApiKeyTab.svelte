<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import CreateApiKeyModal from './ApiKeyModal.svelte';
	import type { ApiKey } from '../types/base';
	import ApiKeyCard from './ApiKeyCard.svelte';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		useApiKeysQuery,
		useUpdateApiKeyMutation,
		useDeleteApiKeyMutation,
		useBulkDeleteApiKeysMutation
	} from '../queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import type { TabProps } from '$lib/shared/types';

	let { isReadOnly = false }: TabProps = $props();

	// Queries
	const tagsQuery = useTagsQuery();
	const apiKeysQuery = useApiKeysQuery();
	const networksQuery = useNetworksQuery();
	// Daemons query to ensure data is loaded (needed for API key display)
	useDaemonsQuery();

	// Mutations
	const updateApiKeyMutation = useUpdateApiKeyMutation();
	const deleteApiKeyMutation = useDeleteApiKeyMutation();
	const bulkDeleteApiKeysMutation = useBulkDeleteApiKeysMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let apiKeysData = $derived(apiKeysQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(apiKeysQuery.isPending);

	let showCreateApiKeyModal = $state(false);
	let editingApiKey = $state<ApiKey | null>(null);

	async function handleDeleteApiKey(apiKey: ApiKey) {
		if (confirm(`Are you sure you want to delete api key "${apiKey.name}"?`)) {
			deleteApiKeyMutation.mutate(apiKey.id);
		}
	}

	async function handleUpdateApiKey(apiKey: ApiKey) {
		await updateApiKeyMutation.mutateAsync(apiKey);
		showCreateApiKeyModal = false;
		editingApiKey = null;
	}

	function handleCreateApiKey() {
		showCreateApiKeyModal = true;
		editingApiKey = null;
	}

	function handleCloseCreateApiKey() {
		showCreateApiKeyModal = false;
		editingApiKey = null;
	}

	function handleEditApiKey(apiKey: ApiKey) {
		showCreateApiKeyModal = true;
		editingApiKey = apiKey;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(`Are you sure you want to delete ${ids.length} Api Keys?`)) {
			await bulkDeleteApiKeysMutation.mutateAsync(ids);
		}
	}

	function getApiKeyTags(apiKey: ApiKey): string[] {
		return apiKey.tags;
	}

	const apiKeyFields: FieldConfig<ApiKey>[] = [
		{
			key: 'name',
			label: 'Name',
			type: 'string',
			searchable: true,
			filterable: false,
			sortable: true
		},
		{
			key: 'network_id',
			type: 'string',
			label: 'Network',
			searchable: false,
			filterable: true,
			sortable: false,
			getValue(item) {
				return networksData.find((n) => n.id == item.network_id)?.name || 'Unknown Network';
			}
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
	];
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title="Daemon API Keys">
		<svelte:fragment slot="actions">
			{#if !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreateApiKey}
					><Plus class="h-5 w-5" />Create Daemon API Key</button
				>
			{/if}
		</svelte:fragment>
	</TabHeader>
	<!-- Loading state -->
	{#if isLoading}
		<Loading />
	{:else if apiKeysData.length === 0}
		<!-- Empty state -->
		<EmptyState
			title="No Daemon API Keys configured yet"
			subtitle=""
			onClick={handleCreateApiKey}
			cta="Create your first Daemon API Key"
		/>
	{:else}
		<DataControls
			items={apiKeysData}
			fields={apiKeyFields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'DaemonApiKey'}
			getItemTags={getApiKeyTags}
			storageKey="scanopy-api-keys-table-state"
			getItemId={(item) => item.id}
		>
			{#snippet children(
				item: ApiKey,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<ApiKeyCard
					apiKey={item}
					{viewMode}
					selected={isSelected}
					{onSelectionChange}
					onDelete={isReadOnly ? undefined : handleDeleteApiKey}
					onEdit={isReadOnly ? undefined : handleEditApiKey}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<CreateApiKeyModal
	isOpen={showCreateApiKeyModal}
	onClose={handleCloseCreateApiKey}
	onUpdate={handleUpdateApiKey}
	apiKey={editingApiKey}
/>
