<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import { Plus } from 'lucide-svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import UserApiKeyCard from './UserApiKeyCard.svelte';
	import UserApiKeyModal from './UserApiKeyModal.svelte';
	import {
		useUserApiKeysQuery,
		useUpdateUserApiKeyMutation,
		useDeleteUserApiKeyMutation,
		useBulkDeleteUserApiKeysMutation,
		type UserApiKey
	} from '../queries';
	import type { TabProps } from '$lib/shared/types';

	let { isReadOnly = false }: TabProps = $props();

	// Queries
	const tagsQuery = useTagsQuery();
	const userApiKeysQuery = useUserApiKeysQuery();
	const networksQuery = useNetworksQuery();

	// Mutations
	const updateMutation = useUpdateUserApiKeyMutation();
	const deleteMutation = useDeleteUserApiKeyMutation();
	const bulkDeleteMutation = useBulkDeleteUserApiKeysMutation();

	// Derived data
	let tagsData = $derived(tagsQuery.data ?? []);
	let userApiKeysData = $derived(userApiKeysQuery.data ?? []);
	let networksData = $derived(networksQuery.data ?? []);
	let isLoading = $derived(userApiKeysQuery.isPending);

	let showModal = $state(false);
	let editingApiKey = $state<UserApiKey | null>(null);

	async function handleDelete(apiKey: UserApiKey) {
		if (confirm(`Are you sure you want to delete API key "${apiKey.name}"?`)) {
			deleteMutation.mutate(apiKey.id);
		}
	}

	async function handleUpdate(apiKey: UserApiKey) {
		await updateMutation.mutateAsync(apiKey);
		showModal = false;
		editingApiKey = null;
	}

	function handleCreate() {
		showModal = true;
		editingApiKey = null;
	}

	function handleClose() {
		showModal = false;
		editingApiKey = null;
	}

	function handleEdit(apiKey: UserApiKey) {
		showModal = true;
		editingApiKey = apiKey;
	}

	async function handleBulkDelete(ids: string[]) {
		if (confirm(`Are you sure you want to delete ${ids.length} API keys?`)) {
			await bulkDeleteMutation.mutateAsync(ids);
		}
	}

	function getUserApiKeyTags(apiKey: UserApiKey): string[] {
		return apiKey.tags ?? [];
	}

	const apiKeyFields: FieldConfig<UserApiKey>[] = [
		{
			key: 'name',
			label: 'Name',
			type: 'string',
			searchable: true,
			filterable: false,
			sortable: true
		},
		{
			key: 'permissions',
			type: 'string',
			label: 'Permissions',
			searchable: false,
			filterable: true,
			sortable: true
		},
		{
			key: 'network_ids',
			type: 'array',
			label: 'Networks',
			searchable: false,
			filterable: false,
			sortable: false,
			getValue(item) {
				const ids = item.network_ids ?? [];
				return ids
					.map((id) => networksData.find((n) => n.id === id)?.name)
					.filter((name): name is string => !!name);
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
				return (entity.tags ?? [])
					.map((id) => tagsData.find((t) => t.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		}
	];
</script>

<div class="space-y-6">
	<TabHeader title="API Keys" subtitle="Manage your personal API keys for programmatic access">
		<svelte:fragment slot="actions">
			{#if !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreate}>
					<Plus class="h-5 w-5" />Create API Key
				</button>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if isLoading}
		<Loading />
	{:else if userApiKeysData.length === 0}
		<EmptyState
			title="No API Keys yet"
			subtitle="Create API keys to access the API programmatically"
			onClick={handleCreate}
			cta="Create your first API Key"
		/>
	{:else}
		<DataControls
			items={userApiKeysData}
			fields={apiKeyFields}
			onBulkDelete={isReadOnly ? undefined : handleBulkDelete}
			entityType={isReadOnly ? undefined : 'UserApiKey'}
			getItemTags={getUserApiKeyTags}
			storageKey="scanopy-user-api-keys-table-state"
			getItemId={(item) => item.id}
		>
			{#snippet children(
				item: UserApiKey,
				viewMode: 'card' | 'list',
				isSelected: boolean,
				onSelectionChange: (selected: boolean) => void
			)}
				<UserApiKeyCard
					apiKey={item}
					{viewMode}
					selected={isSelected}
					{onSelectionChange}
					onDelete={isReadOnly ? undefined : handleDelete}
					onEdit={isReadOnly ? undefined : handleEdit}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>

<UserApiKeyModal
	isOpen={showModal}
	onClose={handleClose}
	onUpdate={handleUpdate}
	apiKey={editingApiKey}
/>
