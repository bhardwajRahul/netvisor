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
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import {
		common_apiKeys,
		common_create,
		common_name,
		common_networks,
		common_permissions,
		common_tags,
		userApiKeys_confirmBulkDelete,
		userApiKeys_confirmDelete,
		userApiKeys_noApiKeysSubtitle,
		userApiKeys_noApiKeysYet,
		userApiKeys_subtitle
	} from '$lib/paraglide/messages';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';

	let { isReadOnly = false }: TabProps = $props();

	// Check if plan has api_access feature before querying
	const organizationQuery = useOrganizationQuery();
	let hasApiAccess = $derived.by(() => {
		const org = organizationQuery.data;
		if (!org?.plan) return false;
		return billingPlans.getMetadata(org.plan.type).features.api_access;
	});

	// Queries
	const tagsQuery = useTagsQuery();
	const userApiKeysQuery = useUserApiKeysQuery({ enabled: () => hasApiAccess });
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
		if (confirm(userApiKeys_confirmDelete({ name: apiKey.name }))) {
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
		if (confirm(userApiKeys_confirmBulkDelete({ count: ids.length }))) {
			await bulkDeleteMutation.mutateAsync(ids);
		}
	}

	function getUserApiKeyTags(apiKey: UserApiKey): string[] {
		return apiKey.tags ?? [];
	}

	// CSV export handler
	async function handleCsvExport() {
		await downloadCsv('UserApiKey', {});
	}

	const apiKeyFields: FieldConfig<UserApiKey>[] = [
		{
			key: 'name',
			label: common_name(),
			type: 'string',
			searchable: true
		},
		{
			key: 'permissions',
			type: 'string',
			label: common_permissions(),
			filterable: true
		},
		{
			key: 'network_ids',
			type: 'array',
			label: common_networks(),
			getValue(item) {
				const ids = item.network_ids ?? [];
				return ids
					.map((id) => networksData.find((n) => n.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		},
		{
			key: 'tags',
			label: common_tags(),
			type: 'array',
			searchable: true,
			filterable: true,
			getValue: (entity) => {
				return (entity.tags ?? [])
					.map((id) => tagsData.find((t) => t.id === id)?.name)
					.filter((name): name is string => !!name);
			}
		}
	];
</script>

<div class="space-y-6">
	<TabHeader title={common_apiKeys()} subtitle={userApiKeys_subtitle()}>
		<svelte:fragment slot="actions">
			<InlineSuccess
				title="Share your integration with the community!"
				dismissableKey="share-integration"
				body="Creating an integration that you think others might benefit from? Scanopy will be adding an integration library in an upcoming release. Go to the <a class='underline hover:no-underline' target='_blank' href='https://github.com/scanopy/integrations'>Scanopy integrations GitHub</a> and create a PR to get started."
			></InlineSuccess>
			{#if !isReadOnly}
				<button class="btn-primary flex items-center" onclick={handleCreate}>
					<Plus class="h-5 w-5" />{common_create()}
				</button>
			{/if}
		</svelte:fragment>
	</TabHeader>

	{#if !hasApiAccess}
		<EmptyState
			title="API Access Not Available"
			subtitle="Your current plan does not include API access. Upgrade to a plan with API access to create and manage API keys."
		/>
	{:else if isLoading}
		<Loading />
	{:else if userApiKeysData.length === 0}
		<EmptyState
			title={userApiKeys_noApiKeysYet()}
			subtitle={userApiKeys_noApiKeysSubtitle()}
			onClick={handleCreate}
			cta={common_create()}
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
			onCsvExport={handleCsvExport}
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
